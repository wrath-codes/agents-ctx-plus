# Synchronization Primitives

The `tokio::sync` module provides async-aware synchronization primitives and message-passing channels. These types are designed to be used in async contexts — their blocking operations yield to the runtime rather than blocking the OS thread.

---

## Channels Overview

Tokio provides four channel types for different communication patterns:

| Channel | Producers | Consumers | Values Kept | Bounded | Use Case |
|---------|-----------|-----------|-------------|---------|----------|
| `mpsc` | Many | One | All (queued) | Yes/No | Work distribution, command queues |
| `oneshot` | One | One | One | N/A | Request-response, task result |
| `broadcast` | Many | Many | All (each receiver gets every value) | Yes | Event bus, pub-sub |
| `watch` | Many | Many | Latest only | N/A | Configuration, state sharing |

---

## mpsc — Multi-Producer, Single-Consumer

### API Reference

```rust
pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>)
pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>)

impl<T> Sender<T> {
    pub async fn send(&self, value: T) -> Result<(), SendError<T>>
    pub fn try_send(&self, message: T) -> Result<(), TrySendError<T>>
    pub async fn reserve(&self) -> Result<Permit<'_, T>, SendError<()>>
    pub async fn reserve_owned(self) -> Result<OwnedPermit<T>, SendError<()>>
    pub fn try_reserve(&self) -> Result<Permit<'_, T>, TrySendError<()>>
    pub fn try_reserve_owned(self) -> Result<OwnedPermit<T>, TrySendError<()>>
    pub async fn send_timeout(&self, value: T, timeout: Duration) -> Result<(), SendTimeoutError<T>>
    pub fn blocking_send(&self, value: T) -> Result<(), SendError<T>>
    pub fn capacity(&self) -> usize
    pub fn max_capacity(&self) -> usize
    pub fn is_closed(&self) -> bool
    pub async fn closed(&self)
    pub fn same_channel(&self, other: &Self) -> bool
    pub fn downgrade(&self) -> WeakSender<T>
}

impl<T> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T>
    pub fn try_recv(&mut self) -> Result<T, TryRecvError>
    pub fn blocking_recv(&mut self) -> Option<T>
    pub fn close(&mut self)
    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>>
}
```

### Usage

`send()` waits until buffer space is available (for bounded channels). `recv()` returns `None` when all senders are dropped. The `Sender` is `Clone`; the `Receiver` is not.

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(32);

let tx2 = tx.clone();
tokio::spawn(async move {
    tx.send("from task 1").await.unwrap();
});
tokio::spawn(async move {
    tx2.send("from task 2").await.unwrap();
});

while let Some(msg) = rx.recv().await {
    println!("received: {}", msg);
}
```

### Unbounded Channel

No capacity limit — sends never block. Use with caution, as a slow consumer can cause unbounded memory growth.

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::unbounded_channel();
tx.send("instant").unwrap(); // never blocks, returns Result
```

### Permit Pattern

Reserve capacity before constructing the value to send. Useful when building the message is expensive.

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(1);

let permit = tx.reserve().await.unwrap();
permit.send(expensive_computation());
```

---

## oneshot — Single Value

### API Reference

```rust
pub fn channel<T>() -> (Sender<T>, Receiver<T>)

impl<T> Sender<T> {
    pub fn send(self, value: T) -> Result<T, T>
    pub fn is_closed(&self) -> bool
    pub async fn closed(&mut self)
}

impl<T> Receiver<T> {
    pub fn close(&mut self)
    pub fn try_recv(&mut self) -> Result<T, TryRecvError>
    pub fn blocking_recv(self) -> Result<T, RecvError>
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, RecvError>;
}
```

### Usage

Sends exactly one value. The `Sender` is consumed on `send()`. The `Receiver` implements `Future` and can be awaited directly.

```rust
use tokio::sync::oneshot;

let (tx, rx) = oneshot::channel();

tokio::spawn(async move {
    let result = compute().await;
    let _ = tx.send(result);
});

