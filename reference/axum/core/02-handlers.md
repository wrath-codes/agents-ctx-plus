# Handlers

A handler is an async function that takes zero or more extractors and returns something implementing `IntoResponse`. Handlers are the primary way to process requests in Axum.

---

## Handler Signature

```rust
async fn handler(
    extractor_1: Extractor1,
    extractor_2: Extractor2,
    // ... up to 16 extractors
) -> impl IntoResponse {
    // process request and return response
}
```

Rules:
- Extractors implementing `FromRequestParts` can appear in any position
- At most ONE extractor implementing `FromRequest` (consumes the body) can appear, and it must be **last**
- The return type must implement `IntoResponse`

---

## Examples

### No Extractors

```rust
async fn root() -> &'static str {
    "Hello, World!"
}
```

### With Extractors

```rust
use axum::extract::{Path, Query, Json, State};
use serde::Deserialize;

#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(pagination): Query<Pagination>,
) -> Json<User> {
    let user = state.db.get_user(id).await;
    Json(user)
}
```

### With Body Extractor (must be last)

```rust
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,  // body extractor — must be last
) -> (StatusCode, Json<User>) {
    let user = state.db.create_user(payload).await;
    (StatusCode::CREATED, Json(user))
}
```

---

## The Handler Trait

```rust
pub trait Handler<T, S>: Clone + Send + Sized + 'static {
    type Future: Future<Output = Response> + Send + 'static;
    fn call(self, req: Request, state: S) -> Self::Future;
}
```

Any async function matching the extractor rules automatically implements `Handler`. You rarely need to implement it manually.

---

## Handler Methods

### with_state

Provide state to a handler outside of the router's `with_state`:

```rust
use axum::handler::Handler;

let handler = get_user.with_state(state);
```

---

## Closures as Handlers

Closures work as handlers if they are `Clone + Send + 'static`:

```rust
let app = Router::new()
    .route("/", get(|| async { "Hello!" }))
    .route("/greet/{name}", get(|Path(name): Path<String>| async move {
        format!("Hello, {}!", name)
    }));
```

---

## debug_handler

The `#[debug_handler]` attribute macro (feature `macros`) generates better compiler error messages when your handler doesn't satisfy trait bounds:

```rust
use axum::debug_handler;

#[debug_handler]
async fn handler(Json(body): Json<MyType>) -> impl IntoResponse {
    // ...
}
```

Without `#[debug_handler]`, handler trait errors can be very cryptic. With it, you get clear messages like "MyType does not implement Deserialize".

---

## See Also

- [Extractors](03-extractors.md) — what handlers can extract from requests
- [Responses](04-responses.md) — what handlers can return
- [Router](01-router.md) — routing handlers
- [Error Handling](../advanced/04-error-handling.md) — handling errors in handlers
