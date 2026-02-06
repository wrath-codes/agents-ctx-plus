# Health Checking and Server Reflection

Tonic provides companion crates for the standard gRPC health checking and server reflection protocols.

---

## Health Checking (tonic-health)

Implements the [gRPC Health Checking Protocol](https://grpc.io/docs/guides/health-checking/).

### Cargo.toml

```toml
[dependencies]
tonic-health = "0.14"
```

### Usage

```rust
use tonic_health::server::health_reporter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut health_reporter, health_service) = health_reporter();

    // Set service health status
    health_reporter
        .set_serving::<GreeterServer<MyGreeter>>()
        .await;

    Server::builder()
        .add_service(health_service)
        .add_service(GreeterServer::new(MyGreeter::default()))
        .serve("[::1]:50051".parse()?)
        .await?;

    Ok(())
}
```

### Health Status

```rust
// Mark service as serving
health_reporter.set_serving::<GreeterServer<MyGreeter>>().await;

// Mark service as not serving
health_reporter.set_not_serving::<GreeterServer<MyGreeter>>().await;

// Clear service status
health_reporter.clear_service_status("hello.Greeter").await;
```

### Client-Side Health Check

```rust
use tonic_health::pb::health_client::HealthClient;
use tonic_health::pb::HealthCheckRequest;

let mut client = HealthClient::connect("http://[::1]:50051").await?;
let request = HealthCheckRequest {
    service: "hello.Greeter".to_string(),
};
let response = client.check(request).await?;
println!("Status: {:?}", response.into_inner().status);
```

---

## Server Reflection (tonic-reflection)

Enables gRPC server reflection, allowing tools like `grpcurl` and `grpcui` to discover services without `.proto` files.

### Cargo.toml

```toml
[dependencies]
tonic-reflection = "0.14"

[build-dependencies]
tonic-build = { version = "0.14", features = ["reflection"] }
```

### build.rs

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("hello_descriptor.bin"))
        .compile_protos(&["proto/hello.proto"], &["proto/"])?;

    Ok(())
}
```

### Server Setup

```rust
use tonic_reflection::server::Builder as ReflectionBuilder;

const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(
    concat!(env!("OUT_DIR"), "/hello_descriptor.bin")
);

let reflection_service = ReflectionBuilder::configure()
    .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
    .build_v1()?;

Server::builder()
    .add_service(reflection_service)
    .add_service(GreeterServer::new(MyGreeter::default()))
    .serve(addr)
    .await?;
```

### Using with grpcurl

```bash
# List services
grpcurl -plaintext localhost:50051 list

# Describe a service
grpcurl -plaintext localhost:50051 describe hello.Greeter

# Call a method
grpcurl -plaintext -d '{"name": "World"}' localhost:50051 hello.Greeter/SayHello
```

---

## Combined Setup

```rust
let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
health_reporter.set_serving::<GreeterServer<MyGreeter>>().await;

let reflection_service = tonic_reflection::server::Builder::configure()
    .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
    .build_v1()?;

Server::builder()
    .add_service(health_service)
    .add_service(reflection_service)
    .add_service(GreeterServer::new(MyGreeter::default()))
    .serve(addr)
    .await?;
```

---

## See Also

- [Server](../core/01-server.md) — server setup
- [Interceptors](01-interceptors.md) — middleware
- [Metadata & Errors](04-metadata-errors.md) — gRPC metadata and status
