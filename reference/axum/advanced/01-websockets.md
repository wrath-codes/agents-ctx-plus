# WebSockets

Axum provides built-in WebSocket support via the `WebSocketUpgrade` extractor and `WebSocket` type.

Requires feature flag: `ws`

---

## Basic Example

```rust
use axum::{
    extract::ws::{WebSocketUpgrade, WebSocket, Message},
    response::Response,
    routing::get,
    Router,
};

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if socket.send(Message::Text(format!("Echo: {}", text))).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

let app = Router::new().route("/ws", get(ws_handler));
```

---

## WebSocketUpgrade

The `WebSocketUpgrade` extractor validates the WebSocket upgrade request and provides the `on_upgrade` method:

```rust
impl WebSocketUpgrade {
    pub fn on_upgrade<F, Fut>(self, callback: F) -> Response
    where
        F: FnOnce(WebSocket) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,

    pub fn protocols<I>(self, protocols: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,

    pub fn max_frame_size(self, max: usize) -> Self
    pub fn max_message_size(self, max: usize) -> Self
    pub fn max_write_buffer_size(self, max: usize) -> Self
    pub fn write_buffer_size(self, size: usize) -> Self
}
```

---

## Message Types

```rust
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<CloseFrame>),
}
```

---

## With State and Extractors

```rust
async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Use state inside the WebSocket handler
}
```

---

## See Also

- [Handlers](../core/02-handlers.md) — handler patterns
- [SSE](02-sse.md) — Server-Sent Events (alternative for server-to-client streaming)
- [Tokio Channels](../../tokio/rust-api/06-sync.md) — for message passing between WebSocket connections
