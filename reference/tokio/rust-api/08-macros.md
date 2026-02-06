# Macros

Tokio provides several macros for composing and managing async operations. These are fundamental building blocks for structuring concurrent async code.

---

## `select!`

### Syntax

```rust
tokio::select! {
    pattern = async_expression => handler,
    pattern = async_expression, if condition => handler,
    else => handler,
}
```

Waits on multiple async expressions simultaneously and executes the handler for the **first** branch that completes. All other branches are cancelled (their futures are dropped).

### Rules and Behavior

- Up to **64 branches** are supported.
- Each branch has the form: `pattern = async_expr => handler_expr`
- The `async_expr` is evaluated and polled. When one completes, its output is matched against `pattern`.
- If the pattern does not match, that branch is disabled and `select!` continues polling the remaining branches.
- The `else` branch runs when all patterns fail to match (all branches disabled). If no `else` is provided and all patterns fail, `select!` panics.
- **Fairness**: By default, branches are polled in random order each time to prevent starvation. Use `biased;` as the first token to poll in top-to-bottom declaration order.
- Unlike `tokio::spawn`, futures in `select!` do **not** require `'static` — they can borrow local data.

### Precondition Guards

Branches can be conditionally enabled with `if condition`:

```rust
use tokio::sync::mpsc;

let (tx1, mut rx1) = mpsc::channel::<i32>(10);
let (tx2, mut rx2) = mpsc::channel::<i32>(10);

let use_rx1 = true;

tokio::select! {
    val = rx1.recv(), if use_rx1 => {
        println!("rx1: {:?}", val);
    }
    val = rx2.recv() => {
        println!("rx2: {:?}", val);
    }
}
```

If `use_rx1` is `false`, the first branch is never polled.

### Basic Example

```rust
use tokio::sync::oneshot;

let (tx1, rx1) = oneshot::channel();
let (tx2, rx2) = oneshot::channel();

tokio::spawn(async move {
    tx1.send("one").unwrap();
});
tokio::spawn(async move {
    tx2.send("two").unwrap();
});

tokio::select! {
    val = rx1 => println!("rx1 completed: {:?}", val),
    val = rx2 => println!("rx2 completed: {:?}", val),
}
```

### Loop with Select

A common pattern is running `select!` in a loop to handle events from multiple sources:

```rust
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

async fn event_loop(
    mut commands: mpsc::Receiver<String>,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) {
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                println!("heartbeat");
            }
            Some(cmd) = commands.recv() => {
                println!("command: {}", cmd);
            }
            _ = shutdown.recv() => {
                println!("shutting down");
                return;
            }
        }
    }
}
```

### Biased Mode

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(10);

tokio::select! {
    biased;

    // High-priority branch polled first
    val = rx.recv() => {
        println!("received: {:?}", val);
    }
    // Low-priority branch
    _ = tokio::time::sleep(Duration::from_secs(1)) => {
        println!("timeout");
    }
}
```

### Borrowing in Select

Since `select!` branches run on the same task (no `spawn`), they can borrow local mutable state:

```rust
use tokio::time::{sleep, Duration};

let mut data = vec![1, 2, 3];

tokio::select! {
    _ = sleep(Duration::from_secs(1)) => {
        data.push(4); // borrows &mut data — no 'static required
    }
    _ = sleep(Duration::from_secs(2)) => {
        data.push(5);
    }
}

println!("{:?}", data);
```

---

## `join!`

### Syntax

```rust
let (a, b, c) = tokio::join!(future_a, future_b, future_c);
```

Runs all futures **concurrently on the same task** and waits until **all** of them complete. Returns a tuple of their outputs.

### Behavior

- All branches run concurrently (interleaved on the same task, not spawned).
- Returns only when every branch has completed.
- If one branch panics, the panic propagates after the other branches complete.
- Futures do not need to be `'static` — they can borrow local data.

### Example

```rust
use tokio::time::{sleep, Duration};

async fn fetch_user(id: u64) -> String {
    sleep(Duration::from_millis(100)).await;
    format!("user_{}", id)
}

async fn fetch_posts(user_id: u64) -> Vec<String> {
    sleep(Duration::from_millis(150)).await;
    vec![format!("post by {}", user_id)]
}

async fn fetch_notifications() -> usize {
    sleep(Duration::from_millis(80)).await;
    42
}

let (user, posts, notif_count) = tokio::join!(
    fetch_user(1),
    fetch_posts(1),
    fetch_notifications(),
);

println!("user: {}, posts: {:?}, notifications: {}", user, posts, notif_count);
```

Total wall time is ~150ms (the maximum of the three), not ~330ms (the sum).

---

## `try_join!`

### Syntax

```rust
let result: Result<(A, B, C), E> = tokio::try_join!(future_a, future_b, future_c);
```

Like `join!` but for futures that return `Result`. Short-circuits on the **first `Err`** — remaining branches are cancelled immediately.

