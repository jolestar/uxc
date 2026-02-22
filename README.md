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
uxc https://api.example.com user.get id=42
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
* Returns structured JSON

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

UXC always outputs a stable JSON envelope:

```json
{
  "ok": true,
  "protocol": "openapi",
  "endpoint": "https://api.example.com",
  "operation": "user.get",
  "result": { ... },
  "meta": {
    "duration_ms": 128
  }
}
```

Errors are structured and predictable:

```json
{
  "ok": false,
  "error": {
    "code": "INVALID_ARGUMENT",
    "message": "Field 'id' must be an integer"
  }
}
```

This makes UXC ideal for:

* Shell pipelines
* Agent runtimes
* Skill systems
* Infrastructure automation

---

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
uxc https://api.example.com users.get --help

# Execute with parameters
uxc https://api.example.com users.get id=42

# Execute with JSON input
uxc https://api.example.com users.create --json '{"name":"Alice","email":"alice@example.com"}'
```

### gRPC Services

```bash
# List all services via reflection
uxc grpc.example.com:443 list

# List operations in a service
uxc grpc.example.com:443 list --service UserService

# Call a unary RPC
uxc grpc.example.com:443 GetUser user_id=42

# Call a server-streaming RPC
uxc grpc.example.com:443 StreamUsers --stream
```

### GraphQL APIs

```bash
# List available queries/mutations/subscriptions
uxc https://graphql.example.com list

# Execute a query
uxc https://graphql.example.com query/Viewer

# Execute with parameters
uxc https://graphql.example.com query/User id=42

# Execute a mutation
uxc https://graphql.example.com mutation/AddStar starredId=123

# Execute with custom fields
uxc https://graphql.example.com query/Viewer --fields "id login avatarUrl"
```

### MCP (Model Context Protocol)

```bash
# HTTP transport (recommended for production)
uxc https://mcp-server.example.com list
uxc https://mcp-server.example.com tool_name param1=value1

# stdio transport (for local development)
uxc mcp://server-name list
uxc "npx @modelcontextprotocol/server-filesystem /tmp" list

# Read an MCP resource
uxc https://mcp-server.example.com resource:/path/to/resource
```

---

## Automatic Protocol Detection

UXC determines the protocol via lightweight probing:

1. Check for OpenAPI endpoints
2. Attempt gRPC reflection
3. Attempt MCP discovery
4. Attempt GraphQL introspection
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

## Roadmap

### ✅ Phase 1 (COMPLETE)

* ✅ OpenAPI adapter
* ✅ MCP adapter (stdio transport)
* ✅ Stable JSON output
* ✅ CLI-only mode

### ✅ Phase 2 (COMPLETE)

* ✅ gRPC reflection support
* ✅ GraphQL support
* ✅ MCP HTTP transport
* ✅ Advanced help generation

### 🚧 Phase 3 (PLANNED)

* UXCd daemon
* Connection pooling
* Authentication profiles
* Schema caching
* Capability allowlists
* Audit logging

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

**Recent Milestones**:
- ✅ Milestone 1: OpenAPI & MCP adapters (Complete)
- ✅ Milestone 2: Multi-protocol support (Complete)
- 🚧 Milestone 3: Advanced features (In Progress)

**Known Limitations**:
- No connection pooling yet
- No authentication/profile management yet
- Schema caching not yet implemented

---

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

**Areas of Interest**:
- Connection pooling and caching
- Authentication profiles
- Additional protocol adapters (SOAP/WSDL, Thrift, etc.)
- Performance optimizations
- Schema caching implementation

---

## License

MIT License - see LICENSE file for details

---

If you'd like, I can next:

* Design the formal CLI command tree (`uxc list`, `uxc inspect`, etc.)
* Write a lightweight technical spec (v0.1 RFC)
* Or design the adapter interface abstraction for clean multi-protocol extensibility
