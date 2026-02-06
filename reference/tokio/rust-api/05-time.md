# Time Utilities

The `tokio::time` module provides async-aware time utilities for sleeping, creating intervals, and applying timeouts to futures. All time types integrate with Tokio's timer system and require a Tokio runtime.

When using the `test-util` feature, time can be paused and advanced programmatically, enabling deterministic testing of time-dependent logic without real wall-clock delays.

---

## API Reference

### Free Functions

```rust
pub async fn sleep(duration: Duration) -> ()
pub async fn sleep_until(deadline: Instant) -> ()
pub fn interval(period: Duration) -> Interval
pub fn interval_at(start: Instant, period: Duration) -> Interval
pub async fn timeout<F: Future>(duration: Duration, future: F) -> Result<F::Output, Elapsed>
pub async fn timeout_at<F: Future>(deadline: Instant, future: F) -> Result<F::Output, Elapsed>
```

### Sleep

```rust
pub struct Sleep { /* ... */ }

impl Future for Sleep {
    type Output = ();
}

impl Sleep {
    pub fn deadline(&self) -> Instant
    pub fn is_elapsed(&self) -> bool
    pub fn reset(self: Pin<&mut Self>, deadline: Instant)
}
```

### Interval

```rust
pub struct Interval { /* ... */ }

impl Interval {
    pub async fn tick(&mut self) -> Instant
    pub fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Instant>
    pub fn reset(&mut self)
    pub fn reset_immediately(&mut self)
    pub fn reset_after(&mut self, after: Duration)
    pub fn reset_at(&mut self, deadline: Instant)
    pub fn period(&self) -> Duration
    pub fn missed_tick_behavior(&self) -> MissedTickBehavior
    pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior)
}
```

### Timeout

```rust
pub struct Timeout<T> { /* ... */ }

impl<T: Future> Future for Timeout<T> {
    type Output = Result<T::Output, Elapsed>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elapsed;

impl std::fmt::Display for Elapsed { /* ... */ }
impl std::error::Error for Elapsed { /* ... */ }
```

### MissedTickBehavior

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedTickBehavior {
    Burst,
    Delay,
    Skip,
}

impl Default for MissedTickBehavior {
    fn default() -> Self { MissedTickBehavior::Burst }
}
```

### Instant

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant { /* ... */ }

impl Instant {
    pub fn now() -> Self
    pub fn from_std(std: std::time::Instant) -> Self
    pub fn into_std(self) -> std::time::Instant
    pub fn elapsed(&self) -> Duration
    pub fn duration_since(&self, earlier: Instant) -> Duration
    pub fn checked_add(&self, duration: Duration) -> Option<Instant>
    pub fn checked_sub(&self, duration: Duration) -> Option<Instant>
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration
}

impl Add<Duration> for Instant { type Output = Instant; }
impl Sub<Duration> for Instant { type Output = Instant; }
impl Sub<Instant> for Instant { type Output = Duration; }
```

---

## `sleep(duration)`

Returns a `Sleep` future that completes after the given `Duration` has elapsed. This is the async equivalent of `std::thread::sleep` — it yields control back to the runtime instead of blocking the thread.

```rust
use tokio::time::{sleep, Duration};

sleep(Duration::from_millis(100)).await;
println!("100ms have elapsed");
```

## `sleep_until(deadline)`

Returns a `Sleep` future that completes at the specified `Instant`. Useful when you have an absolute deadline rather than a relative duration.

```rust
use tokio::time::{sleep_until, Instant, Duration};

let deadline = Instant::now() + Duration::from_secs(5);
sleep_until(deadline).await;
```

## `interval(period)`

Creates an `Interval` that yields on a regular cadence. The **first tick completes immediately**, then subsequent ticks occur every `period`.

### Panics

Panics if `period` is zero.

```rust
use tokio::time::{interval, Duration};

let mut interval = interval(Duration::from_millis(500));

loop {
    interval.tick().await; // first tick returns immediately
    println!("tick at {:?}", std::time::Instant::now());
}
```

## `interval_at(start, period)`

Like `interval` but the first tick fires at the specified `start` instant rather than immediately.

```rust
use tokio::time::{interval_at, Instant, Duration};

let start = Instant::now() + Duration::from_secs(1);
let mut interval = interval_at(start, Duration::from_secs(2));

loop {
    interval.tick().await; // first tick at start, then every 2s
    do_work().await;
}
```

## `timeout(duration, future)`

Wraps a future with a deadline. If the inner future completes before `duration` elapses, returns `Ok(output)`. Otherwise, cancels the inner future and returns `Err(Elapsed)`.

```rust
use tokio::time::{timeout, Duration};

match timeout(Duration::from_secs(5), long_running_task()).await {
    Ok(result) => println!("completed: {:?}", result),
    Err(_) => println!("timed out"),
}
```

## `timeout_at(deadline, future)`

Like `timeout` but accepts an absolute `Instant` deadline instead of a relative duration.

