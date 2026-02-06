# The Layer Trait

The `Layer` trait is Tower's middleware abstraction. A layer wraps an inner service to produce a new service that adds behavior — such as timeouts, logging, or authentication — without modifying the inner service's code.

---

## API Reference

```rust
pub trait Layer<S> {
    type Service;
    fn layer(&self, inner: S) -> Self::Service;
}
```

| Method | Description |
|--------|-------------|
| `layer(&self, inner: S)` | Takes an inner service `S` and returns a new wrapped service |

The `Layer` trait is defined in the `tower-layer` crate (re-exported by `tower`).

---

## How Layers Work

A layer is a factory that produces a wrapping service. The pattern is:

```
Layer  +  InnerService  →  WrappedService

TimeoutLayer::new(30s)  +  MyService  →  Timeout<MyService>
```

The wrapped service intercepts requests and/or responses, adding behavior before or after delegating to the inner service.

```
Request ──► WrappedService ──► InnerService
                                    │
Response ◄── WrappedService ◄───────┘
```

---

## Implementing a Custom Layer

A layer consists of two types: the `Layer` itself (configuration) and the wrapping `Service`.

```rust
use tower::{Layer, Service};
use std::task::{Context, Poll};
use std::time::Instant;

// Step 1: Define the Layer (configuration / factory)
#[derive(Clone)]
struct LatencyLayer;

impl<S> Layer<S> for LatencyLayer {
    type Service = LatencyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LatencyService { inner }
    }
}

// Step 2: Define the wrapping Service
#[derive(Clone)]
struct LatencyService<S> {
    inner: S,
}

impl<S, Request> Service<Request> for LatencyService<S>
where
    S: Service<Request>,
    Request: std::fmt::Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let start = Instant::now();
        println!("Request: {:?}", req);
        let future = self.inner.call(req);
        println!("Call initiated in {:?}", start.elapsed());
        future
    }
}
```

---

## Layer Combinators

Tower provides utilities for combining and transforming layers:

### layer_fn

Create a layer from a closure:

```rust
use tower::layer::layer_fn;

let layer = layer_fn(|service| {
    LatencyService { inner: service }
});
```

### Identity Layer

A layer that does nothing (passes through):

```rust
use tower::layer::Identity;

let layer = Identity::new();
```

### Stack

Combine two layers into one:

```rust
use tower::layer::util::Stack;

let combined = Stack::new(TimeoutLayer::new(Duration::from_secs(30)), RateLimitLayer::new(100, Duration::from_secs(1)));
```

In practice, use `ServiceBuilder` instead of manually stacking layers.

---

## Layer Ordering

Layers wrap from the outside in. When using `ServiceBuilder`, the first layer added is the outermost (processes first on request, last on response):

```
ServiceBuilder::new()
    .layer(A)    // outermost — processes request first
    .layer(B)    // middle
    .layer(C)    // innermost — processes request last, closest to service
    .service(svc)

Request  → A → B → C → svc
Response ← A ← B ← C ←
```

---

## Common Built-in Layers

| Layer | Description |
|-------|-------------|
| `TimeoutLayer` | Fails requests exceeding a duration |
| `RateLimitLayer` | Limits requests per time period |
| `RetryLayer` | Retries failed requests |
| `ConcurrencyLimitLayer` | Limits concurrent in-flight requests |
| `BufferLayer` | Provides a Clone-able handle via mpsc channel |
| `LoadShedLayer` | Rejects when the service is not ready |
| `FilterLayer` | Rejects requests based on a predicate |

---

## Implementing Layer for Parameterized Middleware

When your layer needs configuration:

```rust
use tower::Layer;
use std::time::Duration;

#[derive(Clone)]
struct MyTimeoutLayer {
    duration: Duration,
}

impl MyTimeoutLayer {
    fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl<S> Layer<S> for MyTimeoutLayer {
    type Service = MyTimeoutService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyTimeoutService {
            inner,
            duration: self.duration,
        }
    }
}

struct MyTimeoutService<S> {
    inner: S,
    duration: Duration,
}
```

---

## See Also

- [Service Trait](01-service-trait.md) — the trait that layers wrap
- [ServiceBuilder](03-service-builder.md) — declarative layer composition
- [Custom Middleware](../patterns/02-custom-middleware.md) — complete custom middleware examples
- [tower-http Middleware](../tower-http/01-overview.md) — HTTP-specific layers
