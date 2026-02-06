# Tasks and Spawning

Tasks are the fundamental unit of work in Tokio. A task is a lightweight, non-blocking unit of execution — similar to a green thread — that is cooperatively scheduled onto one or more OS threads managed by the runtime. Tasks are created with `tokio::spawn` and managed through `JoinHandle`, `JoinSet`, and related primitives.

Tasks require `Send + 'static` bounds because they may be moved between worker threads by the runtime's work-stealing scheduler.

---

## API Reference

### Spawning

```rust
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,

pub fn spawn_blocking<F, R>(func: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,

pub fn block_in_place<F, R>(f: F) -> R
where
    F: FnOnce() -> R,

pub async fn yield_now()
```

### JoinHandle

```rust
impl<T> JoinHandle<T> {
    pub async fn await -> Result<T, JoinError>
    pub fn abort(&self)
    pub fn abort_handle(&self) -> AbortHandle
    pub fn is_finished(&self) -> bool
    pub fn id(&self) -> Id
}
```

### JoinSet

```rust
impl<T: 'static> JoinSet<T> {
    pub fn new() -> Self
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
    pub fn spawn<F>(&mut self, task: F) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
    pub fn spawn_on<F>(&mut self, task: F, handle: &Handle) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
    pub fn spawn_blocking<F>(&mut self, f: F) -> AbortHandle
    where
        F: FnOnce() -> T + Send + 'static,
    pub async fn join_next(&mut self) -> Option<Result<T, JoinError>>
    pub async fn join_all(self) -> Vec<T>
    pub fn try_join_next(&mut self) -> Option<Result<T, JoinError>>
    pub fn abort_all(&mut self)
    pub fn detach_all(&mut self)
    pub fn shutdown(&mut self) -> impl Future<Output = ()>
}
```

### AbortHandle

```rust
impl AbortHandle {
    pub fn abort(&self)
    pub fn is_finished(&self) -> bool
    pub fn id(&self) -> Id
}
```

### LocalSet

```rust
impl LocalSet {
    pub fn new() -> Self
    pub fn spawn_local<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    pub async fn run_until<F: Future>(&self, future: F) -> F::Output
    pub fn block_on<F: Future>(&self, rt: &Runtime, future: F) -> F::Output
    pub fn enter(&self) -> LocalEnterGuard
}
```

---

## `tokio::spawn(future)`

Spawns a new asynchronous task onto the Tokio runtime. The task runs concurrently with other tasks and is scheduled by the runtime's work-stealing scheduler. Returns a `JoinHandle` that can be awaited to get the task's result.

The future must satisfy two bounds:

- **`Send`** — The future (and all data it holds across `.await` points) must be `Send` because the runtime may move the task between worker threads.
- **`'static`** — The future must own all its data. It cannot borrow from the calling scope because the task may outlive the scope that spawned it.

```rust
// OK: owned data, no borrows
tokio::spawn(async {
    let data = vec![1, 2, 3];
    println!("{:?}", data);
});

// OK: move owned data into the task
let name = String::from("tokio");
tokio::spawn(async move {
    println!("Hello, {}!", name);
});
```

---

## JoinHandle

The `JoinHandle<T>` is a handle to a spawned task. Awaiting the handle returns `Result<T, JoinError>`, where `JoinError` occurs if the task panicked or was cancelled.

### Awaiting the Result

```rust
let handle = tokio::spawn(async {
    expensive_computation().await
});

match handle.await {
    Ok(value) => println!("Task completed: {}", value),
    Err(e) if e.is_panic() => println!("Task panicked!"),
    Err(e) if e.is_cancelled() => println!("Task was cancelled"),
    Err(e) => println!("Task failed: {}", e),
}
```

### `abort()`

Cancels the task. The next time the task is polled, it will return `JoinError` with `is_cancelled() == true`. If the task is currently idle (waiting on I/O, sleep, etc.), it will be woken and cancelled. The task is not guaranteed to stop immediately — cancellation happens at the next `.await` point.

### `abort_handle()`

Returns an `AbortHandle` that can be used to abort the task remotely. Unlike `JoinHandle`, `AbortHandle` is `Clone` and does not provide access to the task's result. Useful when you need to cancel a task from multiple places.

### `is_finished()`

Returns `true` if the task has completed (successfully, panicked, or cancelled). Non-blocking.

### `id()`

Returns the unique `Id` of the spawned task. Task IDs are unique within a runtime instance and useful for debugging and logging.

---

## JoinSet

`JoinSet` manages a collection of spawned tasks and provides an interface to await their completion. It is the recommended way to manage groups of related tasks.

### Creating and Spawning

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

