# Streaming Patterns

gRPC supports four communication patterns. Tonic implements all of them.

---

## Pattern Overview

```
┌─────────────────────────────────────────────────────────┐
│                   gRPC Patterns                          │
│                                                          │
│  1. Unary           Client ──[1 msg]──► Server           │
│                     Client ◄──[1 msg]── Server           │
│                                                          │
│  2. Server Stream   Client ──[1 msg]──► Server           │
│                     Client ◄──[N msgs]─ Server           │
│                                                          │
│  3. Client Stream   Client ──[N msgs]─► Server           │
│                     Client ◄──[1 msg]── Server           │
│                                                          │
│  4. Bidirectional   Client ──[N msgs]─► Server           │
│                     Client ◄──[N msgs]─ Server           │
└─────────────────────────────────────────────────────────┘
```

---

## Proto Definitions

```protobuf
service Chat {
    // 1. Unary
    rpc GetUser (GetUserRequest) returns (User);

    // 2. Server streaming
    rpc ListUsers (ListUsersRequest) returns (stream User);

    // 3. Client streaming
    rpc RecordRoute (stream Point) returns (RouteSummary);

    // 4. Bidirectional streaming
    rpc Chat (stream ChatMessage) returns (stream ChatMessage);
}
```

---

## 1. Unary RPC

One request, one response.

```rust
#[tonic::async_trait]
impl Chat for MyChatService {
    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<User>, Status> {
        let id = request.into_inner().id;
        let user = self.db.find_user(id).await
            .ok_or_else(|| Status::not_found("User not found"))?;
        Ok(Response::new(user))
    }
}
```

---

## 2. Server Streaming

One request, stream of responses.

```rust
use tokio_stream::wrappers::ReceiverStream;

#[tonic::async_trait]
impl Chat for MyChatService {
    type ListUsersStream = ReceiverStream<Result<User, Status>>;

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<Self::ListUsersStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let users = self.db.get_all_users().await;

        tokio::spawn(async move {
            for user in users {
                if tx.send(Ok(user)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
```

---

## 3. Client Streaming

Stream of requests, one response.

```rust
use tokio_stream::StreamExt;

#[tonic::async_trait]
impl Chat for MyChatService {
    async fn record_route(
        &self,
        request: Request<Streaming<Point>>,
    ) -> Result<Response<RouteSummary>, Status> {
        let mut stream = request.into_inner();
        let mut points = Vec::new();

        while let Some(point) = stream.next().await {
            let point = point?;
            points.push(point);
        }

        let summary = RouteSummary {
            point_count: points.len() as i32,
            // ...
        };

        Ok(Response::new(summary))
    }
}
```

---

## 4. Bidirectional Streaming

Stream of requests and stream of responses simultaneously.

```rust
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

#[tonic::async_trait]
impl Chat for MyChatService {
    type ChatStream = ReceiverStream<Result<ChatMessage, Status>>;

    async fn chat(
        &self,
        request: Request<Streaming<ChatMessage>>,
    ) -> Result<Response<Self::ChatStream>, Status> {
        let mut inbound = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            while let Some(msg) = inbound.next().await {
                match msg {
                    Ok(chat_msg) => {
                        let reply = ChatMessage {
                            message: format!("Echo: {}", chat_msg.message),
                            ..Default::default()
                        };
                        if tx.send(Ok(reply)).await.is_err() {
                            break;
                        }
                    }
                    Err(status) => {
                        eprintln!("Error: {}", status);
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
```

---

## See Also

- [Implementation Details](02-implementation.md) — Streaming type, ReceiverStream, async-stream
- [Code Generation](../core/03-codegen.md) — generated method signatures
- [Server](../core/01-server.md) — implementing the server
- [Client](../core/02-client.md) — consuming streams from the client
