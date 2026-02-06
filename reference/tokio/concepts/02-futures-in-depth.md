# How Futures Work

Futures are the core abstraction behind Rust's async system. Understanding how they work internally — polling, wakers, state machines, and pinning — is essential for writing correct and efficient async code.

---

## The `Future` Trait

The entire async system is built on a single trait from `std::future`:

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

| Component | Purpose |
|-----------|---------|
| `Pin<&mut Self>` | Guarantees the future won't be moved in memory (see [Pinning](#pinning)) |
| `Context<'_>` | Carries a `Waker` that the runtime uses to know when to re-poll |
| `Poll::Ready(val)` | The future has completed with value `val` |
| `Poll::Pending` | The future is not yet ready; the runtime should try again later |

The runtime calls `poll()` repeatedly. Each call either completes the future or tells the runtime to wait.

---

## Poll::Ready vs Poll::Pending

```
                    ┌──────────┐
        poll()      │          │  Poll::Ready(value)
    ───────────────►│  Future  ├──────────────────────► done
        poll()      │          │
    ───────────────►│          │  Poll::Pending
                    └────┬─────┘
                         │
                         │ registers Waker
                         ▼
                    wake() called when
                    I/O is ready
                         │
                         ▼
                    runtime re-polls
```

**Key rule:** A future must **only** return `Poll::Pending` after ensuring its `Waker` is registered with the I/O source. Otherwise the runtime will never know to re-poll, and the future will hang.

---

## Futures as State Machines

The compiler transforms an `async fn` into an enum that represents every suspension point. Consider:

```rust
async fn example(client: &Client) -> Response {
    let req = build_request().await;    // state 0 → state 1
    let resp = client.send(req).await;  // state 1 → state 2
    resp
}
```

This becomes roughly:

```rust
enum ExampleFuture<'a> {
    // State 0: polling build_request()
    BuildingRequest {
        client: &'a Client,
        fut: BuildRequestFuture,
    },
    // State 1: polling client.send()
    Sending {
        client: &'a Client,
        fut: SendFuture,
    },
    // Terminal state
    Done,
}

impl<'a> Future for ExampleFuture<'a> {
    type Output = Response;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Response> {
        loop {
            match self.get_mut() {
                ExampleFuture::BuildingRequest { client, fut } => {
                    let req = ready!(Pin::new(fut).poll(cx));
                    *self = ExampleFuture::Sending {
                        client,
                        fut: client.send(req),
                    };
                }
                ExampleFuture::Sending { fut, .. } => {
                    let resp = ready!(Pin::new(fut).poll(cx));
                    *self = ExampleFuture::Done;
                    return Poll::Ready(resp);
                }
                ExampleFuture::Done => panic!("polled after completion"),
            }
        }
    }
}
```

Each `.await` becomes a state transition. The future stores only the data needed for the **current** state — variables that span suspension points live in the enum variant.

---

## Wakers: How the Runtime Knows When to Re-Poll

When a future returns `Poll::Pending`, the runtime needs a way to know **when** to poll again. This is the job of the `Waker`.

### How Wakers Work

1. The runtime passes a `Context` to `poll()`. The `Context` contains a `Waker`.
2. The future (or the I/O resource it wraps) **clones** the `Waker` and stores it.
3. When the underlying I/O event fires, the resource calls `waker.wake()`.
4. The runtime receives the wake signal and schedules the future for re-polling.

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Delay {
    when: Instant,
    waker: Option<Waker>,
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if Instant::now() >= self.when {
            return Poll::Ready(());
        }

        // Store the LATEST waker — the runtime may change it between polls
        self.waker = Some(cx.waker().clone());

        // Arrange for wake() to be called when the deadline arrives
        // (in a real implementation, register with a timer wheel)
        Poll::Pending
    }
}
```

### Waker Rules

- **Always store the latest Waker.** The runtime may provide a different `Waker` on each `poll()` call. Always replace the stored waker with `cx.waker().clone()`.
- **Wakers are `Send + Sync`.** They can be sent across threads — this is how I/O threads signal the runtime.
- **`wake()` vs `wake_by_ref()`** — `wake()` consumes the `Waker`, `wake_by_ref()` borrows it. Use `wake_by_ref()` when you want to keep the waker around.

---

## Mini-Tokio: A Conceptual Runtime

A minimal runtime is just a queue of tasks and a loop that polls them:

```rust
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct MiniTokio {
    tasks: VecDeque<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl MiniTokio {
    fn new() -> Self {
        MiniTokio { tasks: VecDeque::new() }
    }

    fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tasks.push_back(Box::pin(future));
    }

    fn run(&mut self) {
        // Naive: polls every task repeatedly (busy-loop)
        while let Some(mut task) = self.tasks.pop_front() {
            let waker = futures::task::noop_waker();
            let mut cx = Context::from_waker(&waker);

            if task.as_mut().poll(&mut cx).is_pending() {
                self.tasks.push_back(task);
            }
        }
    }
}
```

This busy-loops, which wastes CPU. A real runtime uses wakers to sleep until work is available.

### Improved Mini-Tokio with Channels

```rust
use std::sync::{Arc, mpsc};
use futures::task::{self, ArcWake};

struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    sender: mpsc::Sender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Re-enqueue the task when woken
        let _ = arc_self.sender.send(arc_self.clone());
    }
}

struct MiniTokio {
    sender: mpsc::Sender<Arc<Task>>,
    receiver: mpsc::Receiver<Arc<Task>>,
}

impl MiniTokio {
    fn run(&self) {
        // Blocks on the channel — no busy-loop
        while let Ok(task) = self.receiver.recv() {
            let waker = task::waker(task.clone());
            let mut cx = Context::from_waker(&waker);

            let mut future = task.future.lock().unwrap();
            if future.as_mut().poll(&mut cx).is_pending() {
                // Task stays alive; wake() will re-enqueue it
            }
        }
    }
}
```

The `ArcWake` trait (from the `futures` crate) simplifies creating wakers from `Arc<T>`. When `wake()` is called, it sends the task back through the channel so the run loop picks it up again.

---

## Composed Futures

When you `.await` inside an `async fn`, the outer future **polls** the inner future. This creates a tree of futures:

```
outer.poll()
  └─► inner_a.poll()  → Pending
      (outer stores inner_a's state, returns Pending)

outer.poll()  (re-polled after wake)
  └─► inner_a.poll()  → Ready(val)
  └─► inner_b.poll()  → Pending
      (outer advances to next state, returns Pending)
```

The waker propagates from the outermost runtime call down through every layer. Each inner future registers the **same** waker with its I/O resource.

---

## Pinning

### Why Pin Exists

Async state machines can contain **self-referential** data. When a variable defined before an `.await` is referenced after it, the compiled state machine stores both the value and a reference to it:

```rust
async fn self_ref() {
    let data = vec![1, 2, 3];
    some_io(&data).await;  // &data must remain valid across suspension
    println!("{data:?}");
}
```

The state machine stores `data` and a pointer to `data`. If the future is moved to a new memory location, that pointer becomes **dangling**. `Pin` prevents this.

### What Pin Guarantees

`Pin<&mut T>` is a wrapper that **prevents moving** the value `T` once pinned. The future's `poll` method takes `self: Pin<&mut Self>`, guaranteeing the runtime won't move it between polls.

```
Memory layout of a self-referential future:

        ┌─────────────────────────┐
        │ state: Waiting          │
        │ data: [1, 2, 3]  ◄─┐   │
        │ data_ref: ──────────┘   │  ← internal pointer to data
        └─────────────────────────┘
                  ▲
                  │
            Pin prevents moving
            this to a new address
```

### Creating Pinned Futures

```rust
use tokio::pin;

// Stack pinning with tokio::pin!
let future = async { /* ... */ };
tokio::pin!(future);
// `future` is now Pin<&mut impl Future>, safe to poll

// Heap pinning with Box::pin
let future: Pin<Box<dyn Future<Output = ()>>> = Box::pin(async {
    // ...
});
```

Stack pinning is cheaper (no allocation) but the pinned value can't outlive the scope. Heap pinning allows the future to be moved as a `Pin<Box<...>>` (the box is moved, not the future inside it).

### Pin Rules

- Once a value is pinned, **it cannot be moved**.
- `Pin<&mut T>` only restricts types that are `!Unpin`. Most basic types implement `Unpin` and can be moved freely even when pinned.
- Async futures generated by the compiler are `!Unpin` because they may be self-referential.
- Use `tokio::pin!` for stack-pinning, `Box::pin()` for heap-pinning.

---

## Key Rules Summary

| Rule | Why |
|------|-----|
| Always store the latest `Waker` | The runtime may change wakers between polls |
| Return `Pending` only after registering a `Waker` | Otherwise the future will never be re-polled |
| Never move a pinned future | Self-referential pointers would dangle |
| `Waker` must be `Send + Sync` | I/O completions may arrive on different threads |
| Don't poll a future after it returns `Ready` | Behavior is undefined (may panic) |

---

## See Also

- [01-async-await.md](01-async-await.md) — High-level async/await usage and the `#[tokio::main]` macro.
- [03-cancellation.md](03-cancellation.md) — What happens when a future is dropped mid-execution.
