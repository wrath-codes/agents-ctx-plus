# Architecture Overview

The OpenCode Rust SDK is designed as an HTTP-first, async-native library that provides comprehensive access to the OpenCode AI coding agent through its REST API and SSE streaming capabilities.

## Design Principles

### 1. HTTP-First Architecture

The SDK is built around OpenCode's HTTP REST API, making it:
- **Language agnostic** at the protocol level
- **Easy to debug** using standard HTTP tools
- **Compatible** with proxies, load balancers, and monitoring
- **Testable** with HTTP mocking libraries

### 2. Async-First Design

Built on Tokio for high-performance async I/O:
- Non-blocking API calls
- Efficient resource utilization
- Concurrent request handling
- Stream-based event processing

### 3. Type Safety

Comprehensive type system coverage:
- Zero-cost abstractions
- Compile-time correctness guarantees
- IDE autocomplete and documentation
- Strong error handling

### 4. Modularity

Clear separation of concerns:
- **Client**: High-level ergonomic API
- **HTTP**: Low-level HTTP operations
- **SSE**: Event streaming
- **Types**: Data models and serialization

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Application                         │
├─────────────────────────────────────────────────────────────┤
│                      opencode_rs                            │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   Client     │  │  HTTP APIs   │  │ SSE Stream   │       │
│  │   (High)     │  │   (Mid)      │  │   (Mid)      │       │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
│         │                 │                  │              │
│  ┌──────┴───────┐  ┌──────┴───────┐  ┌──────┴───────┐       │
│  │ ClientBuilder│  │  HttpClient  │  │ SseSubscriber│       │
│  └──────────────┘  └──────┬───────┘  └──────────────┘       │
│                           │                                 │
│  ┌────────────────────────┴────────────────────────┐        │
│  │                    Types                         │        │
│  │  Session, Message, Event, Tool, Error, etc.    │        │
│  └─────────────────────────────────────────────────┘        │
├─────────────────────────────────────────────────────────────┤
│                     reqwest / tokio                         │
├─────────────────────────────────────────────────────────────┤
│                   OpenCode Server                           │
│               (http://127.0.0.1:4096)                       │
└─────────────────────────────────────────────────────────────┘
```

## Module Structure

### Client Module (`client`)

The ergonomic, high-level API:

```rust
pub struct Client {
    http: HttpClient,
    config: HttpConfig,
}

impl Client {
    pub fn builder() -> ClientBuilder;
    pub async fn run_simple_text(&self, text: impl Into<String>) -> Result<Session>;
    pub fn sessions(&self) -> SessionsApi;
    pub fn messages(&self) -> MessagesApi;
    // ... other APIs
}
```

**Responsibilities:**
- Configuration management
- API endpoint access
- Convenience methods
- Resource lifecycle

### HTTP Module (`http`)

Low-level HTTP client and API modules:

```rust
pub struct HttpClient {
    client: reqwest::Client,
    config: HttpConfig,
}

// Individual API modules
mod sessions;   // Session CRUD operations
mod messages;   // Message sending/receiving
mod files;      // File operations
mod tools;      // Tool management
// ... etc.
```

**Responsibilities:**
- HTTP request/response handling
- Connection pooling
- Header management
- Error translation

### SSE Module (`sse`)

Server-Sent Events streaming:

```rust
pub struct SseSubscriber {
    base_url: String,
    directory: Option<String>,
    last_event_id: Arc<RwLock<Option<String>>>,
}

pub struct SseSubscription {
    receiver: mpsc::Receiver<Result<Event>>,
    cancel: CancellationToken,
}
```

**Responsibilities:**
- Event stream connection
- Automatic reconnection
- Event parsing and filtering
- Client-side filtering

### Types Module (`types`)

Comprehensive type definitions:

```
types/
├── api/         # API request/response types
├── session/     # Session types
├── message/     # Message and content part types
├── event/       # SSE event types (40+ variants)
├── tool/        # Tool and agent types
├── mcp/         # MCP types
├── file/        # File types
├── provider/    # LLM provider types
├── permission/  # Permission types
├── project/     # Project types
├── config/      # Configuration types
├── error/       # Error types
└── pty/         # PTY types
```

## Data Flow

### 1. Creating a Session

```
┌──────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  App     │────▶│   Client     │────▶│  SessionsApi │────▶│  HttpClient  │
│          │     │              │     │              │     │              │
│          │◀────│              │◀────│              │◀────│              │
└──────────┘     └──────────────┘     └──────────────┘     └──────────────┘
      │                                                      │
      │                                                      ▼
      │                                               ┌──────────────┐
      │                                               │ OpenCode API │
      │                                               │  /sessions   │
      │                                               └──────────────┘
      ▼
┌──────────┐
│ Session  │
│  Object  │
└──────────┘
```

### 2. Sending a Message

```
┌──────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  App     │────▶│   Client     │────▶│  MessagesApi │────▶│  HttpClient  │
│          │     │              │     │              │     │              │
│          │◀────│              │◀────│              │◀────│              │
└──────────┘     └──────────────┘     └──────────────┘     └──────────────┘
      │                                                      │
      │                                                      ▼
      │                                               ┌──────────────┐
      │                                               │ OpenCode API │
      │                                               │  /messages   │
      │                                               └──────────────┘
      ▼
┌──────────┐
│ Message  │
│  Object  │
└──────────┘
```

### 3. Receiving Events (SSE)

```
┌──────────────────────────────────────────────────────────────────────────┐
│                              Event Stream                                 │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────┐     ┌──────────────┐     ┌──────────────┐            │
│   │ SseSubscriber│────▶│  EventSource │────▶│ OpenCode SSE │            │
│   │              │     │ (reqwest-es) │     │  /event      │            │
│   └──────┬───────┘     └──────────────┘     └──────────────┘            │
│          │                                                              │
│          │ Parse & Filter                                                │
│          ▼                                                              │
│   ┌──────────────┐     ┌──────────────┐     ┌──────────────┐            │
│   │   Event      │────▶│ SseSubscription│───▶│  Your App    │            │
│   │   (typed)    │     │ (mpsc channel) │    │ (handler)    │            │
│   └──────────────┘     └──────────────┘     └──────────────┘            │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## Request Lifecycle

### 1. Client Building

```rust
let client = Client::builder()
    .base_url("http://127.0.0.1:4096")  // 1. Validate URL
    .directory("/path/to/project")       // 2. Store directory
    .timeout_secs(300)                   // 3. Configure timeout
    .build()?;                           // 4. Build reqwest::Client
```

### 2. API Call

```rust
// User code
let session = client.sessions().create(request).await?;

// Internal flow:
// 1. Serialize request body
// 2. Add headers (x-opencode-directory)
// 3. Send HTTP request via reqwest
// 4. Receive response
// 5. Deserialize response
// 6. Return typed result
```

### 3. Error Handling

```
HTTP Error ──▶ reqwest::Error ──▶ OpencodeError ──▶ Result<T, OpencodeError>
                    │                    │
                    ▼                    ▼
            Network issues      User-friendly error
            Timeout errors      with context
            Connection errors
```

## Concurrency Model

### Thread Safety

All SDK types are thread-safe:

```rust
// Client is Send + Sync
let client: Client = Client::builder().build()?;

// Can be shared across tasks
let client1 = client.clone();
let client2 = client.clone();

tokio::spawn(async move {
    client1.sessions().list().await;
});

tokio::spawn(async move {
    client2.messages().create(request).await;
});
```

### Async Patterns

**Sequential Operations:**
```rust
let session = client.sessions().create(req).await?;
let message = client.messages().create(msg_req).await?;
```

**Concurrent Operations:**
```rust
let (sessions, messages) = tokio::join!(
    client.sessions().list(),
    client.messages().list(&session_id)
);
```

**Streaming:**
```rust
let mut subscription = client.subscribe().await?;
while let Some(event) = subscription.recv().await {
    handle_event(event?).await?;
}
```

## Integration Points

### 1. HTTP Layer

- **Library**: reqwest
- **Features**: Connection pooling, timeouts, redirects
- **Customizable**: Through `HttpConfig`

### 2. Serialization

- **Library**: serde + serde_json
- **Strategy**: Strong typing with fallible deserialization
- **Features**: Custom serializers for complex types

### 3. Async Runtime

- **Library**: tokio
- **Compatibility**: Works with any tokio-based application
- **Features**: Full async/await support

### 4. Logging

- **Library**: tracing
- **Integration**: Automatic span creation for requests
- **Levels**: DEBUG for requests, TRACE for bodies

## Performance Characteristics

### Connection Pooling

HTTP connections are automatically pooled by reqwest:
- Default: Unlimited connections per host
- Idle timeout: 90 seconds
- Connection reuse for multiple requests

### Memory Usage

- **Client**: Lightweight (~few KB)
- **Events**: Streamed, not buffered
- **Sessions**: Loaded on demand

### Throughput

- **HTTP API**: Hundreds of requests/second
- **SSE**: Thousands of events/second
- **Concurrent Sessions**: Limited by OpenCode server

## Security Considerations

### 1. Local Server Only

By default, OpenCode binds to localhost (127.0.0.1), ensuring:
- No external network exposure
- Local-only access
- No authentication required (controlled by environment)

### 2. No Secrets in Code

The SDK doesn't handle API keys directly (OpenCode manages them):
- No credential storage
- No token management
- Relies on OpenCode's auth handling

### 3. HTTPS Support

While OpenCode defaults to HTTP locally, the SDK supports HTTPS:
```rust
let client = Client::builder()
    .base_url("https://opencode.example.com")
    .build()?;
```

## Extension Points

### 1. Custom HTTP Client

```rust
use reqwest::ClientBuilder;

let reqwest_client = ClientBuilder::new()
    .danger_accept_invalid_certs(true)  // Example: dev certs
    .build()?;

// Use with SDK (if exposed)
```

### 2. Middleware

Wrap the client for cross-cutting concerns:
```rust
pub struct LoggingClient {
    inner: Client,
}

impl LoggingClient {
    pub async fn create_session(&self, req: CreateSessionRequest) -> Result<Session> {
        tracing::info!("Creating session: {:?}", req);
        let result = self.inner.sessions().create(req).await;
        tracing::info!("Session created: {:?}", result);
        result
    }
}
```

## Comparison with Other Approaches

| Approach | Pros | Cons |
|----------|------|------|
| **HTTP REST** (this SDK) | Simple, debuggable, language-agnostic | Requires HTTP connection |
| **Native Binary** | Maximum performance | Platform-specific, complex |
| **WebSocket** | Bidirectional, lower latency | More complex protocol |
| **gRPC** | Efficient binary protocol | Requires protobuf, less debuggable |

The HTTP-first approach was chosen for:
1. **Simplicity**: Easy to understand and debug
2. **Compatibility**: Works with standard tools (curl, Postman)
3. **Flexibility**: Easy to proxy, cache, or load balance
4. **Interoperability**: Language-agnostic protocol