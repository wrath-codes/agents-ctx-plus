# Tower Integration

Every Axum `Router` is a Tower `Service`. All Tower middleware works natively with Axum via the `.layer()` method.

---

## Adding Tower Layers

```rust
use axum::Router;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
    catch_panic::CatchPanicLayer,
};
use std::time::Duration;

let app = Router::new()
    .route("/", get(handler))
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CatchPanicLayer::new())
            .layer(CorsLayer::permissive())
            .layer(CompressionLayer::new())
            .layer(TimeoutLayer::new(Duration::from_secs(30)))
            .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
    );
```

---

## layer vs route_layer

| Method | Scope |
|--------|-------|
| `.layer(layer)` | Applies to all routes AND the fallback handler |
| `.route_layer(layer)` | Applies only to matched routes (NOT the fallback) |

```rust
let app = Router::new()
    .route("/protected", get(protected))
    .route_layer(auth_layer)        // only on /protected
    .route("/public", get(public))  // not affected by auth_layer
    .layer(TraceLayer::new_for_http()); // on everything including 404
```

---

## Per-Route Middleware

Apply middleware to specific route groups using nesting:

```rust
let authenticated = Router::new()
    .route("/dashboard", get(dashboard))
    .route("/settings", get(settings))
    .route_layer(RequireAuthorizationLayer::bearer("token"));

let public = Router::new()
    .route("/login", post(login))
    .route("/health", get(health));

let app = Router::new()
    .merge(authenticated)
    .merge(public)
    .layer(TraceLayer::new_for_http());
```

---

## Common tower-http Layers for Axum

```rust
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
    catch_panic::CatchPanicLayer,
    request_id::{SetRequestIdLayer, MakeRequestUuid},
    sensitive_headers::SetSensitiveHeadersLayer,
    normalize_path::NormalizePathLayer,
    services::ServeDir,
};

let app = Router::new()
    .route("/api/data", get(data))
    .nest_service("/static", ServeDir::new("assets"))
    .layer(
        ServiceBuilder::new()
            .layer(SetSensitiveHeadersLayer::new([header::AUTHORIZATION]))
            .layer(SetRequestIdLayer::new(
                HeaderName::from_static("x-request-id"),
                MakeRequestUuid,
            ))
            .layer(TraceLayer::new_for_http())
            .layer(CatchPanicLayer::new())
            .layer(CorsLayer::permissive())
            .layer(CompressionLayer::new())
            .layer(TimeoutLayer::new(Duration::from_secs(30)))
            .layer(RequestBodyLimitLayer::new(5 * 1024 * 1024))
    );
```

---

## Using Tower Services as Route Handlers

Use `_service` routing functions to mount Tower services directly:

```rust
use axum::routing::get_service;
use tower_http::services::ServeFile;

let app = Router::new()
    .route_service("/index.html", ServeFile::new("public/index.html"));
```

---

## See Also

- [from_fn Middleware](01-from-fn.md) — simpler Axum-specific middleware
- [Tower ServiceBuilder](../../tower/core/03-service-builder.md) — composing layers
- [tower-http Overview](../../tower/tower-http/01-overview.md) — available HTTP middleware
- [Router layer/route_layer](../core/01-router.md) — layer scoping
