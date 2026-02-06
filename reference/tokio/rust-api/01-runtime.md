# The Tokio Runtime

The `Runtime` is the core execution engine for asynchronous Rust programs using Tokio. It manages an event loop, I/O driver, timer, and a thread pool that cooperatively schedules and executes futures. Every Tokio application creates (or implicitly uses) a runtime.

`Runtime` is `Send + Sync` — it can be shared across threads. The `Handle` is a cheap, cloneable reference to a running runtime that can be passed anywhere.

---

## API Reference

### Runtime

```rust
impl Runtime {
    pub fn new() -> io::Result<Runtime>
    pub fn handle(&self) -> &Handle
    pub fn block_on<F: Future>(&self, future: F) -> F::Output
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    pub fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    pub fn enter(&self) -> EnterGuard<'_>
    pub fn shutdown_timeout(self, duration: Duration)
    pub fn shutdown_background(self)
    pub fn metrics(&self) -> RuntimeMetrics
}
```

### Builder

```rust
impl Builder {
    pub fn new_multi_thread() -> Builder
    pub fn new_current_thread() -> Builder
    pub fn worker_threads(&mut self, val: usize) -> &mut Self
    pub fn max_blocking_threads(&mut self, val: usize) -> &mut Self
    pub fn thread_name(&mut self, val: impl Into<String>) -> &mut Self
    pub fn thread_name_fn(&mut self, f: impl Fn() -> String + Send + Sync + 'static) -> &mut Self
    pub fn thread_stack_size(&mut self, val: usize) -> &mut Self
    pub fn on_thread_start(&mut self, f: impl Fn() + Send + Sync + 'static) -> &mut Self
    pub fn on_thread_stop(&mut self, f: impl Fn() + Send + Sync + 'static) -> &mut Self
    pub fn enable_all(&mut self) -> &mut Self
    pub fn enable_io(&mut self) -> &mut Self
    pub fn enable_time(&mut self) -> &mut Self
    pub fn build(&mut self) -> io::Result<Runtime>
}
```

### Handle

```rust
impl Handle {
    pub fn current() -> Handle
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    pub fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    pub fn block_on<F: Future>(&self, future: F) -> F::Output
    pub fn enter(&self) -> EnterGuard<'_>
    pub fn runtime_flavor(&self) -> RuntimeFlavor
    pub fn metrics(&self) -> RuntimeMetrics
}
```

---

## Runtime

### `Runtime::new()`

Creates a new multi-threaded runtime with all drivers (I/O and time) enabled. Equivalent to `Builder::new_multi_thread().enable_all().build().unwrap()`. This is the quickest way to get a working runtime, but panics on failure — prefer the builder for production code.

### `Runtime::block_on(future)`

Runs a future to completion on the runtime, blocking the current thread until the future resolves. This is the bridge between synchronous and asynchronous code. Only one `block_on` call can be active on a runtime at a time. Calling `block_on` from within an async context will panic.

### `Runtime::spawn(future)`

Spawns an asynchronous task onto the runtime. The future must be `Send + 'static`. Returns a `JoinHandle` that can be awaited to obtain the task's result. The task runs concurrently with other tasks on the runtime's thread pool.

### `Runtime::enter()`

Enters the runtime context, returning an `EnterGuard`. While the guard is held, `tokio::spawn` and other context-dependent functions will use this runtime. The guard must not be held across an `.await` point.

### `Runtime::shutdown_timeout(duration)`

Consumes the runtime and shuts it down. Waits up to `duration` for all spawned tasks and blocking threads to complete. Tasks that do not complete within the timeout are cancelled. Use this for graceful shutdown with a deadline.

### `Runtime::shutdown_background()`

Consumes the runtime and shuts it down immediately without waiting for spawned tasks to complete. Background threads may continue running briefly. Useful in destructors or signal handlers where blocking is unacceptable.

### `Runtime::metrics()`

Returns a `RuntimeMetrics` handle that provides introspection into the runtime's state: number of workers, active tasks, blocking thread count, I/O driver metrics, and more.

