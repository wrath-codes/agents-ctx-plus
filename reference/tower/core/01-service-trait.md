# The Service Trait

The `Service` trait is the core abstraction of Tower — an asynchronous function from a `Request` to a `Response` with built-in backpressure. It is defined in the `tower-service` crate (re-exported by `tower`) to minimize dependency weight.

---

## API Reference

```rust
pub trait Service<Request> {
    type Response;
    type Error;
    type Future: Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
    fn call(&mut self, req: Request) -> Self::Future;
}
```

| Associated Type | Description |
|-----------------|-------------|
| `Response` | The type returned on success |
| `Error` | The type returned on failure |
| `Future` | The future type returned by `call`, resolving to `Result<Response, Error>` |

| Method | Description |
|--------|-------------|
| `poll_ready(&mut self, cx)` | Signals whether the service can accept a new request (backpressure) |
| `call(&mut self, req)` | Processes a request, returning a future |

---

## poll_ready

`poll_ready` implements backpressure. A service returns `Poll::Ready(Ok(()))` when it can accept a request, or `Poll::Pending` when it needs the caller to wait. This prevents overloading downstream resources.

**Contract**: You MUST call `poll_ready` and receive `Ready(Ok(()))` before calling `call`. Calling `call` without checking readiness may panic or produce incorrect behavior, depending on the implementation.

```rust
use tower::Service;
use std::task::{Context, Poll};

async fn use_service<S>(svc: &mut S, req: String) -> Result<S::Response, S::Error>
where
    S: Service<String>,
{
    // Wait until the service is ready
    futures::future::poll_fn(|cx| svc.poll_ready(cx)).await?;
    svc.call(req).await
}
```

---

## Implementing Service

### From a Closure (service_fn)

The simplest way to create a `Service`:

```rust
use tower::service_fn;
use std::convert::Infallible;

let svc = service_fn(|req: String| async move {
    Ok::<_, Infallible>(format!("Hello, {}!", req))
});
```

### Manual Implementation

```rust
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct EchoService;

impl Service<String> for EchoService {
    type Response = String;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<String, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: String) -> Self::Future {
        Box::pin(async move {
            Ok(format!("Echo: {}", req))
        })
    }
}
```

### With State

```rust
use tower::Service;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

#[derive(Clone)]
struct CounterService {
    count: Arc<AtomicU64>,
}

impl Service<()> for CounterService {
    type Response = u64;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<u64, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: ()) -> Self::Future {
        let count = self.count.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Ok(count) })
    }
}
```

---

## ServiceExt

The `ServiceExt` trait (feature `util`) provides convenience methods on any `Service`:

```rust
pub trait ServiceExt<Request>: Service<Request> {
    fn ready(&mut self) -> Ready<'_, Self, Request>;
    fn ready_oneshot(self) -> ReadyOneshot<Self, Request>;
    fn oneshot(self, req: Request) -> Oneshot<Self, Request>;
    fn call_all<S>(self, reqs: S) -> CallAll<Self, S>;
    fn and_then<F>(self, f: F) -> AndThen<Self, F>;
    fn map_response<F>(self, f: F) -> MapResponse<Self, F>;
    fn map_err<F>(self, f: F) -> MapErr<Self, F>;
    fn map_result<F>(self, f: F) -> MapResult<Self, F>;
    fn map_future<F>(self, f: F) -> MapFuture<Self, F>;
    fn map_request<F>(self, f: F) -> MapRequest<Self, F>;
    fn filter<F>(self, filter: F) -> Filter<Self, F>;
    fn filter_async<F>(self, filter: F) -> AsyncFilter<Self, F>;
    fn then<F>(self, f: F) -> Then<Self, F>;
    fn boxed(self) -> BoxService<Request, Self::Response, Self::Error>;
    fn boxed_clone(self) -> BoxCloneService<Request, Self::Response, Self::Error>;
}
```

### Common Methods

| Method | Description |
|--------|-------------|
| `ready()` | Wait until `poll_ready` returns `Ready` |
| `oneshot(req)` | Call `poll_ready` then `call` in one step (consumes the service) |
| `map_response(f)` | Transform the response with a closure |
| `map_err(f)` | Transform the error with a closure |
| `map_request(f)` | Transform the request before calling the inner service |
| `and_then(f)` | Chain an async function after the service |
| `boxed()` | Type-erase the service into `BoxService` |
| `boxed_clone()` | Type-erase into a cloneable `BoxCloneService` |

### Usage

```rust
use tower::{Service, ServiceExt, service_fn};
use std::convert::Infallible;

let svc = service_fn(|req: u32| async move {
    Ok::<_, Infallible>(req * 2)
});

// oneshot: poll_ready + call in one step
let result = svc.oneshot(21).await.unwrap();
assert_eq!(result, 42);
```

---

## BoxService and BoxCloneService

Type-erased services for dynamic dispatch:

```rust
use tower::util::{BoxService, BoxCloneService};

// BoxService: Send but not Clone
let boxed: BoxService<String, String, std::io::Error> = svc.boxed();

// BoxCloneService: Send + Clone
let boxed_clone: BoxCloneService<String, String, std::io::Error> = svc.boxed_clone();
```

---

## Thread Safety

| Type | `Send` | `Sync` | `Clone` |
|------|--------|--------|---------|
| `ServiceFn` | If closure is Send | If closure is Sync | If closure is Clone |
| `BoxService` | Yes | No | No |
| `BoxCloneService` | Yes | No | Yes |

---

## See Also

- [Layer Trait](02-layer-trait.md) — wrapping services with middleware
- [ServiceBuilder](03-service-builder.md) — declarative layer composition
- [Composition Patterns](../patterns/01-composition.md) — combining services and layers
- [Tokio Tasks](../../tokio/rust-api/02-tasks.md) — spawning services as async tasks
