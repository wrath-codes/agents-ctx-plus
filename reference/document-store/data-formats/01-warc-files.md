# WARC Files

## What is WARC?

WARC (WebARChive) is a file format used for storing web crawl data. It is the standard archival format used by the Internet Archive, Common Crawl, and other web archiving projects to preserve web pages along with their HTTP headers and metadata.

## Brief History

The WARC was preceded by the ARC file format, which the Internet Archive used to contain its collected web archives as far back as 1996. The ARC file was the Internet Archive's original container file for web-native resources. Reflecting the needs of web archivists around the world, the WARC standard was formalized in 2009 as ISO 28500. It was upgraded to version 1.1 in 2017 with added specificity and readability. The standard is maintained by the International Internet Preservation Consortium (IIPC).

## WARC Record Structure

Each WARC file contains multiple records. Each record consists of a header section and a content block (payload).

### Header Example

WARC record headers all start with `WARC/1.0` or `WARC/1.1`:

```text
WARC/1.0
WARC-Type: response
WARC-Target-URI: http://shop.kaze-online.de/images/products/small/PB0281.jpg
WARC-Date: 2011-02-25T18:32:18Z
WARC-Payload-Digest: sha1: QG6N6SOXUHFK2BGT6EEGMNOMALUWSYAE
WARC-IP-Address: 87.119.197.90
WARC-Record-ID: <urn:uuid:bf19c5b5-3756-4885-bc8b-78b76669c987>
Content-Type: application/http; msgtype=response
Content-Length: 3287

HTTP/1.1 200 OK
Date: Fri, 25 Feb 2011 18:32:18 GMT
Server: Apache/2.2.9 (Debian) PHP/5.2.6-1+lenny9 with Suhosin-Patch mod_ssl/2.2.9 OpenSSL/0.9.8g
Last-Modified: Wed, 12 May 2010 17:10:21 GMT
ETag: "56eb28-694-48668b78c7940"
Accept-Ranges: bytes
Content-Length: 2964
Connection: close
Content-Type: image/jpeg
```

### Record Types

| Type | Description |
|------|-------------|
| `warcinfo` | Metadata about the WARC file itself |
| `response` | Complete HTTP response (headers + body) |
| `request` | HTTP request that generated the response |
| `metadata` | Additional metadata about a record |
| `revisit` | Indicates content identical to a previous crawl |
| `resource` | Arbitrary resource data |
| `conversion` | Transformed version of another record |

## Reader/Writer Implementation

### WARC Reader

The document store includes a WARC reader that:

1. Opens compressed WARC files (gzip format)
2. Decompresses using the `libflate` crate [4]
3. Parses individual WARC records (header + payload)
4. Extracts key-value pairs for insertion into the hash table

### WARC Writer

The writer serializes records back to WARC format with proper headers and content length fields.

### Gzip Handling with libflate

WARC files from archive.org and commoncrawl.org are distributed in gzip-compressed format. The `libflate` crate provides pure-Rust gzip decompression:

```rust
use libflate::gzip::Decoder;
use std::io::Read;

fn decompress_warc(compressed_data: &[u8]) -> Vec<u8> {
    let mut decoder = Decoder::new(compressed_data).unwrap();
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();
    decompressed
}
```

Key characteristics:
- Each compressed WARC file is approximately **1GB**
- Each decompressed WARC file is approximately **5GB**
- A full file parse takes approximately **12 minutes**

### Data Sources

The implementation was tested with files from:

- **archive.org** - Internet Archive's Wayback Machine collections
- **commoncrawl.org** - Open crawl datasets (e.g., CC-MAIN-2021-04)

## File Size Characteristics

```text
Compressed (.warc.gz):   ~1 GB per file
Decompressed (.warc):    ~5 GB per file
Compression ratio:       ~5:1
Parse time (full file):  ~12 minutes
```

## Integration with Document Store

```text
WARC File (.warc.gz)
    │
    ▼
┌──────────────────┐
│  gzip decompress │  (libflate crate)
│  (.warc.gz → .warc)│
└────────┬─────────┘
         ▼
┌──────────────────┐
│  WARC Parser     │  (header + payload extraction)
│  (record by record)│
└────────┬─────────┘
         ▼
┌──────────────────┐
│  Key-Value       │  (URL → document content)
│  Extraction      │
└────────┬─────────┘
         ▼
┌──────────────────┐
│  Linear Hash     │  (insert into bucket)
│  Table           │
└──────────────────┘
```

## Performance

| Records Parsed | Time (seconds) |
|---------------|---------------|
| 10,000 | 71.52 |
| 20,000 | 148.09 |
| 50,000 | 432.87 |

See [Performance Results](../experiments/01-performance-results.md) for complete benchmarks.

## Next Steps

- **[CDX Files](./02-cdx-files.md)** - Index format for efficient WARC access
- **[System Overview](../architecture/01-system-overview.md)** - How WARC ingestion fits in the architecture