for i in 0..10 {
    set.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(i * 100)).await;
        i * 2
    });
}
```

### `join_next()`

Awaits the next task in the set to complete. Returns `None` when the set is empty. Tasks are returned in completion order (not insertion order).

```rust
while let Some(result) = set.join_next().await {
    match result {
        Ok(value) => println!("Task completed with: {}", value),
        Err(e) => println!("Task failed: {}", e),
    }
}
```

### `join_all()`

Consumes the `JoinSet` and awaits all tasks, returning a `Vec<T>` of their results. Panics if any task panicked or was cancelled. Use `join_next()` if you need to handle errors per-task.

```rust
let results: Vec<i32> = set.join_all().await;
println!("All results: {:?}", results);
```

### `abort_all()`

Cancels all tasks in the set. The tasks can still be collected with `join_next()` — they will return `JoinError` with `is_cancelled() == true`.

### `try_join_next()`

Non-async variant of `join_next()`. Returns `Some(result)` if a task has already completed, `None` if no tasks are ready yet or the set is empty. Useful for polling without blocking.

### `spawn_on(task, handle)`

Spawns a task onto a specific runtime identified by its `Handle`, rather than the current runtime. Useful when the `JoinSet` is managed from a different runtime context.

### `spawn_blocking(f)`

Spawns a blocking closure on the runtime's blocking thread pool and tracks it in the `JoinSet`.

### `detach_all()`

Removes all tasks from the `JoinSet` without cancelling them. The tasks continue running but can no longer be awaited through this set.

### `shutdown()`

Aborts all tasks and waits for them to finish. Equivalent to calling `abort_all()` followed by draining `join_next()`.

---

## `spawn_blocking(func)`

Runs a blocking or CPU-intensive closure on a dedicated thread pool separate from the async worker threads. Returns a `JoinHandle` that resolves when the closure completes.

Use `spawn_blocking` for:
- CPU-intensive computation (hashing, compression, parsing)
- Synchronous I/O (blocking file reads, database drivers without async support)
- FFI calls to blocking C libraries

```rust
let hash = tokio::task::spawn_blocking(move || {
    // This runs on a blocking thread, not an async worker
    compute_expensive_hash(&data)
}).await.unwrap();
```

The blocking thread pool is separate from the async worker threads. By default, up to 512 blocking threads can be created. Configure this with `Builder::max_blocking_threads()`.

---

## `block_in_place(f)`

Converts the current async worker thread into a blocking thread for the duration of the closure. Unlike `spawn_blocking`, this does not move to a different thread — it runs the closure in place. Other async tasks on the current worker are moved to other worker threads first.

Only works on the multi-thread runtime. Panics on the current-thread runtime.

```rust
use tokio::task;

async fn process_data(data: Vec<u8>) -> Vec<u8> {
    task::block_in_place(|| {
        // Heavy synchronous computation in place
        data.iter().map(|b| b.wrapping_mul(2)).collect()
    })
}
```

---

## `yield_now()`

Yields execution back to the Tokio scheduler, allowing other tasks to run. The current task will be re-scheduled to resume later. This is cooperative scheduling — use it in long-running computation loops to avoid starving other tasks.

```rust
use tokio::task;

async fn long_computation() {
    for i in 0..1_000_000 {
        if i % 1000 == 0 {
            task::yield_now().await;
        }
        // ... work ...
    }
}
```

---

## LocalSet

`LocalSet` enables spawning `!Send` futures — futures that cannot be moved between threads. All tasks spawned on a `LocalSet` are guaranteed to run on the same thread.

### `spawn_local(future)`

Spawns a `!Send` future onto the `LocalSet`. The future only needs `'static`, not `Send`. Must be called from within a `LocalSet` context.

### `run_until(future)`

Runs the `LocalSet` until the provided future completes, processing all spawned local tasks concurrently. This is the async equivalent of `block_on` for local tasks.

```rust
use tokio::task::LocalSet;
use std::rc::Rc;

#[tokio::main]
async fn main() {
    let local = LocalSet::new();

    local.run_until(async {
        // Rc is !Send, but spawn_local doesn't require Send
        let data = Rc::new(vec![1, 2, 3]);

        let data_clone = data.clone();
        tokio::task::spawn_local(async move {
            println!("Local task: {:?}", data_clone);
        }).await.unwrap();

        println!("Main: {:?}", data);
    }).await;
}
```

---

## The `'static` Bound

Spawned tasks must be `'static` because there is no guarantee when the task will complete. The spawning scope may return before the task finishes, so the task cannot borrow from the local stack.

```rust
// COMPILE ERROR: borrows local variable
let data = vec![1, 2, 3];
tokio::spawn(async {
    println!("{:?}", data); // `data` is borrowed, not moved
});

// FIX: move ownership into the task
let data = vec![1, 2, 3];
tokio::spawn(async move {
    println!("{:?}", data); // `data` is owned by the task
});

// FIX: clone if you need the data in both places
let data = vec![1, 2, 3];
let data_for_task = data.clone();
tokio::spawn(async move {
    println!("{:?}", data_for_task);
});
println!("{:?}", data);
```