match rx.await {
    Ok(value) => println!("got: {:?}", value),
    Err(_) => println!("sender dropped"),
}
```

### Request-Response Pattern

```rust
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
enum Command {
    Get { key: String, resp: oneshot::Sender<Option<String>> },
}

async fn run_server(mut rx: mpsc::Receiver<Command>) {
    let mut store = std::collections::HashMap::new();
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Get { key, resp } => {
                let value = store.get(&key).cloned();
                let _ = resp.send(value);
            }
        }
    }
}

async fn client(tx: mpsc::Sender<Command>) {
    let (resp_tx, resp_rx) = oneshot::channel();
    tx.send(Command::Get {
        key: "foo".into(),
        resp: resp_tx,
    }).await.unwrap();

    let result = resp_rx.await.unwrap();
    println!("got: {:?}", result);
}
```

---

## broadcast — Multi-Producer, Multi-Consumer

### API Reference

```rust
pub fn channel<T: Clone>(capacity: usize) -> (Sender<T>, Receiver<T>)

impl<T: Clone> Sender<T> {
    pub fn send(&self, value: T) -> Result<usize, SendError<T>>
    pub fn subscribe(&self) -> Receiver<T>
    pub fn receiver_count(&self) -> usize
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
}

impl<T: Clone> Receiver<T> {
    pub async fn recv(&mut self) -> Result<T, RecvError>
    pub fn try_recv(&mut self) -> Result<T, TryRecvError>
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
    pub fn resubscribe(&self) -> Self
}

impl<T: Clone> Clone for Sender<T> {}
```

### Usage

Every receiver sees every message sent after it subscribed. Messages are cloned for each receiver. If a receiver falls behind, it receives `RecvError::Lagged(n)` indicating `n` messages were skipped.

```rust
use tokio::sync::broadcast;

let (tx, mut rx1) = broadcast::channel(16);
let mut rx2 = tx.subscribe();

tx.send(10).unwrap();
tx.send(20).unwrap();

assert_eq!(rx1.recv().await.unwrap(), 10);
assert_eq!(rx1.recv().await.unwrap(), 20);

assert_eq!(rx2.recv().await.unwrap(), 10);
assert_eq!(rx2.recv().await.unwrap(), 20);
```

### Handling Lagged Receivers

```rust
use tokio::sync::broadcast;

let (tx, mut rx) = broadcast::channel(2);

tx.send(1).unwrap();
tx.send(2).unwrap();
tx.send(3).unwrap(); // oldest message (1) is overwritten

match rx.recv().await {
    Ok(v) => println!("got: {}", v),
    Err(broadcast::error::RecvError::Lagged(n)) => {
        println!("missed {} messages", n);
    }
    Err(broadcast::error::RecvError::Closed) => {
        println!("channel closed");
    }
}
```

---

## watch — Latest Value

### API Reference

```rust
pub fn channel<T>(init: T) -> (Sender<T>, Receiver<T>)

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>>
    pub fn send_modify<F: FnOnce(&mut T)>(&self, modify: F)
    pub fn send_if_modified<F: FnOnce(&mut T) -> bool>(&self, modify: F) -> bool
    pub fn send_replace(&self, value: T) -> T
    pub fn borrow(&self) -> Ref<'_, T>
    pub fn is_closed(&self) -> bool
    pub async fn closed(&self)
    pub fn subscribe(&self) -> Receiver<T>
    pub fn receiver_count(&self) -> usize
}

impl<T> Receiver<T> {
    pub fn borrow(&self) -> Ref<'_, T>
    pub fn borrow_and_update(&mut self) -> Ref<'_, T>
    pub async fn changed(&mut self) -> Result<(), RecvError>
    pub fn has_changed(&self) -> Result<bool, RecvError>
    pub fn mark_changed(&mut self)
    pub fn mark_unchanged(&mut self)
    pub fn same_channel(&self, other: &Self) -> bool
}

impl<T> Clone for Receiver<T> {}
```

### Usage

Only the latest value is retained. Receivers can `borrow()` the current value or `await` `changed()` to be notified of updates. Intermediate values may be skipped if multiple sends occur before a receiver checks.

```rust
use tokio::sync::watch;

