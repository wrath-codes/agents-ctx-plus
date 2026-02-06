# tower — Sub-Index

> Modular service middleware framework for Rust (15 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [core](core/)

|file|description|
|---|---|
|[01-service-trait.md](core/01-service-trait.md)|Service trait — poll_ready, call, Request/Response types|
|[02-layer-trait.md](core/02-layer-trait.md)|Layer trait — wrapping services, layer composition|
|[03-service-builder.md](core/03-service-builder.md)|ServiceBuilder — fluent middleware stacking|

### [middleware](middleware/)

|file|description|
|---|---|
|[01-timeout.md](middleware/01-timeout.md)|Timeout — request deadline enforcement|
|[02-rate-limit.md](middleware/02-rate-limit.md)|Rate limit — request throttling|
|[03-retry.md](middleware/03-retry.md)|Retry — automatic retry with policies|
|[04-concurrency.md](middleware/04-concurrency.md)|Concurrency — ConcurrencyLimit, load shedding|
|[05-other.md](middleware/05-other.md)|Other — buffer, filter, discover|

### [patterns](patterns/)

|file|description|
|---|---|
|[01-composition.md](patterns/01-composition.md)|Composition — stacking layers, ordering|
|[02-custom-middleware.md](patterns/02-custom-middleware.md)|Custom middleware — implementing Layer + Service|

### [tower-http](tower-http/)

|file|description|
|---|---|
|[01-overview.md](tower-http/01-overview.md)|tower-http — HTTP-specific middleware overview|
|[02-request-response.md](tower-http/02-request-response.md)|Request/response — CORS, compression, set-header|
|[03-observability.md](tower-http/03-observability.md)|Observability — TraceLayer, request tracing|

### Key Patterns
```rust
ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(RateLimitLayer::new(100, Duration::from_secs(1)))
    .layer(ConcurrencyLimitLayer::new(64))
    .service(my_service)
```

---
*15 files · Related: [axum](../axum/INDEX.md), [tonic](../tonic/INDEX.md), [tokio](../tokio/INDEX.md)*
