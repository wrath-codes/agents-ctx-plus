# Select

## Overview

`tokio::select!` waits on multiple async operations simultaneously and proceeds with the **first one** to complete. All other branches are **cancelled** (their futures are dropped).

## Basic Syntax

```rust
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    let (tx1, rx1) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();

    tokio::spawn(async { tx1.send("one").unwrap() });
    tokio::spawn(async { tx2.send("two").unwrap() });

    tokio::select! {
        val = rx1 => {
            println!("rx1 completed first with {:?}", val);
        }
        val = rx2 => {
            println!("rx2 completed first with {:?}", val);
        }
    }
}
```

Each branch has the form: `<pattern> = <async expression> => <handler>`

## Cancellation

When one branch completes, all other branches are **dropped**. Dropping a future cancels its operation.

```rust
use tokio::sync::oneshot;

async fn some_operation() -> String {
    // This may or may not run to completion
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    "done".to_string()
}

#[tokio::main]
async fn main() {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tx.send("first").unwrap();
    });

    tokio::select! {
        val = rx => {
            println!("got {:?}", val);
        }
        val = some_operation() => {
            // some_operation() is dropped/cancelled if rx completes first
            println!("operation completed: {}", val);
        }
    }
}
```

For `oneshot::Receiver`, dropping it sends a "closed" notification to the `Sender` side via `Sender::closed()`.

## Under the Hood

`select!` compiles into something like a `Future` that polls all branches. It does **not** spawn tasks — all branches run on the same task and are polled in the same `poll()` call.

## Syntax Details

- Up to **64 branches** per `select!`
- Pattern matching on the async result
- Optional `else` branch for when all patterns fail to match

### Pattern Matching

Use patterns to handle specific variants. If no pattern matches, the branch is disabled.

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(128);

    tokio::spawn(async move {
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        // tx dropped here — channel closes
    });

    loop {
        tokio::select! {
            Some(v) = rx.recv() => {
                println!("got: {}", v);
            }
            else => {
                // All branches' patterns failed to match
                // (rx.recv() returned None = channel closed)
                println!("channel closed");
                break;
            }
        }
    }
}
```

## Borrowing

Unlike `tokio::spawn` (which requires `'static`), `select!` branches can **borrow** data from the enclosing scope.

### Multiple Immutable Borrows

Multiple branches can borrow the same data immutably:

```rust
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

async fn race(
    data: &[u8],
    addr1: &str,
    addr2: &str,
) -> std::io::Result<()> {
    tokio::select! {
        Ok(mut conn) = TcpStream::connect(addr1) => {
            conn.write_all(data).await?; // borrows data
        }
        Ok(mut conn) = TcpStream::connect(addr2) => {
            conn.write_all(data).await?; // borrows data
        }
    }
    Ok(())
}
```

### Mutable Borrow in Handlers

Handlers can mutably borrow because only **one handler** ever runs:

```rust
let mut out = String::new();

tokio::select! {
    v = rx1 => {
        out.push_str(&v.unwrap()); // mutable borrow OK
    }
    v = rx2 => {
        out.push_str(&v.unwrap()); // only one runs
    }
}
```

## Loops with `select!`

A common pattern: event loop that handles multiple sources.

### Basic Loop

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(128);
    let mut done = false;

    loop {
        tokio::select! {
            Some(v) = rx.recv(), if !done => {
                println!("got: {}", v);
            }
            else => break,
        }
    }
}
```

### Resuming Futures Across Iterations

By default, the async expression is recreated each loop iteration. To **resume** a future across iterations, use `tokio::pin!` and `&mut`:

```rust
use tokio::sync::mpsc;

async fn action() -> String {
    // Long-running operation
    "result".to_string()
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<String>(128);

    let operation = action();
    tokio::pin!(operation);

    loop {
        tokio::select! {
            result = &mut operation => {
                // operation completed
                println!("action result: {}", result);
                break;
            }
            Some(v) = rx.recv() => {
                println!("received: {}", v);
            }
            else => break,
        }
    }
}
```

### Resetting a Pinned Future

After a pinned future completes, use `.set()` to replace it:

```rust
use tokio::sync::mpsc;

async fn action(input: Option<String>) -> String {
    // Process input
    input.unwrap_or_default()
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<String>(128);

    let operation = action(None);
    tokio::pin!(operation);

    let mut done = false;

    loop {
        tokio::select! {
            result = &mut operation, if !done => {
                done = true;
                println!("completed: {}", result);
            }
            Some(v) = rx.recv() => {
                if done {
                    // Reset with new input
                    operation.set(action(Some(v)));
                    done = false;
                }
            }
            else => break,
        }
    }
}
```

### Precondition Guards

The `if <condition>` syntax disables a branch:

```rust
tokio::select! {
    val = rx.recv(), if !done => { /* ... */ }
    _ = timeout => { /* ... */ }
}
```

If the guard is `false`, the branch is not polled. If **all** branches are disabled, the `else` branch runs (or the `select!` panics if there is no `else`).

## Per-Task Concurrency: `select!` vs `spawn`

| Feature | `tokio::select!` | `tokio::spawn` |
|---------|-------------------|----------------|
| Runs on | Same task | Independent task |
| Parallelism | Never truly parallel | Can run on different threads |
| Data sharing | Can borrow from scope | Requires `'static + Send` |
| Cancellation | Automatic on completion | Manual via `JoinHandle::abort()` |
| Use case | Multiplexing within a task | True parallel work |

```rust
// select! — concurrent but not parallel
tokio::select! {
    _ = future_a => {}
    _ = future_b => {}
}

// spawn — truly parallel
let a = tokio::spawn(future_a);
let b = tokio::spawn(future_b);
a.await.unwrap();
b.await.unwrap();
```

## See Also

- [I/O](./05-io.md) — combining select with I/O operations
- [Streams](./08-streams.md) — async iteration as an alternative to select loops
- [Graceful Shutdown](../topics/02-graceful-shutdown.md) — using select for shutdown signals
