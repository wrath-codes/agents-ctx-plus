# Rate Limit Middleware

The `limit` module provides middleware that restricts the number of requests a service can handle per time period.

---

## API Reference

```rust
// Layer
pub struct RateLimitLayer {
    rate: Rate,
}

impl RateLimitLayer {
    pub fn new(num: u64, per: Duration) -> Self
}

// Service
pub struct RateLimit<S> {
    inner: S,
    rate: Rate,
    state: State,
}

impl<S> RateLimit<S> {
    pub fn new(inner: S, num: u64, per: Duration) -> Self
    pub fn get_ref(&self) -> &S
    pub fn get_mut(&mut self) -> &mut S
    pub fn into_inner(self) -> S
}
```

Requires feature flag: `limit`

---

## Usage

```rust
use tower::ServiceBuilder;
use std::time::Duration;

// Allow 100 requests per second
let service = ServiceBuilder::new()
    .rate_limit(100, Duration::from_secs(1))
    .service(my_service);
```

---

## Behavior

When the rate limit is exceeded, `poll_ready` returns `Poll::Pending` until the time window resets. This provides natural backpressure — callers wait rather than receiving errors.

The rate limiter uses a sliding window. After `num` requests within `per` duration, subsequent calls to `poll_ready` will not return `Ready` until the window slides forward.

---

## See Also

- [Concurrency Limit](04-concurrency.md) — limiting concurrent in-flight requests
- [Timeout](01-timeout.md) — deadline enforcement
- [ServiceBuilder](../core/03-service-builder.md) — composing middleware
