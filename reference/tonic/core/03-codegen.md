# Code Generation

`tonic-build` generates Rust code from `.proto` files, creating server traits, client structs, and message types.

---

## Generated Code Structure

For a proto service:

```protobuf
service Greeter {
    rpc SayHello (HelloRequest) returns (HelloReply);
    rpc SayHelloStream (HelloRequest) returns (stream HelloReply);
}
```

The following Rust code is generated:

### Server Trait

```rust
#[tonic::async_trait]
pub trait Greeter: Send + Sync + 'static {
    async fn say_hello(
        &self,
        request: tonic::Request<HelloRequest>,
    ) -> Result<tonic::Response<HelloReply>, tonic::Status>;

    type SayHelloStreamStream: tonic::codegen::tokio_stream::Stream<
        Item = Result<HelloReply, tonic::Status>,
    > + Send + 'static;

    async fn say_hello_stream(
        &self,
        request: tonic::Request<HelloRequest>,
    ) -> Result<tonic::Response<Self::SayHelloStreamStream>, tonic::Status>;
}
```

### Server Wrapper

```rust
pub struct GreeterServer<T: Greeter> { /* ... */ }

impl<T: Greeter> GreeterServer<T> {
    pub fn new(inner: T) -> Self
    pub fn from_arc(inner: Arc<T>) -> Self
    pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
}
```

### Client Struct

```rust
pub struct GreeterClient<T> { /* ... */ }

impl<T> GreeterClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
{
    pub fn new(inner: T) -> Self
    pub fn with_interceptor<F>(inner: T, interceptor: F) -> GreeterClient<InterceptedService<T, F>>
    pub fn with_origin(inner: T, origin: Uri) -> Self

    pub async fn say_hello(
        &mut self,
        request: impl tonic::IntoRequest<HelloRequest>,
    ) -> Result<tonic::Response<HelloReply>, tonic::Status>

    pub async fn say_hello_stream(
        &mut self,
        request: impl tonic::IntoRequest<HelloRequest>,
    ) -> Result<tonic::Response<tonic::Streaming<HelloReply>>, tonic::Status>
}
```

### Message Types

```rust
#[derive(Clone, PartialEq, prost::Message)]
pub struct HelloRequest {
    #[prost(string, tag = "1")]
    pub name: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct HelloReply {
    #[prost(string, tag = "1")]
    pub message: String,
}
```

---

## Including Generated Code

```rust
pub mod hello {
    tonic::include_proto!("hello");
}

// Use the generated types
use hello::greeter_server::{Greeter, GreeterServer};
use hello::greeter_client::GreeterClient;
use hello::{HelloRequest, HelloReply};
```

The `include_proto!` macro includes the file from `$OUT_DIR` corresponding to the proto package name.

---

## RPC Method Signatures

| Proto Pattern | Server Trait Signature |
|---------------|----------------------|
| `rpc Foo(Req) returns (Resp)` | `async fn foo(&self, Request<Req>) -> Result<Response<Resp>, Status>` |
| `rpc Foo(Req) returns (stream Resp)` | `async fn foo(&self, Request<Req>) -> Result<Response<Self::FooStream>, Status>` |
| `rpc Foo(stream Req) returns (Resp)` | `async fn foo(&self, Request<Streaming<Req>>) -> Result<Response<Resp>, Status>` |
| `rpc Foo(stream Req) returns (stream Resp)` | `async fn foo(&self, Request<Streaming<Req>>) -> Result<Response<Self::FooStream>, Status>` |

---

## See Also

- [Build Configuration](../getting-started/02-build-rs.md) — configuring code generation
- [Server](01-server.md) — implementing the generated server trait
- [Client](02-client.md) — using the generated client struct
- [Streaming Patterns](../streaming/01-patterns.md) — all four RPC patterns
