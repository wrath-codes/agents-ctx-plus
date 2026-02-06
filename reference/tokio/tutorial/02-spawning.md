# Spawning Tasks

## Overview

We build a Redis server that accepts TCP connections and handles them concurrently using `tokio::spawn()`. This introduces Tokio tasks — lightweight, async green threads.

## Accepting TCP Connections

```rust
use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        process(socket).await;
    }
}

async fn process(socket: TcpStream) {
    let mut connection = Connection::new(socket);

    if let Some(frame) = connection.read_frame().await.unwrap() {
        println!("GOT: {:?}", frame);

        let response = Frame::Error("unimplemented".to_string());
        connection.write_frame(&response).await.unwrap();
    }
}
```

### The Problem: Sequential Processing

The loop above calls `process(socket).await` — it waits for each connection to finish before accepting the next one. Only one client is served at a time.

## Solution: tokio::spawn()

`tokio::spawn()` runs a future concurrently as a separate task:

```rust
#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        // Spawn a new task for each connection
        tokio::spawn(async move {
            process(socket).await;
        });
    }
}
```

Now the accept loop immediately moves on to the next connection while each spawned task handles its client independently.

## Tasks Explained

Tokio tasks are asynchronous green threads:

- **Lightweight**: A task starts at ~64 bytes (plus the size of the future). No OS thread per task.
- **Scheduled by Tokio**: The runtime multiplexes tasks onto a small thread pool.
- **Scale freely**: Spawn thousands or millions of tasks without concern.

### JoinHandle

`tokio::spawn()` returns a `JoinHandle` that you can `.await` to get the task's result:

```rust
#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        // Do some async work
        "return value"
    });

    // Wait for the task to finish
    let out = handle.await.unwrap();
    println!("GOT {}", out);
}
```

Awaiting a `JoinHandle` returns `Result<T, JoinError>`. The `Err` case occurs when the task panics or is cancelled (handle dropped).

## The 'static Bound

Spawned tasks must have a `'static` lifetime — they must own all data they use. You cannot borrow from the spawning scope:

```rust
// This does NOT compile:
#[tokio::main]
async fn main() {
    let v = vec![1, 2, 3];

    tokio::spawn(async {
        println!("Here's a vec: {:?}", v);
        // ERROR: `v` is borrowed, not owned
    });
}
```

### Fix: Use `move`

```rust
#[tokio::main]
async fn main() {
    let v = vec![1, 2, 3];

    tokio::spawn(async move {
        println!("Here's a vec: {:?}", v);
    });
}
```

### Sharing Data with Arc

If multiple tasks need access to the same data, use `Arc`:

```rust
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let data = Arc::new("shared data".to_string());

    for _ in 0..10 {
        let data = data.clone();
        tokio::spawn(async move {
            println!("{}", data);
        });
    }
}
```

### Why 'static?

The spawned task may outlive the scope that spawned it. The runtime can run the task at any time — even after the spawning function returns. The task must own everything it needs.

## The Send Bound

Data held across an `.await` point inside a spawned task must implement `Send`. The runtime may move the task between threads at any `.await` point.

### Rc Is Not Send

```rust
use std::rc::Rc;

// This does NOT compile:
#[tokio::main]
async fn main() {
    tokio::spawn(async {
        let rc = Rc::new("hello");
        // rc is held across the .await → not Send
        some_async_fn().await;
        println!("{}", rc);
    });
}
```

**Fix**: Scope the non-Send value so it's dropped before `.await`:

```rust
#[tokio::main]
async fn main() {
    tokio::spawn(async {
        {
            let rc = Rc::new("hello");
            println!("{}", rc);
            // rc is dropped here
        }
        some_async_fn().await;
    });
}
```

### std::sync::MutexGuard Is Not Send

The same applies to `std::sync::MutexGuard` — it must not be held across `.await`:

```rust
use std::sync::Mutex;

// This does NOT compile:
async fn bad() {
    let mu = Mutex::new(0);
    let guard = mu.lock().unwrap();
    some_async_fn().await; // guard held across .await → !Send
    drop(guard);
}

// Fix: scope the guard
async fn good() {
    let mu = Mutex::new(0);
    {
        let guard = mu.lock().unwrap();
        // use guard
    } // guard dropped before .await
    some_async_fn().await;
}
```

## Complete process() Function

```rust
use tokio::net::TcpStream;
use mini_redis::{Connection, Frame, Command};
use std::collections::HashMap;
use bytes::Bytes;

async fn process(socket: TcpStream) {
    let mut connection = Connection::new(socket);
    let mut db: HashMap<String, Bytes> = HashMap::new();

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Command::Set(cmd) => {
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Command::Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        connection.write_frame(&response).await.unwrap();
    }
}
```

Each connection gets its own `HashMap`. State is not shared between connections — the next tutorial addresses that.

## Next Steps

- **[Shared State](03-shared-state.md)** - Share state across tasks with Mutex and Arc