---

## The `Send` Bound

A spawned future must be `Send` because the multi-thread runtime may move tasks between worker threads at any `.await` point. This means every value held across an `.await` must be `Send`.

```rust
use std::rc::Rc;

// COMPILE ERROR: Rc is !Send
tokio::spawn(async {
    let rc = Rc::new(42);
    some_async_fn().await;  // Rc held across .await
    println!("{}", rc);
});

// FIX: use Arc instead of Rc
use std::sync::Arc;

tokio::spawn(async {
    let arc = Arc::new(42);
    some_async_fn().await;
    println!("{}", arc);
});

// FIX: drop the !Send value before .await
tokio::spawn(async {
    {
        let rc = Rc::new(42);
        println!("{}", rc);
    } // rc dropped here
    some_async_fn().await;
});
```

---

## Task Cancellation

Tasks can be cancelled in two ways:

### Dropping the JoinHandle

Dropping a `JoinHandle` **detaches** the task — it continues running in the background but its result is lost. This is *not* cancellation.

```rust
// Task is detached, not cancelled — it keeps running
let _ = tokio::spawn(async {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        println!("still running");
    }
});
```

### Calling `abort()`

Calling `abort()` on a `JoinHandle` or `AbortHandle` cancels the task. The task is stopped at its next `.await` point.

```rust
let handle = tokio::spawn(async {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
});

// Cancel the task
handle.abort();

// Awaiting the handle returns JoinError with is_cancelled() == true
match handle.await {
    Ok(_) => unreachable!(),
    Err(e) => assert!(e.is_cancelled()),
}
```

Cancellation is cooperative: the task is not interrupted mid-execution. It is cancelled at the next `.await` point. If a task never awaits (e.g., a tight CPU loop), it cannot be cancelled with `abort()`.

---

## Examples

### Fan-Out / Fan-In with JoinSet

```rust
use tokio::task::JoinSet;

#[tokio::main]
async fn main() {
    let urls = vec![
        "https://httpbin.org/delay/1",
        "https://httpbin.org/delay/2",
        "https://httpbin.org/delay/3",
    ];

    let mut set = JoinSet::new();

    for url in urls {
        let url = url.to_string();
        set.spawn(async move {
            let resp = reqwest::get(&url).await.unwrap();
            (url, resp.status())
        });
    }

    while let Some(result) = set.join_next().await {
        match result {
            Ok((url, status)) => println!("{}: {}", url, status),
            Err(e) => eprintln!("Task failed: {}", e),
        }
    }
}
```

### Offloading CPU Work with spawn_blocking

```rust
use tokio::task;

#[tokio::main]
async fn main() {
    let data = vec![0u8; 1_000_000];

    // Offload CPU-heavy work to the blocking thread pool
    let compressed = task::spawn_blocking(move || {
        zstd::encode_all(&data[..], 3).unwrap()
    })
    .await
    .unwrap();

    println!("Compressed {} bytes", compressed.len());
}
```

### Using AbortHandle for Remote Cancellation

```rust
use tokio::task::JoinSet;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut set = JoinSet::new();

    let abort_handle = set.spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Background task tick");
        }
    });

    // Do some work
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Cancel the background task via its AbortHandle
    abort_handle.abort();
    println!("Task cancelled: {}", abort_handle.is_finished());
}
```

### Cooperative Scheduling with yield_now

```rust
use tokio::task;

#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        let mut sum: u64 = 0;
        for i in 0..10_000_000u64 {
            sum = sum.wrapping_add(i);
            if i % 10_000 == 0 {
                // Yield to let other tasks run
                task::yield_now().await;
            }
        }
        sum
    });

    // This task can run concurrently with the computation above
    let printer = tokio::spawn(async {
        for i in 0..5 {
            println!("Printer tick {}", i);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    let (sum, _) = tokio::join!(handle, printer);
    println!("Sum: {}", sum.unwrap());
}
```

---

## Thread Safety

| Type | `Send` | `Sync` | `Clone` | Notes |
|------|--------|--------|---------|-------|
| `JoinHandle<T>` | Yes | Yes | No | Awaiting consumes the handle |
| `JoinSet<T>` | Yes | Yes | No | Owns all spawned tasks |
| `AbortHandle` | Yes | Yes | Yes | Can cancel from multiple places |
| `LocalSet` | No | No | No | Must stay on one thread |
| `Id` | Yes | Yes | Yes | Copy type, unique per runtime |

---

## See Also

- [The Tokio Runtime](01-runtime.md) — configuring the runtime that executes tasks
- [I/O Traits and Utilities](03-io.md) — async I/O for use within tasks
- [TCP, UDP, and Unix Sockets](04-networking.md) — networking primitives commonly used in spawned tasks