let (tx, mut rx) = watch::channel("initial");

tokio::spawn(async move {
    loop {
        rx.changed().await.unwrap();
        let value = rx.borrow().clone();
        println!("config updated: {}", value);
    }
});

tx.send("updated").unwrap();
```

### Configuration Reload Pattern

```rust
use tokio::sync::watch;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct AppConfig {
    max_connections: usize,
    timeout_secs: u64,
}

async fn worker(id: usize, mut config_rx: watch::Receiver<AppConfig>) {
    loop {
        let config = config_rx.borrow_and_update().clone();
        println!("worker {} using max_connections={}", id, config.max_connections);

        tokio::select! {
            _ = config_rx.changed() => continue,
            _ = do_work(&config) => {}
        }
    }
}
```

---

## Mutex

### API Reference

```rust
pub struct Mutex<T: ?Sized> { /* ... */ }

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self
    pub fn const_new(data: T) -> Self
    pub async fn lock(&self) -> MutexGuard<'_, T>
    pub fn blocking_lock(&self) -> MutexGuard<'_, T>
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, TryLockError>
    pub async fn lock_owned(self: Arc<Self>) -> OwnedMutexGuard<T>
    pub fn try_lock_owned(self: Arc<Self>) -> Result<OwnedMutexGuard<T>, TryLockError>
    pub fn into_inner(self) -> T
    pub fn get_mut(&mut self) -> &mut T
}
```

### `tokio::sync::Mutex` vs `std::sync::Mutex`

This is an important design decision:

| | `tokio::sync::Mutex` | `std::sync::Mutex` |
|---|---|---|
| Lock operation | `async`, yields to runtime | Blocking, holds OS thread |
| Holds across `.await` | Yes (designed for this) | No — risks deadlock and blocks the runtime thread |
| Performance | Higher overhead (async bookkeeping) | Lower overhead |
| Poisoning | No | Yes |

**Guidance**: Use `tokio::sync::Mutex` when you need to hold the lock across `.await` points. Use `std::sync::Mutex` for short critical sections that don't cross `.await` boundaries — it is faster and perfectly safe in async code as long as the lock is not held across an `.await`.

### Guard Types

- `MutexGuard<'a, T>` — borrows the `Mutex`, released when dropped.
- `OwnedMutexGuard<T>` — owns an `Arc<Mutex<T>>`, can be sent across tasks.
- `MappedMutexGuard<'a, T>` — projects to a sub-field via `MutexGuard::map()`.

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

let data = Arc::new(Mutex::new(vec![1, 2, 3]));

let data_clone = data.clone();
tokio::spawn(async move {
    let mut lock = data_clone.lock().await;
    lock.push(4);
    // lock held across .await — safe with tokio::sync::Mutex
    some_async_work().await;
    lock.push(5);
});
```

### OwnedMutexGuard

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

let mutex = Arc::new(Mutex::new(String::from("hello")));

let guard = mutex.clone().lock_owned().await;
// guard owns the Arc, can be moved to another task
tokio::spawn(async move {
    println!("{}", *guard);
});
```

---

## RwLock

### API Reference

```rust
pub struct RwLock<T: ?Sized> { /* ... */ }

impl<T> RwLock<T> {
    pub fn new(data: T) -> Self
    pub fn const_new(data: T) -> Self
    pub async fn read(&self) -> RwLockReadGuard<'_, T>
    pub async fn write(&self) -> RwLockWriteGuard<'_, T>
    pub fn try_read(&self) -> Result<RwLockReadGuard<'_, T>, TryLockError>
    pub fn try_write(&self) -> Result<RwLockWriteGuard<'_, T>, TryLockError>
    pub fn blocking_read(&self) -> RwLockReadGuard<'_, T>
    pub fn blocking_write(&self) -> RwLockWriteGuard<'_, T>
    pub async fn read_owned(self: Arc<Self>) -> OwnedRwLockReadGuard<T>
    pub async fn write_owned(self: Arc<Self>) -> OwnedRwLockWriteGuard<T>
    pub fn try_read_owned(self: Arc<Self>) -> Result<OwnedRwLockReadGuard<T>, TryLockError>
    pub fn try_write_owned(self: Arc<Self>) -> Result<OwnedRwLockWriteGuard<T>, TryLockError>
    pub fn into_inner(self) -> T
    pub fn get_mut(&mut self) -> &mut T
}
```

