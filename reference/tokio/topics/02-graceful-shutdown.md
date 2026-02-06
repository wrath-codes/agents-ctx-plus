# Graceful Shutdown

## Overview

A graceful shutdown has three parts:

1. **Detect** the shutdown signal
2. **Propagate** it to all running tasks
3. **Wait** for tasks to complete

## Detecting Shutdown

### Ctrl+C Signal

```rust
use tokio::signal;

#[tokio::main]
async fn main() {
    // Wait for Ctrl+C
    signal::ctrl_c().await.expect("failed to listen for ctrl_c");
    println!("shutting down");
}
```

### Application-Initiated Shutdown

Use an `mpsc` channel when shutdown can be triggered from multiple places:

```rust
use tokio::sync::mpsc;

let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

// Drop all senders to trigger shutdown
// Or send explicitly:
shutdown_tx.send(()).await.unwrap();
```

### Combining Multiple Sources with `select!`

```rust
use tokio::signal;
use tokio::sync::mpsc;

let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

tokio::select! {
    _ = signal::ctrl_c() => {
        println!("ctrl-c received");
    }
    _ = shutdown_rx.recv() => {
        println!("application shutdown");
    }
}
```

## Propagating Shutdown

### CancellationToken (Recommended)

From the `tokio-util` crate:

```toml
[dependencies]
tokio-util = "0.7"
```

```rust
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();

    // Spawn a task that listens for cancellation
    let cloned_token = token.clone();
    let task = tokio::spawn(async move {
        tokio::select! {
            _ = cloned_token.cancelled() => {
                // Shutdown signal received, clean up
                println!("task shutting down");
            }
            _ = do_work() => {
                println!("work completed");
            }
        }
    });

    // Trigger shutdown
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    token.cancel();

    // Wait for task to finish
    task.await.unwrap();
}

async fn do_work() {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        println!("working...");
    }
}
```

### Child Tokens

`CancellationToken` supports hierarchical cancellation:

```rust
let parent = CancellationToken::new();
let child = parent.child_token();

// Cancelling parent also cancels child
parent.cancel();
assert!(child.is_cancelled());
```

### Broadcast Channel Alternative

Use when you need to send a shutdown reason or when many receivers need notification:

```rust
use tokio::sync::broadcast;

let (shutdown_tx, _) = broadcast::channel::<()>(1);

// Each task subscribes
let mut rx = shutdown_tx.subscribe();

tokio::spawn(async move {
    tokio::select! {
        _ = rx.recv() => {
            println!("shutdown received");
        }
        _ = do_work() => {}
    }
});

// Trigger shutdown
drop(shutdown_tx); // or shutdown_tx.send(())
```

## Waiting for Completion

### TaskTracker (Recommended)

From `tokio-util`:

```rust
use tokio_util::task::TaskTracker;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    // Spawn tracked tasks
    for i in 0..10 {
        let token = token.clone();
        tracker.spawn(async move {
            tokio::select! {
                _ = token.cancelled() => {
                    println!("task {} shutting down", i);
                }
                _ = do_work(i) => {
                    println!("task {} completed", i);
                }
            }
        });
    }

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.unwrap();

    // Cancel all tasks
    token.cancel();

    // Close the tracker (no new tasks can be spawned)
    tracker.close();

    // Wait for all tasks to complete
    tracker.wait().await;

    println!("all tasks finished, exiting");
}

async fn do_work(id: usize) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        println!("task {} working", id);
    }
}
```

### JoinSet for Smaller Groups

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

set.spawn(async { task_a().await });
set.spawn(async { task_b().await });

// Wait for all tasks
while let Some(result) = set.join_next().await {
    match result {
        Ok(val) => println!("task completed: {:?}", val),
        Err(e) => eprintln!("task failed: {}", e),
    }
}
```

## Complete Pattern

Putting it all together:

```rust
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();
    let tracker = TaskTracker::new();

    // Spawn application tasks
    for i in 0..5 {
        let token = token.clone();
        tracker.spawn(async move {
            tokio::select! {
                _ = token.cancelled() => {
                    cleanup(i).await;
                }
                _ = run_service(i) => {}
            }
        });
    }

    // Wait for Ctrl+C
    signal::ctrl_c().await.expect("failed to listen for ctrl_c");
    println!("shutdown signal received");

    // Propagate shutdown
    token.cancel();

    // Stop accepting new tasks
    tracker.close();

    // Wait for all tasks to finish cleanup
    tracker.wait().await;

    println!("shutdown complete");
}

async fn run_service(id: usize) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        println!("service {} running", id);
    }
}

async fn cleanup(id: usize) {
    println!("service {} cleaning up...", id);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    println!("service {} done", id);
}
```

## See Also

- [Select](../tutorial/07-select.md) — combining shutdown signals with work
- [Bridging Sync Code](./01-bridging-sync-code.md) — shutting down bridged runtimes
- [Tracing](./03-tracing.md) — observing shutdown behavior
