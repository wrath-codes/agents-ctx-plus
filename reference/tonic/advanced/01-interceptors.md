# Interceptors

Interceptors are middleware for gRPC requests and responses, built on Tower's `Service` trait.

---

## Function Interceptors

The simplest form — a function that receives and returns a `Request<()>`:

```rust
fn my_interceptor(req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
    // Inspect or modify the request
    println!("Intercepting: {:?}", req.metadata());
    Ok(req)
}
```

---

## Server-Side Interceptors

```rust
use tonic::transport::Server;

fn auth_interceptor(req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
    match req.metadata().get("authorization") {
        Some(token) if token == "Bearer valid-token" => Ok(req),
        _ => Err(tonic::Status::unauthenticated("Invalid token")),
    }
}

Server::builder()
    .add_service(GreeterServer::with_interceptor(
        MyGreeter::default(),
        auth_interceptor,
    ))
    .serve(addr)
    .await?;
```

---

## Client-Side Interceptors

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

## Tower Layers as Interceptors

For more complex middleware, use Tower layers directly:

```rust
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

Server::builder()
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_grpc())
            .timeout(Duration::from_secs(30))
    )
    .add_service(GreeterServer::new(greeter))
    .serve(addr)
    .await?;
```

---

## Injecting Extensions

Pass data from interceptors to service handlers via request extensions:

```rust
#[derive(Clone)]
struct AuthenticatedUser {
    id: u64,
    name: String,
}

fn auth_interceptor(mut req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
    let token = req.metadata().get("authorization")
        .ok_or_else(|| tonic::Status::unauthenticated("Missing token"))?;

    let user = validate_token(token)
        .map_err(|_| tonic::Status::unauthenticated("Invalid token"))?;

    req.extensions_mut().insert(user);
    Ok(req)
}

// In the service handler
async fn say_hello(
    &self,
    request: Request<HelloRequest>,
) -> Result<Response<HelloReply>, Status> {
    let user = request.extensions().get::<AuthenticatedUser>()
        .ok_or_else(|| Status::internal("Missing user extension"))?;
    // ...
}
```

---

## See Also

- [Server](../core/01-server.md) — server setup
- [Client](../core/02-client.md) — client setup
- [Tower Service Trait](../../tower/core/01-service-trait.md) — the underlying abstraction
- [Tower Middleware](../../tower/middleware/) — built-in Tower middleware
- [Metadata & Errors](04-metadata-errors.md) — working with metadata
