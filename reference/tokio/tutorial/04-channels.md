# Channels and Message Passing

## Overview

Instead of sharing state with a Mutex, we use message passing: a dedicated task owns the resource, and other tasks communicate with it via channels. This is the actor pattern adapted for Tokio.

## The Problem

Suppose multiple tasks need to issue Redis commands through a single `client::Client`. The `Client` methods take `&mut self` — you can't share a mutable reference across tasks. Wrapping it in `Mutex<Client>` works but prevents pipelining (only one request at a time).

## Message Passing Pattern

A single **manager task** owns the `Client`. Other tasks send commands through a channel and receive responses through a one-shot channel:

```
[Task 1] --cmd--> [mpsc channel] --> [Manager Task] --> Client
[Task 2] --cmd--> [mpsc channel] --> [Manager Task] --> Client
```

## Tokio Channel Types

| Channel | Producers | Consumers | Values | Use Case |
|---------|-----------|-----------|--------|----------|
| `mpsc` | Many | One | Stream | Command queues, work distribution |
| `oneshot` | One | One | Single | Request-response, task results |
| `broadcast` | Many | Many | Stream | Event bus, all consumers see all messages |
| `watch` | One | Many | Latest | Config changes, state updates |

## Implementing with mpsc + oneshot

### Step 1: Define the Command Type

```rust
use bytes::Bytes;
use mini_redis::client::Client;
use tokio::sync::{mpsc, oneshot};

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}
```

Each command variant carries a `oneshot::Sender` so the manager can send the result back to the requesting task.

### Step 2: Create the Channel

```rust
#[tokio::main]
async fn main() {
    // Bounded channel with capacity 32
    let (tx, mut rx) = mpsc::channel::<Command>(32);

    // ...
}
```

### Step 3: Spawn Producer Tasks

Clone the `Sender` for each task that needs to send commands:

```rust
// Spawn task that sends SET
let tx2 = tx.clone();
let t1 = tokio::spawn(async move {
    let (resp_tx, resp_rx) = oneshot::channel();

    tx2.send(Command::Set {
        key: "foo".to_string(),
        val: "bar".into(),
        resp: resp_tx,
    }).await.unwrap();

    let res = resp_rx.await.unwrap();
    println!("SET result: {:?}", res);
});

// Spawn task that sends GET
let t2 = tokio::spawn(async move {
    let (resp_tx, resp_rx) = oneshot::channel();

    tx.send(Command::Get {
        key: "foo".to_string(),
        resp: resp_tx,
    }).await.unwrap();

    let res = resp_rx.await.unwrap();
    println!("GET result: {:?}", res);
});
```

### Step 4: Spawn the Manager Task

The manager owns the `Client` and processes commands sequentially:

```rust
let manager = tokio::spawn(async move {
    let mut client = client::connect("127.0.0.1:6379").await.unwrap();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Get { key, resp } => {
                let res = client.get(&key).await;
                // Ignore error if receiver dropped
                let _ = resp.send(res);
            }
            Command::Set { key, val, resp } => {
                let res = client.set(&key, val).await;
                let _ = resp.send(res);
            }
        }
    }
});
```

The `while let Some(cmd) = rx.recv().await` loop runs until all `Sender` handles are dropped (channel closed).

### Step 5: Wait for Completion

```rust
t1.await.unwrap();
t2.await.unwrap();
manager.await.unwrap();
```

## Complete Example

```rust
use bytes::Bytes;
use mini_redis::client;
use tokio::sync::{mpsc, oneshot};

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get { key, resp } => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                }
                Command::Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    let _ = resp.send(res);
                }
            }
        }
    });

    let tx2 = tx.clone();

    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        tx2.send(Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            resp: resp_tx,
        }).await.unwrap();

        let res = resp_rx.await.unwrap();
        println!("SET result: {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(Command::Get {
            key: "foo".to_string(),
            resp: resp_tx,
        }).await.unwrap();

        let res = resp_rx.await.unwrap();
        println!("GET result: {:?}", res);
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
```

## Backpressure and Bounded Channels

### Why Bounded?

Tokio async operations are lazy — they don't do work until polled. Without backpressure, a fast producer can overwhelm a slow consumer by filling unbounded queues and exhausting memory.

Bounded channels provide natural backpressure:

```rust
// Channel holds at most 32 pending messages
let (tx, rx) = mpsc::channel(32);

// If the channel is full, this suspends the calling task
// until space is available
tx.send(value).await.unwrap();
```

### Choosing Capacity

- **Too small** — producers block frequently, throughput drops
- **Too large** — memory usage grows, latency increases
- **Rule of thumb** — start with a small value (32, 64) and tune based on profiling

### Unbounded Channels

`mpsc::unbounded_channel()` exists but removes backpressure. Use only when you can guarantee the producer won't outpace the consumer (e.g., bounded by external rate limiting).

```rust
let (tx, rx) = mpsc::unbounded_channel();
tx.send(value).unwrap(); // Never blocks, returns Result (not async)
```

## Next Steps

- **[I/O](05-io.md)** - Read and write data with AsyncRead and AsyncWrite
