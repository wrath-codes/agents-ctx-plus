# tokio — Sub-Index

> Rust async runtime — tasks, I/O, networking, synchronization (27 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [concepts](concepts/)

|file|description|
|---|---|
|[01-async-await.md](concepts/01-async-await.md)|Async/await — fundamentals, .await points|
|[02-futures-in-depth.md](concepts/02-futures-in-depth.md)|Futures — Future trait, Pin, polling|
|[03-cancellation.md](concepts/03-cancellation.md)|Cancellation — drop-based, CancellationToken|

### [tutorial](tutorial/)

|file|description|
|---|---|
|[01-setup-and-hello.md](tutorial/01-setup-and-hello.md)|Setup — #[tokio::main], runtime configuration|
|[02-spawning.md](tutorial/02-spawning.md)|Spawning — tokio::spawn, JoinHandle, 'static|
|[03-shared-state.md](tutorial/03-shared-state.md)|Shared state — Arc<Mutex<T>>, sharded state|
|[04-channels.md](tutorial/04-channels.md)|Channels — mpsc, oneshot, broadcast, watch|
|[05-io.md](tutorial/05-io.md)|I/O — AsyncRead, AsyncWrite, copy|
|[06-framing.md](tutorial/06-framing.md)|Framing — codecs, length-delimited, tokio-util|
|[07-select.md](tutorial/07-select.md)|Select — tokio::select!, branch fairness|
|[08-streams.md](tutorial/08-streams.md)|Streams — Stream trait, StreamExt|

### [rust-api](rust-api/)

|file|description|
|---|---|
|[01-runtime.md](rust-api/01-runtime.md)|Runtime — Builder, current_thread, multi_thread|
|[02-tasks.md](rust-api/02-tasks.md)|Tasks — spawn, spawn_blocking, JoinSet|
|[03-io.md](rust-api/03-io.md)|I/O traits — AsyncRead, AsyncWrite, BufReader|
|[04-networking.md](rust-api/04-networking.md)|Networking — TcpListener, TcpStream, UdpSocket|
|[05-time.md](rust-api/05-time.md)|Time — sleep, interval, timeout|
|[06-sync.md](rust-api/06-sync.md)|Sync — Mutex, RwLock, Semaphore, Notify, Barrier|
|[07-fs.md](rust-api/07-fs.md)|Filesystem — tokio::fs async file operations|
|[08-macros.md](rust-api/08-macros.md)|Macros — #[tokio::main], #[tokio::test], select!, join!|

### [topics](topics/)

|file|description|
|---|---|
|[01-bridging-sync-code.md](topics/01-bridging-sync-code.md)|Bridging — spawn_blocking, Handle::block_on|
|[02-graceful-shutdown.md](topics/02-graceful-shutdown.md)|Graceful shutdown — signal handling, drain|
|[03-tracing.md](topics/03-tracing.md)|Tracing — tokio-console, tracing integration|

### [ecosystem](ecosystem/)

|file|description|
|---|---|
|[01-workspace-crates.md](ecosystem/01-workspace-crates.md)|Workspace — tokio, tokio-util, tokio-stream|
|[02-tower-and-hyper.md](ecosystem/02-tower-and-hyper.md)|Tower & Hyper — service middleware, HTTP|
|[03-related-projects.md](ecosystem/03-related-projects.md)|Related — mio, bytes, tracing|

### Key Patterns
```rust
#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async { /* ... */ });
    tokio::select! { _ = handle => {} }
}
```

---
*27 files · Related: [axum](../axum/INDEX.md), [tower](../tower/INDEX.md), [tonic](../tonic/INDEX.md)*
