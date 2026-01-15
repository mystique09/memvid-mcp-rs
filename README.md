# Memvid MCP Server

Unofficial Model Context Protocol (MCP) server interface to [Memvid](https://github.com/memvid/memvid) - a video-frame memory system for AI agents.

## What is Memvid?

Memvid is a memory layer for AI agents that stores knowledge as compressed "Smart Frames" in a single `.mv2` file (similar to how video stores frames). This approach provides:

- **10x more storage efficiency** than traditional vector databases
- **Sub-5ms memory access times**
- **Append-only, crash-safe storage** with time-travel capabilities
- **Single portable file** containing data, embeddings, search structure, and metadata
- **No external database or infrastructure required**

## Features

- **Persistent long-term memory** for AI agents
- **Semantic search** with natural language queries
- **Instant retrieval** from a single portable file
- **Serverless architecture** - no database infrastructure needed
- **Model-agnostic** - works with any LLM that supports MCP

## Installation

### From Cargo (Rust)

```bash
cargo install memvid-mcp
# or
cargo install --git https://github.com/mystique09/memvid-mcp.git
```

### From Source

```bash
git clone https://github.com/mystique09/memvid-mcp.git
cd memvid-mcp
cargo build --release
# The binary will be at target/release/memvid-mcp.exe (Windows) or target/release/memvid-mcp (Linux/Mac)
```

## Configuration

### Continue

Add to your `mcpServers/new-mcp-server.yaml`:

```yaml
mcpServers:
  - name: Memvid MCP
    type: stdio
    command: memvid-mcp
```

### Claude Desktop

Add to your Claude Desktop config (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "memvid": {
      "command": "memvid-mcp",
      "env": {}
    }
  }
}
```

### Cline (VS Code Extension)

Add to your Cline settings:

```json
{
  "mcpServers": [
    {
      "name": "memvid",
      "command": "memvid-mcp"
    }
  ]
}
```

## Available Tools

| Tool | Purpose | Description |
|-------|----------|-------------|
| `ping` | Health check | Returns "pong" to verify server is running |
| `add_chunks` | Store memory | Stores text chunks as memory frames with semantic embeddings. Returns count and sequence IDs |
| `search` | Query memory | Searches using natural language. Returns relevant chunks ranked by semantic similarity |

## Usage Examples

### Store Chunks

```
Use the add_chunks tool to store information:
Add chunks ["Meeting discussed Q1 planning", "Action item: Review API integration", "Decision: Use Memvid for memory"]
```

### Search Memory

```
Use the search tool with natural language queries:
Search for "Q1 planning" with top_k=5 and snippet_chars=200
```

### Health Check

```
Ping to verify server is responsive:
Ping
```

## Storage

All data is stored in `memvid.mv2` in the current working directory. This is a single, portable file that includes:

- Compressed frames
- Full-text index
- Vector index
- Timeline metadata

The `.mv2` file can be shared, versioned, and backed up like any other file.

## Development

### Build

```bash
cargo build
```

### Run (Development)

```bash
cargo run
```

### Test with MCP Client

```bash
# Create test requests
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"clientInfo":{"name":"test","version":"1.0.0"}}}' | cargo run
```

## Requirements

- Rust 1.85.0+
- `memvid-core` crate (included as dependency)

## Learn More

- [Memvid Documentation](https://docs.memvid.com)
- [Memvid GitHub](https://github.com/memvid/memvid)
- [Model Context Protocol Spec](https://modelcontextprotocol.io)

## License

Apache License 2.0

## Contributing

Contributions welcome! Please feel free to submit a Pull Request.
