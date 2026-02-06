# CDX Files

## What is CDX?

CDX (Capture/Crawl inDeX) is an index file format used alongside WARC files to enable efficient random access into large web archive collections. Instead of scanning an entire multi-gigabyte WARC file to find a specific record, a CDX file provides byte offsets that allow direct jumping to the desired location.

## How CDX Indexing Works

A CDX file is essentially a sorted index where each line represents one WARC record. The first line of the file is a header that specifies which fields are present, using single-letter codes.

```text
1. Parse the CDX header to determine field layout
2. Look up the desired URL in the sorted CDX entries
3. Read the byte offset and compressed length
4. Seek directly to that offset in the WARC file
5. Decompress and parse only the target record
```

This avoids the need to decompress and parse the entire WARC file (which can take ~12 minutes for a 5GB file).

## CDX Header Letter Meanings

Each letter in the CDX header line specifies a field in the index entries:

```text
A  canonized url
B  news group
C  rulespace category ***
D  compressed dat file offset
F  canonized frame
G  multi-column language description (* soon)
H  canonized host
I  canonized image
J  canonized jump point
K  Some weird FBIS what's changed kinda thing
L  canonized link
M  meta tags (AIF) *
N  massaged url
P  canonized path
Q  language string
R  canonized redirect
S  compressed record size
U  uniqueness ***
V  compressed arc file offset *
X  canonized url in other href tags
Y  canonized url in other src tags
Z  canonized url found in script
a  original url **
b  date **
c  old style checksum *
d  uncompressed dat file offset
e  IP **
f  frame *
g  file name
h  original host
i  image *
j  original jump point
k  new style checksum *
l  link *
m  mime type of original document
n  arc document length *
o  port
p  original path
r  redirect *
s  response code *
t  title *
v  uncompressed arc file offset *
x  url in other href tages
y  url in other src tags
z  url found in script *
#  comment
```

## CDX File Format Example

A CDX file typically looks like:

```text
 CDX A b a m s k r M S V g
http://example.com 20210101000000 http://example.com text/html 200 HASH - - 12345 67890 filename.warc.gz
http://example.org 20210101000100 http://example.org text/html 200 HASH - - 23456 78901 filename.warc.gz
```

Where the header ` CDX A b a m s k r M S V g` indicates:
- `A` - canonicalized URL
- `b` - date
- `a` - original URL
- `m` - mime type
- `s` - response code
- `k` - new style checksum
- `r` - redirect
- `M` - meta tags
- `S` - compressed arc file offset
- `V` - compressed arc file offset
- `g` - file name

## Key Fields for Document Store

The most important fields for the document store's use case:

| Letter | Field | Purpose |
|--------|-------|---------|
| `a` | Original URL | Key for document lookup |
| `S` | Compressed offset | Byte position in .warc.gz |
| `V` | Compressed offset | Alternative offset field |
| `n` | Document length | Size of the record |
| `g` | File name | Which WARC file contains the record |
| `m` | MIME type | Content type of the document |
| `s` | Response code | HTTP status code |

## Integration with WARC Reader

```text
CDX Index File                    WARC Archive File
┌─────────────────┐              ┌──────────────────┐
│ URL → offset    │──seek to───▶│ ... [record] ... │
│ URL → offset    │   offset     │                  │
│ URL → offset    │              │                  │
└─────────────────┘              └──────────────────┘

Without CDX: Sequential scan (~12 minutes for full file)
With CDX:    Direct seek (milliseconds per record)
```

## Next Steps

- **[WARC Files](./01-warc-files.md)** - WARC file structure and reader/writer
- **[System Overview](../architecture/01-system-overview.md)** - Architecture overview
