# tokio — Sub-Index

> Rust async runtime — tasks, I/O, networking, synchronization (26 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Quick Start](README.md#quick-start) · [Architecture](README.md#architecture) · [Essential Rust Types](README.md#essential-rust-types) · [Documentation Map](README.md#documentation-map) · [Quick Links](README.md#quick-links) · [External Resources](README.md#external-resources)|

### [concepts](concepts/)

|file|description|
|---|---|
|[01-async-await.md](concepts/01-async-await.md)|Async/await — fundamentals, .await points|
| |↳ [What Is Async Programming?](concepts/01-async-await.md#what-is-async-programming) · [`async fn` Returns a Future](concepts/01-async-await.md#async-fn-returns-a-future) · [`.await` Yields Control](concepts/01-async-await.md#await-yields-control) · [Compile-Time State Machines](concepts/01-async-await.md#compile-time-state-machines) · [No Implicit Runtime](concepts/01-async-await.md#no-implicit-runtime) · [Async Blocks](concepts/01-async-await.md#async-blocks) · [When Async Helps](concepts/01-async-await.md#when-async-helps) · [When Async Does NOT Help](concepts/01-async-await.md#when-async-does-not-help) · +2 more|
|[02-futures-in-depth.md](concepts/02-futures-in-depth.md)|Futures — Future trait, Pin, polling|
| |↳ [The `Future` Trait](concepts/02-futures-in-depth.md#the-future-trait) · [Poll::Ready vs Poll::Pending](concepts/02-futures-in-depth.md#pollready-vs-pollpending) · [Futures as State Machines](concepts/02-futures-in-depth.md#futures-as-state-machines) · [Wakers: How the Runtime Knows When to Re-Poll](concepts/02-futures-in-depth.md#wakers-how-the-runtime-knows-when-to-re-poll) · [Mini-Tokio: A Conceptual Runtime](concepts/02-futures-in-depth.md#mini-tokio-a-conceptual-runtime) · [Composed Futures](concepts/02-futures-in-depth.md#composed-futures) · [Pinning](concepts/02-futures-in-depth.md#pinning) · [Key Rules Summary](concepts/02-futures-in-depth.md#key-rules-summary)|
|[03-cancellation.md](concepts/03-cancellation.md)|Cancellation — drop-based, CancellationToken|
| |↳ [Cancellation by Dropping](concepts/03-cancellation.md#cancellation-by-dropping) · [`select!` Cancels Non-Winning Branches](concepts/03-cancellation.md#select-cancels-non-winning-branches) · [`JoinHandle::abort()`](concepts/03-cancellation.md#joinhandleabort) · [Cancellation Safety](concepts/03-cancellation.md#cancellation-safety) · [CancellationToken (tokio-util)](concepts/03-cancellation.md#cancellationtoken-tokio-util) · [Graceful Shutdown Pattern](concepts/03-cancellation.md#graceful-shutdown-pattern)|

### [tutorial](tutorial/)

|file|description|
|---|---|
|[01-setup-and-hello.md](tutorial/01-setup-and-hello.md)|Setup — #[tokio::main], runtime configuration|
| |↳ [Prerequisites](tutorial/01-setup-and-hello.md#prerequisites) · [Project Setup](tutorial/01-setup-and-hello.md#project-setup) · [First Async Program](tutorial/01-setup-and-hello.md#first-async-program) · [Breaking It Down](tutorial/01-setup-and-hello.md#breaking-it-down) · [Feature Flags](tutorial/01-setup-and-hello.md#feature-flags) · [Next Steps](tutorial/01-setup-and-hello.md#next-steps)|
|[02-spawning.md](tutorial/02-spawning.md)|Spawning — tokio::spawn, JoinHandle, 'static|
| |↳ [Accepting TCP Connections](tutorial/02-spawning.md#accepting-tcp-connections) · [Solution: tokio::spawn()](tutorial/02-spawning.md#solution-tokiospawn) · [Tasks Explained](tutorial/02-spawning.md#tasks-explained) · [The 'static Bound](tutorial/02-spawning.md#the-static-bound) · [The Send Bound](tutorial/02-spawning.md#the-send-bound) · [Complete process() Function](tutorial/02-spawning.md#complete-process-function) · [Next Steps](tutorial/02-spawning.md#next-steps)|
|[03-shared-state.md](tutorial/03-shared-state.md)|Shared state — Arc<Mutex<T>>, sharded state|
| |↳ [The Problem](tutorial/03-shared-state.md#the-problem) · [Two Strategies](tutorial/03-shared-state.md#two-strategies) · [Mutex Approach](tutorial/03-shared-state.md#mutex-approach) · [std::sync::Mutex vs tokio::sync::Mutex](tutorial/03-shared-state.md#stdsyncmutex-vs-tokiosyncmutex) · [Holding MutexGuard Across .await](tutorial/03-shared-state.md#holding-mutexguard-across-await) · [Mutex Sharding](tutorial/03-shared-state.md#mutex-sharding) · [Tasks, Threads, and Contention](tutorial/03-shared-state.md#tasks-threads-and-contention) · [Next Steps](tutorial/03-shared-state.md#next-steps)|
|[04-channels.md](tutorial/04-channels.md)|Channels — mpsc, oneshot, broadcast, watch|
| |↳ [The Problem](tutorial/04-channels.md#the-problem) · [Message Passing Pattern](tutorial/04-channels.md#message-passing-pattern) · [Tokio Channel Types](tutorial/04-channels.md#tokio-channel-types) · [Implementing with mpsc + oneshot](tutorial/04-channels.md#implementing-with-mpsc-oneshot) · [Complete Example](tutorial/04-channels.md#complete-example) · [Backpressure and Bounded Channels](tutorial/04-channels.md#backpressure-and-bounded-channels) · [Next Steps](tutorial/04-channels.md#next-steps)|
|[05-io.md](tutorial/05-io.md)|I/O — AsyncRead, AsyncWrite, copy|
| |↳ [Reading](tutorial/05-io.md#reading) · [Writing](tutorial/05-io.md#writing) · [Helper Functions](tutorial/05-io.md#helper-functions) · [Echo Server](tutorial/05-io.md#echo-server) · [Splitting Reader and Writer](tutorial/05-io.md#splitting-reader-and-writer) · [Buffer Allocation](tutorial/05-io.md#buffer-allocation) · [EOF Handling](tutorial/05-io.md#eof-handling)|
|[06-framing.md](tutorial/06-framing.md)|Framing — codecs, length-delimited, tokio-util|
| |↳ [Redis Frame Type](tutorial/06-framing.md#redis-frame-type) · [Connection Struct](tutorial/06-framing.md#connection-struct) · [Buffered Reads](tutorial/06-framing.md#buffered-reads) · [Parsing: Two-Step Check + Parse](tutorial/06-framing.md#parsing-two-step-check-parse) · [Buffered Writes](tutorial/06-framing.md#buffered-writes) · [bytes Crate Integration](tutorial/06-framing.md#bytes-crate-integration)|
|[07-select.md](tutorial/07-select.md)|Select — tokio::select!, branch fairness|
| |↳ [Basic Syntax](tutorial/07-select.md#basic-syntax) · [Cancellation](tutorial/07-select.md#cancellation) · [Under the Hood](tutorial/07-select.md#under-the-hood) · [Syntax Details](tutorial/07-select.md#syntax-details) · [Borrowing](tutorial/07-select.md#borrowing) · [Loops with `select!`](tutorial/07-select.md#loops-with-select) · [Per-Task Concurrency: `select!` vs `spawn`](tutorial/07-select.md#per-task-concurrency-select-vs-spawn)|
|[08-streams.md](tutorial/08-streams.md)|Streams — Stream trait, StreamExt|
| |↳ [tokio-stream Crate](tutorial/08-streams.md#tokio-stream-crate) · [Iteration](tutorial/08-streams.md#iteration) · [Stream Adapters](tutorial/08-streams.md#stream-adapters) · [StreamMap](tutorial/08-streams.md#streammap) · [Implementing Stream Manually](tutorial/08-streams.md#implementing-stream-manually) · [async-stream Crate](tutorial/08-streams.md#async-stream-crate) · [tokio_stream::wrappers](tutorial/08-streams.md#tokio_streamwrappers)|

### [rust-api](rust-api/)

|file|description|
|---|---|
|[01-runtime.md](rust-api/01-runtime.md)|Runtime — Builder, current_thread, multi_thread|
| |↳ [API Reference](rust-api/01-runtime.md#api-reference) · [Runtime](rust-api/01-runtime.md#runtime) · [Builder](rust-api/01-runtime.md#builder) · [Handle](rust-api/01-runtime.md#handle) · [RuntimeFlavor](rust-api/01-runtime.md#runtimeflavor) · [EnterGuard](rust-api/01-runtime.md#enterguard) · [The `#[tokio::main]` Macro](rust-api/01-runtime.md#the-tokiomain-macro) · [The `#[tokio::test]` Macro](rust-api/01-runtime.md#the-tokiotest-macro) · +2 more|
|[02-tasks.md](rust-api/02-tasks.md)|Tasks — spawn, spawn_blocking, JoinSet|
| |↳ [API Reference](rust-api/02-tasks.md#api-reference) · [`tokio::spawn(future)`](rust-api/02-tasks.md#tokiospawnfuture) · [JoinHandle](rust-api/02-tasks.md#joinhandle) · [JoinSet](rust-api/02-tasks.md#joinset) · [`spawn_blocking(func)`](rust-api/02-tasks.md#spawn_blockingfunc) · [`block_in_place(f)`](rust-api/02-tasks.md#block_in_placef) · [`yield_now()`](rust-api/02-tasks.md#yield_now) · [LocalSet](rust-api/02-tasks.md#localset) · +5 more|
|[03-io.md](rust-api/03-io.md)|I/O traits — AsyncRead, AsyncWrite, BufReader|
| |↳ [API Reference](rust-api/03-io.md#api-reference) · [Core Traits](rust-api/03-io.md#core-traits) · [AsyncReadExt Methods](rust-api/03-io.md#asyncreadext-methods) · [AsyncWriteExt Methods](rust-api/03-io.md#asyncwriteext-methods) · [AsyncBufReadExt Methods](rust-api/03-io.md#asyncbufreadext-methods) · [Helper Functions](rust-api/03-io.md#helper-functions) · [Buffered I/O](rust-api/03-io.md#buffered-io) · [ReadHalf and WriteHalf](rust-api/03-io.md#readhalf-and-writehalf) · +5 more|
|[04-networking.md](rust-api/04-networking.md)|Networking — TcpListener, TcpStream, UdpSocket|
| |↳ [API Reference](rust-api/04-networking.md#api-reference) · [TcpListener](rust-api/04-networking.md#tcplistener) · [TcpStream](rust-api/04-networking.md#tcpstream) · [TcpSocket](rust-api/04-networking.md#tcpsocket) · [UdpSocket](rust-api/04-networking.md#udpsocket) · [Unix Domain Sockets](rust-api/04-networking.md#unix-domain-sockets) · [ToSocketAddrs](rust-api/04-networking.md#tosocketaddrs) · [Examples](rust-api/04-networking.md#examples) · +1 more|
|[05-time.md](rust-api/05-time.md)|Time — sleep, interval, timeout|
| |↳ [API Reference](rust-api/05-time.md#api-reference) · [`sleep(duration)`](rust-api/05-time.md#sleepduration) · [`sleep_until(deadline)`](rust-api/05-time.md#sleep_untildeadline) · [`interval(period)`](rust-api/05-time.md#intervalperiod) · [`interval_at(start, period)`](rust-api/05-time.md#interval_atstart-period) · [`timeout(duration, future)`](rust-api/05-time.md#timeoutduration-future) · [`timeout_at(deadline, future)`](rust-api/05-time.md#timeout_atdeadline-future) · [MissedTickBehavior](rust-api/05-time.md#missedtickbehavior) · +3 more|
|[06-sync.md](rust-api/06-sync.md)|Sync — Mutex, RwLock, Semaphore, Notify, Barrier|
| |↳ [Channels Overview](rust-api/06-sync.md#channels-overview) · [mpsc — Multi-Producer, Single-Consumer](rust-api/06-sync.md#mpsc-multi-producer-single-consumer) · [oneshot — Single Value](rust-api/06-sync.md#oneshot-single-value) · [broadcast — Multi-Producer, Multi-Consumer](rust-api/06-sync.md#broadcast-multi-producer-multi-consumer) · [watch — Latest Value](rust-api/06-sync.md#watch-latest-value) · [Mutex](rust-api/06-sync.md#mutex) · [RwLock](rust-api/06-sync.md#rwlock) · [Semaphore](rust-api/06-sync.md#semaphore) · +3 more|
|[07-fs.md](rust-api/07-fs.md)|Filesystem — tokio::fs async file operations|
| |↳ [File](rust-api/07-fs.md#file) · [OpenOptions](rust-api/07-fs.md#openoptions) · [Convenience Functions](rust-api/07-fs.md#convenience-functions) · [Directory Operations](rust-api/07-fs.md#directory-operations) · [DirBuilder](rust-api/07-fs.md#dirbuilder) · [File Operations](rust-api/07-fs.md#file-operations) · [Link Operations](rust-api/07-fs.md#link-operations) · [Path Operations](rust-api/07-fs.md#path-operations) · +2 more|
|[08-macros.md](rust-api/08-macros.md)|Macros — #[tokio::main], #[tokio::test], select!, join!|
| |↳ [`select!`](rust-api/08-macros.md#select) · [`join!`](rust-api/08-macros.md#join) · [`try_join!`](rust-api/08-macros.md#try_join) · [`pin!`](rust-api/08-macros.md#pin) · [`task_local!`](rust-api/08-macros.md#task_local) · [Comparison Table](rust-api/08-macros.md#comparison-table)|

### [topics](topics/)

|file|description|
|---|---|
|[01-bridging-sync-code.md](topics/01-bridging-sync-code.md)|Bridging — spawn_blocking, Handle::block_on|
| |↳ [`#[tokio::main]` Expansion](topics/01-bridging-sync-code.md#tokiomain-expansion) · [Creating a Runtime Manually](topics/01-bridging-sync-code.md#creating-a-runtime-manually) · [BlockingClient Pattern](topics/01-bridging-sync-code.md#blockingclient-pattern) · [Runtime Flavors for Bridging](topics/01-bridging-sync-code.md#runtime-flavors-for-bridging) · [Three Approaches](topics/01-bridging-sync-code.md#three-approaches) · [`Handle::current()`](topics/01-bridging-sync-code.md#handlecurrent) · [`spawn_blocking()` — Blocking from Async](topics/01-bridging-sync-code.md#spawn_blocking-blocking-from-async)|
|[02-graceful-shutdown.md](topics/02-graceful-shutdown.md)|Graceful shutdown — signal handling, drain|
| |↳ [Detecting Shutdown](topics/02-graceful-shutdown.md#detecting-shutdown) · [Propagating Shutdown](topics/02-graceful-shutdown.md#propagating-shutdown) · [Waiting for Completion](topics/02-graceful-shutdown.md#waiting-for-completion) · [Complete Pattern](topics/02-graceful-shutdown.md#complete-pattern)|
|[03-tracing.md](topics/03-tracing.md)|Tracing — tokio-console, tracing integration|
| |↳ [Setup](topics/03-tracing.md#setup) · [Emitting Spans](topics/03-tracing.md#emitting-spans) · [Emitting Events](topics/03-tracing.md#emitting-events) · [Layers](topics/03-tracing.md#layers) · [tokio-console](topics/03-tracing.md#tokio-console)|

### [ecosystem](ecosystem/)

|file|description|
|---|---|
|[01-workspace-crates.md](ecosystem/01-workspace-crates.md)|Workspace — tokio, tokio-util, tokio-stream|
| |↳ [tokio](ecosystem/01-workspace-crates.md#tokio) · [tokio-macros](ecosystem/01-workspace-crates.md#tokio-macros) · [tokio-stream](ecosystem/01-workspace-crates.md#tokio-stream) · [tokio-util](ecosystem/01-workspace-crates.md#tokio-util) · [tokio-test](ecosystem/01-workspace-crates.md#tokio-test) · [bytes](ecosystem/01-workspace-crates.md#bytes) · [mio](ecosystem/01-workspace-crates.md#mio) · [Summary Table](ecosystem/01-workspace-crates.md#summary-table) · +1 more|
|[02-tower-and-hyper.md](ecosystem/02-tower-and-hyper.md)|Tower & Hyper — service middleware, HTTP|
| |↳ [Tower](ecosystem/02-tower-and-hyper.md#tower) · [tower-http](ecosystem/02-tower-and-hyper.md#tower-http) · [Hyper](ecosystem/02-tower-and-hyper.md#hyper) · [Axum](ecosystem/02-tower-and-hyper.md#axum) · [How They Fit Together](ecosystem/02-tower-and-hyper.md#how-they-fit-together) · [Next Steps](ecosystem/02-tower-and-hyper.md#next-steps)|
|[03-related-projects.md](ecosystem/03-related-projects.md)|Related — mio, bytes, tracing|
| |↳ [Networking](ecosystem/03-related-projects.md#networking) · [Observability](ecosystem/03-related-projects.md#observability) · [TLS](ecosystem/03-related-projects.md#tls) · [WebSockets](ecosystem/03-related-projects.md#websockets) · [Utilities](ecosystem/03-related-projects.md#utilities) · [Testing](ecosystem/03-related-projects.md#testing) · [Serialization](ecosystem/03-related-projects.md#serialization) · [Summary Table](ecosystem/03-related-projects.md#summary-table) · +1 more|

### Key Patterns
```rust
#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async { /* ... */ });
    tokio::select! { _ = handle => {} }
}
```

---
*26 files · Related: [axum](../axum/INDEX.md), [tower](../tower/INDEX.md), [tonic](../tonic/INDEX.md)*
