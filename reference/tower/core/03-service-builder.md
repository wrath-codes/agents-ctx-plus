# ServiceBuilder

`ServiceBuilder` provides a declarative, chainable API for composing multiple Tower layers into a service stack. It is the primary way to assemble middleware in Tower applications.

---

## API Reference

```rust
impl ServiceBuilder<L> {
    pub fn new() -> ServiceBuilder<Identity>
    pub fn layer<T>(self, layer: T) -> ServiceBuilder<Stack<T, L>>
    pub fn layer_fn<F>(self, f: F) -> ServiceBuilder<Stack<LayerFn<F>, L>>
    pub fn option_layer<T>(self, layer: Option<T>) -> ServiceBuilder<Stack<Either<T, Identity>, L>>
    pub fn map_request<F>(self, f: F) -> ServiceBuilder<Stack<MapRequestLayer<F>, L>>
    pub fn map_response<F>(self, f: F) -> ServiceBuilder<Stack<MapResponseLayer<F>, L>>
    pub fn map_err<F>(self, f: F) -> ServiceBuilder<Stack<MapErrLayer<F>, L>>
    pub fn map_result<F>(self, f: F) -> ServiceBuilder<Stack<MapResultLayer<F>, L>>
    pub fn map_future<F>(self, f: F) -> ServiceBuilder<Stack<MapFutureLayer<F>, L>>
    pub fn then<F>(self, f: F) -> ServiceBuilder<Stack<ThenLayer<F>, L>>
    pub fn and_then<F>(self, f: F) -> ServiceBuilder<Stack<AndThenLayer<F>, L>>
    pub fn filter<F>(self, filter: F) -> ServiceBuilder<Stack<FilterLayer<F>, L>>
    pub fn filter_async<F>(self, filter: F) -> ServiceBuilder<Stack<AsyncFilterLayer<F>, L>>
    pub fn into_inner(self) -> L
    pub fn service<S>(self, service: S) -> L::Service where L: Layer<S>
    pub fn service_fn<F>(self, f: F) -> L::Service where L: Layer<ServiceFn<F>>
}
```

### With Feature-Gated Methods

```rust
impl ServiceBuilder<L> {
    // feature = "timeout"
    pub fn timeout(self, timeout: Duration) -> ServiceBuilder<Stack<TimeoutLayer, L>>

    // feature = "limit"
    pub fn rate_limit(self, num: u64, per: Duration) -> ServiceBuilder<Stack<RateLimitLayer, L>>
    pub fn concurrency_limit(self, max: usize) -> ServiceBuilder<Stack<ConcurrencyLimitLayer, L>>

    // feature = "buffer"
    pub fn buffer(self, bound: usize) -> ServiceBuilder<Stack<BufferLayer<Request>, L>>

    // feature = "load-shed"
    pub fn load_shed(self) -> ServiceBuilder<Stack<LoadShedLayer, L>>

    // feature = "retry"
    pub fn retry<P>(self, policy: P) -> ServiceBuilder<Stack<RetryLayer<P>, L>>
}
```

---

## Basic Usage

```rust
use tower::ServiceBuilder;
use std::time::Duration;

let service = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .rate_limit(100, Duration::from_secs(1))
    .concurrency_limit(50)
    .buffer(100)
    .service(my_service);
```

---

## Layer Ordering

Layers are applied bottom-up from the builder chain. The first `.layer()` call wraps outermost:

```rust
ServiceBuilder::new()
    .layer(A)          // outermost: first to see request, last to see response
    .layer(B)          // middle
    .layer(C)          // innermost: last to see request, first to see response
    .service(svc)

// Equivalent to:
// A::layer(B::layer(C::layer(svc)))

// Request flow:  A → B → C → svc
// Response flow: A ← B ← C ← svc
```

---

## Common Patterns

### With Custom Layers

```rust
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
};

let service = ServiceBuilder::new()
    .layer(TraceLayer::new_for_http())
    .layer(CorsLayer::permissive())
    .layer(CompressionLayer::new())
    .timeout(Duration::from_secs(30))
    .service(my_http_service);
```

### Optional Layers

Conditionally include a layer:

```rust
let maybe_timeout = if config.enable_timeout {
    Some(TimeoutLayer::new(Duration::from_secs(30)))
} else {
    None
};

let service = ServiceBuilder::new()
    .option_layer(maybe_timeout)
    .service(my_service);
```

### Transforming Requests and Responses

```rust
let service = ServiceBuilder::new()
    .map_request(|req: String| req.to_uppercase())
    .map_response(|resp: String| format!("Response: {}", resp))
    .map_err(|err: MyError| std::io::Error::new(std::io::ErrorKind::Other, err))
    .service(my_service);
```

### Using service_fn Directly

```rust
use tower::ServiceBuilder;
use std::convert::Infallible;

let service = ServiceBuilder::new()
    .timeout(Duration::from_secs(10))
    .service_fn(|req: String| async move {
        Ok::<_, Infallible>(format!("Hello, {}!", req))
    });
```

---

## ServiceBuilder as a Layer

`ServiceBuilder` itself implements `Layer`, so it can be passed to anything that accepts a layer:

```rust
use tower::ServiceBuilder;

let middleware_stack = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .rate_limit(100, Duration::from_secs(1));

// Use as a layer
let service = middleware_stack.service(my_service);

// Or pass to Axum's .layer()
// app.layer(middleware_stack)
```

---

## Integration with Axum

```rust
use axum::Router;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/", get(handler))
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .timeout(Duration::from_secs(30))
    );
```

---

## See Also

- [Service Trait](01-service-trait.md) — the trait being wrapped
- [Layer Trait](02-layer-trait.md) — individual layer implementations
- [Composition Patterns](../patterns/01-composition.md) — advanced composition strategies
- [Axum Middleware](../../axum/middleware/02-tower-integration.md) — using Tower with Axum
