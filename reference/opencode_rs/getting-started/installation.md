# Installation

This guide covers various ways to install and set up the OpenCode Rust SDK.

## Requirements

- **Rust**: Version 1.70 or later with Cargo
- **OpenCode**: A running OpenCode server instance
- **Tokio**: Async runtime (included automatically)

## Adding to a Rust Project

### Using Cargo Add (Recommended)

```bash
# Add the crate
cargo add opencode_rs

# Add with specific features
cargo add opencode_rs --features http,sse
```

### Manual Cargo.toml

Add to your `Cargo.toml`:

```toml
[dependencies]
opencode_rs = "0.1.2"
```

With optional dependencies:

```toml
[dependencies]
opencode_rs = "0.1.2"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

## Feature Flags

The SDK uses Cargo features to control functionality:

### Default Features

```toml
[dependencies]
opencode_rs = "0.1.2"  # Includes http and sse by default
```

### Minimal Installation

For HTTP-only (no SSE streaming):

```toml
[dependencies]
opencode_rs = { version = "0.1.2", default-features = false, features = ["http"] }
```

### Available Features

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `http` | HTTP client functionality | reqwest, serde_json |
| `sse` | SSE streaming support | reqwest-eventsource |
| `retry` | Retry policies with backoff | backon |
| `test-utils` | Testing utilities | portpicker, wiremock |

### Custom Feature Set

```toml
[dependencies]
opencode_rs = { 
    version = "0.1.2", 
    default-features = false, 
    features = ["http", "sse", "retry"] 
}
```

## Development Dependencies

For testing and development:

```toml
[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.6"
tracing-subscriber = "0.3"
anyhow = "1"
```

## Verifying Installation

Create a test program to verify installation:

```rust
// tests/installation_check.rs
use opencode_rs::{Client, Result};

#[tokio::test]
async fn test_client_creation() -> Result<()> {
    let client = Client::builder()
        .base_url("http://127.0.0.1:4096")
        .build()?;
    
    // Client created successfully
    Ok(())
}
```

Run the test:

```bash
cargo test test_client_creation
```

## Installing OpenCode Server

The SDK requires a running OpenCode server. Install OpenCode:

### Via NPM

```bash
npm install -g opencode
```

### Via NPX (No Install)

```bash
npx opencode
```

### Via Homebrew (macOS)

```bash
brew install opencode
```

### Starting the Server

Start OpenCode in your project directory:

```bash
# Start in current directory
opencode

# Start in specific directory
opencode /path/to/project

# Start with custom port
opencode --port 8080
```

## Docker Setup (Optional)

If you prefer containerized development:

```dockerfile
FROM rust:1.75

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

CMD ["./target/release/my-opencode-app"]
```

## IDE Setup

### VS Code

Recommended extensions:
- rust-analyzer
- Even Better TOML
- Error Lens
- CodeLLDB (for debugging)

### RustRover / IntelliJ

- Install the Rust plugin
- Enable Cargo features in project settings

### Vim / Neovim

With rust-analyzer LSP:

```lua
-- Neovim with lspconfig
require('lspconfig').rust_analyzer.setup({
  settings = {
    ['rust-analyzer'] = {
      cargo = {
        features = { "http", "sse" }
      }
    }
  }
})
```

## Updating

Update to the latest version:

```bash
cargo update -p opencode_rs
```

Or update Cargo.toml:

```toml
[dependencies]
opencode_rs = "0.1.3"  # Update version
```

Check for updates:

```bash
cargo outdated  # Requires cargo-outdated
```

## Troubleshooting Installation

### Compilation Errors

If you encounter compilation errors:

1. **Update Rust**:
   ```bash
   rustup update
   ```

2. **Clean build**:
   ```bash
   cargo clean
   cargo build
   ```

3. **Check feature flags**:
   Ensure required features are enabled

### Missing Dependencies

On Linux, you may need development libraries:

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# Fedora
sudo dnf install openssl-devel pkgconfig

# Arch
sudo pacman -S openssl pkgconf
```

### OpenSSL Issues

If you encounter OpenSSL linking errors:

```toml
[dependencies]
opencode_rs = "0.1.2"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
```

Or use the `native-tls-vendored` feature:

```toml
[dependencies]
reqwest = { version = "0.12", features = ["native-tls-vendored"] }
```

## Next Steps

- [Quick Start Guide](quickstart.md) - Build your first application
- [Client Configuration](../configuration/client-config.md) - Configure the SDK
- [Examples](../examples/basic-usage.md) - See more usage examples