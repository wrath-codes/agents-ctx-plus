# Client

Tonic clients connect to gRPC servers via `Channel` (the connection) and generated client stubs (with methods for each RPC).

---

## Channel and Endpoint

```rust
impl Endpoint {
    pub fn from_static(s: &'static str) -> Self
    pub fn from_shared(s: impl Into<Bytes>) -> Result<Self, Error>
    pub async fn connect(&self) -> Result<Channel, Error>
    pub fn connect_lazy(&self) -> Channel
    pub fn timeout(self, dur: Duration) -> Self
    pub fn connect_timeout(self, dur: Duration) -> Self
    pub fn concurrency_limit(self, limit: usize) -> Self
    pub fn rate_limit(self, limit: u64, duration: Duration) -> Self
    pub fn tls_config(self, tls_config: ClientTlsConfig) -> Result<Self, Error>
    pub fn tcp_keepalive(self, interval: Option<Duration>) -> Self
    pub fn tcp_nodelay(self, enabled: bool) -> Self
    pub fn http2_keep_alive_interval(self, interval: Duration) -> Self
    pub fn keep_alive_timeout(self, duration: Duration) -> Self
    pub fn keep_alive_while_idle(self, enabled: bool) -> Self
    pub fn initial_stream_window_size(self, sz: impl Into<Option<u32>>) -> Self
    pub fn initial_connection_window_size(self, sz: impl Into<Option<u32>>) -> Self
}
```

`Channel` implements `tower::Service<http::Request<BoxBody>>` and is `Clone`.

---

## Basic Client

```rust
use tonic::transport::Channel;

pub mod hello {
    tonic::include_proto!("hello");
}

use hello::greeter_client::GreeterClient;
use hello::HelloRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(HelloRequest {
        name: "World".into(),
    });

    let response = client.say_hello(request).await?;
    println!("Response: {}", response.into_inner().message);

    Ok(())
}
```

---

## Lazy Connection

Connect on first use rather than eagerly:

```rust
let channel = Channel::from_static("http://[::1]:50051").connect_lazy();
let mut client = GreeterClient::new(channel);
```

---

## Client Configuration

```rust
use std::time::Duration;
use tonic::transport::Endpoint;

let endpoint = Endpoint::from_static("http://[::1]:50051")
    .timeout(Duration::from_secs(10))
    .connect_timeout(Duration::from_secs(5))
    .concurrency_limit(256)
    .tcp_keepalive(Some(Duration::from_secs(60)))
    .tcp_nodelay(true)
    .http2_keep_alive_interval(Duration::from_secs(20))
    .keep_alive_timeout(Duration::from_secs(5))
    .keep_alive_while_idle(true);

let channel = endpoint.connect().await?;
let client = GreeterClient::new(channel);
```

---

## Adding Metadata to Requests

```rust
use tonic::Request;

let mut request = Request::new(HelloRequest { name: "World".into() });
request.metadata_mut().insert("authorization", "Bearer my-token".parse().unwrap());
request.metadata_mut().insert("x-request-id", "req-123".parse().unwrap());

let response = client.say_hello(request).await?;
```

---

## With Interceptor

```rust
fn add_auth(mut req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
    req.metadata_mut().insert(
        "authorization",
        "Bearer my-token".parse().unwrap(),
    );
    Ok(req)
}

let channel = Channel::from_static("http://[::1]:50051").connect().await?;
let client = GreeterClient::with_interceptor(channel, add_auth);
```

---

## Load Balancing

Connect to multiple endpoints:

```rust
let channel = Channel::balance_list(
    vec![
        "http://server1:50051".parse().unwrap(),
        "http://server2:50051".parse().unwrap(),
        "http://server3:50051".parse().unwrap(),
    ].into_iter(),
);

let client = GreeterClient::new(channel);
```

---

## Error Handling

Client calls return `Result<Response<T>, Status>`:

```rust
match client.say_hello(request).await {
    Ok(response) => {
        let reply = response.into_inner();
        println!("Message: {}", reply.message);
    }
    Err(status) => {
        eprintln!("gRPC error: {} - {}", status.code(), status.message());
        match status.code() {
            tonic::Code::NotFound => { /* handle not found */ }
            tonic::Code::Unauthenticated => { /* handle auth */ }
            _ => { /* handle other errors */ }
        }
    }
}
```

---

## See Also

- [Server](01-server.md) — implementing a gRPC server
- [Code Generation](03-codegen.md) — understanding generated client code
- [Interceptors](../advanced/01-interceptors.md) — request middleware
- [TLS](../advanced/02-tls.md) — secure connections
- [Metadata & Errors](../advanced/04-metadata-errors.md) — metadata and Status
