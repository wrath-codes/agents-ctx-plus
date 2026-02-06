# Client Configuration

Configure the OpenCode SDK client for your specific needs.

## Basic Configuration

```rust
use opencode_rs::{Client, ClientBuilder};

let client = Client::builder()
    .base_url("http://127.0.0.1:4096")
    .directory("/path/to/project")
    .timeout_secs(300)
    .build()?;
```

## Configuration Options

### base_url

The OpenCode server URL.

```rust
let client = Client::builder()
    .base_url("http://127.0.0.1:4096")
    .build()?;
```

Default: `http://127.0.0.1:4096`

### directory

Working directory for all operations.

```rust
let client = Client::builder()
    .directory("/home/user/my-project")
    .build()?;
```

Sets the `x-opencode-directory` header.

### timeout_secs

HTTP request timeout.

```rust
let client = Client::builder()
    .timeout_secs(600)  // 10 minutes
    .build()?;
```

Default: `300` (5 minutes)

## Environment Variables

Configure via environment:

```rust
use std::env;

let client = Client::builder()
    .base_url(env::var("OPENCODE_URL")?)
    .directory(env::var("OPENCODE_DIR")?)
    .build()?;
```

Common variables:
- `OPENCODE_URL` - Server URL
- `OPENCODE_DIR` - Working directory
- `RUST_LOG` - Logging level

## Feature Flags

```toml
[dependencies]
opencode_rs = { version = "0.1.2", features = ["http", "sse"] }
```

Available features:
- `http` - HTTP client (default)
- `sse` - SSE streaming (default)
- `retry` - Retry policies

## Multiple Clients

Create clients for different directories:

```rust
let frontend_client = Client::builder()
    .directory("./frontend")
    .build()?;

let backend_client = Client::builder()
    .directory("./backend")
    .build()?;
```