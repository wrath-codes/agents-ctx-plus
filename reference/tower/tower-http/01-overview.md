# tower-http Overview

`tower-http` provides HTTP-specific Tower middleware. It works with any service that handles `http::Request<B>` and returns `http::Response<B>`, making it compatible with Hyper, Axum, Tonic, and Warp.

---

## Crate Info

| | |
|---|---|
| **Crate** | [`tower-http`](https://crates.io/crates/tower-http) |
| **Docs** | [docs.rs/tower-http](https://docs.rs/tower-http) |
| **Repository** | [tower-rs/tower-http](https://github.com/tower-rs/tower-http) |

---

## All Available Middleware

| Middleware | Feature | Description |
|-----------|---------|-------------|
| `AddExtensionLayer` | `add-extension` | Inject values into request extensions |
| `AuthLayer` / `RequireAuthorizationLayer` | `auth` | Authentication and authorization |
| `CatchPanicLayer` | `catch-panic` | Convert panics into 500 responses |
| `CompressionLayer` | `compression-*` | Compress response bodies (gzip, br, deflate, zstd) |
| `CorsLayer` | `cors` | Cross-Origin Resource Sharing headers |
| `DecompressionLayer` | `decompression-*` | Decompress request/response bodies |
| `FollowRedirectLayer` | `follow-redirect` | Follow HTTP redirects (client-side) |
| `MapRequestBodyLayer` | `map-request-body` | Transform request bodies |
| `MapResponseBodyLayer` | `map-response-body` | Transform response bodies |
| `MetricsLayer` / `InFlightRequestsLayer` | `metrics` | Request metrics and in-flight tracking |
| `NormalizePathLayer` | `normalize-path` | Normalize URL paths (trailing slashes, etc.) |
| `PropagateHeaderLayer` | `propagate-header` | Copy headers from request to response |
| `RequestBodyLimitLayer` | `limit` | Limit request body size |
| `RequestIdLayer` / `SetRequestIdLayer` | `request-id` | Generate and propagate request IDs |
| `SensitiveHeadersLayer` | `sensitive-headers` | Mark headers as sensitive (redacted in logs) |
| `ServeDir` / `ServeFile` | `fs` | Static file serving |
| `SetRequestHeaderLayer` | `set-header` | Add/override request headers |
| `SetResponseHeaderLayer` | `set-header` | Add/override response headers |
| `SetStatusLayer` | `set-status` | Override response status codes |
| `TimeoutLayer` | `timeout` | HTTP-aware request timeout |
| `TraceLayer` | `trace` | Request/response tracing with `tracing` |
| `ValidateRequestHeaderLayer` | `validate-request` | Validate incoming request headers |

---

## Extension Traits

| Trait | Description |
|-------|-------------|
| `ServiceBuilderExt` | Adds tower-http methods to `tower::ServiceBuilder` |
| `ServiceExt` | Adds tower-http methods to any `Service` |

---

## Quick Example

```rust
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
};
use std::time::Duration;

let middleware = ServiceBuilder::new()
    .layer(TraceLayer::new_for_http())
    .layer(CorsLayer::permissive())
    .layer(CompressionLayer::new())
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(RequestBodyLimitLayer::new(1024 * 1024)); // 1 MB
```

---

## See Also

- [Request & Response Middleware](02-request-response.md) — CORS, compression, headers, body limits
- [Observability Middleware](03-observability.md) — tracing, catch-panic, request IDs, metrics
- [ServiceBuilder](../core/03-service-builder.md) — composing layers
- [Axum Middleware](../../axum/middleware/02-tower-integration.md) — using tower-http with Axum
