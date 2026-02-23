# UXC

**Universal X-Protocol Call**

[![CI](https://github.com/jolestar/uxc/workflows/CI/badge.svg)](https://github.com/jolestar/uxc/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org)

UXC is a schema-driven, multi-protocol RPC execution runtime.

It turns remote, schema-exposed services into executable command-line capabilities — without SDKs, code generation, or preconfigured server aliases.

If a service exposes a machine-readable interface, UXC can discover it, understand it, and execute it.

---

## Vision

Modern systems increasingly expose machine-readable schemas:

* OpenAPI (`/openapi.json`)
* gRPC reflection
* MCP (Model Context Protocol)
* GraphQL introspection
* WSDL (SOAP)

Yet interacting with them still requires:

* Static client generation
* SDK installation
* Custom configuration files
* Or embedding tool definitions into AI prompts

UXC removes that friction.

It provides a **universal execution layer** that dynamically transforms remote schema definitions into immediately usable commands.

Schema becomes execution.

---

## Core Principles

### 1. URL-First, Not Config-First

UXC does not require registering server aliases.

```bash
uxc https://api.example.com list
uxc https://api.example.com "GET /users/42"
```

Any compliant endpoint can be called directly.

This makes UXC safe to use inside:

* Automation scripts
* CI pipelines
* AI skills and agents
* Sandboxed execution environments

---

### 2. Schema-Driven Execution

UXC automatically:

* Detects protocol type
* Retrieves remote schema
* Generates contextual help
* Validates arguments
* Executes calls
* Returns structured JSON by default

No manual client definitions required.

---

### 3. Multi-Protocol by Design

UXC supports multiple schema-exposing protocols through adapters:

* OpenAPI / Swagger
* gRPC (with reflection)
* MCP
* GraphQL
* Extensible adapter system

The CLI interface remains consistent across protocols.

---

### 4. Deterministic Machine Output

`uxc ...` outputs a stable JSON envelope by default:

```json
{
  "ok": true,
  "kind": "call_result",
  "protocol": "openapi",
  "endpoint": "https://api.example.com",
  "operation": "user.get",
  "data": { ... },
  "meta": {
    "version": "v1",
    "duration_ms": 128
  }
}
```

Command failures are structured and predictable:

```json
{
  "ok": false,
  "error": {
    "code": "INVALID_ARGUMENT",
    "message": "Field 'id' must be an integer"
  },
  "meta": {
    "version": "v1"
  }
}
```

Use `--text` (or `--format text`) for human-readable output.

This makes UXC ideal for:

* Shell pipelines
* Agent runtimes
* Skill systems
* Infrastructure automation

---

## Cache Management

```bash
# View cache statistics
uxc cache stats

# Clear cache for specific endpoint
uxc cache clear https://api.example.com

# Clear all cache
uxc cache clear --all

# Disable cache for this operation
uxc https://api.example.com list --no-cache

# Use custom TTL (in seconds)
uxc https://api.example.com list --cache-ttl 3600
```

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/jolestar/uxc.git
cd uxc

# Build and install
cargo install --path .
```

### Using Cargo (Coming Soon)

```bash
cargo install uxc
```

## Example Usage

### OpenAPI / REST APIs

```bash
# List available operations
uxc https://api.example.com list

# Get operation help
uxc https://api.example.com describe "GET /users/{id}"
uxc https://api.example.com "GET /users/{id}" help

# Execute with parameters
uxc https://api.example.com "GET /users/{id}" --json '{"id":42}'

# Execute with JSON input
uxc https://api.example.com "POST /users" --json '{"name":"Alice","email":"alice@example.com"}'
```

### gRPC Services

```bash
# List all services via reflection
uxc grpc.example.com:9000 list

# Call a unary RPC
uxc grpc.example.com:9000 addsvc.Add/Sum --json '{"a":1,"b":2}'
```

Note: gRPC unary invocation uses the `grpcurl` binary at runtime.

### GraphQL APIs

```bash
# List available queries/mutations/subscriptions
uxc https://graphql.example.com list

# Execute a query
uxc https://graphql.example.com query/viewer

# Execute with parameters
uxc https://graphql.example.com query/user --json '{"id":"42"}'

# Execute a mutation
uxc https://graphql.example.com mutation/addStar --json '{"starredId":"123"}'
```

### MCP (Model Context Protocol)

```bash
# HTTP transport (recommended for production)
uxc https://mcp-server.example.com list
uxc https://mcp-server.example.com tool_name --json '{"param1":"value1"}'

# stdio transport (for local development)
uxc "npx -y @modelcontextprotocol/server-filesystem /tmp" list
uxc "npx -y @modelcontextprotocol/server-filesystem /tmp" list_directory --json '{"path":"/tmp"}'
```

## Public Test Endpoints (No API Key)

These endpoints are useful for protocol availability checks without API keys.
Verified on 2026-02-23.

### OpenAPI

- Endpoint: `https://petstore3.swagger.io/api/v3`
- Verify schema:

```bash
curl -sS https://petstore3.swagger.io/api/v3/openapi.json | jq -r '.openapi, .info.title'
```

### GraphQL

- Endpoint: `https://countries.trevorblades.com/`
- Verify introspection:

```bash
curl -sS https://countries.trevorblades.com/ \
  -H 'content-type: application/json' \
  --data '{"query":"{ __schema { queryType { name } } }"}' \
  | jq -r '.data.__schema.queryType.name'
```

### gRPC (Server Reflection)

- Endpoint (plaintext): `grpcb.in:9000`
- Endpoint (TLS): `grpcb.in:9001`
- Verify reflection:

```bash
grpcurl -plaintext grpcb.in:9000 list
grpcurl grpcb.in:9001 list
```

### MCP (HTTP)

- Endpoint: `https://mcp.deepwiki.com/mcp`
- Verify `initialize` (DeepWiki uses streamable HTTP/SSE response):

```bash
curl -sS https://mcp.deepwiki.com/mcp \
  -H 'content-type: application/json' \
  -H 'accept: application/json, text/event-stream' \
  --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"uxc-check","version":"0.1"}}}'
```

Note: this endpoint is publicly reachable without an API key for basic calls. Some MCP clients that only expect JSON (not SSE) may need transport updates.

### MCP (stdio, local)

- Command: `npx -y @modelcontextprotocol/server-filesystem /tmp`
- This is useful as a local no-key MCP baseline.

---

## Automatic Protocol Detection

UXC determines the protocol via lightweight probing:

1. Attempt MCP stdio/HTTP discovery
2. Attempt GraphQL introspection
3. Check OpenAPI endpoints
4. Attempt gRPC reflection
5. Fallback or fail gracefully

Each protocol is handled by a dedicated adapter.

---

## Architecture Overview

```
User / Skill / Agent
          ↓
          UXC CLI
          ↓
    Protocol Detector
          ↓
       Adapter Layer
   ├── OpenAPI Adapter
   ├── gRPC Adapter
   ├── MCP Adapter
   ├── GraphQL Adapter
          ↓
     Remote Endpoint
```

Optional:

```
UXCd (local daemon)
  - Connection pooling
  - Schema caching
  - Authentication management
  - Rate limiting
```

The CLI works independently, but can transparently use the daemon for performance and stability.

---

## Target Use Cases

### AI Agents & Skills

* Dynamically call remote capabilities
* Avoid injecting large tool schemas into context
* Maintain deterministic execution boundaries

### Infrastructure & DevOps

* Replace SDK generation with runtime discovery
* Interact with heterogeneous services via a unified interface
* Simplify testing across protocols

### Execution Sandboxes

* Provide a controlled, auditable capability layer
* Enforce allowlists and rate limits
* Record structured invocation logs

---

## Non-Goals

UXC is not:

* A code generator
* An SDK replacement
* An API gateway
* A reverse proxy

It is an execution interface.

---

## Why Universal X-Protocol Call?

Because infrastructure is no longer protocol-bound.

Because services describe themselves.

Because execution should be dynamic.

UXC makes remote schema executable.

---

## Development Status

**Current Version**: v0.1.0 (Alpha)

**Supported Protocols**:
- ✅ OpenAPI 3.x
- ✅ gRPC (with Server Reflection Protocol)
- ✅ GraphQL (with Introspection)
- ✅ MCP (Model Context Protocol) - HTTP & stdio transports

**Platforms**:
- ✅ Linux (x86_64)
- ✅ macOS (x86_64, ARM64)
- ✅ Windows (x86_64)

**Known Limitations**:
- gRPC currently supports unary invocation only
- gRPC runtime calls require `grpcurl` to be installed
- No connection pooling yet

---

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

**Areas of Interest**:
- Connection pooling
- Authentication profiles
- Additional protocol adapters (SOAP/WSDL, Thrift, etc.)
- Performance optimizations
- UXCd daemon
- Capability allowlists
- Audit logging

---

## License

MIT License - see LICENSE file for details
