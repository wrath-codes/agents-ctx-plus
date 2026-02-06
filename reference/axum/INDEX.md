# axum — Sub-Index

> Ergonomic Rust web framework built on Tower and Hyper (14 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [core](core/)

|file|description|
|---|---|
|[01-router.md](core/01-router.md)|Router — routing, nesting, merging, fallbacks, .with_state()|
|[02-handlers.md](core/02-handlers.md)|Handler trait — async fns, handler composition, Handler::with_state()|
|[03-extractors.md](core/03-extractors.md)|Extractors — Path, Query, Json, State, Form, Headers, FromRequest/Parts|
|[04-responses.md](core/04-responses.md)|IntoResponse — Html, Json, Redirect, SSE, tuples, custom types|

### [middleware](middleware/)

|file|description|
|---|---|
|[01-from-fn.md](middleware/01-from-fn.md)|from_fn middleware — quick middleware from async functions|
|[02-tower-integration.md](middleware/02-tower-integration.md)|Tower integration — ServiceBuilder, Layer, per-route vs global|

### [advanced](advanced/)

|file|description|
|---|---|
|[01-websockets.md](advanced/01-websockets.md)|WebSocket — upgrade, bidirectional communication|
|[02-sse.md](advanced/02-sse.md)|Server-Sent Events — real-time streaming|
|[03-state-management.md](advanced/03-state-management.md)|State — State<S>, FromRef, substates|
|[04-error-handling.md](advanced/04-error-handling.md)|Errors — error model, rejection handling, anyhow integration|

### [extras](extras/)

|file|description|
|---|---|
|[01-axum-extra.md](extras/01-axum-extra.md)|axum-extra — TypedHeader, CookieJar, typed routing, protobuf|
|[02-testing.md](extras/02-testing.md)|Testing — TestClient, integration testing patterns|

### Key Patterns
```
Router::new().route("/path", get(handler)).with_state(state)
async fn handler(State(s): State<S>, Path(id): Path<u64>) -> impl IntoResponse
.layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
```

---
*14 files · Related: [tower](../tower/INDEX.md), [tokio](../tokio/INDEX.md), [tonic](../tonic/INDEX.md)*