```rust
use tokio::time::{timeout_at, Instant, Duration};

let deadline = Instant::now() + Duration::from_secs(10);
let result = timeout_at(deadline, some_async_work()).await;
```

---

## MissedTickBehavior

Controls how `Interval` handles ticks that were missed because the task took longer than one period to process a tick.

| Variant | Behavior |
|---------|----------|
| `Burst` (default) | Fires missed ticks as fast as possible to "catch up", then resumes normal cadence. |
| `Delay` | Resets the interval clock from the current time. Next tick is one full `period` from now. Missed ticks are lost. |
| `Skip` | Skips missed ticks entirely and fires at the next aligned tick boundary. Maintains the original cadence alignment. |

```rust
use tokio::time::{interval, Duration, MissedTickBehavior};

let mut interval = interval(Duration::from_secs(1));
interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

loop {
    interval.tick().await;
    // If this takes >1s, missed ticks are skipped
    expensive_work().await;
}
```

---

## Instant

`tokio::time::Instant` wraps `std::time::Instant` and integrates with Tokio's test time system. When time is paused (via `tokio::time::pause()`), `Instant::now()` returns the mocked time instead of the real wall clock.

In production code, `tokio::time::Instant` behaves identically to `std::time::Instant`.

---

## Test Utilities

Requires the `test-util` feature flag on the `tokio` crate.

```rust
pub fn pause()
pub fn resume()
pub async fn advance(duration: Duration)
```

### `pause()`

Freezes time. After this call, `Instant::now()` returns a frozen value and all timer futures (sleep, interval, timeout) only advance when `advance()` is called.

### `resume()`

Resumes real-time clock. Timer futures revert to wall-clock behavior.

### `advance(duration)`

Moves mocked time forward by `duration`. All timers that would fire during this window are resolved. Must be called after `pause()`.

```rust
use tokio::time::{self, sleep, Duration, Instant};

#[tokio::test]
async fn test_with_paused_time() {
    time::pause();

    let start = Instant::now();
    sleep(Duration::from_secs(60)).await;

    // Only microseconds of real time elapsed, but mocked time advanced
    assert!(start.elapsed() >= Duration::from_secs(60));
}
```

```rust
use tokio::time::{self, sleep, Duration};

#[tokio::test]
async fn test_explicit_advance() {
    time::pause();

    let handle = tokio::spawn(async {
        sleep(Duration::from_secs(10)).await;
        42
    });

    time::advance(Duration::from_secs(10)).await;

    let result = handle.await.unwrap();
    assert_eq!(result, 42);
}
```

---

## Examples

### Periodic Background Work

```rust
use tokio::time::{interval, Duration, MissedTickBehavior};

async fn run_periodic_cleanup(db: Database) {
    let mut interval = interval(Duration::from_secs(60));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        interval.tick().await;
        if let Err(e) = db.cleanup_expired_sessions().await {
            eprintln!("cleanup error: {}", e);
        }
    }
}
```

### Timeout with Fallback

```rust
use tokio::time::{timeout, Duration};

async fn fetch_with_fallback(url: &str) -> String {
    match timeout(Duration::from_secs(3), fetch(url)).await {
        Ok(Ok(body)) => body,
        Ok(Err(e)) => {
            eprintln!("fetch error: {}", e);
            String::from("default")
        }
        Err(_elapsed) => {
            eprintln!("request timed out");
            String::from("default")
        }
    }
}
```

### Resettable Sleep (Debounce Pattern)

```rust
use tokio::time::{sleep, Duration, Instant};
use tokio::pin;

async fn debounce(mut rx: tokio::sync::mpsc::Receiver<()>) {
    let timeout = Duration::from_millis(300);
    let sleep = sleep(timeout);
    pin!(sleep);

    loop {
        tokio::select! {
            _ = &mut sleep => {
                println!("debounced action triggered");
                sleep.as_mut().reset(Instant::now() + timeout);
            }
            msg = rx.recv() => {
                match msg {
                    Some(()) => {
                        // Reset the timer on each incoming event
                        sleep.as_mut().reset(Instant::now() + timeout);
                    }
                    None => break,
                }
            }
        }
    }
}
```

### Nested Timeout

```rust
use tokio::time::{timeout, Duration};

async fn connect_and_query() -> Result<String, Box<dyn std::error::Error>> {
    // Overall deadline for the entire operation
    let result = timeout(Duration::from_secs(30), async {
        // Sub-deadline for connection
        let conn = timeout(Duration::from_secs(5), establish_connection())
            .await
            .map_err(|_| "connection timed out")??;

        // Sub-deadline for query
        let data = timeout(Duration::from_secs(10), conn.query("SELECT 1"))
            .await
            .map_err(|_| "query timed out")??;

        Ok(data)
    })
    .await
    .map_err(|_| "overall operation timed out")?;

    result
}
```

---

## See Also

- [Synchronization Primitives](06-sync.md) — channels, mutexes, and other sync types
- [Macros](08-macros.md) — `select!` for combining timers with other futures
