# tonic — Sub-Index

> Rust gRPC framework built on Tower and Hyper (12 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Quick Start](README.md#quick-start) · [Architecture](README.md#architecture) · [Essential Rust Types](README.md#essential-rust-types) · [Documentation Map](README.md#documentation-map) · [Quick Links](README.md#quick-links) · [Related References](README.md#related-references) · [External Resources](README.md#external-resources)|

### [getting-started](getting-started/)

|file|description|
|---|---|
|[01-protobuf-setup.md](getting-started/01-protobuf-setup.md)|Protobuf setup — .proto files, prost|
| |↳ [Prerequisites](getting-started/01-protobuf-setup.md#prerequisites) · [Writing a .proto File](getting-started/01-protobuf-setup.md#writing-a-proto-file) · [Project Structure](getting-started/01-protobuf-setup.md#project-structure) · [Cargo.toml](getting-started/01-protobuf-setup.md#cargotoml)|
|[02-build-rs.md](getting-started/02-build-rs.md)|build.rs — tonic-build code generation|
| |↳ [Basic build.rs](getting-started/02-build-rs.md#basic-buildrs) · [Configuration](getting-started/02-build-rs.md#configuration) · [Multiple Proto Files](getting-started/02-build-rs.md#multiple-proto-files) · [Adding Serde Support](getting-started/02-build-rs.md#adding-serde-support) · [Using Google APIs](getting-started/02-build-rs.md#using-google-apis) · [Including Generated Code](getting-started/02-build-rs.md#including-generated-code) · [tonic-prost-build](getting-started/02-build-rs.md#tonic-prost-build)|

### [core](core/)

|file|description|
|---|---|
|[01-server.md](core/01-server.md)|Server — service impl, Server::builder(), routing|
| |↳ [Server Builder API](core/01-server.md#server-builder-api) · [Basic Server](core/01-server.md#basic-server) · [Multiple Services](core/01-server.md#multiple-services) · [Server Configuration](core/01-server.md#server-configuration) · [With Tower Middleware](core/01-server.md#with-tower-middleware) · [Graceful Shutdown](core/01-server.md#graceful-shutdown) · [With Interceptor](core/01-server.md#with-interceptor)|
|[02-client.md](core/02-client.md)|Client — generated client, Channel, Endpoint|
| |↳ [Channel and Endpoint](core/02-client.md#channel-and-endpoint) · [Basic Client](core/02-client.md#basic-client) · [Lazy Connection](core/02-client.md#lazy-connection) · [Client Configuration](core/02-client.md#client-configuration) · [Adding Metadata to Requests](core/02-client.md#adding-metadata-to-requests) · [With Interceptor](core/02-client.md#with-interceptor) · [Load Balancing](core/02-client.md#load-balancing) · [Error Handling](core/02-client.md#error-handling)|
|[03-codegen.md](core/03-codegen.md)|Codegen — tonic-build options, attributes|
| |↳ [Generated Code Structure](core/03-codegen.md#generated-code-structure) · [Including Generated Code](core/03-codegen.md#including-generated-code) · [RPC Method Signatures](core/03-codegen.md#rpc-method-signatures)|

### [streaming](streaming/)

|file|description|
|---|---|
|[01-patterns.md](streaming/01-patterns.md)|Streaming patterns — unary, server/client/bidi streaming|
| |↳ [Pattern Overview](streaming/01-patterns.md#pattern-overview) · [Proto Definitions](streaming/01-patterns.md#proto-definitions) · [1. Unary RPC](streaming/01-patterns.md#1-unary-rpc) · [2. Server Streaming](streaming/01-patterns.md#2-server-streaming) · [3. Client Streaming](streaming/01-patterns.md#3-client-streaming) · [4. Bidirectional Streaming](streaming/01-patterns.md#4-bidirectional-streaming)|
|[02-implementation.md](streaming/02-implementation.md)|Implementation — Streaming<T>, ReceiverStream|
| |↳ [The Streaming Type](streaming/02-implementation.md#the-streaming-type) · [Creating Response Streams](streaming/02-implementation.md#creating-response-streams) · [Client-Side Streaming](streaming/02-implementation.md#client-side-streaming) · [Required Dependencies](streaming/02-implementation.md#required-dependencies)|

### [advanced](advanced/)

|file|description|
|---|---|
|[01-interceptors.md](advanced/01-interceptors.md)|Interceptors — request/response middleware|
| |↳ [Function Interceptors](advanced/01-interceptors.md#function-interceptors) · [Server-Side Interceptors](advanced/01-interceptors.md#server-side-interceptors) · [Client-Side Interceptors](advanced/01-interceptors.md#client-side-interceptors) · [Tower Layers as Interceptors](advanced/01-interceptors.md#tower-layers-as-interceptors) · [Injecting Extensions](advanced/01-interceptors.md#injecting-extensions)|
|[02-tls.md](advanced/02-tls.md)|TLS — rustls, native-tls configuration|
| |↳ [Server TLS](advanced/02-tls.md#server-tls) · [Client TLS](advanced/02-tls.md#client-tls) · [TLS Types](advanced/02-tls.md#tls-types) · [Root Certificates](advanced/02-tls.md#root-certificates)|
|[03-health-reflection.md](advanced/03-health-reflection.md)|Health/reflection — gRPC health check, server reflection|
| |↳ [Health Checking (tonic-health)](advanced/03-health-reflection.md#health-checking-tonic-health) · [Server Reflection (tonic-reflection)](advanced/03-health-reflection.md#server-reflection-tonic-reflection) · [Combined Setup](advanced/03-health-reflection.md#combined-setup)|
|[04-metadata-errors.md](advanced/04-metadata-errors.md)|Metadata/errors — headers, Status codes|
| |↳ [Request and Response](advanced/04-metadata-errors.md#request-and-response) · [MetadataMap](advanced/04-metadata-errors.md#metadatamap) · [Status (Error Type)](advanced/04-metadata-errors.md#status-error-type) · [Code Enum](advanced/04-metadata-errors.md#code-enum) · [Error Handling Patterns](advanced/04-metadata-errors.md#error-handling-patterns)|

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
*12 files · Related: [tower](../tower/INDEX.md), [tokio](../tokio/INDEX.md), [axum](../axum/INDEX.md)*
