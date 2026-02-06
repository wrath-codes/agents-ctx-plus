# Testing Axum Applications

Axum applications can be tested without starting a real HTTP server by using Tower's `ServiceExt::oneshot`.

---

## Using Tower oneshot

Since every Axum `Router` is a Tower `Service`, you can call it directly:

```rust
use axum::{routing::get, Router, body::Body, http::Request};
use tower::ServiceExt;
use http::StatusCode;
use http_body_util::BodyExt;

#[tokio::test]
async fn test_root() {
    let app = Router::new().route("/", get(|| async { "Hello!" }));

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"Hello!");
}
```

---

## Testing with State

```rust
#[tokio::test]
async fn test_with_state() {
    let state = AppState { /* ... */ };
    let app = Router::new()
        .route("/users", get(list_users))
        .with_state(state);

    let response = app
        .oneshot(Request::builder().uri("/users").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

---

## Testing JSON Endpoints

```rust
use serde_json::json;

#[tokio::test]
async fn test_create_user() {
    let app = create_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Alice"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let user: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(user["name"], "Alice");
}
```

---

## Multiple Requests

`oneshot` consumes the service. For multiple requests, clone the router or use `into_service`:

```rust
#[tokio::test]
async fn test_multiple_requests() {
    let app = create_app();

    // First request
    let response = app.clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Second request
    let response = app
        .oneshot(Request::builder().uri("/other").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
```

---

## Testing Helper Pattern

Create a helper function to build your app for tests:

```rust
fn test_app() -> Router {
    let state = AppState::for_testing();
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user))
        .with_state(state)
}
```

---

## See Also

- [Router](../core/01-router.md) — Router API
- [Handlers](../core/02-handlers.md) — handler functions
- [Tokio Test](../../tokio/rust-api/01-runtime.md) — `#[tokio::test]` macro
