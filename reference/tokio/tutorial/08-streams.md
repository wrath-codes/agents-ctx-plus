# Streams

## Overview

The `Stream` trait is the async equivalent of `Iterator`. Where `Iterator::next()` blocks, `Stream::poll_next()` returns `Poll::Pending` when no value is ready yet.

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Stream {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}
```

## tokio-stream Crate

The `Stream` trait and utilities live in the `tokio-stream` crate (not in `tokio` itself):

```toml
[dependencies]
tokio-stream = "0.1"
```

The `StreamExt` trait provides convenient methods similar to `Iterator` adapters.

```rust
use tokio_stream::StreamExt;
```

## Iteration

Use `while let` with `.next().await`:

```rust
use tokio_stream::StreamExt;

async fn process_stream(mut stream: impl tokio_stream::Stream<Item = i32> + Unpin) {
    while let Some(v) = stream.next().await {
        println!("got: {}", v);
    }
}
```

For streams that are not `Unpin`, pin them first:

```rust
use tokio_stream::StreamExt;

let stream = some_stream();
tokio::pin!(stream);

while let Some(v) = stream.next().await {
    println!("got: {}", v);
}
```

## Stream Adapters

Adapters transform streams, just like iterator adapters. They are composable.

```rust
use tokio_stream::StreamExt;

let values = tokio_stream::iter(vec![1, 2, 3, 4, 5]);

let doubled_evens = values
    .filter(|v| v % 2 == 0)
    .map(|v| v * 2)
    .take(2);

tokio::pin!(doubled_evens);

while let Some(v) = doubled_evens.next().await {
    println!("{}", v); // 4, 8
}
```

### Common Adapters

| Adapter | Description |
|---------|-------------|
| `map(fn)` | Transform each item |
| `filter(fn)` | Keep items matching predicate |
| `filter_map(fn)` | Filter and transform in one step |
| `take(n)` | Take first n items |
| `merge(other)` | Interleave items from two streams |
| `chain(other)` | Append another stream after this one exhausts |
| `throttle(duration)` | Rate-limit item emission |

### Real-World Example

```rust
use tokio_stream::StreamExt;
use mini_redis::client;

let mut subscriber = client.subscribe(vec!["numbers".into()]).await?;

let messages = subscriber
    .into_stream()
    .filter(|msg| msg.content.len() > 0)
    .map(|msg| msg.content)
    .take(3);

tokio::pin!(messages);

while let Some(content) = messages.next().await {
    println!("got: {:?}", content);
}
```

## StreamMap

Combines multiple keyed streams into one. Useful when you have a dynamic set of streams.

```rust
use tokio_stream::{StreamExt, StreamMap};

let mut map = StreamMap::new();

map.insert("ints", tokio_stream::iter(vec![1, 2, 3]));
map.insert("more", tokio_stream::iter(vec![4, 5, 6]));

while let Some((key, val)) = map.next().await {
    println!("{}: {}", key, val);
}
```

`StreamMap` yields `(K, V)` tuples identifying which stream produced each item. Streams are removed when exhausted.

## Implementing Stream Manually

Implement `poll_next` by delegating to inner futures:

```rust
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::Stream;
use tokio::sync::mpsc;

struct RecvStream {
    rx: mpsc::Receiver<i32>,
}

impl Stream for RecvStream {
    type Item = i32;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}
```

## async-stream Crate

The `async-stream` crate provides a `stream!` macro for creating streams without manual `poll_next` implementations:

```toml
[dependencies]
async-stream = "0.3"
```

```rust
use async_stream::stream;
use tokio_stream::Stream;

fn countdown(from: u32) -> impl Stream<Item = u32> {
    stream! {
        for i in (0..=from).rev() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            yield i;
        }
    }
}

#[tokio::main]
async fn main() {
    let s = countdown(5);
    tokio::pin!(s);

    use tokio_stream::StreamExt;
    while let Some(v) = s.next().await {
        println!("{}...", v);
    }
}
```

## tokio_stream::wrappers

Convert Tokio types into streams:

```rust
use tokio_stream::wrappers::{IntervalStream, ReceiverStream};
use tokio::sync::mpsc;
use tokio::time;
use tokio_stream::StreamExt;

// Interval as a stream
let interval = time::interval(time::Duration::from_millis(100));
let mut stream = IntervalStream::new(interval);
stream.next().await; // first tick

// mpsc::Receiver as a stream
let (tx, rx) = mpsc::channel(32);
let mut stream = ReceiverStream::new(rx);
```

### Available Wrappers

| Wrapper | Source Type |
|---------|------------|
| `IntervalStream` | `tokio::time::Interval` |
| `ReceiverStream` | `mpsc::Receiver` |
| `UnboundedReceiverStream` | `mpsc::UnboundedReceiver` |
| `BroadcastStream` | `broadcast::Receiver` |
| `WatchStream` | `watch::Receiver` |
| `TcpListenerStream` | `TcpListener` |
| `SignalStream` | `signal::unix::Signal` |

## See Also

- [Select](./07-select.md) — alternative approach to handling multiple async sources
- [Framing](./06-framing.md) — producing streams of frames from byte streams
