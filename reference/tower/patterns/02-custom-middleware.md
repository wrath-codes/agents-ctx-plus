# Writing Custom Middleware

A complete guide to implementing custom Tower `Service` and `Layer` types.

---

## The Pattern

Every Tower middleware consists of two types:

1. **Layer** — a factory that wraps an inner service (implements `Layer<S>`)
2. **Service** — the wrapper that intercepts requests/responses (implements `Service<Request>`)

```
┌──────────┐         ┌──────────────────┐
│  Layer   │─────────│  WrappedService  │
│ (config) │ .layer()│  (inner + logic) │
└──────────┘         └──────────────────┘
```

---

## Complete Example: Request Logging

```rust
use tower::{Layer, Service};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

// --- Layer ---

#[derive(Clone)]
pub struct LogLayer {
    target: &'static str,
}

impl LogLayer {
    pub fn new(target: &'static str) -> Self {
        Self { target }
    }
}

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogService {
            inner,
            target: self.target,
        }
    }
}

// --- Service ---

#[derive(Clone)]
pub struct LogService<S> {
    inner: S,
    target: &'static str,
}

impl<S, Req, Res> Service<Req> for LogService<S>
where
    S: Service<Req, Response = Res>,
    Req: std::fmt::Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = LogFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let start = Instant::now();
        tracing::info!(target: self.target, request = ?req, "processing request");
        LogFuture {
            inner: self.inner.call(req),
            start,
        }
    }
}

// --- Future ---

#[pin_project::pin_project]
pub struct LogFuture<F> {
    #[pin]
    inner: F,
    start: Instant,
}

impl<F, Res, Err> Future for LogFuture<F>
where
    F: Future<Output = Result<Res, Err>>,
{
    type Output = Result<Res, Err>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Ready(result) => {
                let elapsed = this.start.elapsed();
                match &result {
                    Ok(_) => tracing::info!(latency = ?elapsed, "request completed"),
                    Err(_) => tracing::error!(latency = ?elapsed, "request failed"),
                }
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
```

---

## Simpler Alternative: Boxed Future

If you don't want to define a custom future type, use `Pin<Box<dyn Future>>`:

```rust
impl<S, Req> Service<Req> for LogService<S>
where
    S: Service<Req> + Clone + Send + 'static,
    S::Future: Send,
    Req: Send + std::fmt::Debug + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let start = Instant::now();
            let result = inner.call(req).await;
            tracing::info!(latency = ?start.elapsed(), "done");
            result
        })
    }
}
```

---

## HTTP-Specific Middleware

For HTTP services, your middleware works with `http::Request<B>` and `http::Response<ResB>`:

```rust
use http::{Request, Response};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct AddServerHeaderLayer;

impl<S> Layer<S> for AddServerHeaderLayer {
    type Service = AddServerHeader<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AddServerHeader { inner }
    }
}

#[derive(Clone)]
pub struct AddServerHeader<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for AddServerHeader<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = AddServerHeaderFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        AddServerHeaderFuture {
            inner: self.inner.call(req),
        }
    }
}
```

---

## Using with Axum

Alternatively, Axum's `from_fn` provides a simpler way to write middleware without implementing traits:

```rust
use axum::{middleware, Router};
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

async fn my_middleware(request: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let elapsed = start.elapsed();
    tracing::info!(?elapsed, "request handled");
    response
}

let app = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn(my_middleware));
```

See [Axum from_fn](../../axum/middleware/01-from-fn.md) for details.

---

## Testing Middleware

```rust
use tower::{Service, ServiceExt, service_fn};
use std::convert::Infallible;

#[tokio::test]
async fn test_log_middleware() {
    let service = LogLayer::new("test")
        .layer(service_fn(|req: String| async move {
            Ok::<_, Infallible>(format!("Hello, {}!", req))
        }));

    let response = service.oneshot("world".to_string()).await.unwrap();
    assert_eq!(response, "Hello, world!");
}
```

---

## See Also

- [Service Trait](../core/01-service-trait.md) — implementing Service
- [Layer Trait](../core/02-layer-trait.md) — implementing Layer
- [Composition Patterns](01-composition.md) — combining middleware
- [Axum from_fn](../../axum/middleware/01-from-fn.md) — simpler middleware in Axum
