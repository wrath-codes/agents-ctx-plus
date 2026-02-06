# tonic — Sub-Index

> Rust gRPC framework built on Tower and Hyper (13 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [getting-started](getting-started/)

|file|description|
|---|---|
|[01-protobuf-setup.md](getting-started/01-protobuf-setup.md)|Protobuf setup — .proto files, prost|
|[02-build-rs.md](getting-started/02-build-rs.md)|build.rs — tonic-build code generation|

### [core](core/)

|file|description|
|---|---|
|[01-server.md](core/01-server.md)|Server — service impl, Server::builder(), routing|
|[02-client.md](core/02-client.md)|Client — generated client, Channel, Endpoint|
|[03-codegen.md](core/03-codegen.md)|Codegen — tonic-build options, attributes|

### [streaming](streaming/)

|file|description|
|---|---|
|[01-patterns.md](streaming/01-patterns.md)|Streaming patterns — unary, server/client/bidi streaming|
|[02-implementation.md](streaming/02-implementation.md)|Implementation — Streaming<T>, ReceiverStream|

### [advanced](advanced/)

|file|description|
|---|---|
|[01-interceptors.md](advanced/01-interceptors.md)|Interceptors — request/response middleware|
|[02-tls.md](advanced/02-tls.md)|TLS — rustls, native-tls configuration|
|[03-health-reflection.md](advanced/03-health-reflection.md)|Health/reflection — gRPC health check, server reflection|
|[04-metadata-errors.md](advanced/04-metadata-errors.md)|Metadata/errors — headers, Status codes|

### Key Patterns
```rust
// build.rs
tonic_build::compile_protos("proto/service.proto")?;
// server
Server::builder().add_service(MyServiceServer::new(svc)).serve(addr).await?;
// client
let mut client = MyServiceClient::connect("http://[::1]:50051").await?;
```

---
*13 files · Related: [tower](../tower/INDEX.md), [tokio](../tokio/INDEX.md), [axum](../axum/INDEX.md)*
