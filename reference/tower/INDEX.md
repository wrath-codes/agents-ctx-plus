# tower — Sub-Index

> Modular service middleware framework for Rust (14 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Quick Start](README.md#quick-start) · [Architecture](README.md#architecture) · [Essential Rust Types](README.md#essential-rust-types) · [Documentation Map](README.md#documentation-map) · [Quick Links](README.md#quick-links) · [Related References](README.md#related-references) · [External Resources](README.md#external-resources)|

### [core](core/)

|file|description|
|---|---|
|[01-service-trait.md](core/01-service-trait.md)|Service trait — poll_ready, call, Request/Response types|
| |↳ [API Reference](core/01-service-trait.md#api-reference) · [poll_ready](core/01-service-trait.md#poll_ready) · [Implementing Service](core/01-service-trait.md#implementing-service) · [ServiceExt](core/01-service-trait.md#serviceext) · [BoxService and BoxCloneService](core/01-service-trait.md#boxservice-and-boxcloneservice) · [Thread Safety](core/01-service-trait.md#thread-safety)|
|[02-layer-trait.md](core/02-layer-trait.md)|Layer trait — wrapping services, layer composition|
| |↳ [API Reference](core/02-layer-trait.md#api-reference) · [How Layers Work](core/02-layer-trait.md#how-layers-work) · [Implementing a Custom Layer](core/02-layer-trait.md#implementing-a-custom-layer) · [Layer Combinators](core/02-layer-trait.md#layer-combinators) · [Layer Ordering](core/02-layer-trait.md#layer-ordering) · [Common Built-in Layers](core/02-layer-trait.md#common-built-in-layers) · [Implementing Layer for Parameterized Middleware](core/02-layer-trait.md#implementing-layer-for-parameterized-middleware)|
|[03-service-builder.md](core/03-service-builder.md)|ServiceBuilder — fluent middleware stacking|
| |↳ [API Reference](core/03-service-builder.md#api-reference) · [Basic Usage](core/03-service-builder.md#basic-usage) · [Layer Ordering](core/03-service-builder.md#layer-ordering) · [Common Patterns](core/03-service-builder.md#common-patterns) · [ServiceBuilder as a Layer](core/03-service-builder.md#servicebuilder-as-a-layer) · [Integration with Axum](core/03-service-builder.md#integration-with-axum)|

### [middleware](middleware/)

|file|description|
|---|---|
|[01-timeout.md](middleware/01-timeout.md)|Timeout — request deadline enforcement|
| |↳ [API Reference](middleware/01-timeout.md#api-reference) · [Usage](middleware/01-timeout.md#usage) · [Error Type](middleware/01-timeout.md#error-type)|
|[02-rate-limit.md](middleware/02-rate-limit.md)|Rate limit — request throttling|
| |↳ [API Reference](middleware/02-rate-limit.md#api-reference) · [Usage](middleware/02-rate-limit.md#usage) · [Behavior](middleware/02-rate-limit.md#behavior)|
|[03-retry.md](middleware/03-retry.md)|Retry — automatic retry with policies|
| |↳ [API Reference](middleware/03-retry.md#api-reference) · [Policy Trait](middleware/03-retry.md#policy-trait) · [Implementing a Policy](middleware/03-retry.md#implementing-a-policy) · [Usage](middleware/03-retry.md#usage) · [Retry with Backoff](middleware/03-retry.md#retry-with-backoff)|
|[04-concurrency.md](middleware/04-concurrency.md)|Concurrency — ConcurrencyLimit, load shedding|
| |↳ [ConcurrencyLimit](middleware/04-concurrency.md#concurrencylimit) · [Buffer](middleware/04-concurrency.md#buffer) · [LoadShed](middleware/04-concurrency.md#loadshed) · [Combining Concurrency Controls](middleware/04-concurrency.md#combining-concurrency-controls)|
|[05-other.md](middleware/05-other.md)|Other — buffer, filter, discover|
| |↳ [Filter](middleware/05-other.md#filter) · [Balance](middleware/05-other.md#balance) · [Hedge](middleware/05-other.md#hedge) · [Reconnect](middleware/05-other.md#reconnect) · [SpawnReady](middleware/05-other.md#spawnready) · [Steer](middleware/05-other.md#steer) · [ReadyCache](middleware/05-other.md#readycache) · [MakeService](middleware/05-other.md#makeservice) · +1 more|

### [patterns](patterns/)

|file|description|
|---|---|
|[01-composition.md](patterns/01-composition.md)|Composition — stacking layers, ordering|
| |↳ [Layer Ordering](patterns/01-composition.md#layer-ordering) · [Per-Route vs Global Middleware](patterns/01-composition.md#per-route-vs-global-middleware) · [Conditional Middleware](patterns/01-composition.md#conditional-middleware) · [Type Erasure](patterns/01-composition.md#type-erasure) · [Error Handling in Middleware Stacks](patterns/01-composition.md#error-handling-in-middleware-stacks)|
|[02-custom-middleware.md](patterns/02-custom-middleware.md)|Custom middleware — implementing Layer + Service|
| |↳ [The Pattern](patterns/02-custom-middleware.md#the-pattern) · [Complete Example: Request Logging](patterns/02-custom-middleware.md#complete-example-request-logging) · [Simpler Alternative: Boxed Future](patterns/02-custom-middleware.md#simpler-alternative-boxed-future) · [HTTP-Specific Middleware](patterns/02-custom-middleware.md#http-specific-middleware) · [Using with Axum](patterns/02-custom-middleware.md#using-with-axum) · [Testing Middleware](patterns/02-custom-middleware.md#testing-middleware)|

### [tower-http](tower-http/)

|file|description|
|---|---|
|[01-overview.md](tower-http/01-overview.md)|tower-http — HTTP-specific middleware overview|
| |↳ [Crate Info](tower-http/01-overview.md#crate-info) · [All Available Middleware](tower-http/01-overview.md#all-available-middleware) · [Extension Traits](tower-http/01-overview.md#extension-traits) · [Quick Example](tower-http/01-overview.md#quick-example)|
|[02-request-response.md](tower-http/02-request-response.md)|Request/response — CORS, compression, set-header|
| |↳ [CorsLayer](tower-http/02-request-response.md#corslayer) · [CompressionLayer / DecompressionLayer](tower-http/02-request-response.md#compressionlayer-decompressionlayer) · [SetRequestHeaderLayer / SetResponseHeaderLayer](tower-http/02-request-response.md#setrequestheaderlayer-setresponseheaderlayer) · [PropagateHeaderLayer](tower-http/02-request-response.md#propagateheaderlayer) · [RequestBodyLimitLayer](tower-http/02-request-response.md#requestbodylimitlayer) · [NormalizePathLayer](tower-http/02-request-response.md#normalizepathlayer) · [ValidateRequestHeaderLayer](tower-http/02-request-response.md#validaterequestheaderlayer) · [AddExtensionLayer](tower-http/02-request-response.md#addextensionlayer) · +4 more|
|[03-observability.md](tower-http/03-observability.md)|Observability — TraceLayer, request tracing|
| |↳ [TraceLayer](tower-http/03-observability.md#tracelayer) · [CatchPanicLayer](tower-http/03-observability.md#catchpaniclayer) · [RequestIdLayer / SetRequestIdLayer](tower-http/03-observability.md#requestidlayer-setrequestidlayer) · [SensitiveHeadersLayer](tower-http/03-observability.md#sensitiveheaderslayer) · [Metrics / InFlightRequestsLayer](tower-http/03-observability.md#metrics-inflightrequestslayer) · [Auth / RequireAuthorizationLayer](tower-http/03-observability.md#auth-requireauthorizationlayer) · [Composing Observability Middleware](tower-http/03-observability.md#composing-observability-middleware)|

### Key Patterns
```rust
ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(RateLimitLayer::new(100, Duration::from_secs(1)))
    .layer(ConcurrencyLimitLayer::new(64))
    .service(my_service)
```

---
*14 files · Related: [axum](../axum/INDEX.md), [tonic](../tonic/INDEX.md), [tokio](../tokio/INDEX.md)*