### Usage

Multiple concurrent readers or one exclusive writer. Like `Mutex`, the Tokio `RwLock` is designed to be held across `.await` points.

The Tokio `RwLock` is **write-preferring** — pending writers take priority over pending readers to prevent writer starvation.

```rust
use tokio::sync::RwLock;

let lock = RwLock::new(5);

{
    let r1 = lock.read().await;
    let r2 = lock.read().await; // multiple readers OK
    assert_eq!(*r1 + *r2, 10);
}

{
    let mut w = lock.write().await;
    *w += 1;
    assert_eq!(*w, 6);
}
```

### Guard Types

- `RwLockReadGuard<'a, T>` — shared read access.
- `RwLockWriteGuard<'a, T>` — exclusive write access. Can be downgraded to a read guard via `RwLockWriteGuard::downgrade()`.
- `OwnedRwLockReadGuard<T>` / `OwnedRwLockWriteGuard<T>` — owned variants that hold an `Arc<RwLock<T>>`.
- `MappedRwLockReadGuard<'a, T>` / `MappedRwLockWriteGuard<'a, T>` — projected to a sub-field.

---

## Semaphore

### API Reference

```rust
pub struct Semaphore { /* ... */ }

impl Semaphore {
    pub const fn new(permits: usize) -> Self
    pub const MAX_PERMITS: usize
    pub fn available_permits(&self) -> usize
    pub fn add_permits(&self, n: usize)
    pub async fn acquire(&self) -> Result<SemaphorePermit<'_>, AcquireError>
    pub async fn acquire_many(&self, n: u32) -> Result<SemaphorePermit<'_>, AcquireError>
    pub fn try_acquire(&self) -> Result<SemaphorePermit<'_>, TryAcquireError>
    pub fn try_acquire_many(&self, n: u32) -> Result<SemaphorePermit<'_>, TryAcquireError>
    pub async fn acquire_owned(self: Arc<Self>) -> Result<OwnedSemaphorePermit, AcquireError>
    pub async fn acquire_many_owned(self: Arc<Self>, n: u32) -> Result<OwnedSemaphorePermit, AcquireError>
    pub fn try_acquire_owned(self: Arc<Self>) -> Result<OwnedSemaphorePermit, TryAcquireError>
    pub fn try_acquire_many_owned(self: Arc<Self>, n: u32) -> Result<OwnedSemaphorePermit, TryAcquireError>
    pub fn close(&self)
    pub fn is_closed(&self) -> bool
}

pub struct SemaphorePermit<'a> { /* ... */ }
impl Drop for SemaphorePermit<'_> { /* returns permit on drop */ }
impl SemaphorePermit<'_> {
    pub fn forget(self) // consume without returning permit
}

pub struct OwnedSemaphorePermit { /* ... */ }
impl Drop for OwnedSemaphorePermit { /* returns permit on drop */ }
impl OwnedSemaphorePermit {
    pub fn forget(self)
}
```

### Usage

RAII-based permit system. Permits are returned automatically when the guard is dropped.

```rust
use tokio::sync::Semaphore;

let sem = Semaphore::new(3);

{
    let _permit = sem.acquire().await.unwrap();
    assert_eq!(sem.available_permits(), 2);
    // permit returned when _permit is dropped
}

assert_eq!(sem.available_permits(), 3);
```

### Rate Limiting

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

