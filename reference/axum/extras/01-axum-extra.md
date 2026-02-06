# axum-extra

The `axum-extra` crate provides additional extractors, responses, routing utilities, and middleware beyond what's in the core `axum` crate.

---

## Crate Info

| | |
|---|---|
| **Crate** | [`axum-extra`](https://crates.io/crates/axum-extra) |
| **Docs** | [docs.rs/axum-extra](https://docs.rs/axum-extra) |

---

## TypedHeader

Extract and return typed HTTP headers using the `headers` crate.

```rust
use axum_extra::TypedHeader;
use headers::{UserAgent, ContentType, Authorization, authorization::Bearer};

async fn handler(
    TypedHeader(user_agent): TypedHeader<UserAgent>,
) -> String {
    format!("User-Agent: {}", user_agent)
}

async fn auth_handler(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> String {
    format!("Token: {}", auth.token())
}
```

Feature: `typed-header`

---

## CookieJar

Extract and set cookies.

```rust
use axum_extra::extract::cookie::{CookieJar, Cookie};

async fn handler(jar: CookieJar) -> (CookieJar, &'static str) {
    let updated = jar.add(Cookie::new("session", "abc123"));
    (updated, "Cookie set!")
}

async fn read_cookie(jar: CookieJar) -> String {
    jar.get("session")
        .map(|c| c.value().to_string())
        .unwrap_or_else(|| "No session".to_string())
}
```

Also available: `PrivateCookieJar` (encrypted) and `SignedCookieJar` (signed with HMAC).

Feature: `cookie`, `cookie-private`, `cookie-signed`

---

## Query (with better errors)

A `Query` extractor with improved error messages compared to the core one.

Feature: `query`

---

## Protobuf

Extract and return Protocol Buffer messages.

```rust
use axum_extra::protobuf::Protobuf;

async fn handler(Protobuf(msg): Protobuf<MyProtoMessage>) -> Protobuf<MyProtoMessage> {
    Protobuf(msg)
}
```

Feature: `protobuf`

---

## JSON Lines

Newline-delimited JSON for streaming.

```rust
use axum_extra::json_lines::JsonLines;
use tokio_stream::Stream;

async fn handler() -> JsonLines<impl Stream<Item = serde_json::Value>> {
    let stream = tokio_stream::iter(vec![
        serde_json::json!({"id": 1}),
        serde_json::json!({"id": 2}),
    ]);
    JsonLines::new(stream)
}
```

Feature: `json-lines`

---

## Erased JSON

Type-erased JSON responses that can serialize any `Serialize` type without generics.

```rust
use axum_extra::response::ErasedJson;

async fn handler() -> ErasedJson {
    ErasedJson::new(serde_json::json!({"key": "value"}))
}

// Or using the macro
use axum_extra::json;
async fn handler2() -> ErasedJson {
    json!({"key": "value"})
}
```

Feature: `erased-json`

---

## Typed Routing

Define routes using types instead of string paths.

```rust
use axum_extra::routing::{TypedPath, RouterExt};
use serde::Deserialize;

#[derive(TypedPath, Deserialize)]
#[typed_path("/users/{id}")]
struct UserPath {
    id: u64,
}

async fn get_user(UserPath { id }: UserPath) -> String {
    format!("User {}", id)
}

let app = Router::new()
    .typed_get(get_user);
```

Feature: `typed-routing`

---

## Either Types

Combine extractors or responses:

```rust
use axum_extra::either::Either;

async fn handler(
    body: Either<Json<Data>, Form<Data>>,
) -> impl IntoResponse {
    match body {
        Either::E1(Json(data)) => { /* JSON */ }
        Either::E2(Form(data)) => { /* Form */ }
    }
}
```

---

## All Features

| Feature | Description |
|---------|-------------|
| `typed-header` | TypedHeader extractor/response |
| `cookie` | CookieJar extractor |
| `cookie-private` | Encrypted cookies |
| `cookie-signed` | Signed cookies |
| `query` | Improved Query extractor |
| `form` | Improved Form extractor |
| `protobuf` | Protocol Buffer support |
| `json-lines` | Newline-delimited JSON |
| `erased-json` | Type-erased JSON responses |
| `typed-routing` | Type-safe route definitions |
| `routing` | RouterExt with additional routing methods |
| `handler` | HandlerCallWithExtractors |
| `middleware` | Additional middleware utilities |

---

## See Also

- [Core Extractors](../core/03-extractors.md) — built-in extractors
- [Responses](../core/04-responses.md) — built-in response types
- [Testing](02-testing.md) — testing Axum applications
