use std::{borrow::Cow, sync::Arc};
use tokio::sync::Mutex;

use memvid_core::{Memvid, SearchRequest};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, InitializeRequestParam, InitializeResult, ServerCapabilities,
        ServerInfo,
    },
    service::RequestContext,
    tool, tool_router, transport,
};
use rmcp::{ServiceExt, tool_handler};
use tracing::{info, subscriber::set_global_default};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt}; // provides .serve(...)

fn setup_tracing() -> eyre::Result<()> {
    let crate_name = env!("CARGO_CRATE_NAME");
    let crate_version = env!("CARGO_PKG_VERSION");

    LogTracer::init()?;

    let default_filter = format!("info,{}=debug,tokio=info", crate_name);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    let subscriber = Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false), // <- important for stdio MCP
        )
        .with(env_filter);

    set_global_default(subscriber)?;

    info!("[IDATE-EMU] {} v{}", crate_name, crate_version);
    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    setup_tracing()?;

    let transport = transport::stdio();

    let memvid = Memvid::create("memvid.mv2")?;
    let memvid = Arc::new(Mutex::new(memvid));
    let service = MemvidService::new(memvid);

    let service = service
        .serve(transport)
        .await
        .map_err(|e| eyre::eyre!("{e}"))?;

    service.waiting().await?;

    Ok(())
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddChunkRequest {
    pub chunks: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SearchToolRequest {
    pub query: String,
    pub top_k: usize,
    pub snippet_chars: usize,
}

#[derive(Clone)]
struct MemvidService {
    memvid: Arc<Mutex<Memvid>>,
    tool_router: ToolRouter<MemvidService>,
}

fn internal(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

fn invalid_params(msg: impl Into<String>) -> McpError {
    McpError::invalid_params(msg.into(), None)
}

#[tool_router]
impl MemvidService {
    #[allow(dead_code)]
    pub fn new(memvid: Arc<Mutex<Memvid>>) -> Self {
        Self {
            memvid,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Health check tool - returns 'pong' to verify the MCP server is running and responsive"
    )]
    pub async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text("pong")]))
    }

    #[tool(
        description = "Store text chunks as memory frames in Memvid. Each chunk is encoded with semantic embeddings and appended to the .mv2 file in a crash-safe, append-only format. Returns the number of inserted chunks and their sequence IDs for reference."
    )]
    pub async fn add_chunks(
        &self,
        Parameters(AddChunkRequest { chunks }): Parameters<AddChunkRequest>,
    ) -> Result<CallToolResult, McpError> {
        if chunks.is_empty() {
            return Err(invalid_params("chunks must not be empty"));
        }

        let mut mem = self.memvid.lock().await;

        let mut seqs = Vec::with_capacity(chunks.len());
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.trim().is_empty() {
                return Err(invalid_params(format!("chunk[{i}] is empty")));
            }

            // simplest insert API from memvid example [file:44]
            let seq = mem
                .put_bytes(chunk.as_bytes())
                .map_err(|e| internal(format!("put_bytes failed: {e}")))?;

            seqs.push(seq);
        }

        mem.commit()
            .map_err(|e| internal(format!("commit failed: {e}")))?;

        Ok(CallToolResult::success(vec![Content::text(Cow::Owned(
            format!("Inserted {} chunks. Sequences: {:?}", seqs.len(), seqs),
        ))]))
    }

    #[tool(description = "Search")]
    pub async fn search(
        &self,
        Parameters(req): Parameters<SearchToolRequest>,
    ) -> Result<CallToolResult, McpError> {
        if req.query.trim().is_empty() {
            return Err(invalid_params("query must not be empty"));
        }
        if req.top_k == 0 {
            return Err(invalid_params("top_k must be >= 1"));
        }

        let request = SearchRequest {
            query: req.query,
            top_k: req.top_k,
            snippet_chars: req.snippet_chars,
            uri: None,
            scope: None,
            cursor: None,
            // keep the cfg-gated temporal field out here; your crate features decide it
            as_of_frame: None,
            as_of_ts: None,
            no_sketch: false,
        };

        let mut mem = self.memvid.lock().await;
        let resp = mem
            .search(request)
            .map_err(|e| internal(format!("search failed: {e}")))?;

        let mut out = String::new();
        out.push_str(&format!(
            "total_hits={}, elapsed_ms={}\n",
            resp.total_hits, resp.elapsed_ms
        ));
        for hit in &resp.hits {
            let title = hit.title.as_deref().unwrap_or("Untitled");
            let score = hit.score.unwrap_or(0.0);
            out.push_str(&format!(
                "- frame_id={}, title={}, score={:.3}\n  snippet={}\n",
                hit.frame_id, title, score, hit.text
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(Cow::Owned(
            out,
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for MemvidService {
    fn get_info(&self) -> ServerInfo {
        let caps = ServerCapabilities::builder().enable_tools().build();

        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: caps,
            server_info: rmcp::model::Implementation {
                name: "Memvid MCP".to_string(),
                title: Some("Memvid Memory Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://github.com/memvid/memvid".to_string()),
            },
            instructions: Some(
                r#"
                <memvid_mcp_server>
                    <overview>
                        Model Context Protocol interface to Memvid's video-frame memory system</overview>
                    <what_is_memvid>
                        Memvid stores knowledge as compressed Smart Frames in a single .mv2 file (like video stores frames). Provides: 10x storage efficiency, sub-5ms access, append-only crash-safe storage with time-travel, single portable file with data/embeddings/search/metadata, no external infrastructure needed
                    </what_is_memvid>
                    <tools>
                        <tool>
                            <name>ping</name>
                            <purpose>Health check</purpose>
                            <returns>'pong'</returns>
                        </tool>
                        <tool>
                            <name>add_chunks</name>
                            <purpose>Store text chunks as memory frames with semantic embeddings</purpose>
                            <returns>Number of inserted chunks and sequence IDs</returns>
                        </tool>
                        <tool>
                            <name>search</name>
                            <purpose>Query memory using natural language</purpose>
                            <returns>Relevant chunks ranked by semantic similarity with scores and snippets</returns>
                        </tool>
                    </tools>
                    <use_cases>Agent long-term memory, enterprise knowledge bases, offline AI systems, codebase understanding, personal knowledge, auditable AI workflows</use_cases>
                    <storage>All data in memvid.mv2 - single portable file with compressed frames, full-text index, vector index, timeline metadata. Shareable, versionable, backup-friendly</storage>
                </memvid_mcp_server>"#
                .to_string(),
            ),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}
