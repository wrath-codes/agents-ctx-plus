# Linear Hashing

## Overview

Linear hashing is a dynamic hashing scheme that allows the hash table to grow incrementally without requiring a full rebuild. Unlike static hashing where the number of buckets is fixed, linear hashing adds buckets one at a time as the load factor exceeds a threshold.

## How It Works

### Initial State

- **N** = 2 (number of buckets)
- **I** = 1 (number of index bits used from the 32-bit hash output)
- **S** = 0 (split pointer, indicates which bucket to split next)

### Hash Function

The key is hashed to produce a 32-bit output. The first **I** bits of this output determine which bucket the record belongs to.

```text
Key → Hash Function → 32-bit output → Take first I bits → Bucket index
```

### Load Factor

The load factor determines when the hash table should grow:

```text
Load Factor = number of items / (number of buckets × average items in a bucket)
```

When the load factor exceeds **80%**, a new bucket is added and the bucket at index **S** is split.

### Bucket Splitting

When a split is triggered:

1. A new bucket is added at the end of the table
2. The bucket at split pointer **S** is rehashed
3. Records that now map to the new bucket are moved
4. **S** is incremented
5. When **S** reaches **N**, **N** doubles, **S** resets to 0, and **I** is incremented

The rule for incrementing **I**: when **N > (2^I - 1)**, increment **I** by 1.

### Growth Example

```text
Step 0: N=2, I=1, S=0
  Bucket 0: [records where hash bit 0 = 0]
  Bucket 1: [records where hash bit 0 = 1]

Step 1: Load factor > 80%, split bucket 0
  Bucket 0: [rehashed records]
  Bucket 1: [unchanged]
  Bucket 2: [records moved from bucket 0]  ← NEW
  N=3, I=2, S=1

Step 2: Load factor > 80%, split bucket 1
  Bucket 0: [unchanged]
  Bucket 1: [rehashed records]
  Bucket 2: [unchanged]
  Bucket 3: [records moved from bucket 1]  ← NEW
  N=4, I=2, S=0  ← S resets, N doubled
```

## Implementation

### Algorithm Flow

```text
INSERT(key, value):
  1. Hash the key → 32-bit hash
  2. Take first I bits → bucket_index
  3. If bucket_index < S, take first (I+1) bits instead
  4. Find the bucket at bucket_index
  5. Bucket is a linked list of pages
  6. Find the right page (or create overflow page)
  7. Fetch page from disk (4KB read)
  8. Modify page in memory (insert record)
  9. Save page to disk (4KB write)
  10. Update item count
  11. If load factor > 80%, split bucket at S

RETRIEVE(key):
  1. Hash the key → 32-bit hash
  2. Take first I bits → bucket_index
  3. If bucket_index < S, take first (I+1) bits instead
  4. Find the bucket at bucket_index
  5. Traverse linked list of pages
  6. For each page, search for matching key
  7. Return value if found
```

## Pages and Buffer Pool

Each bucket is a linked list of pages. Each page is **4KB** and contains a header and a body of records. When we want to read or write some chunk of bytes in a page, we read the page into memory, make changes on the in-memory copy, then save the updated page to disk.

Pages are buffered in memory as long as possible rather than flushed after each operation (buffer pool).

### Page Struct

```rust
pub struct Page {
    pub id: usize,
    pub storage: [u8; PAGE_SIZE],
    pub num_records: usize,
    pub next: Option<usize>,
    pub dirty: bool,
    keysize: usize,
    valsize: usize,
}
```

Fields:
- `id` — which page in the file this page corresponds to
- `storage` — all bytes in the page copied to a byte array
- `num_records` — how many records this page contains
- `next` — links overflow pages in a linked list; also tracks freed overflow pages
- `dirty` — whether the in-memory page is out of sync with disk
- `keysize` / `valsize` — the fixed byte length of keys and values in stored records

### Reading and Writing Headers

The metadata is stored within the page itself. The first 16 bytes hold `num_records` (bytes 0–7) and `next` (bytes 8–15):

```rust
pub fn read_header(&mut self) {
    let num_records: usize = bytearray_to_usize(&self.storage[0..8].to_vec());
    let next_usize: usize = bytearray_to_usize(&self.storage[8..16].to_vec());
    self.num_records = num_records;
    self.next = if next_usize != 0 {
        Some(next_usize)
    } else {
        None
    };
}

pub fn write_header(&mut self) {
    mem_move(&mut self.storage[0..8], &usize_to_bytearray(self.num_records));
    mem_move(&mut self.storage[8..16], &usize_to_bytearray(self.next.unwrap_or(0)));
}
```

