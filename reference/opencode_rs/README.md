# OpenCode Rust SDK (`opencode_rs`)

A native Rust SDK for [OpenCode](https://opencode.ai), providing an HTTP-first hybrid interface with SSE (Server-Sent Events) streaming capabilities. This SDK enables programmatic interaction with the OpenCode AI coding agent for building custom tools, integrations, and automation workflows.

[![Crates.io](https://img.shields.io/crates/v/opencode_rs)](https://crates.io/crates/opencode_rs)
[![docs.rs](https://img.shields.io/docsrs/opencode_rs)](https://docs.rs/opencode_rs)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

## Overview

The OpenCode Rust SDK provides a type-safe, async-first API for interacting with OpenCode's HTTP REST API and real-time event streaming. It is designed for:

- **Custom Integrations**: Build tools that interact with OpenCode programmatically
- **Automation Workflows**: Create scripts and applications that leverage AI coding capabilities
- **MCP Server Development**: Implement Model Context Protocol servers
- **Session Management**: Manage coding sessions, send prompts, and receive responses
- **Real-time Event Handling**: Subscribe to OpenCode events via SSE streaming

## Key Features

### HTTP REST API Client
- Complete coverage of OpenCode's HTTP endpoints
- Type-safe request/response handling with Serde
- Async/await support with Tokio
- Configurable timeouts and connection settings

### SSE Streaming Support
- Real-time event subscription via Server-Sent Events
- Automatic reconnection with exponential backoff
- Session-specific or global event filtering
- 40+ event types covering all OpenCode operations

### Type Safety
- Comprehensive type definitions for all API responses
- Strong typing for sessions, messages, tools, and events
- Error handling with custom `OpencodeError` type

### Async-First Design
- Built on Tokio for high-performance async I/O
- Non-blocking API calls
- Stream-based event handling

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
opencode_rs = "0.1.2"
tokio = { version = "1", features = ["full"] }
```

Basic usage:

```rust
use opencode_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a client with default configuration
    let client = Client::builder()
        .base_url("http://127.0.0.1:4096")
        .directory("/path/to/project")
        .build()?;

    // Create a session and send a simple text prompt
    let session = client.run_simple_text("Hello, OpenCode!").await?;
    println!("Session created: {}", session.id);

    Ok(())
}
```

## Documentation Structure

### Getting Started
- [Installation](getting-started/installation.md) - Setting up the SDK
- [Quick Start Guide](getting-started/quickstart.md) - Your first OpenCode integration
- [Authentication](getting-started/authentication.md) - Configuring API access

### Core Concepts
- [Architecture Overview](core-concepts/architecture.md) - Understanding the SDK design
- [Client Lifecycle](core-concepts/client-lifecycle.md) - Building and managing clients
- [Sessions](core-concepts/sessions.md) - Working with coding sessions
- [Messages & Parts](core-concepts/messages-parts.md) - Understanding message structure
- [Event System](core-concepts/event-system.md) - Real-time event handling

### API Reference
- [Client API](api-reference/client.md) - Main client interface
- [HTTP APIs](api-reference/http-apis.md) - Individual API modules
- [SSE Streaming](api-reference/sse.md) - Event streaming
- [Error Handling](api-reference/error-handling.md) - Working with errors

### Types
- [Core Types](types/core-types.md) - Session, Message, and Event types
- [Request/Response Types](types/request-response.md) - API input/output types
- [Event Types](types/events.md) - 40+ SSE event variants
- [Tool Types](types/tools.md) - Tool and agent definitions

### Examples
- [Basic Usage](examples/basic-usage.md) - Simple integrations
- [Session Management](examples/session-management.md) - Managing sessions
- [Event Streaming](examples/event-streaming.md) - Real-time event handling
- [Advanced Patterns](examples/advanced-patterns.md) - Complex use cases

### Configuration
- [Client Configuration](configuration/client-config.md) - Client settings
- [Feature Flags](configuration/feature-flags.md) - Compile-time options
- [Environment Variables](configuration/environment.md) - Runtime configuration

## Architecture

The SDK is organized into several key modules:

```
opencode_rs/
├── client/          # High-level client API
├── http/            # HTTP client and API modules
│   ├── sessions/    # Session management
│   ├── messages/    # Message operations
│   ├── files/       # File operations
│   └── ...          # Other API modules
├── sse/             # SSE streaming support
└── types/           # Type definitions
    ├── session/     # Session types
    ├── message/     # Message types
    ├── event/       # Event types
    └── ...          # Other type modules
```

## Requirements

- **Rust**: 1.70+ (async/await support)
- **OpenCode**: Running server instance (default: http://127.0.0.1:4096)
- **Tokio**: Async runtime (included as dependency)

## Dependencies

Core dependencies:
- `reqwest` - HTTP client (optional, enabled by default)
- `reqwest-eventsource` - SSE streaming (optional)
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `chrono` - Date/time handling
- `thiserror` - Error handling
- `tracing` - Logging and instrumentation

Optional features:
- `backon` - Retry policies
- `portpicker` - Port selection utilities

## Version

Current version: **0.1.2**

## License

This project is licensed under the Apache-2.0 License.

## Repository

- **GitHub**: [allisoneer/agentic_auxilary](https://github.com/allisoneer/agentic_auxilary)
- **Crate**: [crates.io/crates/opencode_rs](https://crates.io/crates/opencode_rs)
- **Documentation**: [docs.rs/opencode_rs](https://docs.rs/opencode_rs)

## Contributing

Contributions are welcome! Please see the repository for contribution guidelines.

## Related Projects

- [OpenCode CLI](https://opencode.ai) - The OpenCode terminal application
- [Agentic Tools](https://github.com/allisoneer/agentic_auxilary) - Collection of agentic AI development tools
- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) - Protocol for AI tool integration