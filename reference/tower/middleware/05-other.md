# Other Middleware

Tower includes additional middleware for filtering, load balancing, reconnection, steering, and more.

---

## Filter

Conditionally dispatch requests based on a predicate.

```rust
use tower::ServiceBuilder;

let service = ServiceBuilder::new()
    .filter(|req: &String| {
        if req.is_empty() {
            Err("empty request")
        } else {
            Ok(())
        }
    })
    .service(my_service);
```

There is also `filter_async` for async predicates:

```rust
let service = ServiceBuilder::new()
    .filter_async(|req: String| async move {
        if validate(&req).await {
            Ok(req)
        } else {
            Err("invalid request")
        }
    })
    .service(my_service);
```

Requires feature flag: `filter`

---

## Balance

Load-balance requests across a set of services using the Power of Two Random Choices (P2C) algorithm.

```rust
use tower::balance::p2c::Balance;
use tower::discover::ServiceList;

let services = vec![service_a, service_b, service_c];
let discover = ServiceList::new(services);
let balanced = Balance::new(discover);
```

Key types:

| Type | Description |
|------|-------------|
| `Balance<D, Req>` | P2C load balancer over a `Discover` stream |
| `Pool<Req, S>` | Auto-scaling pool that creates/destroys services |
| `p2c::Balance` | Power of Two Choices selection |

Requires feature flag: `balance`

---

## Hedge

Pre-emptively retry requests that have been outstanding longer than a latency percentile. Sends a second request if the first is "slow", returning whichever completes first.

```rust
use tower::hedge::{Hedge, Policy};

let service = Hedge::new(my_service, policy);
```

Requires feature flag: `hedge`

---

## Reconnect

Automatically reconnect to a service when it fails.

```rust
use tower::reconnect::Reconnect;

let service = Reconnect::new::<_, ()>(make_service, target);
```

Wraps a `MakeService` and lazily creates connections. If a call fails, the next `poll_ready` will attempt to create a new connection.

Requires feature flag: `reconnect`

---

## SpawnReady

Drive a service to readiness on a background Tokio task. This is useful when `poll_ready` involves I/O or other async work that you don't want to block the caller.

```rust
use tower::spawn_ready::SpawnReadyLayer;

let service = ServiceBuilder::new()
    .layer(SpawnReadyLayer::new())
    .service(my_service);
```

Requires feature flag: `spawn-ready`

---

## Steer

Route requests between multiple services based on a picker function.

```rust
use tower::steer::Steer;

let services = vec![service_a, service_b];
let router = Steer::new(
    services,
    |req: &MyRequest, _services: &[_]| {
        if req.is_priority() { 0 } else { 1 }
    },
);
```

Requires feature flag: `steer`

---

## ReadyCache

Maintains a cache of services, driving them to readiness and evicting those that fail.

```rust
use tower::ready_cache::ReadyCache;

let mut cache = ReadyCache::default();
cache.push("key1", service_a);
cache.push("key2", service_b);

// Poll to drive services to readiness
cache.poll_pending(cx);
```

Requires feature flag: `ready-cache`

---

## MakeService

A trait alias for services that produce other services (connection-level vs request-level):

```rust
pub trait MakeService<Target, Request>:
    Service<Target, Response = impl Service<Request>>
{ }
```

Used by Hyper's server to create a new service for each incoming connection.

Requires feature flag: `make`

---

## Summary

| Middleware | Feature | Description |
|-----------|---------|-------------|
| `Filter` | `filter` | Predicate-based request filtering |
| `AsyncFilter` | `filter` | Async predicate filtering |
| `Balance` | `balance` | P2C load balancing |
| `Hedge` | `hedge` | Latency hedging (speculative retry) |
| `Reconnect` | `reconnect` | Automatic reconnection |
| `SpawnReady` | `spawn-ready` | Background readiness driving |
| `Steer` | `steer` | Request routing between services |
| `ReadyCache` | `ready-cache` | Cache of ready services |
| `MakeService` | `make` | Connection-level service factory |

---

## See Also

- [Timeout](01-timeout.md) — per-request deadline
- [Rate Limit](02-rate-limit.md) — request rate control
- [Concurrency](04-concurrency.md) — concurrency, buffer, load shed
- [Composition Patterns](../patterns/01-composition.md) — combining middleware