---

## Builder

The `Builder` provides fine-grained control over runtime construction.

### `Builder::new_multi_thread()`

Creates a builder for a multi-threaded runtime. The runtime will spawn worker threads (defaulting to the number of CPU cores) that cooperatively schedule tasks using a work-stealing strategy. This is the default for production applications.

### `Builder::new_current_thread()`

Creates a builder for a single-threaded runtime. All tasks run on the thread that calls `block_on`. No worker threads are spawned. Ideal for tests, embedded contexts, or when you need deterministic single-threaded execution.

### `Builder::worker_threads(val)`

Sets the number of worker threads for the multi-thread runtime. Defaults to the number of available CPU cores. Has no effect on the current-thread runtime.

### `Builder::max_blocking_threads(val)`

Sets the maximum number of threads that can be created by `spawn_blocking`. Defaults to 512. If all blocking threads are in use, additional `spawn_blocking` calls will wait until a thread becomes available.

### `Builder::enable_all()`

Enables both the I/O driver and the time driver. Equivalent to calling `enable_io()` and `enable_time()` separately.

### `Builder::enable_io()`

Enables the I/O driver, which is required for networking, file I/O, and other I/O operations. Without this, attempting to use `TcpListener`, `TcpStream`, etc. will panic.

### `Builder::enable_time()`

Enables the time driver, which is required for `tokio::time::sleep`, `tokio::time::interval`, timeouts, and other time-based operations.

### `Builder::thread_name(val)`

Sets the name prefix for worker threads. Thread names appear in debuggers, profilers, and log output (e.g., `"my-app-worker-0"`, `"my-app-worker-1"`).

### `Builder::thread_name_fn(f)`

Sets a closure that generates unique thread names. Called once per worker thread. Use this when you need dynamic or numbered thread names.

### `Builder::build()`

Consumes the builder configuration and constructs the runtime. Returns `io::Result<Runtime>`, which may fail if OS resources (threads, epoll/kqueue descriptors) cannot be allocated.

---

## Handle

### `Handle::current()`

Returns a `Handle` to the currently running runtime. Panics if called outside of a Tokio runtime context. This is the standard way to obtain a handle from within async code or from synchronous code inside `Runtime::block_on`.

### `Handle::spawn(future)`

Spawns a task onto the runtime associated with this handle. Identical to `Runtime::spawn`, but can be called from anywhere as long as you hold a handle. The handle can be cloned and sent to other threads.

### `Handle::block_on(future)`

Runs a future to completion on the runtime associated with this handle, blocking the calling thread. Unlike `Runtime::block_on`, this can be called from a thread outside the runtime. Panics if called from within an async context.

### `Handle::runtime_flavor()`

Returns the `RuntimeFlavor` enum indicating whether the runtime is `CurrentThread`, `MultiThread`, or `MultiThreadAlt`.

---

## RuntimeFlavor

```rust
pub enum RuntimeFlavor {
    CurrentThread,
    MultiThread,
    MultiThreadAlt,
}
```

Identifies the scheduler variant of the runtime. `MultiThreadAlt` is an alternative multi-thread scheduler available behind a feature flag.

---

## EnterGuard

```rust
pub struct EnterGuard<'a> { /* ... */ }
```

An RAII guard returned by `Runtime::enter()` and `Handle::enter()`. While held, the runtime is set as the "current" runtime for the thread. When dropped, the previous runtime context is restored. This allows `tokio::spawn` and other context-dependent operations to work outside of `block_on`.

---

## The `#[tokio::main]` Macro

The `#[tokio::main]` attribute macro transforms an `async fn main()` into a synchronous entry point that creates a runtime and calls `block_on`. This is the standard way to write a Tokio application.

```rust
#[tokio::main]
async fn main() {
    println!("Hello from Tokio!");
}
```

Expands to:

```rust
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            println!("Hello from Tokio!");
        })
}
```

The macro accepts optional arguments to configure the runtime:

