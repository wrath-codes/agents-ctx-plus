# Router

The `Router` is the central type in Axum. It maps URL paths and HTTP methods to handlers and services, supports nesting and merging of sub-routers, and applies middleware via Tower layers.

---

## API Reference

```rust
impl<S> Router<S> {
    pub fn new() -> Self
    pub fn route(self, path: &str, method_router: MethodRouter<S>) -> Self
    pub fn route_service<T>(self, path: &str, service: T) -> Self
    pub fn nest(self, path: &str, router: Router<S>) -> Self
    pub fn nest_service<T>(self, path: &str, service: T) -> Self
    pub fn merge<R>(self, other: R) -> Self
    pub fn layer<L>(self, layer: L) -> Router<S>
    pub fn route_layer<L>(self, layer: L) -> Self
    pub fn fallback<H, T>(self, handler: H) -> Self
    pub fn fallback_service<T>(self, service: T) -> Self
    pub fn with_state<S2>(self, state: S) -> Router<S2>
}
```

---

## Basic Routing

```rust
use axum::{routing::{get, post, put, delete}, Router};

let app = Router::new()
    .route("/", get(root_handler))
    .route("/users", get(list_users).post(create_user))
    .route("/users/{id}", get(get_user).put(update_user).delete(delete_user));
```

### Path Parameters

Use `{name}` syntax in route paths:

```rust
.route("/users/{id}", get(get_user))
.route("/posts/{post_id}/comments/{comment_id}", get(get_comment))
```

### Wildcard Routes

Use `{*path}` to match the rest of the path:

```rust
.route("/files/{*path}", get(serve_file))
```

---

## Method Routing Functions

| Function | HTTP Method |
|----------|-------------|
| `get(handler)` | GET |
| `post(handler)` | POST |
| `put(handler)` | PUT |
| `delete(handler)` | DELETE |
| `patch(handler)` | PATCH |
| `head(handler)` | HEAD |
| `options(handler)` | OPTIONS |
| `trace(handler)` | TRACE |
| `any(handler)` | Any method |
| `on(method_filter, handler)` | Custom method filter |

Each has a `_service` variant (e.g., `get_service`) for using Tower services directly instead of handlers.

### Chaining Methods

```rust
use axum::routing::{get, post, put, delete};

.route("/items/{id}", get(show).put(update).delete(remove))
```

---

## Nesting

Mount a sub-router at a path prefix:

```rust
let api = Router::new()
    .route("/users", get(list_users))
    .route("/items", get(list_items));

let app = Router::new()
    .nest("/api/v1", api)           // /api/v1/users, /api/v1/items
    .route("/health", get(health));
```

Nesting strips the prefix from the path before the inner router sees it. Use `OriginalUri` to access the full original path.

---

## Merging

Combine two routers at the same level:

```rust
let user_routes = Router::new()
    .route("/users", get(list_users));

let item_routes = Router::new()
    .route("/items", get(list_items));

let app = Router::new()
    .merge(user_routes)
    .merge(item_routes);
```

Merge will panic if both routers have overlapping routes.

---

## Fallback

Handle requests that don't match any route:

```rust
async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

let app = Router::new()
    .route("/", get(root))
    .fallback(not_found);
```

---

## Layer vs route_layer

| Method | Applies To |
|--------|-----------|
| `.layer(layer)` | All routes AND the fallback |
| `.route_layer(layer)` | Only matched routes (not the fallback) |

```rust
let app = Router::new()
    .route("/protected", get(protected_handler))
    .route_layer(RequireAuthorizationLayer::bearer("token"))  // only on matched routes
    .layer(TraceLayer::new_for_http());                        // on everything
```

---

## State

Provide shared state to handlers:

```rust
#[derive(Clone)]
struct AppState {
    db: DatabasePool,
}

let state = AppState { db: pool };

let app = Router::new()
    .route("/users", get(list_users))
    .with_state(state);

async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    let users = state.db.get_users().await;
    Json(users)
}
```

---

## Serving

```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app).await.unwrap();
```

---

## See Also

- [Handlers](02-handlers.md) — handler trait and async functions
- [Extractors](03-extractors.md) — pulling data from requests
- [State Management](../advanced/03-state-management.md) — state patterns
- [Tower Integration](../middleware/02-tower-integration.md) — using layers
