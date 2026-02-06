# Cancellation and Cleanup

In Tokio, cancellation happens by **dropping** a future. There is no special cancel signal — when a future is dropped, it stops executing. Understanding this model is critical for writing correct async code that doesn't lose data.

---

## Cancellation by Dropping

A future is cancelled when it is dropped before returning `Poll::Ready`. The Rust `Drop` trait runs destructors as normal, but the async code after the last `.await` point **never executes**.

```rust
let handle = tokio::spawn(async {
    step_one().await;
    step_two().await;   // if dropped here...
    step_three().await; // ...this never runs
    cleanup();          // ...and neither does this
});

// Drop the JoinHandle — the task may or may not be cancelled depending
// on whether the runtime has already picked it up.
drop(handle);
```

Dropping a `JoinHandle` does **not** cancel the spawned task — it just detaches it. To actually cancel a spawned task, use `JoinHandle::abort()`.

---

## `select!` Cancels Non-Winning Branches

`tokio::select!` polls multiple futures concurrently and completes when the **first** one finishes. All other branches are **dropped** (cancelled):

```rust
use tokio::time::{sleep, Duration};

async fn fetch_data() -> String {
    // Simulate slow network call
    sleep(Duration::from_secs(10)).await;
    "data".into()
}

async fn with_timeout() -> Option<String> {
    tokio::select! {
        data = fetch_data() => Some(data),
        _ = sleep(Duration::from_secs(5)) => {
            // fetch_data() is DROPPED here — cancelled
            None
        }
    }
}
```

When the timeout wins, the `fetch_data()` future is dropped mid-execution. Any work it had in progress is abandoned.

---

## `JoinHandle::abort()`

For spawned tasks, `abort()` requests cancellation. The task is cancelled at its next `.await` point:

```rust
let handle = tokio::spawn(async {
    loop {
        do_work().await;
    }
});

// Cancel the task
handle.abort();

// Awaiting an aborted task returns Err(JoinError) with is_cancelled() == true
match handle.await {
    Ok(_) => println!("task completed normally"),
    Err(e) if e.is_cancelled() => println!("task was cancelled"),
    Err(e) => println!("task panicked: {e}"),
}
```

---

## Cancellation Safety

When a future is dropped between `.await` points, partially-completed operations may lose data. This is the **cancellation safety** problem.

### Unsafe Operations Under Cancellation

| Operation | Risk |
|-----------|------|
| `AsyncRead` / `AsyncWrite` with internal buffering | Buffered data is lost on drop |
| `tokio::sync::mpsc::Receiver::recv()` | A message may be removed from the channel but never processed |
| `tokio::io::AsyncBufReadExt::read_line()` | Partially read data in the internal buffer is lost |
| Multi-step operations without atomicity | Intermediate state may be inconsistent |

### Cancellation-Safe Operations

| Operation | Why Safe |
|-----------|----------|
| `tokio::sync::mpsc::Sender::send()` | Either the message is sent or it isn't |
| `tokio::sync::oneshot::Receiver::recv()` | The value stays in the channel until fully received |
| `tokio::net::TcpListener::accept()` | Connections queue in the OS; nothing is lost |
| `tokio::time::sleep()` | Dropping just stops the timer |

### Patterns for Cancellation-Safe Code

**Pattern 1: Move receive out of `select!`**

```rust
// UNSAFE — if another branch wins, the received message is lost
tokio::select! {
    Some(msg) = rx.recv() => { process(msg).await; }
    _ = shutdown.recv() => { return; }
}

// SAFE — use a persistent buffer
let mut next_msg = None;
loop {
    let msg = match next_msg.take() {
        Some(msg) => msg,
        None => match rx.recv().await {
            Some(msg) => msg,
            None => return,
        },
    };

    tokio::select! {
        _ = process(&msg) => {}
        _ = shutdown.recv() => {
            // msg is still available for cleanup
            return;
        }
    }
}
```

**Pattern 2: Use `tokio::sync::mpsc::Receiver::try_recv()` outside of `select!`**

```rust
loop {
    tokio::select! {
        _ = some_future() => {
            // After waking, drain the channel synchronously
            while let Ok(msg) = rx.try_recv() {
                process(msg);
            }
        }
        _ = shutdown.recv() => return,
    }
}
```

---

## CancellationToken (tokio-util)

`CancellationToken` provides **explicit, cooperative** cancellation — tasks check for cancellation and shut down cleanly rather than being dropped mid-operation.

### Basic Usage

