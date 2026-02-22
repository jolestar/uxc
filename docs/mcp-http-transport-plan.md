# MCP HTTP Transport Implementation Plan

## Current State

The MCP adapter currently only supports **stdio transport**, which spawns a child process and communicates via stdin/stdout with JSON-RPC messages.

## Goal

Implement **HTTP transport** as the primary transport method for MCP, with stdio as a fallback/secondary option.

## Requirements

1. **HTTP Transport** (Primary)
   - Support HTTP/HTTPS connections to MCP servers
   - Standard JSON-RPC over HTTP POST
   - Proper error handling and timeout management
   - Support for both short-lived and persistent connections

2. **Transport Selection**
   - Auto-detect transport type from URL scheme
   - `http://` or `https://` → HTTP transport
   - `mcp://` or command string → stdio transport (backward compatible)

3. **Backward Compatibility**
   - Keep existing stdio transport working
   - Allow users to choose transport method explicitly

## Implementation Plan

### Phase 1: HTTP Transport Implementation

1. Create `src/adapters/mcp/http_transport.rs`
   - HTTP client using `reqwest`
   - JSON-RPC request/response handling
   - Error mapping and timeout handling

2. Create transport enum
   - `enum McpTransport { Stdio(McpStdioTransport), Http(McpHttpTransport) }`
   - Unified interface for both transports

3. Update adapter initialization
   - Parse URL to determine transport type
   - Create appropriate transport instance

### Phase 2: Integration

1. Update MCP adapter to use transport enum
2. Update URL parsing logic
3. Add tests for HTTP transport

### Phase 3: Documentation

1. Update README with HTTP transport examples
2. Add usage documentation

## File Changes

- **New**: `src/adapters/mcp/http_transport.rs` (~200 lines)
- **Modified**: `src/adapters/mcp/mod.rs` (~50 lines)
- **Modified**: `src/adapters/mcp/transport.rs` (rename/refactor)

## MCP Protocol Details

### HTTP JSON-RPC Format

**Request**:
```json
POST / HTTP/1.1
Host: mcp.example.com
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": { ... },
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

## Testing Strategy

1. Unit tests for HTTP transport
2. Integration tests with mock MCP server
3. Ensure stdio transport still works

## Estimated Effort

- HTTP transport implementation: ~3 hours
- Integration and testing: ~2 hours
- Documentation: ~1 hour
- **Total**: ~6 hours
