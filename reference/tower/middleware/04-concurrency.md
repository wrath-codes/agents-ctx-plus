# Concurrency, Buffer, and Load Shed Middleware

These middleware control how many requests are processed concurrently and what happens when the service is overloaded.

---

## ConcurrencyLimit

Limits the number of in-flight requests to a service.

### API Reference

```rust
pub struct ConcurrencyLimitLayer {
    max: usize,
}

impl ConcurrencyLimitLayer {
    pub fn new(max: usize) -> Self
}

pub struct ConcurrencyLimit<S> {
    inner: S,
    semaphore: PollSemaphore,
    permit: Option<OwnedSemaphorePermit>,
}
```

Requires feature flag: `limit`

### Usage

```rust
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .concurrency_limit(50)
    .service(my_service);
```

### Behavior

Uses a `tokio::sync::Semaphore` internally. When `max` requests are in flight, `poll_ready` returns `Poll::Pending` until a permit becomes available. This provides backpressure to callers.

---

## Buffer

Provides a `Clone`-able handle to a service via an internal mpsc channel. The service runs on a background task, processing requests from the channel.

### API Reference

```rust
pub struct BufferLayer<Request> {
    bound: usize,
    _phantom: PhantomData<fn(Request)>,
}

impl<Request> BufferLayer<Request> {
    pub fn new(bound: usize) -> Self
}

pub struct Buffer<T, Request> {
    tx: mpsc::Sender<Message<Request, T::Future>>,
}

impl<T, Request> Clone for Buffer<T, Request> { /* ... */ }
```

Requires feature flag: `buffer`

### Usage

```rust
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .buffer(100)
    .service(my_service);

// The returned service is Clone, even if my_service isn't
let svc2 = service.clone();
```

### When to Use Buffer

| Use Case | Description |
|----------|-------------|
| Need `Clone` | When your service must be cloneable (e.g., for `Router`) |
| Decouple caller and service | Requests queued in channel, processed independently |
| Combine with other middleware | Buffer at the boundary between sync and async |

### Error Handling

If the buffer is full, `poll_ready` returns `Poll::Pending`. If the background worker panics or is dropped, calls return a `ServiceError`.

---

## LoadShed

Immediately rejects requests when the inner service is not ready, rather than waiting.

### API Reference

```rust
pub struct LoadShedLayer;

impl LoadShedLayer {
    pub fn new() -> Self
}

pub struct LoadShed<S> {
    inner: S,
    is_ready: bool,
}
```

Requires feature flag: `load-shed`

### Usage

```rust
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .load_shed()
    .concurrency_limit(50)
    .service(my_service);
```

### Behavior

When `poll_ready` on the inner service returns `Poll::Pending`, `LoadShed` immediately returns `Poll::Ready(Ok(()))` but marks itself as "not ready". If `call` is invoked in this state, it returns a `tower::load_shed::error::Overloaded` error.

This is useful for failing fast when a service is overloaded, rather than queuing requests.

---

## Combining Concurrency Controls

A common pattern stacks these middleware:

```rust
use tower::ServiceBuilder;
use std::time::Duration;

let service = ServiceBuilder::new()
    .load_shed()                              // fail fast when overloaded
    .concurrency_limit(100)                   // max 100 concurrent requests
    .rate_limit(1000, Duration::from_secs(1)) // max 1000 req/sec
    .buffer(200)                              // queue up to 200 requests
    .timeout(Duration::from_secs(30))         // per-request timeout
    .service(my_service);
```

---

## See Also

- [Rate Limit](02-rate-limit.md) — requests per time period
- [Timeout](01-timeout.md) — per-request deadlines
- [ServiceBuilder](../core/03-service-builder.md) — composing middleware
- [Tokio Semaphore](../../tokio/rust-api/06-sync.md) — the primitive backing ConcurrencyLimit
