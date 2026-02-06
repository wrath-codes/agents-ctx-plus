# Request & Response Middleware

HTTP-specific Tower middleware for manipulating requests, responses, headers, bodies, and cross-origin policies.

---

## CorsLayer

Adds Cross-Origin Resource Sharing headers to responses.

```rust
use tower_http::cors::{CorsLayer, Any};
use http::Method;
use std::time::Duration;

// Permissive (allow everything)
let cors = CorsLayer::permissive();

// Very permissive (like permissive but also allows credentials)
let cors = CorsLayer::very_permissive();

// Custom configuration
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers(Any)
    .max_age(Duration::from_secs(3600));

// Specific origins
use tower_http::cors::AllowOrigin;
let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::exact("https://example.com".parse().unwrap()));
```

Feature: `cors`

---

## CompressionLayer / DecompressionLayer

Compress response bodies or decompress request bodies.

```rust
use tower_http::compression::CompressionLayer;
use tower_http::decompression::DecompressionLayer;

// Compress responses (auto-negotiates via Accept-Encoding)
let compression = CompressionLayer::new();

// Decompress requests
let decompression = DecompressionLayer::new();
```

Supported algorithms: gzip, brotli, deflate, zstd. Each has its own feature flag (`compression-gzip`, `compression-br`, etc.).

---

## SetRequestHeaderLayer / SetResponseHeaderLayer

Add or override headers on requests or responses.

```rust
use tower_http::set_header::SetResponseHeaderLayer;
use http::HeaderValue;

// Always set the header
let layer = SetResponseHeaderLayer::overriding(
    http::header::SERVER,
    HeaderValue::from_static("my-server/1.0"),
);

// Only set if not already present
let layer = SetResponseHeaderLayer::if_not_present(
    http::header::CONTENT_TYPE,
    HeaderValue::from_static("application/json"),
);
```

Feature: `set-header`

---

## PropagateHeaderLayer

Copy a header from the request to the response.

```rust
use tower_http::propagate_header::PropagateHeaderLayer;

let layer = PropagateHeaderLayer::new(http::header::HeaderName::from_static("x-request-id"));
```

Feature: `propagate-header`

---

## RequestBodyLimitLayer

Limit the size of request bodies.

```rust
use tower_http::limit::RequestBodyLimitLayer;

// Limit to 2 MB
let layer = RequestBodyLimitLayer::new(2 * 1024 * 1024);
```

Feature: `limit`

---

## NormalizePathLayer

Normalize URL paths by removing trailing slashes or merging consecutive slashes.

```rust
use tower_http::normalize_path::NormalizePathLayer;

let layer = NormalizePathLayer::trim_trailing_slash();
```

Feature: `normalize-path`

---

## ValidateRequestHeaderLayer

Validate incoming request headers.

```rust
use tower_http::validate_request::ValidateRequestHeaderLayer;

// Require a specific content-type
let layer = ValidateRequestHeaderLayer::accept("application/json");

// Require basic auth
let layer = ValidateRequestHeaderLayer::basic("username", "password");

// Require bearer token
let layer = ValidateRequestHeaderLayer::bearer("my-secret-token");
```

Feature: `validate-request`

---

## AddExtensionLayer

Inject a value into request extensions (accessible by handlers):

```rust
use tower_http::add_extension::AddExtensionLayer;

#[derive(Clone)]
struct AppConfig {
    database_url: String,
}

let layer = AddExtensionLayer::new(AppConfig {
    database_url: "postgres://localhost/mydb".to_string(),
});
```

Feature: `add-extension`

---

## ServeDir / ServeFile

Serve static files from a directory or a single file.

```rust
use tower_http::services::{ServeDir, ServeFile};

// Serve files from a directory
let serve_dir = ServeDir::new("static/")
    .not_found_service(ServeFile::new("static/404.html"));

// Serve a single file
let serve_file = ServeFile::new("static/index.html");
```

Feature: `fs`

---

## FollowRedirectLayer

Follow HTTP redirects (for client-side usage).

```rust
use tower_http::follow_redirect::FollowRedirectLayer;

let layer = FollowRedirectLayer::new();
```

Feature: `follow-redirect`

---

## MapRequestBodyLayer / MapResponseBodyLayer

Transform request or response bodies.

```rust
use tower_http::map_request_body::MapRequestBodyLayer;

let layer = MapRequestBodyLayer::new(|body| {
    // transform body
    body
});
```

Features: `map-request-body`, `map-response-body`

---

## SetStatusLayer

Override the response status code.

```rust
use tower_http::set_status::SetStatusLayer;
use http::StatusCode;

let layer = SetStatusLayer::new(StatusCode::OK);
```

Feature: `set-status`

---

## See Also

- [Observability Middleware](03-observability.md) — tracing, catch-panic, request IDs
- [Overview](01-overview.md) — all available middleware
- [Axum with tower-http](../../axum/middleware/02-tower-integration.md) — integration patterns
