# Consistent Hashing for Distributed Document Store

## Overview

The current document store runs on a single node. To scale horizontally, a distributed architecture using consistent hashing is proposed. This would allow the document store to span multiple servers while minimizing data redistribution when servers are added or removed.

## Consistent Hashing Concept

### Hash Function: xxhash

The proposed implementation uses **xxhash**, a fast non-cryptographic hash function, for mapping both keys and server nodes to positions on a hash ring.

### Circle/Ring Mapping

Consistent hashing maps both keys and servers onto a circular hash space (ring):

```text
                    0 / 2^32
                      │
              ┌───────┼───────┐
          S3 ●│               │● S1
             │                │
             │    Hash Ring   │
             │   (0 to 2^32) │
          K2 ○│               │○ K1
              └───────┼───────┘
                      │
                   S2 ●
                   K3 ○
```

- **● Server nodes** are hashed to positions on the ring
- **○ Keys** are hashed to positions on the ring

### Key Assignment

Each key is assigned to the **nearest server node** in the clockwise direction on the ring:

```text
Key K1 → assigned to Server S2 (next clockwise)
Key K2 → assigned to Server S3 (next clockwise)
Key K3 → assigned to Server S1 (next clockwise)
```

This means each server is responsible for all keys between itself and the previous server on the ring.

## Server Addition

When a new server is added to the ring, only a fraction of keys need to be redistributed:

```text
Before: S1, S2, S3 on the ring
After:  S1, S2, S3, S4 added between S1 and S2

Only keys between S1 and S4 need to move from S2 to S4.
All other keys remain on their current servers.
```

### Redistribution Formula

When a server is added, the expected fraction of keys that need to be redistributed is:

```text
K / N

Where:
  K = total number of keys
  N = total number of servers (after addition)
```

This is significantly better than naive hashing, where adding a server would require redistributing nearly all keys.

## Server Removal

When a server is removed, its keys are redistributed to the next clockwise server:

```text
Before: S1, S2, S3 on the ring
After:  S2 removed

All keys that were on S2 move to S3 (next clockwise from S2's position).
All other keys remain unchanged.
```

The same K/N formula applies - only 1/N of the total keys are affected.

## Virtual Nodes

To improve load distribution, each physical server can be mapped to multiple positions on the ring (virtual nodes):

```text
Physical Server S1 → Virtual nodes: S1_0, S1_1, S1_2, ...
Physical Server S2 → Virtual nodes: S2_0, S2_1, S2_2, ...

More virtual nodes = more even distribution of keys
```

## Architecture Vision

```text
┌─────────────────────────────────────────────┐
│              Client (Yioop)                 │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│           Routing Layer                      │
│    (xxhash key → find server on ring)        │
└──────┬──────────┬──────────┬────────────────┘
       │          │          │
       ▼          ▼          ▼
┌──────────┐ ┌──────────┐ ┌──────────┐
│ Server 1 │ │ Server 2 │ │ Server 3 │
│ (Linear  │ │ (Linear  │ │ (Linear  │
│  Hash    │ │  Hash    │ │  Hash    │
│  Table)  │ │  Table)  │ │  Table)  │
└──────────┘ └──────────┘ └──────────┘
```

Each server runs its own local linear hash table, and the consistent hashing ring determines which server handles each key.

## Advantages Over Naive Distribution

| Aspect | Naive Hashing | Consistent Hashing |
|--------|--------------|-------------------|
| Server addition | ~100% keys redistributed | K/N keys redistributed |
| Server removal | ~100% keys redistributed | K/N keys redistributed |
| Load balance | Uniform (with good hash) | Uniform (with virtual nodes) |
| Complexity | Simple | Moderate |

## Next Steps

- **[Planned Improvements](./02-improvements.md)** - Other future enhancements
- **[Linear Hashing](../architecture/02-linear-hashing.md)** - Current single-node hash table
- **[System Overview](../architecture/01-system-overview.md)** - Current architecture
