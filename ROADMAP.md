# UXC Roadmap & Issues

## Project Milestones

### 🎯 Milestone 1: MVP (Phase 1)
**Target**: Core functionality working for OpenAPI and MCP protocols
**Estimated**: 2-3 weeks

**Goal**: Basic CLI that can discover and call OpenAPI and MCP services

---

### 🎯 Milestone 2: Multi-Protocol (Phase 2)
**Target**: Full gRPC and GraphQL support
**Estimated**: 3-4 weeks

**Goal**: Universal execution layer for all 4 protocols

---

### 🎯 Milestone 3: Production Ready (Phase 3)
**Target**: Daemon mode, caching, auth profiles
**Estimated**: 4-5 weeks

**Goal**: Production-grade tool suitable for enterprise use

---

## Issues by Milestone

### Milestone 1: MVP Issues

#### Infrastructure
- [ ] #1 - Setup CI/CD pipeline (GitHub Actions)
- [ ] #2 - Add comprehensive error handling
- [ ] #3 - Add logging and tracing infrastructure
- [ ] #4 - Write integration test framework

#### OpenAPI Adapter
- [ ] #5 - Implement OpenAPI schema parser
- [ ] #6 - Add OpenAPI operation listing
- [ ] #7 - Implement OpenAPI operation execution
- [ ] #8 - Add OpenAPI argument validation
- [ ] #9 - Generate operation help from OpenAPI schema

#### MCP Adapter
- [ ] #10 - Implement MCP stdio client
- [ ] #11 - Add MCP tool discovery
- [ ] #12 - Implement MCP tool execution
- [ ] #13 - Add MCP resource and prompt support
- [ ] #14 - Generate tool help from MCP schemas

#### CLI Core
- [ ] #15 - Implement protocol detection
- [ ] #16 - Add `uxc <url> list` command
- [ ] #17 - Add `uxc <url> <operation> --help` command
- [ ] #18 - Add `uxc <url> <operation> [args]` execution
- [ ] #19 - Implement JSON output envelope
- [ ] #20 - Add structured error responses

#### Documentation
- [ ] #21 - Write getting started guide
- [ ] #22 - Add example usage with public APIs
- [ ] #23 - Create contribution guidelines

---

### Milestone 2: Multi-Protocol Issues

#### gRPC Adapter
- [ ] #24 - Implement gRPC reflection client
- [ ] #25 - Add gRPC service discovery
- [ ] #26 - Implement gRPC method execution
- [ ] #27 - Add Protobuf message encoding/decoding
- [ ] #28 - Generate method help from reflection

#### GraphQL Adapter
- [ ] #29 - Implement GraphQL introspection parser
- [ ] #30 - Add query/mutation discovery
- [ ] #31 - Implement GraphQL query execution
- [ ] #32 - Add variable and argument handling
- [ ] #33 - Generate field help from introspection

#### Schema Caching
- [ ] #34 - Design schema cache format
- [ ] #35 - Implement in-memory schema caching
- [ ] #36 - Add persistent schema cache (file-based)
- [ ] #37 - Add cache invalidation strategy
- [ ] #38 - Add `--no-cache` flag

#### Advanced Help
- [ ] #39 - Generate rich help text with examples
- [ ] #40 - Add auto-completion scripts (bash/zsh/fish)
- [ ] #41 - Implement fuzzy search for operations
- [ ] #42 - Add operation usage examples in help

---

### Milestone 3: Production Ready Issues

#### UXCd Daemon
- [ ] #43 - Design daemon protocol (IPC)
- [ ] #44 - Implement daemon process manager
- [ ] #45 - Add daemon discovery (auto-start)
- [ ] #46 - Implement connection pooling
- [ ] #47 - Add daemon health check

#### Authentication
- [ ] #48 - Design auth profile system
- [ ] #49 - Add API key storage (secure)
- [ ] #50 - Add OAuth2 flow support
- [ ] #51 - Add mTLS support
- [ ] #52 - Implement auth profile CLI commands

#### Security & Governance
- [ ] #53 - Design capability allowlist system
- [ ] #54 - Implement operation allowlist/denylist
- [ ] #55 - Add rate limiting
- [ ] #56 - Implement audit logging
- [ ] #57 - Add security audit

#### Performance
- [ ] #58 - Optimize adapter selection
- [ ] #59 - Add parallel operation support
- [ ] #60 - Implement request batching
- [ ] #61 - Add performance benchmarks

#### Developer Experience
- [ ] #62 - Add configuration file support (optional)
- [ ] #63 - Implement plugin system for custom adapters
- [ ] #64 - Add telemetry/metrics collection
- [ ] #65 - Create web dashboard (optional)

---

## Quick Start Issues (First Week)

### Priority Order
1. #5 - OpenAPI schema parser
2. #15 - Protocol detection
3. #6 - OpenAPI operation listing
4. #16 - `list` command implementation
5. #7 - OpenAPI operation execution
6. #18 - Execution command implementation
7. #19 - JSON output envelope
8. #21 - Getting started guide

---

## Label System

### Priority
- `priority: critical` - Blocks release
- `priority: high` - Important for milestone
- `priority: medium` - Nice to have
- `priority: low` - Backlog

### Type
- `type: feature` - New feature
- `type: bug` - Bug fix
- `type: docs` - Documentation
- `type: tests` - Test coverage
- `type: refactor` - Code improvement

### Component
- `component: cli` - CLI core
- `component: openapi` - OpenAPI adapter
- `component: grpc` - gRPC adapter
- `component: mcp` - MCP adapter
- `component: graphql` - GraphQL adapter
- `component: daemon` - UXCd daemon

### Phase
- `phase: 1` - MVP
- `phase: 2` - Multi-Protocol
- `phase: 3` - Production

---

## GitHub Actions Workflows

### Required CI Checks
- [ ] Linting (clippy)
- [ ] Formatting (rustfmt)
- [ ] Unit tests
- [ ] Integration tests
- [ ] Security audit
- [ ] Dependency check

### Release Automation
- [ ] Automated release on tag
- [ ] Build binaries for multiple platforms
- [ ] Generate release notes from commits
- [ ] Publish to crates.io

---

## Contribution Guidelines

See [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Code review process
- Commit message conventions
- PR template
- Issue template

---

## Version Strategy

- v0.1.0 - Milestone 1 (MVP)
- v0.2.0 - Milestone 2 (Multi-Protocol)
- v0.3.0 - Milestone 3 (Production Ready)
- v1.0.0 - Stable release

---

## Tracking

- **Project Board**: [GitHub Projects](https://github.com/jolestar/uxc/projects)
- **Burndown Chart**: Updated weekly
- **Milestone Progress**: Tracked via GitHub Milestones
