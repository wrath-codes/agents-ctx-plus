# document-store — Sub-Index

> Linear-hash document storage with WARC/CDX support and GraphQL (12 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [architecture](architecture/)

|file|description|
|---|---|
|[01-system-overview.md](architecture/01-system-overview.md)|System overview — components and design|
|[02-linear-hashing.md](architecture/02-linear-hashing.md)|Linear hashing — dynamic hash table, 4KB pages, load factor|
|[03-indexing.md](architecture/03-indexing.md)|Indexing — document lookup strategies|

### [data-formats](data-formats/)

|file|description|
|---|---|
|[01-warc-files.md](data-formats/01-warc-files.md)|WARC — Web ARChive format specification|
|[02-cdx-files.md](data-formats/02-cdx-files.md)|CDX — Capture inDeX format|

### [query-engine](query-engine/)

|file|description|
|---|---|
|[01-graphql-server.md](query-engine/01-graphql-server.md)|GraphQL — query server implementation|

### [experiments](experiments/)

|file|description|
|---|---|
|[01-performance-results.md](experiments/01-performance-results.md)|Performance — benchmarks and results|

### [challenges](challenges/)

|file|description|
|---|---|
|[01-implementation-challenges.md](challenges/01-implementation-challenges.md)|Challenges — implementation difficulties|

### [future-work](future-work/)

|file|description|
|---|---|
|[01-consistent-hashing.md](future-work/01-consistent-hashing.md)|Consistent hashing — distributed design|
|[02-improvements.md](future-work/02-improvements.md)|Improvements — planned enhancements|

---
*12 files · Related: [turso](../turso/INDEX.md)*
