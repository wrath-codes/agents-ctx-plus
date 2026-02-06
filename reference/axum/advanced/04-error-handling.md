# Error Handling

Axum's error handling is built on the `IntoResponse` trait. Handlers return `Result<T, E>` where both `T` and `E` implement `IntoResponse`.

---

## Basic Pattern

```rust
use axum::{http::StatusCode, Json};

async fn handler() -> Result<Json<Data>, (StatusCode, String)> {
    let data = fetch_data().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    Ok(Json(data))
}
```

---

## Custom Error Type

```rust
use axum::response::{IntoResponse, Response};
use http::StatusCode;

enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg).into_response()
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
            AppError::Internal(err) => {
                tracing::error!(?err, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}

async fn handler() -> Result<Json<Data>, AppError> {
    let data = fetch_data().await?; // auto-converts via From
    Ok(Json(data))
}
```

---

## JSON Error Responses

```rust
use serde_json::json;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error".into()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

## Extractor Rejections

When an extractor fails, it returns a "rejection" — a type implementing `IntoResponse`. You can customize rejection behavior:

```rust
use axum::extract::rejection::JsonRejection;

async fn handler(
    payload: Result<Json<Data>, JsonRejection>,
) -> impl IntoResponse {
    match payload {
        Ok(Json(data)) => (StatusCode::OK, Json(data)).into_response(),
        Err(rejection) => {
            let message = format!("Invalid JSON: {}", rejection);
            (StatusCode::BAD_REQUEST, message).into_response()
        }
    }
}
```

---

## HandleError

For Tower services that return errors that don't implement `IntoResponse`, use `HandleError`:

```rust
use axum::error_handling::HandleError;

let service = tower::ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .service(my_service);

let app = Router::new()
    .route_service("/", HandleError::new(service, |err: tower::BoxError| async move {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", err))
    }));
```

---

## See Also

- [Responses](../core/04-responses.md) — IntoResponse trait
- [Extractors](../core/03-extractors.md) — extractor rejections
- [Handlers](../core/02-handlers.md) — handler return types
