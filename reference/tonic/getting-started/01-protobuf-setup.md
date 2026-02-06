# Protobuf Setup

Tonic uses Protocol Buffers (protobuf) to define gRPC services. The `.proto` files are compiled into Rust code at build time.

---

## Prerequisites

### Install protoc

The Protocol Buffers compiler (`protoc`) must be installed:

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt install -y protobuf-compiler

# Windows (via Chocolatey)
choco install protoc

# Verify
protoc --version
```

---

## Writing a .proto File

```protobuf
// proto/hello.proto
syntax = "proto3";

package hello;

service Greeter {
    // Unary RPC
    rpc SayHello (HelloRequest) returns (HelloReply);

    // Server-streaming RPC
    rpc SayHelloStream (HelloRequest) returns (stream HelloReply);
}

message HelloRequest {
    string name = 1;
}

message HelloReply {
    string message = 1;
}
```

### Service Definition

| Keyword | Meaning |
|---------|---------|
| `syntax = "proto3"` | Use proto3 syntax (required) |
| `package hello` | Rust module path for generated code |
| `service Greeter` | Defines a gRPC service |
| `rpc Name (Req) returns (Resp)` | Unary RPC method |
| `rpc Name (stream Req) returns (Resp)` | Client-streaming |
| `rpc Name (Req) returns (stream Resp)` | Server-streaming |
| `rpc Name (stream Req) returns (stream Resp)` | Bidirectional streaming |

### Message Types

```protobuf
message User {
    uint64 id = 1;
    string name = 2;
    string email = 3;
    repeated string tags = 4;           // Vec<String>
    optional string nickname = 5;       // Option<String>
    google.protobuf.Timestamp created_at = 6;
}

enum UserRole {
    UNKNOWN = 0;
    ADMIN = 1;
    USER = 2;
}
```

| Proto Type | Rust Type |
|-----------|-----------|
| `string` | `String` |
| `bytes` | `Vec<u8>` / `Bytes` |
| `bool` | `bool` |
| `int32` / `int64` | `i32` / `i64` |
| `uint32` / `uint64` | `u32` / `u64` |
| `float` / `double` | `f32` / `f64` |
| `repeated T` | `Vec<T>` |
| `optional T` | `Option<T>` |
| `map<K, V>` | `HashMap<K, V>` |
| `enum` | Rust `enum` (as `i32`) |
| `message` | Rust `struct` |

---

## Project Structure

```
my-grpc-project/
├── Cargo.toml
├── build.rs
├── proto/
│   └── hello.proto
└── src/
    ├── main.rs          # or server.rs / client.rs
    └── lib.rs
```

---

## Cargo.toml

```toml
[package]
name = "my-grpc-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tonic = "0.14"
prost = "0.13"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tonic-build = "0.14"
```

---

## See Also

- [Build Configuration](02-build-rs.md) — configuring `build.rs` for code generation
- [Code Generation](../core/03-codegen.md) — understanding generated code
- [Server](../core/01-server.md) — implementing a gRPC server
