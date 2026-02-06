# Observability Middleware

HTTP middleware for tracing, error recovery, request identification, and metrics.

---

## TraceLayer

Adds structured tracing to HTTP services using the `tracing` crate. Emits spans and events for request lifecycle.

```rust
use tower_http::trace::TraceLayer;

// Default configuration for HTTP services
let layer = TraceLayer::new_for_http();

// Custom configuration
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse};
use tracing::Level;

let layer = TraceLayer::new_for_http()
    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
    .on_request(DefaultOnRequest::new().level(Level::INFO))
    .on_response(DefaultOnResponse::new().level(Level::INFO));
```

### Custom Callbacks

```rust
use tower_http::trace::TraceLayer;
use http::{Request, Response};
use tracing::Span;
use std::time::Duration;

let layer = TraceLayer::new_for_http()
    .make_span_with(|request: &Request<_>| {
        tracing::info_span!(
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
        )
    })
    .on_request(|_request: &Request<_>, _span: &Span| {
        tracing::info!("request started");
    })
    .on_response(|response: &Response<_>, latency: Duration, _span: &Span| {
        tracing::info!(
            status = response.status().as_u16(),
            latency = ?latency,
            "response sent"
        );
    })
    .on_failure(|error, latency: Duration, _span: &Span| {
        tracing::error!(?error, ?latency, "request failed");
    });
```

Feature: `trace`

---

## CatchPanicLayer

Catches panics in the service and converts them into 500 Internal Server Error responses, preventing the server from crashing.

```rust
use tower_http::catch_panic::CatchPanicLayer;

let layer = CatchPanicLayer::new();

// Custom panic handler
use tower_http::catch_panic::CatchPanicLayer;
let layer = CatchPanicLayer::custom(|_err| {
    http::Response::builder()
        .status(http::StatusCode::INTERNAL_SERVER_ERROR)
        .body("Internal Server Error".into())
        .unwrap()
});
```

Feature: `catch-panic`

---

## RequestIdLayer / SetRequestIdLayer

Generate and propagate unique request identifiers for correlation.

```rust
use tower_http::request_id::{
    SetRequestIdLayer, PropagateRequestIdLayer, MakeRequestUuid,
};
use http::HeaderName;

let x_request_id = HeaderName::from_static("x-request-id");

// Generate a UUID for each request
let set_id = SetRequestIdLayer::new(
    x_request_id.clone(),
    MakeRequestUuid,
);

// Propagate from request to response
let propagate_id = PropagateRequestIdLayer::new(x_request_id);
```

Feature: `request-id`

---

## SensitiveHeadersLayer

Mark specific headers as sensitive so they are redacted in debug output and logging.

```rust
use tower_http::sensitive_headers::SetSensitiveHeadersLayer;
use http::header;

let layer = SetSensitiveHeadersLayer::new([
    header::AUTHORIZATION,
    header::COOKIE,
    header::SET_COOKIE,
]);
```

Feature: `sensitive-headers`

---

## Metrics / InFlightRequestsLayer

Track request metrics including in-flight request counts.

```rust
use tower_http::metrics::InFlightRequestsLayer;

let (in_flight_layer, counter) = InFlightRequestsLayer::pair();

// Use counter to read current in-flight count
tokio::spawn(async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        println!("In-flight requests: {}", counter.get());
    }
});
```

Feature: `metrics`

---

## Auth / RequireAuthorizationLayer

Basic authentication and authorization middleware.

```rust
use tower_http::auth::RequireAuthorizationLayer;

// Basic auth
let layer = RequireAuthorizationLayer::basic("admin", "password123");

// Bearer token
let layer = RequireAuthorizationLayer::bearer("my-secret-token");
```

For custom auth logic, implement the `AuthorizeRequest` trait.

Feature: `auth`

---

## Composing Observability Middleware

A typical production stack:

```rust
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
    catch_panic::CatchPanicLayer,
    request_id::{SetRequestIdLayer, PropagateRequestIdLayer, MakeRequestUuid},
    sensitive_headers::SetSensitiveHeadersLayer,
};
use http::header;

let x_request_id = http::HeaderName::from_static("x-request-id");

let middleware = ServiceBuilder::new()
    .layer(SetSensitiveHeadersLayer::new([
        header::AUTHORIZATION,
        header::COOKIE,
    ]))
    .layer(SetRequestIdLayer::new(x_request_id.clone(), MakeRequestUuid))
    .layer(TraceLayer::new_for_http())
    .layer(PropagateRequestIdLayer::new(x_request_id))
    .layer(CatchPanicLayer::new());
```

---

## See Also

- [Request & Response Middleware](02-request-response.md) — CORS, compression, headers
- [Overview](01-overview.md) — all available middleware
- [Tracing](../../tokio/topics/03-tracing.md) — structured diagnostics with `tracing`