```rust
// Use a current-thread runtime
#[tokio::main(flavor = "current_thread")]

// Use multi-thread runtime with 4 worker threads
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]

// Start with time paused (useful for tests)
#[tokio::main(start_paused = true)]
```

---

## The `#[tokio::test]` Macro

The `#[tokio::test]` macro works like `#[tokio::main]` but for test functions. By default it uses a current-thread runtime for deterministic test execution.

```rust
#[tokio::test]
async fn test_something() {
    let result = my_async_function().await;
    assert_eq!(result, 42);
}
```

Expands to:

```rust
#[test]
fn test_something() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let result = my_async_function().await;
            assert_eq!(result, 42);
        })
}
```

Optional arguments:

```rust
// Use multi-thread runtime in tests
#[tokio::test(flavor = "multi_thread")]

// Multi-thread with specific worker count
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]

// Start with time paused for deterministic time-based tests
#[tokio::test(start_paused = true)]
```

---

## Examples

### Custom Runtime Configuration

```rust
use tokio::runtime::Builder;
use std::time::Duration;

fn main() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .max_blocking_threads(64)
        .thread_name("my-app-worker")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_all()
        .on_thread_start(|| {
            println!("Worker thread started: {:?}", std::thread::current().name());
        })
        .on_thread_stop(|| {
            println!("Worker thread stopping");
        })
        .build()
        .expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        println!("Running on custom runtime");

        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            42
        });

        let result = handle.await.unwrap();
        println!("Task returned: {}", result);
    });

    runtime.shutdown_timeout(Duration::from_secs(5));
}
```

### Current-Thread vs Multi-Thread

```rust
use tokio::runtime::Builder;

fn run_current_thread() {
    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // All tasks run on this single thread.
        // Good for tests, lightweight services, or !Send futures.
        let handle = tokio::spawn(async {
            println!(
                "Running on thread: {:?}",
                std::thread::current().name()
            );
            "single-threaded"
        });
        println!("Result: {}", handle.await.unwrap());
    });
}

fn run_multi_thread() {
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // Tasks are distributed across 2 worker threads via work-stealing.
        let mut handles = vec![];
        for i in 0..4 {
            handles.push(tokio::spawn(async move {
                println!(
                    "Task {} on thread {:?}",
                    i,
                    std::thread::current().name()
                );
                i * 10
            }));
        }

        for handle in handles {
            let result = handle.await.unwrap();
            println!("Got: {}", result);
        }
    });
}
```

### Using Handle from a Non-Async Context

```rust
use tokio::runtime::Runtime;
use std::thread;

fn main() {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();

    // Pass the handle to a background thread
    let bg = thread::spawn(move || {
        // Use the handle to spawn async work from a sync thread
        handle.block_on(async {
            let resp = reqwest::get("https://httpbin.org/get").await.unwrap();
            println!("Status: {}", resp.status());
        });
    });

    bg.join().unwrap();
}
```

### Entering the Runtime Context

```rust
use tokio::runtime::Runtime;

fn main() {
    let rt = Runtime::new().unwrap();

    // Enter the runtime context without calling block_on
    let _guard = rt.enter();

    // Now tokio::spawn works even though we're not inside block_on
    let handle = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        "spawned outside block_on"
    });

    // block_on to drive the spawned task
    let result = rt.block_on(handle).unwrap();
    println!("{}", result);
}
```

---

## Thread Safety

| Type | `Send` | `Sync` | `Clone` | Notes |
|------|--------|--------|---------|-------|
| `Runtime` | Yes | Yes | No | Owns the thread pool and drivers |
| `Handle` | Yes | Yes | Yes | Cheap reference to a running runtime |
| `EnterGuard` | No | No | No | Scoped to the thread that created it |
| `RuntimeMetrics` | Yes | Yes | Yes | Read-only snapshot |
| `Builder` | No | No | No | Used only during construction |

---

## See Also

- [Tasks and Spawning](02-tasks.md) — spawning and managing async tasks
- [I/O Traits and Utilities](03-io.md) — async read/write traits and helpers
- [TCP, UDP, and Unix Sockets](04-networking.md) — networking primitives built on the runtime
