# Retry Middleware

The `retry` module provides middleware that retries failed requests according to a configurable policy.

---

## API Reference

```rust
// Policy trait
pub trait Policy<Req, Res, E>: Sized {
    type Future: Future<Output = Self>;
    fn retry(&self, req: &Req, result: Result<&Res, &E>) -> Option<Self::Future>;
    fn clone_request(&self, req: &Req) -> Option<Req>;
}

// Layer
pub struct RetryLayer<P> {
    policy: P,
}

impl<P> RetryLayer<P> {
    pub fn new(policy: P) -> Self
}

// Service
pub struct Retry<P, S> {
    policy: P,
    service: S,
}

impl<P, S> Retry<P, S> {
    pub fn new(policy: P, service: S) -> Self
    pub fn get_ref(&self) -> &S
    pub fn get_mut(&mut self) -> &mut S
    pub fn into_inner(self) -> S
}
```

Requires feature flag: `retry`

---

## Policy Trait

The `Policy` trait controls retry behavior:

| Method | Description |
|--------|-------------|
| `retry(&self, req, result)` | Decide whether to retry. Return `Some(future)` to retry after the future completes, `None` to stop. |
| `clone_request(&self, req)` | Clone the request for retry. Return `None` if the request cannot be cloned (no retry possible). |

---

## Implementing a Policy

```rust
use tower::retry::Policy;
use std::future;

#[derive(Clone)]
struct RetryOnError {
    max_retries: usize,
    remaining: usize,
}

impl RetryOnError {
    fn new(max_retries: usize) -> Self {
        Self {
            max_retries,
            remaining: max_retries,
        }
    }
}

impl<Req: Clone, Res, E> Policy<Req, Res, E> for RetryOnError {
    type Future = future::Ready<Self>;

    fn retry(&self, _req: &Req, result: Result<&Res, &E>) -> Option<Self::Future> {
        match result {
            Ok(_) => None,
            Err(_) if self.remaining > 0 => {
                Some(future::ready(Self {
                    max_retries: self.max_retries,
                    remaining: self.remaining - 1,
                }))
            }
            Err(_) => None,
        }
    }

    fn clone_request(&self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}
```

---

## Usage

```rust
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .retry(RetryOnError::new(3))
    .service(my_service);
```

Or using the layer:

```rust
use tower::retry::RetryLayer;

let layer = RetryLayer::new(RetryOnError::new(3));
```

---

## Retry with Backoff

Implement a delay between retries by returning a future that sleeps:

```rust
use tower::retry::Policy;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
struct ExponentialBackoff {
    attempt: usize,
    max_retries: usize,
}

impl<Req: Clone, Res, E> Policy<Req, Res, E> for ExponentialBackoff {
    type Future = Pin<Box<dyn Future<Output = Self> + Send>>;

    fn retry(&self, _req: &Req, result: Result<&Res, &E>) -> Option<Self::Future> {
        match result {
            Ok(_) => None,
            Err(_) if self.attempt < self.max_retries => {
                let next = Self {
                    attempt: self.attempt + 1,
                    max_retries: self.max_retries,
                };
                let delay = Duration::from_millis(100 * 2u64.pow(self.attempt as u32));
                Some(Box::pin(async move {
                    sleep(delay).await;
                    next
                }))
            }
            Err(_) => None,
        }
    }

    fn clone_request(&self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}
```

---

## See Also

- [Timeout](01-timeout.md) — deadline enforcement
- [Rate Limit](02-rate-limit.md) — request rate control
- [ServiceBuilder](../core/03-service-builder.md) — composing middleware
- [tokio-retry](../../tokio/ecosystem/03-related-projects.md) — alternative retry crate
