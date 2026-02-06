# TLS

Tonic supports TLS via `rustls` for both server and client connections.

Requires feature flag: `tls` (or `tls-native-roots` / `tls-webpki-roots`)

---

## Server TLS

```rust
use tonic::transport::{Server, Identity, ServerTlsConfig};

let cert = std::fs::read("server.pem")?;
let key = std::fs::read("server.key")?;
let identity = Identity::from_pem(cert, key);

let tls_config = ServerTlsConfig::new().identity(identity);

Server::builder()
    .tls_config(tls_config)?
    .add_service(GreeterServer::new(greeter))
    .serve("[::1]:50051".parse()?)
    .await?;
```

### Mutual TLS (mTLS)

```rust
use tonic::transport::Certificate;

let ca_cert = std::fs::read("ca.pem")?;
let ca = Certificate::from_pem(ca_cert);

let tls_config = ServerTlsConfig::new()
    .identity(identity)
    .client_ca_root(ca);  // require client certificates
```

---

## Client TLS

```rust
use tonic::transport::{Channel, Certificate, ClientTlsConfig};

let ca_cert = std::fs::read("ca.pem")?;
let ca = Certificate::from_pem(ca_cert);

let tls_config = ClientTlsConfig::new()
    .ca_certificate(ca)
    .domain_name("example.com");

let channel = Channel::from_static("https://example.com:50051")
    .tls_config(tls_config)?
    .connect()
    .await?;

let client = GreeterClient::new(channel);
```

### Client Certificate (mTLS)

```rust
let client_cert = std::fs::read("client.pem")?;
let client_key = std::fs::read("client.key")?;
let client_identity = Identity::from_pem(client_cert, client_key);

let tls_config = ClientTlsConfig::new()
    .ca_certificate(ca)
    .identity(client_identity)
    .domain_name("example.com");
```

---

## TLS Types

| Type | Description |
|------|-------------|
| `Identity` | Server/client identity (certificate + private key) |
| `Certificate` | CA certificate for verification |
| `ServerTlsConfig` | TLS configuration for the server |
| `ClientTlsConfig` | TLS configuration for the client |

---

## Root Certificates

| Feature | Source |
|---------|--------|
| `tls-native-roots` | System's native certificate store |
| `tls-webpki-roots` | Mozilla's bundled root certificates |

```toml
[dependencies]
tonic = { version = "0.14", features = ["tls", "tls-native-roots"] }
```

---

## See Also

- [Server](../core/01-server.md) — server configuration
- [Client](../core/02-client.md) — client configuration
- [tokio-rustls](../../tokio/ecosystem/03-related-projects.md) — the underlying TLS implementation
