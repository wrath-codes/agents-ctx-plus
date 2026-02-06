# System Overview

## Architecture

The document store is a server-based application that returns documents based on keys provided in requests. It consists of several interconnected components: a linear hashing-based data store, a WARC file reader/writer for data ingestion, a CDX indexer for efficient lookups, and a GraphQL server for query handling.

## Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Yioop Search Engine                      │
│                      (HTTP Client)                           │
└──────────────────────────┬──────────────────────────────────┘
                           │ HTTP Request
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     GraphQL Server                           │
│              (Request Parsing + Response)                     │
└──────────────────────────┬──────────────────────────────────┘
                           │ Query Execution
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Document Store Core                        │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Linear Hash Table                         │  │
│  │                                                        │  │
│  │   Bucket 0      Bucket 1      Bucket 2     Bucket N    │  │
│  │   ┌──────┐      ┌──────┐      ┌──────┐     ┌──────┐   │  │
│  │   │Page 0│─────▶│Page 0│      │Page 0│     │Page 0│   │  │
│  │   │(4KB) │      │(4KB) │      │(4KB) │     │(4KB) │   │  │
│  │   └──┬───┘      └──────┘      └──┬───┘     └──────┘   │  │
│  │      │                           │                      │  │
│  │   ┌──▼───┐                    ┌──▼───┐                  │  │
│  │   │Page 1│                    │Page 1│   (overflow)     │  │
│  │   │(4KB) │                    │(4KB) │                  │  │
│  │   └──────┘                    └──────┘                  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Buffer Pool / Page I/O                     │  │
│  │         (Read/Write 4KB pages to/from disk)             │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────┬──────────────────────────────────┘
                           │ Data Feed
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Data Ingestion Layer                        │
│                                                              │
│  ┌──────────────────────┐    ┌────────────────────────────┐  │
│  │    WARC Reader       │    │      CDX Indexer           │  │
│  │  (gzip via libflate) │    │  (offset-based jumping)    │  │
│  │  archive.org feed    │    │  (letter-coded fields)     │  │
│  │  commoncrawl.org     │    │                            │  │
│  └──────────────────────┘    └────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

### Write Path (Ingestion)

1. WARC files are downloaded from archive.org or commoncrawl.org
2. The WARC reader decompresses gzip data using the `libflate` crate
3. Individual WARC records are parsed (header + payload)
4. Each record is assigned a key and stored in the linear hash table
5. The hash function determines the target bucket
6. The record is written to the appropriate page within that bucket
7. If the load factor exceeds 80%, a new bucket is added and records are redistributed

### Read Path (Retrieval)

1. A search engine (Yioop) sends an HTTP request to the GraphQL server
2. The GraphQL server parses the query and extracts the document key
3. The key is hashed to determine the bucket index
4. The bucket's linked list of pages is traversed
5. Each page is loaded from disk (4KB reads)
6. The record matching the key is returned via the GraphQL response

## Component Details

### Linear Hash Table

The core data structure. Uses dynamic hashing to grow the number of buckets as data is inserted, avoiding the need to rebuild the entire table. See [Linear Hashing](./02-linear-hashing.md) for full implementation details.

- Starts with N=2 buckets and I=1 bit
- Each bucket is a linked list of 4KB pages
- Bucket split triggered at 80% load factor
- Hash output is 32 bits; first I bits select the bucket

### Buffer Pool / Page I/O

All disk operations use a fixed 4KB page size. Pages are the unit of I/O:

- **Read**: Load a 4KB page from disk into memory
- **Write**: Modify page in memory, then flush to disk
- Pages contain a header (metadata) and a body (records)

### WARC Reader/Writer

Handles WebArchive files for data ingestion. Each compressed WARC file is approximately 1GB, decompressing to roughly 5GB. A full file parse takes approximately 12 minutes. See [WARC Files](../data-formats/01-warc-files.md).

### CDX Indexer

CDX files provide an index into WARC archives, allowing efficient jumping to specific byte offsets rather than scanning entire files. See [CDX Files](../data-formats/02-cdx-files.md).

### GraphQL Server

Handles HTTP requests from the Yioop search engine, executing queries against the document store and returning results. See [GraphQL Server](../query-engine/01-graphql-server.md).

## Storage Layout

```
Document Store on Disk
├── Hash Table Metadata
│   ├── Number of buckets (N)
│   ├── Number of index bits (I)
│   ├── Split pointer (S)
│   └── Item count
├── Bucket Files
│   ├── Bucket 0: [Page 0] → [Page 1] → ...
│   ├── Bucket 1: [Page 0] → ...
│   ├── Bucket 2: [Page 0] → [Page 1] → [Page 2] → ...
│   └── ...
└── Each Page (4KB)
    ├── Header (metadata: record count, next page pointer)
    └── Body (key-value records)
```

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | Memory safety, no GC, C-level performance |
| Hash Table | Linear hashing | Dynamic growth without full rehash |
| Page Size | 4KB | Matches OS page size, efficient disk I/O |
| Value Size | Fixed | Simplifies page layout (flexible sizes planned) |
| API | GraphQL | Flexible querying, integrates with Yioop |
| Compression | gzip (libflate) | Standard for WARC files |

## Next Steps

- **[Linear Hashing](./02-linear-hashing.md)** - Algorithm and implementation details
- **[Indexing](./03-indexing.md)** - PackedTableTools packing format
- **[WARC Files](../data-formats/01-warc-files.md)** - Data ingestion from web archives
