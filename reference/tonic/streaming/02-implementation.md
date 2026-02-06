# Streaming Implementation

Details on implementing streaming RPCs with Tonic: the `Streaming` type, creating streams, and client-side consumption.

---

## The Streaming Type

`tonic::Streaming<T>` implements `Stream<Item = Result<T, Status>>`:

```rust
pub struct Streaming<T> { /* ... */ }

impl<T> Stream for Streaming<T> {
    type Item = Result<T, Status>;
}
```

Used in client-streaming and bidirectional patterns:

```rust
async fn my_method(
    &self,
    request: Request<Streaming<MyMessage>>,
) -> Result<Response<MyReply>, Status> {
    let mut stream = request.into_inner();

    while let Some(msg) = stream.next().await {
        let msg = msg?;
        // process msg
    }

    Ok(Response::new(MyReply { /* ... */ }))
}
```

---

## Creating Response Streams

### Using ReceiverStream

The most common approach — use a `tokio::sync::mpsc` channel:

```rust
use tokio_stream::wrappers::ReceiverStream;

type MyStream = ReceiverStream<Result<MyMessage, Status>>;

async fn list_items(
    &self,
    _request: Request<ListRequest>,
) -> Result<Response<Self::ListItemsStream>, Status> {
    let (tx, rx) = tokio::sync::mpsc::channel(128);

    tokio::spawn(async move {
        for i in 0..100 {
            let msg = MyMessage { id: i };
            if tx.send(Ok(msg)).await.is_err() {
                break;
            }
        }
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}
```

### Using async-stream

Create streams with `async`/`await` syntax:

```rust
use async_stream::try_stream;

type MyStream = Pin<Box<dyn Stream<Item = Result<MyMessage, Status>> + Send>>;

async fn list_items(
    &self,
    _request: Request<ListRequest>,
) -> Result<Response<Self::ListItemsStream>, Status> {
    let stream = try_stream! {
        for i in 0..100 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            yield MyMessage { id: i };
        }
    };

    Ok(Response::new(Box::pin(stream)))
}
```

### Using tokio_stream::iter

For static data:

```rust
use tokio_stream::iter;

let items = vec![
    Ok(MyMessage { id: 1 }),
    Ok(MyMessage { id: 2 }),
    Ok(MyMessage { id: 3 }),
];

let stream = iter(items);
```

---

## Client-Side Streaming

### Consuming Server Streams

```rust
let mut stream = client.list_items(Request::new(ListRequest {}))
    .await?
    .into_inner();

while let Some(item) = stream.next().await {
    match item {
        Ok(msg) => println!("Received: {:?}", msg),
        Err(status) => {
            eprintln!("Error: {}", status);
            break;
        }
    }
}
```

### Sending Client Streams

```rust
use tokio_stream::iter;

let messages = vec![
    Point { latitude: 1, longitude: 2 },
    Point { latitude: 3, longitude: 4 },
];

let request = Request::new(iter(messages));
let response = client.record_route(request).await?;
println!("Summary: {:?}", response.into_inner());
```

### Bidirectional from Client

```rust
let (tx, rx) = tokio::sync::mpsc::channel(128);

// Send messages in a background task
tokio::spawn(async move {
    for i in 0..10 {
        tx.send(ChatMessage { message: format!("msg {}", i) }).await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
});

let request = Request::new(ReceiverStream::new(rx));
let mut response_stream = client.chat(request).await?.into_inner();

while let Some(reply) = response_stream.next().await {
    println!("Reply: {:?}", reply?);
}
```

---

## Required Dependencies

```toml
[dependencies]
tokio-stream = "0.1"
async-stream = "0.3"  # optional, for try_stream! macro
futures = "0.3"        # optional, for Stream utilities
```

---

## See Also

- [Streaming Patterns](01-patterns.md) — all four gRPC patterns
- [Server](../core/01-server.md) — server implementation
- [Client](../core/02-client.md) — client usage
- [Tokio Streams](../../tokio/tutorial/08-streams.md) — async iteration
