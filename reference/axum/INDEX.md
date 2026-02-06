# axum — Sub-Index

> Ergonomic Rust web framework built on Tower and Hyper (13 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Quick Start](README.md#quick-start) · [Architecture](README.md#architecture) · [Essential Rust Types](README.md#essential-rust-types) · [Documentation Map](README.md#documentation-map) · [Quick Links](README.md#quick-links) · [Related References](README.md#related-references) · [External Resources](README.md#external-resources)|

### [core](core/)

|file|description|
|---|---|
|[01-router.md](core/01-router.md)|Router — routing, nesting, merging, fallbacks, .with_state()|
| |↳ [API Reference](core/01-router.md#api-reference) · [Basic Routing](core/01-router.md#basic-routing) · [Method Routing Functions](core/01-router.md#method-routing-functions) · [Nesting](core/01-router.md#nesting) · [Merging](core/01-router.md#merging) · [Fallback](core/01-router.md#fallback) · [Layer vs route_layer](core/01-router.md#layer-vs-route_layer) · [State](core/01-router.md#state) · +1 more|
|[02-handlers.md](core/02-handlers.md)|Handler trait — async fns, handler composition, Handler::with_state()|
| |↳ [Handler Signature](core/02-handlers.md#handler-signature) · [Examples](core/02-handlers.md#examples) · [The Handler Trait](core/02-handlers.md#the-handler-trait) · [Handler Methods](core/02-handlers.md#handler-methods) · [Closures as Handlers](core/02-handlers.md#closures-as-handlers) · [debug_handler](core/02-handlers.md#debug_handler)|
|[03-extractors.md](core/03-extractors.md)|Extractors — Path, Query, Json, State, Form, Headers, FromRequest/Parts|
| |↳ [Extractor Traits](core/03-extractors.md#extractor-traits) · [All Built-in Extractors](core/03-extractors.md#all-built-in-extractors) · [Path](core/03-extractors.md#path) · [Query](core/03-extractors.md#query) · [Json](core/03-extractors.md#json) · [State](core/03-extractors.md#state) · [Form](core/03-extractors.md#form) · [Headers](core/03-extractors.md#headers) · +3 more|
|[04-responses.md](core/04-responses.md)|IntoResponse — Html, Json, Redirect, SSE, tuples, custom types|
| |↳ [IntoResponse Trait](core/04-responses.md#intoresponse-trait) · [Built-in Response Types](core/04-responses.md#built-in-response-types) · [Examples](core/04-responses.md#examples) · [Custom IntoResponse](core/04-responses.md#custom-intoresponse) · [IntoResponseParts](core/04-responses.md#intoresponseparts) · [Result as Response](core/04-responses.md#result-as-response)|

### [middleware](middleware/)

|file|description|
|---|---|
|[01-from-fn.md](middleware/01-from-fn.md)|from_fn middleware — quick middleware from async functions|
| |↳ [API Reference](middleware/01-from-fn.md#api-reference) · [Basic Usage](middleware/01-from-fn.md#basic-usage) · [With Extractors](middleware/01-from-fn.md#with-extractors) · [With State](middleware/01-from-fn.md#with-state) · [Modifying Request/Response](middleware/01-from-fn.md#modifying-requestresponse) · [Early Return](middleware/01-from-fn.md#early-return) · [Comparison with Tower Layers](middleware/01-from-fn.md#comparison-with-tower-layers) · [Other Middleware Functions](middleware/01-from-fn.md#other-middleware-functions)|
|[02-tower-integration.md](middleware/02-tower-integration.md)|Tower integration — ServiceBuilder, Layer, per-route vs global|
| |↳ [Adding Tower Layers](middleware/02-tower-integration.md#adding-tower-layers) · [layer vs route_layer](middleware/02-tower-integration.md#layer-vs-route_layer) · [Per-Route Middleware](middleware/02-tower-integration.md#per-route-middleware) · [Common tower-http Layers for Axum](middleware/02-tower-integration.md#common-tower-http-layers-for-axum) · [Using Tower Services as Route Handlers](middleware/02-tower-integration.md#using-tower-services-as-route-handlers)|

### [advanced](advanced/)

|file|description|
|---|---|
|[01-websockets.md](advanced/01-websockets.md)|WebSocket — upgrade, bidirectional communication|
| |↳ [Basic Example](advanced/01-websockets.md#basic-example) · [WebSocketUpgrade](advanced/01-websockets.md#websocketupgrade) · [Message Types](advanced/01-websockets.md#message-types) · [With State and Extractors](advanced/01-websockets.md#with-state-and-extractors)|
|[02-sse.md](advanced/02-sse.md)|Server-Sent Events — real-time streaming|
| |↳ [Basic Example](advanced/02-sse.md#basic-example) · [Event Builder](advanced/02-sse.md#event-builder) · [KeepAlive](advanced/02-sse.md#keepalive)|
|[03-state-management.md](advanced/03-state-management.md)|State — State<S>, FromRef, substates|
| |↳ [Basic State](advanced/03-state-management.md#basic-state) · [Arc Pattern](advanced/03-state-management.md#arc-pattern) · [Substates with FromRef](advanced/03-state-management.md#substates-with-fromref) · [Nested Routers with Different State](advanced/03-state-management.md#nested-routers-with-different-state) · [State vs Extension](advanced/03-state-management.md#state-vs-extension)|
|[04-error-handling.md](advanced/04-error-handling.md)|Errors — error model, rejection handling, anyhow integration|
| |↳ [Basic Pattern](advanced/04-error-handling.md#basic-pattern) · [Custom Error Type](advanced/04-error-handling.md#custom-error-type) · [JSON Error Responses](advanced/04-error-handling.md#json-error-responses) · [Extractor Rejections](advanced/04-error-handling.md#extractor-rejections) · [HandleError](advanced/04-error-handling.md#handleerror)|

### [extras](extras/)

|file|description|
|---|---|
|[01-axum-extra.md](extras/01-axum-extra.md)|axum-extra — TypedHeader, CookieJar, typed routing, protobuf|
| |↳ [Crate Info](extras/01-axum-extra.md#crate-info) · [TypedHeader](extras/01-axum-extra.md#typedheader) · [CookieJar](extras/01-axum-extra.md#cookiejar) · [Query (with better errors)](extras/01-axum-extra.md#query-with-better-errors) · [Protobuf](extras/01-axum-extra.md#protobuf) · [JSON Lines](extras/01-axum-extra.md#json-lines) · [Erased JSON](extras/01-axum-extra.md#erased-json) · [Typed Routing](extras/01-axum-extra.md#typed-routing) · +2 more|
|[02-testing.md](extras/02-testing.md)|Testing — TestClient, integration testing patterns|
| |↳ [Using Tower oneshot](extras/02-testing.md#using-tower-oneshot) · [Testing with State](extras/02-testing.md#testing-with-state) · [Testing JSON Endpoints](extras/02-testing.md#testing-json-endpoints) · [Multiple Requests](extras/02-testing.md#multiple-requests) · [Testing Helper Pattern](extras/02-testing.md#testing-helper-pattern)|

### Key Patterns
```
Router::new().route("/path", get(handler)).with_state(state)
async fn handler(State(s): State<S>, Path(id): Path<u64>) -> impl IntoResponse
.layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
```

---
*13 files · Related: [tower](../tower/INDEX.md), [tokio](../tokio/INDEX.md), [tonic](../tonic/INDEX.md)*
