# document-store — Sub-Index

> Linear-hash document storage with WARC/CDX support and GraphQL (11 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Architecture Overview](README.md#architecture-overview) · [Performance Highlights](README.md#performance-highlights) · [Documentation Map](README.md#documentation-map) · [Quick Links](README.md#quick-links) · [Citation](README.md#citation)|

### [architecture](architecture/)

|file|description|
|---|---|
|[01-system-overview.md](architecture/01-system-overview.md)|System overview — components and design|
| |↳ [Architecture](architecture/01-system-overview.md#architecture) · [Component Diagram](architecture/01-system-overview.md#component-diagram) · [Data Flow](architecture/01-system-overview.md#data-flow) · [Component Details](architecture/01-system-overview.md#component-details) · [Storage Layout](architecture/01-system-overview.md#storage-layout) · [Design Decisions](architecture/01-system-overview.md#design-decisions) · [Next Steps](architecture/01-system-overview.md#next-steps)|
|[02-linear-hashing.md](architecture/02-linear-hashing.md)|Linear hashing — dynamic hash table, 4KB pages, load factor|
| |↳ [How It Works](architecture/02-linear-hashing.md#how-it-works) · [Implementation](architecture/02-linear-hashing.md#implementation) · [Pages and Buffer Pool](architecture/02-linear-hashing.md#pages-and-buffer-pool) · [Buckets](architecture/02-linear-hashing.md#buckets) · [Performance Characteristics](architecture/02-linear-hashing.md#performance-characteristics) · [Key Parameters](architecture/02-linear-hashing.md#key-parameters) · [Next Steps](architecture/02-linear-hashing.md#next-steps)|
|[03-indexing.md](architecture/03-indexing.md)|Indexing — document lookup strategies|
| |↳ [Packing Format](architecture/03-indexing.md#packing-format) · [Data Types Summary](architecture/03-indexing.md#data-types-summary) · [Packing Process](architecture/03-indexing.md#packing-process) · [Unpacking Process](architecture/03-indexing.md#unpacking-process) · [Design Rationale](architecture/03-indexing.md#design-rationale) · [Next Steps](architecture/03-indexing.md#next-steps)|

### [data-formats](data-formats/)

|file|description|
|---|---|
|[01-warc-files.md](data-formats/01-warc-files.md)|WARC — Web ARChive format specification|
| |↳ [What is WARC?](data-formats/01-warc-files.md#what-is-warc) · [Brief History](data-formats/01-warc-files.md#brief-history) · [WARC Record Structure](data-formats/01-warc-files.md#warc-record-structure) · [Reader/Writer Implementation](data-formats/01-warc-files.md#readerwriter-implementation) · [File Size Characteristics](data-formats/01-warc-files.md#file-size-characteristics) · [Integration with Document Store](data-formats/01-warc-files.md#integration-with-document-store) · [Performance](data-formats/01-warc-files.md#performance) · [Next Steps](data-formats/01-warc-files.md#next-steps)|
|[02-cdx-files.md](data-formats/02-cdx-files.md)|CDX — Capture inDeX format|
| |↳ [What is CDX?](data-formats/02-cdx-files.md#what-is-cdx) · [How CDX Indexing Works](data-formats/02-cdx-files.md#how-cdx-indexing-works) · [CDX Header Letter Meanings](data-formats/02-cdx-files.md#cdx-header-letter-meanings) · [CDX File Format Example](data-formats/02-cdx-files.md#cdx-file-format-example) · [Key Fields for Document Store](data-formats/02-cdx-files.md#key-fields-for-document-store) · [Integration with WARC Reader](data-formats/02-cdx-files.md#integration-with-warc-reader) · [Next Steps](data-formats/02-cdx-files.md#next-steps)|

### [query-engine](query-engine/)

|file|description|
|---|---|
|[01-graphql-server.md](query-engine/01-graphql-server.md)|GraphQL — query server implementation|
| |↳ [Purpose](query-engine/01-graphql-server.md#purpose) · [How It Works](query-engine/01-graphql-server.md#how-it-works) · [Integration with Yioop](query-engine/01-graphql-server.md#integration-with-yioop) · [Future Enhancements](query-engine/01-graphql-server.md#future-enhancements) · [Next Steps](query-engine/01-graphql-server.md#next-steps)|

### [experiments](experiments/)

|file|description|
|---|---|
|[01-performance-results.md](experiments/01-performance-results.md)|Performance — benchmarks and results|
| |↳ [Table 1: Insertion Times by Key-Value Size](experiments/01-performance-results.md#table-1-insertion-times-by-key-value-size) · [Table 2: Retrieval Times by Key-Value Size](experiments/01-performance-results.md#table-2-retrieval-times-by-key-value-size) · [Table 3: Rust vs JavaScript Comparison](experiments/01-performance-results.md#table-3-rust-vs-javascript-comparison) · [Table 4: Impact of Initial Bucket Count](experiments/01-performance-results.md#table-4-impact-of-initial-bucket-count) · [Table 5: WARC Parsing Performance](experiments/01-performance-results.md#table-5-warc-parsing-performance) · [Key Findings](experiments/01-performance-results.md#key-findings) · [Summary](experiments/01-performance-results.md#summary) · [Next Steps](experiments/01-performance-results.md#next-steps)|

### [challenges](challenges/)

|file|description|
|---|---|
|[01-implementation-challenges.md](challenges/01-implementation-challenges.md)|Challenges — implementation difficulties|
| |↳ [Limited Rust Web Support](challenges/01-implementation-challenges.md#limited-rust-web-support) · [Strongly vs Loosely Typed Language Porting](challenges/01-implementation-challenges.md#strongly-vs-loosely-typed-language-porting) · [Trade-offs](challenges/01-implementation-challenges.md#trade-offs) · [Next Steps](challenges/01-implementation-challenges.md#next-steps)|

### [future-work](future-work/)

|file|description|
|---|---|
|[01-consistent-hashing.md](future-work/01-consistent-hashing.md)|Consistent hashing — distributed design|
| |↳ [Consistent Hashing Concept](future-work/01-consistent-hashing.md#consistent-hashing-concept) · [Server Addition](future-work/01-consistent-hashing.md#server-addition) · [Server Removal](future-work/01-consistent-hashing.md#server-removal) · [Virtual Nodes](future-work/01-consistent-hashing.md#virtual-nodes) · [Architecture Vision](future-work/01-consistent-hashing.md#architecture-vision) · [Advantages Over Naive Distribution](future-work/01-consistent-hashing.md#advantages-over-naive-distribution) · [Next Steps](future-work/01-consistent-hashing.md#next-steps)|
|[02-improvements.md](future-work/02-improvements.md)|Improvements — planned enhancements|
| |↳ [Flexible Value Sizes](future-work/02-improvements.md#flexible-value-sizes) · [LRU Cache](future-work/02-improvements.md#lru-cache) · [Record Deletion](future-work/02-improvements.md#record-deletion) · [Custom Indexing](future-work/02-improvements.md#custom-indexing) · [Admin Mutations via GraphQL](future-work/02-improvements.md#admin-mutations-via-graphql) · [Production Performance Testing](future-work/02-improvements.md#production-performance-testing) · [Priority Summary](future-work/02-improvements.md#priority-summary) · [Next Steps](future-work/02-improvements.md#next-steps)|

---
*11 files · Related: [turso](../turso/INDEX.md)*
