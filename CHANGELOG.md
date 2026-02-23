# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-23

### Added

#### Authentication Profiles
- Multiple authentication profile storage with `uxc auth set` command
- Support for Bearer token authentication
- Support for API key authentication (X-API-Key header)
- Support for Basic HTTP authentication
- Profile management commands: `list`, `set`, `remove`, `info`
- `--profile` CLI flag for selecting profiles
- `UXC_PROFILE` environment variable support
- Profile selection precedence: CLI flag > env var > "default"
- API key masking in sensitive outputs

#### Protocol Support
- OpenAPI/Swagger specification support with full HTTP method coverage
- GraphQL API support with introspection and query execution
- gRPC service support with server reflection
- MCP (Model Context Protocol) server support with stdio and HTTP transports

#### CLI Features
- `uxc list <url>` - List available operations for any protocol
- `uxc call <url> <operation>` - Execute operations with parameters
- `uxc inspect <url>` - Inspect endpoint schema and capabilities
- `uxc auth` commands - Manage authentication profiles
- `uxc cache stats|clear` - View and clear schema cache
- JSON output envelope with `--output json` flag
- Schema caching with configurable TTL
- Cache configuration via `--cache-ttl` flag and `UXC_CACHE_TTL` env var

#### Developer Experience
- Automatic protocol detection from URLs
- Built-in schema caching to reduce network calls
- Color-coded JSON output with syntax highlighting
- Comprehensive error messages
- Progress indicators for long-running operations

#### Configuration
- Profile storage in `~/.uxc/profiles.toml`
- Schema cache in `~/.uxc/cache/`
- Environment variable support for all major settings
- TOML-based configuration format

### Security
- Input validation for profile names
- API key masking in logs and outputs
- Secure profile storage (non-encrypted in v0.1.0, encryption planned for v0.2.0)

### Technical
- Built with Rust 2021 edition
- Async runtime powered by Tokio 1.35
- Zero-copy parsing where possible
- Cross-platform support (Linux, macOS, Windows)

### Known Limitations
- gRPC dynamic invocation returns placeholder (requires generated types at build time)
- Profile encryption not implemented (planned for v0.2.0, see Issue #29)
- No per-endpoint profile configuration yet
- auth_integration_test.rs has 2 pre-existing test failures (not in release scope)

### Documentation
- Comprehensive help text for all commands
- Usage examples in command descriptions
- Clear error messages with suggestions

## [0.0.1] - Initial Release

### Added
- Initial project structure
- Basic CLI framework
- Protocol detection infrastructure

---

[Unreleased]: https://github.com/jolestar/uxc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jolestar/uxc/releases/tag/v0.1.0
[0.0.1]: https://github.com/jolestar/uxc/releases/tag/v0.0.1
