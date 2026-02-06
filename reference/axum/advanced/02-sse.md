# Server-Sent Events (SSE)

Axum supports Server-Sent Events for streaming data from server to client over HTTP.

---

## Basic Example

```rust
use axum::{
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use tokio_stream::StreamExt;
use std::convert::Infallible;
use std::time::Duration;

async fn sse_handler() -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = tokio_stream::wrappers::IntervalStream::new(
        tokio::time::interval(Duration::from_secs(1))
    )
    .map(|_| {
        Ok(Event::default().data("tick"))
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

let app = Router::new().route("/sse", get(sse_handler));
```

---

## Event Builder

```rust
use axum::response::sse::Event;

// Simple data
let event = Event::default().data("hello");

// Named event
let event = Event::default()
    .event("update")
    .data("new data");

// With ID
let event = Event::default()
    .id("msg-1")
    .data("hello");

// JSON data
let event = Event::default()
    .json_data(serde_json::json!({"key": "value"}))
    .unwrap();

// Retry interval
let event = Event::default()
    .retry(Duration::from_secs(5))
    .data("reconnect after 5s");

// Comment (for keep-alive)
let event = Event::default().comment("keep-alive");
```

---

## KeepAlive

```rust
use axum::response::sse::KeepAlive;
use std::time::Duration;

let keep_alive = KeepAlive::new()
    .interval(Duration::from_secs(15))
    .text("ping");
```

---

## See Also

- [WebSockets](01-websockets.md) — bidirectional real-time communication
- [Responses](../core/04-responses.md) — other response types
- [Tokio Streams](../../tokio/tutorial/08-streams.md) — async iteration