```rust
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();

// Spawn a task that respects cancellation
let cloned_token = token.clone();
let handle = tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = cloned_token.cancelled() => {
                // Perform cleanup
                println!("task cancelled, cleaning up");
                return;
            }
            _ = do_work() => {
                println!("work done");
            }
        }
    }
});

// Later, signal cancellation
token.cancel();

// The task will finish its current iteration and then exit cleanly
handle.await.unwrap();
```

### Child Tokens

Child tokens are cancelled when their parent is cancelled, but cancelling a child does **not** cancel the parent. This enables hierarchical cancellation:

```rust
let parent = CancellationToken::new();

let child_a = parent.child_token();
let child_b = parent.child_token();

// Cancel just one subsystem
child_a.cancel();
assert!(child_a.is_cancelled());
assert!(!child_b.is_cancelled());
assert!(!parent.is_cancelled());

// Cancel everything
parent.cancel();
assert!(child_b.is_cancelled()); // child inherits parent cancellation
```

```
                 ┌──────────┐
                 │  parent   │
                 └────┬──┬──┘
            cancel()  │  │
              ┌───────┘  └───────┐
              ▼                  ▼
        ┌──────────┐      ┌──────────┐
        │ child_a  │      │ child_b  │
        └──────────┘      └──────────┘

  parent.cancel() ──► both children cancelled
  child_a.cancel() ──► only child_a cancelled
```

### Key API

| Method | Description |
|--------|-------------|
| `CancellationToken::new()` | Create a new token |
| `token.clone()` | Cheap clone (shared state via `Arc`) |
| `token.cancel()` | Signal cancellation |
| `token.cancelled()` | Returns a `Future` that completes when cancelled |
| `token.is_cancelled()` | Synchronous check |
| `token.child_token()` | Create a child that inherits parent cancellation |

---

## Graceful Shutdown Pattern

A robust shutdown follows three steps:

1. **Detect** the shutdown signal
2. **Propagate** it to all tasks
3. **Wait** for tasks to finish

### Step 1: Detect Shutdown

```rust
use tokio::signal;

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("failed to install ctrl+c handler");
}
```

### Step 2: Propagate with CancellationToken

```rust
use tokio_util::sync::CancellationToken;

let shutdown = CancellationToken::new();
```

### Step 3: Track Tasks with TaskTracker

`TaskTracker` (from `tokio-util`) tracks spawned tasks and provides a way to wait for all of them to complete:

```rust
use tokio_util::task::TaskTracker;

let tracker = TaskTracker::new();
```

### Complete Example

```rust
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[tokio::main]
async fn main() {
    let shutdown = CancellationToken::new();
    let tracker = TaskTracker::new();

    // Spawn worker tasks
    for i in 0..4 {
        let token = shutdown.clone();
        tracker.spawn(async move {
            worker(i, token).await;
        });
    }

    // Wait for ctrl+c
    signal::ctrl_c().await.expect("failed to listen for ctrl+c");
    println!("shutdown signal received");

    // Signal all tasks to stop
    shutdown.cancel();

    // Close the tracker (no new tasks can be spawned)
    tracker.close();

    // Wait for all tasks to finish their cleanup
    tracker.wait().await;

    println!("all tasks shut down cleanly");
}

async fn worker(id: usize, token: CancellationToken) {
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                println!("worker {id}: shutting down");
                // Perform cleanup: flush buffers, close connections, etc.
                return;
            }
            _ = do_work(id) => {
                println!("worker {id}: completed work cycle");
            }
        }
    }
}

async fn do_work(id: usize) {
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
}
```

### Shutdown Flow

```
ctrl+c received
       │
       ▼
  shutdown.cancel()  ───► all tokens fire cancelled()
       │
       ▼
  tracker.close()    ───► no new tasks accepted
       │
       ▼
  tracker.wait()     ───► blocks until all tracked tasks complete
       │
       ▼
  main exits cleanly
```

### Alternative: Broadcast Channel for Shutdown

If you don't need `tokio-util`, a `tokio::sync::broadcast` channel works for propagation:

```rust
use tokio::sync::broadcast;

let (shutdown_tx, _) = broadcast::channel::<()>(1);

// Each task subscribes
let mut rx = shutdown_tx.subscribe();
tokio::spawn(async move {
    tokio::select! {
        _ = rx.recv() => { /* shutdown */ }
        _ = do_work() => {}
    }
});

// Signal shutdown
drop(shutdown_tx); // all recv() calls return Err(RecvError::Closed)
```

---

## See Also

- [01-async-await.md](01-async-await.md) — Async/await fundamentals and runtime setup.
- [02-futures-in-depth.md](02-futures-in-depth.md) — How dropping a future stops its state machine mid-execution.
