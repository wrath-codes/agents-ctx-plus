# Shared State

## Overview

The previous tutorial gave each connection its own `HashMap`. Here we share state across all connections using `Arc<Mutex<T>>`, and discuss when to use `std::sync::Mutex` vs `tokio::sync::Mutex`.

## The Problem

Each spawned task has its own `db: HashMap`. A value set by one client is invisible to another. We need a single shared store.

## Two Strategies

1. **Mutex** — protect shared data with a lock (simple, synchronous access)
2. **Message passing** — a dedicated task owns the resource, others send messages (actor pattern)

Use a Mutex when the data is simple and the critical section doesn't need async operations. Use message passing when you need async work while accessing the resource.

## Mutex Approach

### Type Setup

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bytes::Bytes;

type Db = Arc<Mutex<HashMap<String, Bytes>>>;
```

### Initialize and Share

```rust
use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame, Command};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let db = db.clone(); // Clone the Arc, not the data

        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}
```

### Updated process()

```rust
async fn process(socket: TcpStream, db: Db) {
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Command::Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Command::Get(cmd) => {
                let db = db.lock().unwrap();
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

The lock is acquired, used, and implicitly dropped (end of `match` arm) before any `.await` — this is critical.

## std::sync::Mutex vs tokio::sync::Mutex

| | `std::sync::Mutex` | `tokio::sync::Mutex` |
|---|---|---|
| **Lock contention** | Blocks the OS thread | Yields to the runtime |
| **Held across .await** | No (compiler error — `!Send`) | Yes |
| **Performance** | Faster (no async overhead) | Slower (async machinery) |
| **When to use** | Lock is short-lived, no .await inside critical section | Lock must be held across .await points |

**Rule of thumb**: Use `std::sync::Mutex` by default. Only reach for `tokio::sync::Mutex` when you genuinely need to hold the lock across an `.await`.

## Holding MutexGuard Across .await

### The Problem

`std::sync::MutexGuard` is `!Send`. If it's alive across an `.await`, the compiler rejects it because the runtime may move the task to another thread:

```rust
// Does NOT compile:
async fn bad(db: &Mutex<HashMap<String, Bytes>>) {
    let mut guard = db.lock().unwrap();
    guard.insert("key".to_string(), "value".into());
    some_async_fn().await; // guard held across .await → !Send error
}
```

### Solution 1: Scope the Guard

Use a block to drop the guard before `.await`:

```rust
async fn good(db: &Mutex<HashMap<String, Bytes>>) {
    {
        let mut guard = db.lock().unwrap();
        guard.insert("key".to_string(), "value".into());
    } // guard dropped here

    some_async_fn().await; // safe — no guard held
}
```

### Solution 2: Wrap in a Struct

Encapsulate the Mutex in a struct with synchronous methods:

```rust
struct SharedDb {
    inner: Mutex<HashMap<String, Bytes>>,
}

impl SharedDb {
    fn get(&self, key: &str) -> Option<Bytes> {
        let db = self.inner.lock().unwrap();
        db.get(key).cloned()
    }

    fn set(&self, key: String, value: Bytes) {
        let mut db = self.inner.lock().unwrap();
        db.insert(key, value);
    }
}
```

The guard never escapes the synchronous method — no risk of holding it across `.await`.

### Solution 3: tokio::sync::Mutex (Last Resort)

```rust
use tokio::sync::Mutex;

async fn with_tokio_mutex(db: &Mutex<HashMap<String, Bytes>>) {
    let mut guard = db.lock().await; // async lock
    guard.insert("key".to_string(), "value".into());
    some_async_fn().await; // OK — tokio Mutex guard is Send
}
```

This works but adds async overhead. Prefer the scoping approaches first.

## Mutex Sharding

For high-contention workloads, a single Mutex becomes a bottleneck. Shard the data across multiple Mutexes:

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

type ShardedDb = Arc<Vec<Mutex<HashMap<String, Bytes>>>>;

fn new_sharded_db(num_shards: usize) -> ShardedDb {
    let mut shards = Vec::with_capacity(num_shards);
    for _ in 0..num_shards {
        shards.push(Mutex::new(HashMap::new()));
    }
    Arc::new(shards)
}

fn get_shard(db: &ShardedDb, key: &str) -> &Mutex<HashMap<String, Bytes>> {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let index = hasher.finish() as usize % db.len();
    &db[index]
}
```

**Usage**:

```rust
let db = new_sharded_db(16);

// Access a specific shard
let shard = get_shard(&db, "my-key");
let mut guard = shard.lock().unwrap();
guard.insert("my-key".to_string(), "value".into());
```

For production, consider the [`dashmap`](https://docs.rs/dashmap) crate, which provides a concurrent HashMap with internal sharding.

## Tasks, Threads, and Contention

Tokio's multi-threaded runtime runs tasks on a thread pool (default: one thread per CPU core). Multiple tasks may contend on the same Mutex simultaneously.

- Keep critical sections short — lock, operate, unlock.
- Avoid I/O or `.await` inside the critical section.
- If contention is measurable, shard the data.
- The Mutex itself is cheap — contention is the cost.

## Next Steps

- **[Channels](04-channels.md)** - Use message passing instead of shared state
