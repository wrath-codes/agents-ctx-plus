# Responses

Axum uses the `IntoResponse` trait to convert handler return values into HTTP responses. Many types implement this trait, and tuples allow composing status codes, headers, and bodies.

---

## IntoResponse Trait

```rust
pub trait IntoResponse {
    fn into_response(self) -> Response;
}
```

Implemented for many types out of the box.

---

## Built-in Response Types

| Type | Description |
|------|-------------|
| `String` / `&str` | Text body with `text/plain` content type |
| `Json<T>` | JSON body with `application/json` content type |
| `Html<T>` | HTML body with `text/html` content type |
| `Redirect` | 3xx redirect response |
| `NoContent` | 204 No Content |
| `StatusCode` | Empty body with the given status code |
| `(StatusCode, impl IntoResponse)` | Response with custom status code |
| `(HeaderMap, impl IntoResponse)` | Response with custom headers |
| `(StatusCode, HeaderMap, impl IntoResponse)` | Status + headers + body |
| `AppendHeaders` | Append headers to a response |
| `ErrorResponse` | IntoResponse-based error |
| `Sse` | Server-Sent Events stream |
| `Response` | Raw `http::Response<Body>` |
| `Result<T, E>` | Where both `T` and `E` implement `IntoResponse` |

---

## Examples

### Simple Responses

```rust
// String
async fn text() -> &'static str {
    "Hello, World!"
}

// JSON
async fn json() -> Json<User> {
    Json(User { id: 1, name: "Alice".into() })
}

// HTML
use axum::response::Html;
async fn html() -> Html<&'static str> {
    Html("<h1>Hello!</h1>")
}

// Status code only
async fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}
```

### Tuples for Status + Headers + Body

```rust
use axum::http::{StatusCode, HeaderMap, header};

// Status + body
async fn created() -> (StatusCode, Json<User>) {
    (StatusCode::CREATED, Json(user))
}

// Status + headers + body
async fn with_headers() -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain")],
        "Hello".to_string(),
    )
}
```

### Redirect

```rust
use axum::response::Redirect;

async fn redirect() -> Redirect {
    Redirect::to("/new-location")
}

async fn permanent_redirect() -> Redirect {
    Redirect::permanent("/new-home")
}

async fn temporary_redirect() -> Redirect {
    Redirect::temporary("/temp")
}
```

### AppendHeaders

```rust
use axum::response::AppendHeaders;
use http::header;

async fn handler() -> (AppendHeaders<[(header::HeaderName, &'static str); 1]>, &'static str) {
    (
        AppendHeaders([(header::X_REQUEST_ID, "req-123")]),
        "Hello",
    )
}
```

---

## Custom IntoResponse

Implement `IntoResponse` for your own types:

```rust
use axum::response::{IntoResponse, Response};
use http::StatusCode;

struct AppError {
    code: StatusCode,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.code, self.message).into_response()
    }
}

async fn handler() -> Result<Json<User>, AppError> {
    let user = get_user().await.map_err(|e| AppError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: e.to_string(),
    })?;
    Ok(Json(user))
}
```

---

## IntoResponseParts

For types that contribute headers or extensions to a response without being the full body:

```rust
pub trait IntoResponseParts {
    type Error: IntoResponse;
    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error>;
}
```

---

## Result as Response

When both `T` and `E` implement `IntoResponse`, `Result<T, E>` does too:

```rust
async fn handler() -> Result<Json<Data>, (StatusCode, String)> {
    let data = fetch_data().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    Ok(Json(data))
}
```

---

## See Also

- [Extractors](03-extractors.md) — parsing requests
- [Handlers](02-handlers.md) — handler return types
- [Error Handling](../advanced/04-error-handling.md) — error response patterns
- [SSE](../advanced/02-sse.md) — Server-Sent Events responses
