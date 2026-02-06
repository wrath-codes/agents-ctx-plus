# Bridging with Synchronous Code

## Overview

Sometimes you need to call async code from synchronous contexts — CLI tools wrapping async libraries, GUI applications, or mixed codebases. Tokio provides several strategies for bridging the sync/async boundary.

## `#[tokio::main]` Expansion

The `#[tokio::main]` macro expands to a runtime builder:

```rust
#[tokio::main]
async fn main() {
    println!("hello");
}

// Expands to:
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            println!("hello");
        })
}
```

## Creating a Runtime Manually

For sync wrappers, create a `Runtime` instance directly:

```rust
use tokio::runtime::Runtime;

fn main() {
    let rt = Runtime::new().unwrap();

    let result = rt.block_on(async {
        // async work here
        do_something_async().await
    });
}
```

## BlockingClient Pattern

Wrap an async client with a runtime for synchronous usage:

```rust
use tokio::runtime::Runtime;
use tokio::net::TcpStream;

struct BlockingClient {
    inner: Client, // async client
    rt: Runtime,
}

impl BlockingClient {
    pub fn connect(addr: &str) -> Result<Self> {
        let rt = Runtime::new()?;
        let inner = rt.block_on(Client::connect(addr))?;
        Ok(BlockingClient { inner, rt })
    }

    pub fn get(&mut self, key: &str) -> Result<Option<Bytes>> {
        self.rt.block_on(self.inner.get(key))
    }

    pub fn set(&mut self, key: &str, value: Bytes) -> Result<()> {
        self.rt.block_on(self.inner.set(key, value))
    }
}
```

## Runtime Flavors for Bridging

### `current_thread`

Lightweight, single-threaded. Tasks only make progress during `block_on()` calls.

```rust
let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
```

- Low overhead
- Spawned tasks **stop** when `block_on()` returns
- Good for simple sync wrappers

### `multi_thread`

Full multi-threaded runtime. Spawned tasks continue running between `block_on()` calls.

```rust
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(2)
    .enable_all()
    .build()
    .unwrap();
```

- Spawned tasks run in the background
- Better for long-lived applications
- Higher resource usage

## Three Approaches

### Approach 1: Runtime + `block_on` (Simplest)

Direct blocking on async operations. Best for simple wrappers.

```rust
use tokio::runtime::Runtime;

fn main() {
    let rt = Runtime::new().unwrap();

    let data = rt.block_on(async {
        fetch_data("https://example.com").await
    });

    println!("got: {:?}", data);
}
```

### Approach 2: Runtime + `spawn` (Background Tasks)

Spawn tasks that run independently. Good for GUI applications that need background work.

```rust
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

fn main() {
    let rt = Runtime::new().unwrap();
    let (tx, mut rx) = mpsc::channel(32);

    // Spawn a background task
    rt.spawn(async move {
        loop {
            let data = fetch_updates().await;
            if tx.send(data).await.is_err() {
                break;
            }
        }
    });

    // Process results synchronously
    loop {
        match rt.block_on(rx.recv()) {
            Some(data) => update_ui(data),
            None => break,
        }
    }
}
```

### Approach 3: Runtime on Separate Thread + Message Passing (Most Flexible)

Run the runtime on a dedicated thread, communicate via channels. Follows the actor pattern.

```rust
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};
use std::thread;

enum Command {
    Get {
        key: String,
        resp: oneshot::Sender<Option<Bytes>>,
    },
    Set {
        key: String,
        value: Bytes,
        resp: oneshot::Sender<()>,
    },
}

fn main() {
    let (tx, mut rx) = mpsc::channel::<Command>(32);

    // Spawn runtime on a separate thread
    let runtime_thread = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            let mut client = Client::connect("127.0.0.1:6379").await.unwrap();

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Command::Get { key, resp } => {
                        let val = client.get(&key).await.unwrap();
                        let _ = resp.send(val);
                    }
                    Command::Set { key, value, resp } => {
                        client.set(&key, value).await.unwrap();
                        let _ = resp.send(());
                    }
                }
            }
        });
    });

    // Use from sync code
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.blocking_send(Command::Get {
        key: "foo".into(),
        resp: resp_tx,
    }).unwrap();
    let value = resp_rx.blocking_recv().unwrap();

    runtime_thread.join().unwrap();
}
```

## `Handle::current()`

Get a handle to the currently running runtime from within async code, useful for passing to sync callbacks:

```rust
use tokio::runtime::Handle;

async fn setup() {
    let handle = Handle::current();

    // Pass handle to sync code
    std::thread::spawn(move || {
        handle.block_on(async {
            // async work on the existing runtime
            do_async_work().await
        });
    });
}
```

**Warning:** Do not call `Handle::current()` outside of a Tokio runtime — it will panic.

## `spawn_blocking()` — Blocking from Async

Run blocking (sync) code from within async context without stalling the async runtime:

```rust
use tokio::task;

async fn process() -> Result<()> {
    // Offload CPU-intensive or blocking work to a dedicated thread pool
    let result = task::spawn_blocking(|| {
        // This runs on a blocking thread, not the async worker
        expensive_computation()
    }).await?;

    println!("result: {}", result);
    Ok(())
}
```

`spawn_blocking` is the inverse of `block_on`:

| Direction | Function | Context |
|-----------|----------|---------|
| Sync → Async | `Runtime::block_on()` | Call async from sync |
| Async → Sync | `task::spawn_blocking()` | Call blocking from async |

## See Also

- [Graceful Shutdown](./02-graceful-shutdown.md) — shutting down a bridged runtime cleanly
- [Select](../tutorial/07-select.md) — combining async operations
