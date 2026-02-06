# Timeout Middleware

The `timeout` module provides middleware that fails requests which do not complete within a specified duration.

---

## API Reference

```rust
// Layer
pub struct TimeoutLayer {
    timeout: Duration,
}

impl TimeoutLayer {
    pub fn new(timeout: Duration) -> Self
}

// Service
pub struct Timeout<S> {
    inner: S,
    timeout: Duration,
}

impl<S> Timeout<S> {
    pub fn new(service: S, timeout: Duration) -> Self
    pub fn get_ref(&self) -> &S
    pub fn get_mut(&mut self) -> &mut S
    pub fn into_inner(self) -> S
    pub fn layer(timeout: Duration) -> TimeoutLayer
}
```

Requires feature flag: `timeout`

---

## Usage

```rust
use tower::ServiceBuilder;
use std::time::Duration;

let service = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .service(my_service);
```

Or using the layer directly:

```rust
use tower::timeout::TimeoutLayer;

let layer = TimeoutLayer::new(Duration::from_secs(30));
```

---

## Error Type

When a timeout occurs, the service returns a `tower::timeout::error::Elapsed` error. This wraps an inner error that can be matched:

```rust
use tower::timeout::error::Elapsed;

match result {
    Ok(response) => { /* ... */ }
    Err(err) => {
        if err.is::<Elapsed>() {
            eprintln!("Request timed out!");
        }
    }
}
```

---

## See Also

- [Rate Limit](02-rate-limit.md) — limiting request rate
- [ServiceBuilder](../core/03-service-builder.md) — composing with other middleware
- [tower-http Timeout](../tower-http/02-request-response.md) — HTTP-aware timeout
