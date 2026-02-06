# from_fn Middleware

Axum's `from_fn` creates middleware from async functions without implementing Tower traits. This is the simplest way to write middleware in Axum.

---

## API Reference

```rust
pub fn from_fn<F, T>(f: F) -> FromFnLayer<F, (), T>
pub fn from_fn_with_state<F, S, T>(state: S, f: F) -> FromFnLayer<F, S, T>
```

---

## Basic Usage

```rust
use axum::{
    middleware::{self, Next},
    extract::Request,
    response::Response,
    Router,
    routing::get,
};

async fn my_middleware(request: Request, next: Next) -> Response {
    // Do something before the handler
    println!("Request: {} {}", request.method(), request.uri());

    // Call the next middleware/handler
    let response = next.run(request).await;

    // Do something after the handler
    println!("Response: {}", response.status());

    response
}

let app = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn(my_middleware));
```

---

## With Extractors

Middleware functions can use extractors just like handlers. Extractors implementing `FromRequestParts` can appear before the `Request` parameter:

```rust
use axum::extract::{State, Request};
use axum::middleware::Next;
use axum::response::Response;
use http::StatusCode;

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(token) if state.validate_token(token) => {
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

---

## With State

```rust
use axum::middleware;

let app = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
    .with_state(state);
```

---

## Modifying Request/Response

### Add Headers to Response

```rust
async fn add_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-custom-header",
        "value".parse().unwrap(),
    );
    response
}
```

### Inject Extension

```rust
async fn inject_user(mut request: Request, next: Next) -> Response {
    let user = authenticate(&request).await;
    request.extensions_mut().insert(user);
    next.run(request).await
}
```

### Timing Middleware

```rust
async fn timing(request: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let elapsed = start.elapsed();
    tracing::info!(?elapsed, "request handled");
    response
}
```

---

## Early Return

Return early (without calling `next.run()`) to short-circuit the middleware chain:

```rust
async fn rate_limit(request: Request, next: Next) -> Result<Response, StatusCode> {
    if is_rate_limited(&request) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(request).await)
}
```

---

## Comparison with Tower Layers

| Approach | Complexity | Reusability | Performance |
|----------|-----------|-------------|-------------|
| `from_fn` | Low — just an async function | Axum-only | Slight overhead from boxing |
| Tower `Layer` + `Service` | Higher — two trait implementations | Universal (Hyper, Tonic, etc.) | Zero-cost (no boxing needed) |

Use `from_fn` for quick, Axum-specific middleware. Use Tower layers for reusable middleware shared across frameworks.

---

## Other Middleware Functions

| Function | Description |
|----------|-------------|
| `from_extractor::<T>()` | Run an extractor as middleware, discard the value |
| `from_extractor_with_state::<T>(state)` | Same, with state |
| `map_request(f)` | Transform the request |
| `map_request_with_state(state, f)` | Transform the request with state |
| `map_response(f)` | Transform the response |
| `map_response_with_state(state, f)` | Transform the response with state |

### map_request

```rust
use axum::middleware::map_request;

async fn set_header(mut request: Request) -> Request {
    request.headers_mut().insert("x-custom", "value".parse().unwrap());
    request
}

let app = Router::new()
    .route("/", get(handler))
    .layer(map_request(set_header));
```

---

## See Also

- [Tower Integration](02-tower-integration.md) — using Tower layers with Axum
- [Custom Tower Middleware](../../tower/patterns/02-custom-middleware.md) — writing reusable Tower middleware
- [Error Handling](../advanced/04-error-handling.md) — error responses from middleware
