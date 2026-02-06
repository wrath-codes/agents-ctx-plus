# Framing

## Overview

Framing is the process of converting a byte stream into a sequence of **frames** — discrete units of data that your protocol understands. Raw TCP provides an ordered byte stream with no message boundaries; framing adds that structure.

## Redis Frame Type

A Redis frame represents one unit of the RESP protocol:

```rust
use bytes::Bytes;

enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}
```

## Connection Struct

Wraps a `TcpStream` with read and write buffers to handle framing:

```rust
use bytes::BytesMut;
use tokio::io::BufWriter;
use tokio::net::TcpStream;

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(4 * 1024),
        }
    }
}
```

## Buffered Reads

### `read_frame()` — The Read Loop

The core pattern: try to parse a frame from the buffer, read more data if incomplete, handle EOF.

```rust
use tokio::io::AsyncReadExt;
use bytes::Buf;
use mini_redis::frame::Frame;
use std::io::Cursor;

impl Connection {
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            // Step 1: Try to parse a frame from buffered data
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // Step 2: Read more data from the socket
            // read_buf auto-advances the BytesMut cursor
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                // EOF: Ok(0) bytes read
                if self.buffer.is_empty() {
                    return Ok(None); // Clean close
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }
}
```

### `read_buf()` vs `read()`

| Method | Buffer Type | Cursor Tracking | Uninitialized Memory |
|--------|-------------|-----------------|----------------------|
| `read(&mut [u8])` | Byte slice | Manual | Must zero-initialize |
| `read_buf(&mut BufMut)` | `BufMut` trait | Automatic | Safe optimization, skips zeroing |

`read_buf()` takes a `BufMut` implementor (like `BytesMut`) and automatically advances the internal cursor by the number of bytes read.

### `Buf` Trait — Consuming Parsed Data

After parsing a frame, advance past the consumed bytes:

```rust
use bytes::Buf;
use std::io::Cursor;

fn parse_frame(&mut self) -> Result<Option<Frame>> {
    let mut buf = Cursor::new(&self.buffer[..]);

    match Frame::check(&mut buf) {
        Ok(_) => {
            // Frame::check succeeded — we know the length
            let len = buf.position() as usize;

            // Reset cursor for parsing
            buf.set_position(0);
            let frame = Frame::parse(&mut buf)?;

            // Consume the parsed bytes from the buffer
            self.buffer.advance(len);

            Ok(Some(frame))
        }
        // Not enough data yet
        Err(Incomplete) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
```

## Parsing: Two-Step Check + Parse

Parsing uses `std::io::Cursor<&[u8]>` which implements `Buf`:

1. **`Frame::check()`** — validates enough data exists for a complete frame, advances cursor past it, but doesn't allocate or construct anything.
2. **`Frame::parse()`** — constructs the `Frame` value from validated data.

This two-step approach lets you determine the frame length before committing to allocation.

## Buffered Writes

### `BufWriter<TcpStream>`

Wraps the stream to batch small writes into fewer syscalls. Without it, each `write()` call would be a separate syscall.

```rust
use tokio::io::BufWriter;

let stream = BufWriter::new(tcp_stream);
```

### `write_frame()` — Encoding

```rust
use tokio::io::AsyncWriteExt;

impl Connection {
    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Bulk(val) => {
                let len = val.len();
                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }
            Frame::Array(_val) => unimplemented!(),
        }

        // Flush the BufWriter to ensure data reaches the socket
        self.stream.flush().await?;

        Ok(())
    }
}
```

`flush()` at the end is critical — without it, data may sit in the `BufWriter` buffer and never reach the socket.

## bytes Crate Integration

The `bytes` crate provides efficient byte buffer types used throughout Tokio:

| Type | Description | Key Trait |
|------|-------------|-----------|
| `Bytes` | Immutable bytes, cheap `clone()` via ref-counting | `Buf` |
| `BytesMut` | Mutable byte buffer, grows as needed | `BufMut` |

### `Bytes` — Cheap Cloning

```rust
use bytes::Bytes;

let a = Bytes::from(&b"hello"[..]);
let b = a.clone(); // Increments ref count, no data copy

let c = a.slice(0..2); // "he" — shares underlying allocation
```

### `BytesMut` — Mutable Buffer

```rust
use bytes::{BytesMut, BufMut};

let mut buf = BytesMut::with_capacity(1024);
buf.put(&b"hello"[..]);
buf.put_u8(b' ');
buf.put(&b"world"[..]);

// Freeze into immutable Bytes
let frozen: Bytes = buf.freeze();
```

### `Buf` and `BufMut` Traits

- **`Buf`**: Read cursor over bytes. `advance()` consumes bytes, `chunk()` returns current slice.
- **`BufMut`**: Write cursor. `put_slice()`, `put_u8()`, etc. Auto-grows if needed.

```rust
use bytes::Buf;

let mut cursor = std::io::Cursor::new(&b"+OK\r\n"[..]);
assert_eq!(cursor.get_u8(), b'+');
assert_eq!(cursor.remaining(), 4);
cursor.advance(2); // skip "OK"
```

## See Also

- [I/O](./05-io.md) — low-level async read/write
- [Streams](./08-streams.md) — async iteration over sequences of frames
