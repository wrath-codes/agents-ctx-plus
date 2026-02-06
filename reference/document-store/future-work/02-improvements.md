# Planned Improvements

## Overview

Several enhancements are planned to improve the document store's functionality, performance, and production readiness.

## Flexible Value Sizes

**Current limitation**: The document store only supports fixed-size values. All records in a page must have the same value size.

**Planned improvement**: Support variable-length values by:
- Using a length prefix before each value
- Implementing a free-space map within pages
- Handling records that span multiple pages for large values

This would allow storing documents of arbitrary size without padding or truncation.

## LRU Cache

**Current limitation**: The buffer pool uses a FIFO (First-In, First-Out) eviction strategy for pages.

**Planned improvement**: Replace FIFO with an LRU (Least Recently Used) cache to keep frequently accessed pages in memory longer. This would improve read performance for workloads with temporal locality (where recently accessed documents are likely to be accessed again).

Benefits:
- Hot pages stay in memory
- Reduced disk I/O for repeated reads
- Better performance for search engine workloads (popular pages accessed frequently)

## Record Deletion

**Current limitation**: Records cannot be deleted from the hash table once inserted.

**Planned improvement**: Implement record deletion with:
- Tombstone markers for deleted records
- Compaction to reclaim space from deleted records
- Updating the load factor calculation to account for deletions

## Custom Indexing

**Current limitation**: The only index is the linear hash table itself (key-based lookup).

**Planned improvement**: Support custom secondary indexes that allow querying by fields other than the primary key. For example:
- Index by document creation date
- Index by content type (MIME type)
- Index by domain name

## Admin Mutations via GraphQL

**Current limitation**: The GraphQL API only supports read queries.

**Planned improvement**: Add GraphQL mutation operations for administrative tasks:
- Creating and managing indexes
- Configuring bucket parameters
- Monitoring hash table statistics (load factor, bucket count, split history)
- Triggering manual compaction

## Production Performance Testing

**Current limitation**: Benchmarks were run in controlled conditions with synthetic data.

**Planned improvement**: Conduct performance testing under production-like conditions:
- Concurrent read/write workloads
- Large-scale datasets (millions of records)
- Network latency simulation
- Memory pressure scenarios
- Long-running stability tests

## Priority Summary

| Improvement | Impact | Complexity |
|------------|--------|------------|
| Flexible value sizes | High | Medium |
| LRU cache | High | Low |
| Record deletion | Medium | Medium |
| Custom indexing | Medium | High |
| Admin mutations | Low | Low |
| Production testing | High | Medium |

## Next Steps

- **[Consistent Hashing](./01-consistent-hashing.md)** - Distributed architecture design
- **[Linear Hashing](../architecture/02-linear-hashing.md)** - Current implementation details
- **[Performance Results](../experiments/01-performance-results.md)** - Current benchmark data
