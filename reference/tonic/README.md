# Tonic - Quick Introduction

> **A Rust implementation of gRPC built on Hyper and Tower**

Tonic is a gRPC over HTTP/2 implementation focused on high performance, interoperability, and flexibility. It provides first-class support for async/await and acts as a core building block for production systems written in Rust. Tonic uses `prost` for Protocol Buffer serialization and generates client/server code from `.proto` files at build time.

## Key Features

| Feature | Description |
|---------|-------------|
| **gRPC** | Full gRPC implementation over HTTP/2 |
| **Streaming** | Unary, server-streaming, client-streaming, and bidirectional streaming |
| **Code Generation** | Generate client and server stubs from `.proto` files |
| **Tower Integration** | Interceptors and middleware via Tower `Service` and `Layer` |
| **TLS** | Built-in TLS support via `rustls` |
| **Health Checking** | Standard gRPC health checking protocol |
| **Reflection** | gRPC server reflection for discovery |
| **Interop** | Compatible with any gRPC implementation (Go, Java, Python, etc.) |

## Quick Start

### Proto File

```protobuf
// proto/hello.proto
syntax = "proto3";
package hello;

service Greeter {
    rpc SayHello (HelloRequest) returns (HelloReply);
}

message HelloRequest {
    string name = 1;
}

message HelloReply {
    string message = 1;
}
```

### Cargo.toml

```toml
[dependencies]
tonic = "0.14"
prost = "0.13"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tonic-build = "0.14"
```

### build.rs

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/hello.proto")?;
    Ok(())
}
```

### Server

```rust
use tonic::{transport::Server, Request, Response, Status};

pub mod hello {
    tonic::include_proto!("hello");
}

use hello::greeter_server::{Greeter, GreeterServer};
use hello::{HelloReply, HelloRequest};

#[derive(Default)]
pub struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    Server::builder()
        .add_service(GreeterServer::new(MyGreeter::default()))
        .serve(addr)
        .await?;
    Ok(())
}
```

## Architecture

```
┌──────────────────────────────────────────────┐
│              Your Application                 │
│                                               │
│  ┌──────────────┐      ┌──────────────────┐  │
│  │  .proto file │──────│  tonic-build     │  │
│  │  (service    │ build│  (code gen via   │  │
│  │   definition)│  .rs │   prost)         │  │
│  └──────────────┘      └───────┬──────────┘  │
│                                │              │
│                    ┌───────────▼──────────┐   │
│                    │  Generated Code      │   │
│                    │  - Server trait       │   │
│                    │  - Client struct      │   │
│                    │  - Message types      │   │
│                    └───────────┬──────────┘   │
│                                │              │
│         ┌──────────────────────┼───────────┐  │
│         │                      │           │  │
│  ┌──────▼──────┐        ┌─────▼─────┐     │  │
│  │   Server    │        │  Client   │     │  │
│  │             │        │           │     │  │
│  │ Interceptors│        │ Channel   │     │  │
│  │ TLS Config  │        │ Endpoint  │     │  │
│  │ Health/Refl │        │ TLS Config│     │  │
│  └──────┬──────┘        └─────┬─────┘     │  │
│         │                     │            │  │
├─────────┼─────────────────────┼────────────┤  │
│         │     Tower Service   │            │  │
├─────────┼─────────────────────┼────────────┤  │
│         │     Hyper HTTP/2    │            │  │
├─────────┼─────────────────────┼────────────┤  │
│         │     Tokio Runtime   │            │  │
└─────────┴─────────────────────┴────────────┘  │
└──────────────────────────────────────────────┘
```

## Essential Rust Types

| Type | Purpose |
|------|---------|
| `Request<T>` | gRPC request with message, metadata, and extensions |
| `Response<T>` | gRPC response with message, metadata, and extensions |
| `Status` | gRPC status code and message (error type) |
| `Code` | gRPC status code enum (Ok, NotFound, Internal, etc.) |
| `Streaming<T>` | Stream of messages (implements `Stream`) |
| `Server` | Transport server builder |
| `Channel` | Client transport connection |
| `Endpoint` | Client connection builder |
| `MetadataMap` | gRPC metadata (custom headers) |

## Documentation Map

```
reference/tonic/
├── index.md                    # Comprehensive reference and navigation
├── README.md                   # This file - quick introduction
├── getting-started/            # Setup and code generation
│   ├── 01-protobuf-setup.md
│   └── 02-build-rs.md
├── core/                       # Server, client, generated code
│   ├── 01-server.md
│   ├── 02-client.md
│   └── 03-codegen.md
├── streaming/                   # Streaming RPC patterns
│   ├── 01-patterns.md
│   └── 02-implementation.md
└── advanced/                    # Advanced features
    ├── 01-interceptors.md
    ├── 02-tls.md
    ├── 03-health-reflection.md
    └── 04-metadata-errors.md
```

## Quick Links

- **[Complete Reference](index.md)** - Comprehensive documentation and navigation
- **[Getting Started](getting-started/)** - Protobuf setup, build.rs
- **[Core](core/)** - Server, client, code generation
- **[Streaming](streaming/)** - All four streaming patterns
- **[Advanced](advanced/)** - Interceptors, TLS, health, reflection

## Related References

- **[Tower](../tower/)** - Service abstraction Tonic builds on
- **[Tokio](../tokio/)** - The async runtime
- **[Axum](../axum/)** - Web framework (sibling project)

## External Resources

- **[Crates.io](https://crates.io/crates/tonic)** - Tonic crate
- **[API Docs](https://docs.rs/tonic)** - docs.rs reference
- **[GitHub Repository](https://github.com/hyperium/tonic)** - Source code and issues
- **[Examples](https://github.com/hyperium/tonic/tree/master/examples)** - Official examples
- **[Discord](https://discord.gg/6yGkFeN)** - Community chat

---

**Tonic - High-performance, interoperable gRPC for Rust.**
