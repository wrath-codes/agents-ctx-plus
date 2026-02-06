# State Management

Axum provides the `State` extractor for sharing application state across handlers.

---

## Basic State

```rust
use axum::{extract::State, routing::get, Router};

#[derive(Clone)]
struct AppState {
    db: DatabasePool,
    config: AppConfig,
}

async fn handler(State(state): State<AppState>) -> String {
    format!("Config: {}", state.config.name)
}

let state = AppState { db: pool, config };
let app = Router::new()
    .route("/", get(handler))
    .with_state(state);
```

State must implement `Clone`. Use `Arc` for types that don't or are expensive to clone.

---

## Arc Pattern

```rust
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    inner: Arc<InnerState>,
}

struct InnerState {
    db: DatabasePool,
    cache: Cache,
}

let state = AppState {
    inner: Arc::new(InnerState { db: pool, cache }),
};
```

---

## Substates with FromRef

Extract a subset of state using the `FromRef` trait:

```rust
use axum::extract::FromRef;

#[derive(Clone)]
struct AppState {
    db: DatabasePool,
    config: AppConfig,
}

#[derive(Clone)]
struct DatabasePool { /* ... */ }

#[derive(Clone)]
struct AppConfig { /* ... */ }

impl FromRef<AppState> for DatabasePool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

// Handler can extract just the piece it needs
async fn handler(State(db): State<DatabasePool>) -> String {
    // only has access to db, not the full AppState
    "ok".to_string()
}
```

### Derive FromRef

```rust
#[derive(Clone, FromRef)]
struct AppState {
    db: DatabasePool,
    config: AppConfig,
}
```

The derive macro (feature `macros`) generates `FromRef` implementations for each field.

---

## Nested Routers with Different State

```rust
let api_router = Router::new()
    .route("/users", get(list_users))
    .with_state(api_state);

let admin_router = Router::new()
    .route("/stats", get(stats))
    .with_state(admin_state);

let app = Router::new()
    .nest("/api", api_router)
    .nest("/admin", admin_router);
```

---

## State vs Extension

| Aspect | `State<S>` | `Extension<T>` |
|--------|-----------|----------------|
| Set by | `Router::with_state()` | Middleware (e.g., `AddExtensionLayer`) |
| Compile-time checked | Yes | No (panics at runtime if missing) |
| Scope | Per-router | Per-request |
| Use when | App-wide shared state | Request-specific data (auth user, etc.) |

---

## See Also

- [Extractors](../core/03-extractors.md) — State extractor details
- [Router](../core/01-router.md) — with_state API
- [Error Handling](04-error-handling.md) — errors when state is missing