### Reading and Writing Records

Records are stored in the body portion of each page after the header. The current implementation supports **fixed-size values only**.

```rust
pub fn read_record(&mut self, row_num: usize) -> (&[u8], &[u8]) {
    let offsets: RowOffsets = self.compute_offsets(row_num);
    let key: &[u8] = &self.storage[offsets.key_offset..offsets.val_offset];
    let val: &[u8] = &self.storage[offsets.val_offset..offsets.row_end];
    (key, val)
}

pub fn write_record(&mut self, row_num: usize, key: &[u8], val: &[u8]) {
    let offsets: RowOffsets = self.compute_offsets(row_num);
    mem_move(&mut self.storage[offsets.key_offset..offsets.val_offset], key);
    mem_move(&mut self.storage[offsets.val_offset..offsets.row_end], val);
}
```

## Buckets

Each bucket is a linked list of pages. When a page is full, an overflow page is allocated and linked. The search traverses pages until it finds the matching key or exhausts the list.

### SearchResult Struct

The search result indicates where a record was found, or where it should be inserted:

```rust
pub struct SearchResult {
    pub page_id: Option<usize>,
    pub row_num: Option<usize>,
    pub val: Option<Vec<u8>>,
}
```

- If the key is found, `page_id` and `row_num` point to the record and `val` contains the value
- If the key is not found but there is space, `page_id` and `row_num` indicate the insertion point
- If there is no space, `page_id` points to the last page (used to create an overflow page)

### Searching a Bucket

```rust
pub fn search_bucket(&mut self, bucket_id: usize, key: &[u8]) -> SearchResult {
    let mut page_id: usize = self.bucket_to_page(bucket_id);
    let mut buffer_index: usize;
    let mut first_free_row = SearchResult {
        page_id: None,
        row_num: None,
        val: None,
    };
    loop {
        buffer_index = self.fetch_page(page_id);
        let next_page: Option<usize> = self.buffers[buffer_index].next;
        let page_records: Vec<(Vec<u8>, Vec<u8>)> = self.all_records_in_page(page_id);
        let len: usize = page_records.len();

        for (row_num, (k, v)) in page_records.into_iter().enumerate() {
            if slices_eq(&k, key) {
                return SearchResult {
                    page_id: Some(page_id),
                    row_num: Some(row_num),
                    val: Some(v),
                };
            }
        }

        let row_num: Option<usize> = if len < self.records_per_page {
            Some(len)
        } else {
            None
        };

        match (first_free_row.page_id, first_free_row.row_num) {
            (None, _) => {
                first_free_row = SearchResult {
                    page_id: Some(page_id),
                    row_num: row_num,
                    val: None,
                };
            }
            _ => (),
        }

        if let Some(p) = next_page {
            page_id = p;
        } else {
            break;
        }
    }
    first_free_row
}
```

## Performance Characteristics

### Strengths

- **O(1) average** lookup time — direct bucket access via hash
- **Incremental growth** — no full rehash needed, only one bucket splits at a time
- **Predictable I/O** — each page read is exactly 4KB
- **Low memory overhead** — only active pages need to be in memory

### Impact of Initial Bucket Count

Starting with more buckets reduces the number of splits needed:

| Initial Buckets | 100K Insertions (seconds) |
|----------------|--------------------------|
| 2 | 294.19 |
| 256 | 195.34 |
| 1024 | 156.72 |

### Limitations

- **Fixed-size values only** — current implementation does not support variable-length values
- **No deletion** — records cannot be removed (planned for future work)
- **FIFO page replacement** — no LRU cache (planned improvement)
- **Single-node only** — not distributed (consistent hashing planned)

## Key Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Page size | 4KB | Fixed page size for all I/O |
| Initial N | 2 | Starting number of buckets |
| Initial I | 1 | Starting number of index bits |
| Split threshold | 80% | Load factor triggering bucket split |
| Hash output | 32 bits | Full hash output before bit extraction |

## Next Steps

- **[Indexing](./03-indexing.md)** — PackedTableTools packing format
- **[System Overview](./01-system-overview.md)** — Full architecture diagram
- **[Performance Results](../experiments/01-performance-results.md)** — Complete benchmark data
