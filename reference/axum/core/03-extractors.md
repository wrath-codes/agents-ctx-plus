# Extractors

Extractors pull data from HTTP requests. They implement either `FromRequestParts` (for data from headers, path, query) or `FromRequest` (for data that consumes the body).

---

## Extractor Traits

```rust
pub trait FromRequestParts<S>: Sized {
    type Rejection: IntoResponse;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection>;
}

pub trait FromRequest<S, M = ViaRequest>: Sized {
    type Rejection: IntoResponse;
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection>;
}
```

- `FromRequestParts`: Can appear anywhere in handler parameters, multiple allowed
- `FromRequest`: Consumes the request body, must be the **last** parameter, only one allowed

---

## All Built-in Extractors

| Extractor | Trait | Feature | Source | Description |
|-----------|-------|---------|--------|-------------|
| `Path<T>` | Parts | — | URL path | Path parameters, `T: Deserialize` |
| `Query<T>` | Parts | `query` | Query string | Query params, `T: Deserialize` |
| `HeaderMap` | Parts | — | Headers | All request headers |
| `State<S>` | Parts | — | App state | Shared application state, `S: Clone` |
| `Extension<T>` | Parts | — | Extensions | Request extensions, `T: Clone + Send + Sync` |
| `ConnectInfo<T>` | Parts | `tokio` | Connection | Client connection info (e.g., remote address) |
| `MatchedPath` | Parts | `matched-path` | Router | The matched route pattern |
| `NestedPath` | Parts | — | Router | The prefix where the router is nested |
| `OriginalUri` | Parts | `original-uri` | Request | Original URI before nesting |
| `RawQuery` | Parts | — | Query string | Unparsed query string |
| `RawPathParams` | Parts | — | URL path | Unparsed path parameters |
| `Json<T>` | Request | `json` | Body | JSON body, `T: Deserialize` |
| `Form<T>` | Request | `form` | Body | URL-encoded form, `T: Deserialize` |
| `Multipart` | Request | `multipart` | Body | Multipart form data |
| `RawForm` | Request | — | Body | Raw form bytes |
| `String` | Request | — | Body | Body as UTF-8 string |
| `Bytes` | Request | — | Body | Raw body bytes |
| `Request` | Request | — | Full request | The entire `http::Request` |
| `WebSocketUpgrade` | Parts | `ws` | Headers | WebSocket upgrade |
| `DefaultBodyLimit` | — | — | — | Layer to configure body size limit |

---

## Path

Extract typed parameters from the URL path:

```rust
use axum::extract::Path;

// Single parameter
async fn get_user(Path(id): Path<u64>) -> String {
    format!("User {}", id)
}

// Multiple parameters
async fn get_comment(
    Path((post_id, comment_id)): Path<(u64, u64)>,
) -> String {
    format!("Post {} Comment {}", post_id, comment_id)
}

// Named parameters with struct
#[derive(Deserialize)]
struct PostComment {
    post_id: u64,
    comment_id: u64,
}

async fn get_comment(Path(params): Path<PostComment>) -> String {
    format!("Post {} Comment {}", params.post_id, params.comment_id)
}
```

---

## Query

Extract query string parameters:

```rust
use axum::extract::Query;

#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    per_page: Option<u32>,
}

// GET /items?page=2&per_page=10
async fn list_items(Query(pagination): Query<Pagination>) -> String {
    let page = pagination.page.unwrap_or(1);
    format!("Page {}", page)
}
```

---

## Json

Extract JSON from the request body (also usable as a response):

```rust
use axum::Json;

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

async fn create_user(Json(payload): Json<CreateUser>) -> (StatusCode, Json<User>) {
    let user = User { id: 1, name: payload.name, email: payload.email };
    (StatusCode::CREATED, Json(user))
}
```

---

## State

Extract shared application state:

```rust
use axum::extract::State;

#[derive(Clone)]
struct AppState {
    db: DatabasePool,
}

async fn handler(State(state): State<AppState>) -> String {
    let count = state.db.count().await;
    format!("Count: {}", count)
}
```

See [State Management](../advanced/03-state-management.md) for advanced patterns.

---

## Form

Extract URL-encoded form data:

```rust
use axum::Form;

#[derive(Deserialize)]
struct Login {
    username: String,
    password: String,
}

async fn login(Form(input): Form<Login>) -> String {
    format!("Welcome, {}!", input.username)
}
```

---

## Headers

Extract the full header map:

```rust
use axum::http::HeaderMap;

async fn handler(headers: HeaderMap) -> String {
    let user_agent = headers.get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    format!("User-Agent: {}", user_agent)
}
```

---

## Extension

Extract values from request extensions (set by middleware):

```rust
use axum::Extension;

#[derive(Clone)]
struct CurrentUser {
    id: u64,
    name: String,
}

async fn handler(Extension(user): Extension<CurrentUser>) -> String {
    format!("Hello, {}!", user.name)
}
```

---

## Optional Extractors

Wrap any extractor in `Option<T>` to make it optional:

```rust
async fn handler(user: Option<Extension<CurrentUser>>) -> String {
    match user {
        Some(Extension(u)) => format!("Hello, {}!", u.name),
        None => "Hello, anonymous!".to_string(),
    }
}
```

---

## Custom Extractors

Implement `FromRequestParts` or `FromRequest` for your own types:

```rust
use axum::extract::FromRequestParts;
use http::request::Parts;

struct ApiKey(String);

impl<S> FromRequestParts<S> for ApiKey
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.headers.get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|v| ApiKey(v.to_string()))
            .ok_or((StatusCode::UNAUTHORIZED, "Missing API key"))
    }
}
```

Or use the derive macros (feature `macros`):

```rust
#[derive(FromRequestParts)]
struct MyExtractor {
    state: State<AppState>,
    path: Path<u64>,
}
```

---

## See Also

- [Handlers](02-handlers.md) — extractor ordering rules
- [Responses](04-responses.md) — returning data from handlers
- [State Management](../advanced/03-state-management.md) — State patterns
- [axum-extra Extractors](../extras/01-axum-extra.md) — TypedHeader, CookieJar, etc.
