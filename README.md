# UXC

**Universal X-Protocol Call**

[![CI](https://github.com/holon-run/uxc/workflows/CI/badge.svg)](https://github.com/holon-run/uxc/actions)
[![Coverage](https://github.com/holon-run/uxc/workflows/Coverage/badge.svg)](https://github.com/holon-run/uxc/actions/workflows/coverage.yml)
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
* JSON-RPC (OpenRPC discovery)
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
uxc https://api.example.com get:/users/42
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
* JSON-RPC (with OpenRPC)
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
  "operation": "get:/users/{id}",
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

Global discovery commands are also JSON-first:

```bash
uxc
uxc help
```

Use `--text` when you want CLI-style help text:

```bash
uxc --text help
```

If an operation ID conflicts with a CLI keyword (for example `help`/`list`), use explicit `call`:

```bash
uxc <host> call <operation_id> [--json '{...}']
```

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

## Debugging and Logging

UXC uses structured logging with the `tracing` crate. By default, only warnings and errors are displayed.

```bash
# Default: warnings and errors only
uxc https://api.example.com list

# Enable info logs (HTTP requests, responses, etc.)
RUST_LOG=info uxc https://api.example.com list

# Enable debug logs (detailed debugging information)
RUST_LOG=debug uxc https://api.example.com list

# Enable trace logs (maximum verbosity)
RUST_LOG=trace uxc https://api.example.com list

# Enable logs for specific modules only
RUST_LOG=uxc::adapters::openapi=debug uxc https://api.example.com list
```

**Log Levels:**
- `error` - Critical failures that prevent operation completion
- `warn` - **[Default]** Non-critical issues and warnings
- `info` - Informational messages (HTTP requests, protocol detection, etc.)
- `debug` - Detailed debugging information
- `trace` - Extremely verbose tracing information

Logs are written to stderr to avoid interfering with JSON output on stdout.

## Installation

### Homebrew (macOS/Linux)

```bash
brew tap holon-run/homebrew-tap
brew install uxc
```

### Install Script (macOS/Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/holon-run/uxc/main/scripts/install.sh | bash
```

If you prefer to review before execution:

```bash
curl -fsSL https://raw.githubusercontent.com/holon-run/uxc/main/scripts/install.sh -o install-uxc.sh
less install-uxc.sh
bash install-uxc.sh
```

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/holon-run/uxc/main/scripts/install.sh | bash -s -- -v v0.1.1
```

### Cargo

```bash
cargo install uxc
```

### From Source

```bash
git clone https://github.com/holon-run/uxc.git
cd uxc
cargo install --path .
```

## Example Usage

### Operation ID Conventions

UXC uses protocol-native, machine-friendly `operation_id` values:

- OpenAPI: `method:/path` (e.g. `get:/users/{id}`, `post:/pet`)
- gRPC: `Service/Method`
- GraphQL: `query/viewer`, `mutation/addStar`, `subscription/onEvent`
- MCP: tool name (e.g. `ask_question`)
- JSON-RPC: method name (e.g. `eth_getBalance`, `net_version`)

### OpenAPI / REST APIs

```bash
# List available operations
uxc https://api.example.com list
uxc petstore3.swagger.io/api/v3 list  # scheme can be omitted for common HTTP targets

# Schema-separated service: runtime endpoint and schema URL are different
uxc https://api.github.com list \
  --schema-url https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.json

# Get operation help
uxc https://api.example.com describe get:/users/{id}
uxc https://api.example.com get:/users/{id} help

# Execute with parameters
uxc https://api.example.com get:/users/{id} --json '{"id":42}'

# Execute with JSON input
uxc https://api.example.com call post:/users --json '{"name":"Alice","email":"alice@example.com"}'
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

# If a tool name conflicts with CLI subcommands, use explicit call
uxc https://mcp-server.example.com call help --json '{}'

# stdio transport (for local development)
uxc "npx -y @modelcontextprotocol/server-filesystem /tmp" list
uxc "npx -y @modelcontextprotocol/server-filesystem /tmp" list_directory --json '{"path":"/tmp"}'
```

### JSON-RPC (OpenRPC)

```bash
# Discover methods (requires rpc.discover or openrpc.json)
uxc https://rpc.example.com list

# Describe one method
uxc https://rpc.example.com describe eth_getBalance

# Execute a method
uxc https://rpc.example.com eth_getBalance --json '{"address":"0xabc...","block":"latest"}'
```

Note: JSON-RPC support is OpenRPC-driven for predictable `list/describe` discovery.

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
3. Check OpenAPI schema sources:
   - `--schema-url` override
   - user/builtin schema mappings
   - default well-known OpenAPI endpoints (`/openapi.json`, `/swagger.json`, etc.)
4. Attempt JSON-RPC OpenRPC discovery
5. Attempt gRPC reflection
6. Fallback or fail gracefully

Each protocol is handled by a dedicated adapter.

### OpenAPI Schema Mapping

For services where the OpenAPI document is hosted separately from the runtime endpoint
(for example `api.github.com`), UXC supports:

1. Explicit override via `--schema-url`
2. Builtin mappings for known services
3. User mappings in `~/.uxc/schema_mappings.json`

Example user mapping file:

```json
{
  "version": 1,
  "openapi": [
    {
      "host": "api.github.com",
      "path_prefix": "/",
      "schema_url": "https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.json",
      "priority": 100
    }
  ]
}
```

For tests or custom environments, the mapping file path can be overridden via:
`UXC_SCHEMA_MAPPINGS_FILE=/path/to/schema_mappings.json`.

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
   ├── JSON-RPC Adapter
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
- ✅ JSON-RPC (with OpenRPC discovery)

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

**Development Requirements**:
- All code must be formatted with `cargo fmt`
- No clippy warnings allowed
- Minimum 65% code coverage required
- All tests must pass

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development workflow, testing guidelines, and coverage instructions.

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