### Behavior

- All futures must return `Result<T, E>` with the **same error type `E`**.
- Returns `Ok((a, b, c))` if all succeed.
- Returns `Err(e)` immediately when any branch fails, dropping the other futures.

### Example

```rust
use tokio::time::{sleep, Duration};

async fn fetch_config() -> Result<String, String> {
    sleep(Duration::from_millis(50)).await;
    Ok("config_data".to_string())
}

async fn fetch_schema() -> Result<String, String> {
    sleep(Duration::from_millis(100)).await;
    Ok("schema_data".to_string())
}

async fn init() -> Result<(), String> {
    let (config, schema) = tokio::try_join!(
        fetch_config(),
        fetch_schema(),
    )?;

    println!("config: {}, schema: {}", config, schema);
    Ok(())
}
```

### Error Short-Circuit

```rust
async fn will_fail() -> Result<i32, &'static str> {
    Err("something went wrong")
}

async fn slow_success() -> Result<i32, &'static str> {
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(42)
}

// Returns Err immediately — slow_success is cancelled
let result = tokio::try_join!(will_fail(), slow_success());
assert!(result.is_err());
```

---

## `pin!`

### Syntax

```rust
tokio::pin!(future);
tokio::pin!(future_a, future_b);
```

Pins a future to the stack, producing a `Pin<&mut F>`. This is required when you need a mutable reference to a future across multiple `select!` iterations.

### Why It's Needed

`select!` takes futures by mutable reference. If you create a future and want to reuse it across loop iterations (instead of restarting it), you need to pin it first.

### Example

```rust
use tokio::time::{sleep, Duration, Instant};
use tokio::sync::mpsc;

async fn process_with_deadline(mut rx: mpsc::Receiver<String>) {
    let deadline = sleep(Duration::from_secs(30));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => {
                println!("deadline reached");
                return;
            }
            Some(msg) = rx.recv() => {
                println!("received: {}", msg);
            }
            else => {
                println!("channel closed");
                return;
            }
        }
    }
}
```

Without `pin!`, the `sleep` future would be re-created on each loop iteration, resetting the deadline every time.

### Pinning Multiple Futures

```rust
use tokio::time::{sleep, Duration};

let fut1 = sleep(Duration::from_secs(5));
let fut2 = sleep(Duration::from_secs(10));
tokio::pin!(fut1, fut2);

tokio::select! {
    _ = &mut fut1 => println!("5s timer fired"),
    _ = &mut fut2 => println!("10s timer fired"),
}
```

---

## `task_local!`

### Syntax

```rust
tokio::task_local! {
    static NAME: Type;
    pub static PUB_NAME: Type;
}
```

Declares task-local storage, analogous to `thread_local!` but scoped to a Tokio task. The value is set for the duration of a future's execution via `.scope()` or `.sync_scope()`.

### API

```rust
pub struct LocalKey<T: 'static> { /* ... */ }

impl<T: 'static> LocalKey<T> {
    pub async fn scope<F: Future>(&'static self, value: T, f: F) -> F::Output
    pub fn sync_scope<F: FnOnce() -> R, R>(&'static self, value: T, f: F) -> R
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R
}
```

### Example: Request ID Propagation

```rust
use tokio::task_local;

task_local! {
    static REQUEST_ID: String;
}

async fn handle_request(id: String) {
    REQUEST_ID.scope(id, async {
        process().await;
    }).await;
}

async fn process() {
    REQUEST_ID.with(|id| {
        println!("processing request: {}", id);
    });
    inner_work().await;
}

async fn inner_work() {
    REQUEST_ID.with(|id| {
        println!("inner work for request: {}", id);
    });
}
```

### Accessing Outside Scope

`try_with` returns `Err(AccessError)` if called outside any `.scope()`:

```rust
task_local! {
    static TRACE_ID: u64;
}

fn maybe_log() {
    match TRACE_ID.try_with(|id| *id) {
        Ok(id) => println!("trace: {}", id),
        Err(_) => println!("no trace context"),
    }
}
```

---

## Comparison Table

| Macro | Concurrency | Completion | Cancellation | Requires `'static` |
|-------|-------------|------------|--------------|---------------------|
| `select!` | Concurrent (same task) | First branch | Remaining branches dropped | No |
| `join!` | Concurrent (same task) | All branches | None (waits for all) | No |
| `try_join!` | Concurrent (same task) | All branches or first `Err` | Remaining on `Err` | No |
| `pin!` | N/A | N/A | N/A | No |
| `task_local!` | N/A | N/A | N/A | Yes (values are `'static`) |

---

## See Also

- [Time Utilities](05-time.md) — `sleep` and `timeout` commonly used with `select!`
- [Synchronization Primitives](06-sync.md) — channels used as `select!` branches
- [Async File System](07-fs.md) — file operations that can be wrapped in `timeout` or `select!`
