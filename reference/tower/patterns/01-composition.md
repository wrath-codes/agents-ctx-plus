# Composition Patterns

Strategies for composing Tower services and layers into production-grade middleware stacks.

---

## Layer Ordering

The most important concept when composing layers: **the first layer in the chain is the outermost**, meaning it processes the request first and the response last.

```
ServiceBuilder::new()
    .layer(A)    // outermost: request first, response last
    .layer(B)    // middle
    .layer(C)    // innermost: request last, response first
    .service(svc)

Request  flow: A → B → C → svc
Response flow: A ← B ← C ← svc
```

### Recommended Ordering

```rust
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
    compression::CompressionLayer,
    catch_panic::CatchPanicLayer,
    limit::RequestBodyLimitLayer,
};
use std::time::Duration;

let service = ServiceBuilder::new()
    // 1. Tracing (outermost — captures total latency)
    .layer(TraceLayer::new_for_http())
    // 2. Catch panics (before they propagate)
    .layer(CatchPanicLayer::new())
    // 3. CORS (early rejection of disallowed origins)
    .layer(CorsLayer::permissive())
    // 4. Rate limiting (protect downstream)
    .rate_limit(1000, Duration::from_secs(1))
    // 5. Concurrency limiting
    .concurrency_limit(100)
    // 6. Timeout
    .timeout(Duration::from_secs(30))
    // 7. Body limit (before decompression/parsing)
    .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
    // 8. Compression (innermost — compress just before sending)
    .layer(CompressionLayer::new())
    .service(my_service);
```

---

## Per-Route vs Global Middleware

### Global Middleware (applies to all routes)

```rust
use axum::Router;

let app = Router::new()
    .route("/api/users", get(list_users))
    .route("/api/items", get(list_items))
    .layer(TraceLayer::new_for_http()); // applies to all routes
```

### Per-Route Middleware (applies to specific routes)

```rust
use axum::Router;
use axum::routing::get;

let api = Router::new()
    .route("/users", get(list_users))
    .route_layer(RequireAuthorizationLayer::bearer("token"));

let public = Router::new()
    .route("/health", get(health_check));

let app = Router::new()
    .nest("/api", api)
    .merge(public)
    .layer(TraceLayer::new_for_http());
```

---

## Conditional Middleware

Use `option_layer` for middleware that should only be applied sometimes:

```rust
use tower::ServiceBuilder;

let timeout_layer = if config.enable_timeout {
    Some(TimeoutLayer::new(Duration::from_secs(30)))
} else {
    None
};

let service = ServiceBuilder::new()
    .option_layer(timeout_layer)
    .service(my_service);
```

---

## Type Erasure

When you need to store services in collections or return different services from branches:

```rust
use tower::util::{BoxService, BoxCloneService};

// Type-erased service (not Clone)
let boxed: BoxService<Request, Response, Error> = service.boxed();

// Type-erased and Clone
let boxed: BoxCloneService<Request, Response, Error> = service.boxed_clone();
```

---

## Error Handling in Middleware Stacks

Each layer may introduce its own error type. Use `map_err` or `HandleError` to unify error types:

```rust
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .map_err(|err| {
        // Convert timeout::Elapsed to your error type
        MyError::Timeout
    })
    .service(my_service);
```

---

## See Also

- [Custom Middleware](02-custom-middleware.md) — writing your own layers
- [ServiceBuilder](../core/03-service-builder.md) — the builder API
- [Layer Trait](../core/02-layer-trait.md) — how layers work
- [Axum Middleware](../../axum/middleware/02-tower-integration.md) — Tower layers in Axum