async fn rate_limited_fetch(urls: Vec<String>) {
    let semaphore = Arc::new(Semaphore::new(10)); // max 10 concurrent requests
    let mut handles = vec![];

    for url in urls {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            reqwest::get(&url).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }
}
```

### Connection Pool

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

struct ConnectionPool {
    semaphore: Arc<Semaphore>,
    max_size: usize,
}

impl ConnectionPool {
    fn new(max_size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_size)),
            max_size,
        }
    }

    async fn get_connection(&self) -> PooledConnection {
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        let conn = create_connection().await;
        PooledConnection { conn, _permit: permit }
    }
}

struct PooledConnection {
    conn: Connection,
    _permit: tokio::sync::OwnedSemaphorePermit, // returned on drop
}
```

---

## Notify

### API Reference

```rust
pub struct Notify { /* ... */ }

impl Notify {
    pub const fn new() -> Self
    pub fn notify_one(&self)
    pub fn notify_waiters(&self)
    pub fn notified(&self) -> Notified<'_>
}

pub struct Notified<'a> { /* ... */ }

impl<'a> Future for Notified<'a> {
    type Output = ();
}

impl<'a> Notified<'a> {
    pub fn enable(self: Pin<&mut Self>)
}
```

### Usage

A basic async notification primitive. `notify_one()` wakes a single waiter. `notify_waiters()` wakes all current waiters. If `notify_one()` is called before anyone is waiting, the next call to `notified().await` completes immediately (one stored permit).

```rust
use tokio::sync::Notify;
use std::sync::Arc;

let notify = Arc::new(Notify::new());
let notify_clone = notify.clone();

tokio::spawn(async move {
    // Do some work...
    notify_clone.notify_one();
});

notify.notified().await;
println!("received notification");
```

### Shutdown Signal

```rust
use tokio::sync::Notify;
use std::sync::Arc;

let shutdown = Arc::new(Notify::new());

for i in 0..4 {
    let shutdown = shutdown.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    println!("worker {} shutting down", i);
                    return;
                }
                _ = do_work() => {}
            }
        }
    });
}

// Later, signal all workers to stop
shutdown.notify_waiters();
```

---

## Barrier

### API Reference

```rust
pub struct Barrier { /* ... */ }

impl Barrier {
    pub const fn new(n: usize) -> Self
    pub async fn wait(&self) -> BarrierWaitResult
}

pub struct BarrierWaitResult { /* ... */ }

impl BarrierWaitResult {
    pub fn is_leader(&self) -> bool
}
```

### Usage

Blocks all callers until `n` tasks have called `wait()`. Exactly one waiter is designated the "leader" (`is_leader()` returns `true`).

```rust
use tokio::sync::Barrier;
use std::sync::Arc;

let barrier = Arc::new(Barrier::new(3));

for i in 0..3 {
    let b = barrier.clone();
    tokio::spawn(async move {
        println!("task {} before barrier", i);
        let result = b.wait().await;
        println!("task {} after barrier (leader: {})", i, result.is_leader());
    });
}
```

---

## OnceCell

### API Reference

```rust
pub struct OnceCell<T> { /* ... */ }

impl<T> OnceCell<T> {
    pub const fn new() -> Self
    pub const fn new_with(value: T) -> Self
    pub fn initialized(&self) -> bool
    pub fn get(&self) -> Option<&T>
    pub fn set(&self, value: T) -> Result<(), SetError<T>>
    pub async fn get_or_init<F, Fut>(&self, f: F) -> &T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>
    pub async fn get_or_try_init<F, Fut, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>
    pub fn into_inner(self) -> Option<T>
    pub fn take(&mut self) -> Option<T>
}
```

### Usage

Async equivalent of `std::sync::OnceLock`. Initializes a value at most once using an async function.

```rust
use tokio::sync::OnceCell;

static DB: OnceCell<Database> = OnceCell::const_new();

async fn get_db() -> &'static Database {
    DB.get_or_init(|| async {
        Database::connect("postgres://localhost/mydb").await.unwrap()
    }).await
}
```

---

## See Also

- [Time Utilities](05-time.md) — sleep, interval, and timeout
- [Async File System](07-fs.md) — async file I/O
- [Macros](08-macros.md) — `select!` and `join!` for combining async operations
