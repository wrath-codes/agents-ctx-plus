# Server

The Tonic server uses `transport::Server` to bind to an address and serve gRPC services.

---

## Server Builder API

```rust
impl Server {
    pub fn builder() -> Self
    pub fn tls_config(self, tls_config: ServerTlsConfig) -> Result<Self, Error>
    pub fn concurrency_limit_per_connection(self, limit: usize) -> Self
    pub fn timeout(self, timeout: Duration) -> Self
    pub fn initial_stream_window_size(self, sz: impl Into<Option<u32>>) -> Self
    pub fn initial_connection_window_size(self, sz: impl Into<Option<u32>>) -> Self
    pub fn max_concurrent_streams(self, max: impl Into<Option<u32>>) -> Self
    pub fn tcp_keepalive(self, tcp_keepalive: Option<Duration>) -> Self
    pub fn tcp_nodelay(self, enabled: bool) -> Self
    pub fn http2_keepalive_interval(self, interval: Option<Duration>) -> Self
    pub fn http2_keepalive_timeout(self, timeout: Option<Duration>) -> Self
    pub fn max_frame_size(self, sz: impl Into<Option<u32>>) -> Self
    pub fn accept_http1(self, accept_http1: bool) -> Self
    pub fn layer<L>(self, layer: L) -> Self
    pub fn add_service<S>(self, svc: S) -> Router
    pub fn add_optional_service<S>(self, svc: Option<S>) -> Router
}
```

---

## Basic Server

```rust
use tonic::transport::Server;

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
        request: tonic::Request<HelloRequest>,
    ) -> Result<tonic::Response<HelloReply>, tonic::Status> {
        let name = request.into_inner().name;
        let reply = HelloReply {
            message: format!("Hello {}!", name),
        };
        Ok(tonic::Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
```

---

## Multiple Services

```rust
Server::builder()
    .add_service(GreeterServer::new(greeter))
    .add_service(UserServiceServer::new(user_service))
    .add_service(ItemServiceServer::new(item_service))
    .serve(addr)
    .await?;
```

---

## Server Configuration

```rust
use std::time::Duration;

Server::builder()
    .timeout(Duration::from_secs(30))
    .concurrency_limit_per_connection(256)
    .tcp_keepalive(Some(Duration::from_secs(60)))
    .tcp_nodelay(true)
    .http2_keepalive_interval(Some(Duration::from_secs(20)))
    .http2_keepalive_timeout(Some(Duration::from_secs(5)))
    .max_concurrent_streams(Some(200))
    .max_frame_size(Some(16 * 1024))
    .add_service(GreeterServer::new(greeter))
    .serve(addr)
    .await?;
```

---

## With Tower Middleware

```rust
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

Server::builder()
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_grpc())
    )
    .add_service(GreeterServer::new(greeter))
    .serve(addr)
    .await?;
```

---

## Graceful Shutdown

```rust
use tokio::signal;

let (tx, rx) = tokio::sync::oneshot::channel::<()>();

tokio::spawn(async move {
    signal::ctrl_c().await.unwrap();
    tx.send(()).unwrap();
});

Server::builder()
    .add_service(GreeterServer::new(greeter))
    .serve_with_shutdown(addr, async { rx.await.ok(); })
    .await?;
```

---

## With Interceptor

```rust
use tonic::service::interceptor;

fn auth_interceptor(req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
    match req.metadata().get("authorization") {
        Some(token) if token == "Bearer valid-token" => Ok(req),
        _ => Err(tonic::Status::unauthenticated("Invalid token")),
    }
}

Server::builder()
    .add_service(GreeterServer::with_interceptor(greeter, auth_interceptor))
    .serve(addr)
    .await?;
```

---

## See Also

- [Client](02-client.md) — connecting to a gRPC server
- [Code Generation](03-codegen.md) — understanding generated server traits
- [Interceptors](../advanced/01-interceptors.md) — request/response middleware
- [TLS](../advanced/02-tls.md) — TLS configuration
- [Health & Reflection](../advanced/03-health-reflection.md) — health checking and reflection
