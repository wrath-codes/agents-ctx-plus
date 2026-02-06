# Build Configuration

Tonic generates Rust code from `.proto` files at build time using `tonic-build` in a `build.rs` script.

---

## Basic build.rs

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/hello.proto")?;
    Ok(())
}
```

This compiles the `.proto` file and writes generated Rust code to `$OUT_DIR`.

---

## Configuration

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)              // generate server code
        .build_client(true)              // generate client code
        .build_transport(true)           // generate transport helpers
        .out_dir("src/generated")        // custom output directory
        .compile_well_known_types(true)  // include google.protobuf types
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["proto/hello.proto", "proto/users.proto"],
            &["proto/"],  // include paths
        )?;
    Ok(())
}
```

### Builder Methods

| Method | Description |
|--------|-------------|
| `.build_server(bool)` | Generate server trait and service wrapper |
| `.build_client(bool)` | Generate client struct with RPC methods |
| `.build_transport(bool)` | Generate transport-level helpers |
| `.out_dir(path)` | Write generated code to a specific directory |
| `.compile_well_known_types(bool)` | Include google.protobuf types |
| `.type_attribute(path, attr)` | Add attributes to generated types |
| `.field_attribute(path, attr)` | Add attributes to generated fields |
| `.server_mod_attribute(path, attr)` | Add attributes to server modules |
| `.client_mod_attribute(path, attr)` | Add attributes to client modules |
| `.extern_path(proto, rust)` | Map proto paths to existing Rust types |
| `.protoc_arg(arg)` | Pass additional arguments to protoc |

---

## Multiple Proto Files

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile_protos(
            &[
                "proto/users.proto",
                "proto/items.proto",
                "proto/orders.proto",
            ],
            &["proto/"],
        )?;
    Ok(())
}
```

---

## Adding Serde Support

```rust
tonic_build::configure()
    .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
    .compile_protos(&["proto/hello.proto"], &["proto/"])?;
```

---

## Using Google APIs

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &["proto/googleapis/google/pubsub/v1/pubsub.proto"],
            &["proto/googleapis"],
        )?;
    Ok(())
}
```

---

## Including Generated Code

In your Rust source:

```rust
// Default: include from OUT_DIR
pub mod hello {
    tonic::include_proto!("hello");
}

// Or if using out_dir("src/generated"):
mod generated {
    include!("generated/hello.rs");
}
```

---

## tonic-prost-build

For newer versions, `tonic-prost-build` is the preferred crate:

```toml
[build-dependencies]
tonic-prost-build = "0.14"
```

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/hello.proto")?;
    Ok(())
}
```

---

## See Also

- [Protobuf Setup](01-protobuf-setup.md) — writing `.proto` files
- [Code Generation](../core/03-codegen.md) — understanding generated code structure
- [Server](../core/01-server.md) — using generated server code
- [Client](../core/02-client.md) — using generated client code
