# Getting Started with Tracing

## Overview

The `tracing` crate provides structured, event-based diagnostics designed for async code. Unlike traditional logging, tracing understands **spans** (units of work with a beginning and end) and **events** (points in time), making it possible to follow execution across `.await` points and task switches.

## Setup

### Dependencies

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Basic Subscriber

A subscriber collects and records trace data. `FmtSubscriber` writes formatted output to stdout:

```rust
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt::init();

    // Now tracing macros will produce output
    tracing::info!("application started");
}
```

### Custom Configuration

```rust
use tracing_subscriber::fmt;

fn main() {
    fmt()
        .compact()                    // Compact output format
        .with_file(true)              // Include file path
        .with_line_number(true)       // Include line number
        .with_thread_ids(true)        // Include thread IDs
        .with_target(false)           // Hide target module
        .with_env_filter("my_app=debug,tokio=info") // Filter by level
        .init();
}
```

### Manual Subscriber Setup

```rust
use tracing::subscriber::set_global_default;
use tracing_subscriber::FmtSubscriber;

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    set_global_default(subscriber)
        .expect("setting default subscriber failed");
}
```

## Emitting Spans

### `#[tracing::instrument]` Attribute

Automatically creates a span for each function call. Function arguments are recorded as span fields.

```rust
use tracing::instrument;

#[instrument]
async fn process_request(id: u64, payload: &str) {
    // Span "process_request{id=42, payload="hello"}" is active here
    tracing::info!("processing");
    handle_payload(payload).await;
}
```

### Configuring `#[instrument]`

```rust
// Skip specific arguments (e.g., large or sensitive data)
#[instrument(skip(db_pool, password))]
async fn authenticate(username: &str, password: &str, db_pool: &Pool) -> bool {
    // password and db_pool are NOT recorded
    tracing::info!("authenticating user");
    true
}

// Custom span name
#[instrument(name = "handle_connection")]
async fn process(stream: TcpStream) {
    // Span is named "handle_connection" instead of "process"
}

// Add extra fields
#[instrument(fields(request_id = %uuid::Uuid::new_v4()))]
async fn handle_request() {
    // request_id field added to span
}

// Set level
#[instrument(level = "debug")]
async fn internal_detail() {
    // Span is at DEBUG level
}

// Skip all arguments, record return value
#[instrument(skip_all, ret)]
async fn compute(x: i32, y: i32) -> i32 {
    x + y
}
```

## Emitting Events

Events are discrete points in time within a span:

```rust
use tracing::{info, warn, error, debug, trace};

async fn process() {
    trace!("entering process");
    debug!("checking preconditions");
    info!("processing started");
    warn!("resource usage high");
    error!("operation failed");
}
```

### Structured Key-Value Logging

Events can include structured fields:

```rust
use tracing::{info, warn, error};

fn handle_connection(addr: &str, port: u16) {
    info!(address = addr, port, "connection established");

    // Display formatting with %
    let err = std::io::Error::new(std::io::ErrorKind::Other, "timeout");
    warn!(%err, "connection issue");

    // Debug formatting with ?
    let config = vec!["a", "b"];
    info!(?config, "loaded configuration");

    // Mixed
    error!(
        %err,
        retries = 3,
        "operation failed after retries"
    );
}
```

Output:
```
INFO handle_connection: connection established address="127.0.0.1" port=8080
WARN handle_connection: connection issue err=timeout
INFO handle_connection: loaded configuration config=["a", "b"]
ERROR handle_connection: operation failed after retries err=timeout retries=3
```

## Layers

Compose multiple subscriber behaviors using layers:

```rust
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())  // Filter layer
        .with(fmt::layer().compact())          // Stdout formatting layer
        // .with(opentelemetry_layer)          // Optional: export to Jaeger/OTLP
        .init();
}
```

### EnvFilter

Control verbosity with the `RUST_LOG` environment variable:

```bash
# Show debug for your crate, info for everything else
RUST_LOG="my_app=debug,info" cargo run

# Show trace for a specific module
RUST_LOG="my_app::db=trace" cargo run

# Multiple targets
RUST_LOG="my_app=debug,tower_http=debug,tokio=info" cargo run
```

## tokio-console

A runtime debugging tool for inspecting Tokio tasks, resources, and async operations in real time.

### Setup

```toml
[dependencies]
console-subscriber = "0.4"
tokio = { version = "1", features = ["full", "tracing"] }
```

```rust
fn main() {
    console_subscriber::init();  // Replaces tracing_subscriber::fmt::init()

    // ... rest of your application
}
```

### Usage

```bash
# Install the console CLI
cargo install tokio-console

# Run your application (with RUSTFLAGS to enable tokio's tracing instrumentation)
RUSTFLAGS="--cfg tokio_unstable" cargo run

# In another terminal, connect the console
tokio-console
```

The console shows:
- Active tasks and their state (idle, running, scheduled)
- Task poll durations and waker counts
- Resource usage (timers, I/O handles)
- Warnings for potential issues (tasks that haven't been polled)

## See Also

- [Graceful Shutdown](./02-graceful-shutdown.md) — observing shutdown with tracing
- [Bridging Sync Code](./01-bridging-sync-code.md) — tracing across sync/async boundaries
