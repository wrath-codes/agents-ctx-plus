# TCP, UDP, and Unix Sockets

Tokio provides async networking primitives for TCP, UDP, and Unix domain sockets. These types implement the `AsyncRead` and `AsyncWrite` traits (where applicable) and integrate with the runtime's I/O driver for non-blocking operation.

All networking types require the I/O driver to be enabled (`enable_io()` or `enable_all()` on the runtime builder).

---

## API Reference

### TcpListener

```rust
impl TcpListener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener>
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)>
    pub fn local_addr(&self) -> io::Result<SocketAddr>
    pub fn from_std(listener: std::net::TcpListener) -> io::Result<TcpListener>
    pub fn into_std(self) -> io::Result<std::net::TcpListener>
    pub fn poll_accept(
        &self,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<(TcpStream, SocketAddr)>>
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()>
    pub fn ttl(&self) -> io::Result<u32>
}
```

### TcpStream

```rust
impl TcpStream {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream>
    pub fn local_addr(&self) -> io::Result<SocketAddr>
    pub fn peer_addr(&self) -> io::Result<SocketAddr>
    pub fn split<'a>(&'a mut self) -> (ReadHalf<'a>, WriteHalf<'a>)
    pub fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf)
    pub async fn peek(&self, buf: &mut [u8]) -> io::Result<usize>
    pub async fn readable(&self) -> io::Result<()>
    pub async fn writable(&self) -> io::Result<()>
    pub async fn ready(&self, interest: Interest) -> io::Result<Ready>
    pub fn try_read(&self, buf: &mut [u8]) -> io::Result<usize>
    pub fn try_write(&self, buf: &[u8]) -> io::Result<usize>
    pub fn try_read_vectored(&self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize>
    pub fn try_write_vectored(&self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize>
    pub fn nodelay(&self) -> io::Result<bool>
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()>
    pub fn linger(&self) -> io::Result<Option<Duration>>
    pub fn set_linger(&self, dur: Option<Duration>) -> io::Result<()>
    pub fn ttl(&self) -> io::Result<u32>
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()>
    pub fn from_std(stream: std::net::TcpStream) -> io::Result<TcpStream>
    pub fn into_std(self) -> io::Result<std::net::TcpStream>
}
```

### TcpSocket

```rust
impl TcpSocket {
    pub fn new_v4() -> io::Result<TcpSocket>
    pub fn new_v6() -> io::Result<TcpSocket>
    pub fn bind(&self, addr: SocketAddr) -> io::Result<()>
    pub async fn connect(self, addr: SocketAddr) -> io::Result<TcpStream>
    pub fn listen(self, backlog: u32) -> io::Result<TcpListener>
    pub fn set_reuseaddr(&self, reuseaddr: bool) -> io::Result<()>
    pub fn reuseaddr(&self) -> io::Result<bool>
    pub fn set_reuseport(&self, reuseport: bool) -> io::Result<()>
    pub fn reuseport(&self) -> io::Result<bool>
    pub fn set_keepalive(&self, keepalive: bool) -> io::Result<()>
    pub fn keepalive(&self) -> io::Result<bool>
    pub fn set_send_buffer_size(&self, size: u32) -> io::Result<()>
    pub fn send_buffer_size(&self) -> io::Result<u32>
    pub fn set_recv_buffer_size(&self, size: u32) -> io::Result<()>
    pub fn recv_buffer_size(&self) -> io::Result<u32>
    pub fn local_addr(&self) -> io::Result<SocketAddr>
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()>
    pub fn nodelay(&self) -> io::Result<bool>
}
```

### UdpSocket

```rust
impl UdpSocket {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<UdpSocket>
    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()>
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize>
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize>
    pub async fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], target: A) -> io::Result<usize>
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>
    pub async fn peek_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>
    pub fn local_addr(&self) -> io::Result<SocketAddr>
    pub fn peer_addr(&self) -> io::Result<SocketAddr>
    pub async fn readable(&self) -> io::Result<()>
    pub async fn writable(&self) -> io::Result<()>
    pub async fn ready(&self, interest: Interest) -> io::Result<Ready>
    pub fn try_send(&self, buf: &[u8]) -> io::Result<usize>
    pub fn try_recv(&self, buf: &mut [u8]) -> io::Result<usize>
    pub fn try_send_to(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize>
    pub fn try_recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>
    pub fn set_broadcast(&self, on: bool) -> io::Result<()>
    pub fn broadcast(&self) -> io::Result<bool>
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()>
    pub fn ttl(&self) -> io::Result<u32>
    pub fn from_std(socket: std::net::UdpSocket) -> io::Result<UdpSocket>
    pub fn into_std(self) -> io::Result<std::net::UdpSocket>
}
```

### Unix Domain Sockets (Unix-only)

```rust
impl UnixListener {
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixListener>
    pub async fn accept(&self) -> io::Result<(UnixStream, SocketAddr)>
    pub fn local_addr(&self) -> io::Result<SocketAddr>
    pub fn from_std(listener: std::os::unix::net::UnixListener) -> io::Result<UnixListener>
    pub fn into_std(self) -> io::Result<std::os::unix::net::UnixListener>
}

impl UnixStream {
    pub async fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream>
    pub fn pair() -> io::Result<(UnixStream, UnixStream)>
    pub fn local_addr(&self) -> io::Result<SocketAddr>
    pub fn peer_addr(&self) -> io::Result<SocketAddr>
    pub fn split<'a>(&'a mut self) -> (ReadHalf<'a>, WriteHalf<'a>)
    pub fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf)
    pub async fn ready(&self, interest: Interest) -> io::Result<Ready>
    pub fn from_std(stream: std::os::unix::net::UnixStream) -> io::Result<UnixStream>
    pub fn into_std(self) -> io::Result<std::os::unix::net::UnixStream>
}

impl UnixDatagram {
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixDatagram>
    pub fn pair() -> io::Result<(UnixDatagram, UnixDatagram)>
    pub async fn connect<P: AsRef<Path>>(&self, path: P) -> io::Result<()>
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize>
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize>
    pub async fn send_to<P: AsRef<Path>>(&self, buf: &[u8], path: P) -> io::Result<usize>
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>
    pub fn local_addr(&self) -> io::Result<SocketAddr>
    pub fn peer_addr(&self) -> io::Result<SocketAddr>
    pub fn from_std(datagram: std::os::unix::net::UnixDatagram) -> io::Result<UnixDatagram>
    pub fn into_std(self) -> io::Result<std::os::unix::net::UnixDatagram>
}
```

---

## TcpListener

A TCP socket server, listening for incoming connections.

### `TcpListener::bind(addr)`

Creates a new `TcpListener` bound to the specified address. The address is resolved using `ToSocketAddrs`, so you can pass `"127.0.0.1:8080"`, `("0.0.0.0", 8080)`, or a `SocketAddr`. Fails if the port is already in use or the address is invalid.

### `accept()`

Waits for and accepts an incoming TCP connection. Returns the new `TcpStream` and the remote peer's address. This method can be called concurrently from multiple tasks — each call accepts one connection.

### `local_addr()`

Returns the local socket address the listener is bound to. Useful when binding to port `0` to get the OS-assigned port.

### `from_std(listener)` / `into_std()`

Converts between Tokio and std `TcpListener`. The std listener must be set to non-blocking mode before conversion. `into_std` deregisters the listener from the Tokio I/O driver.

---

## TcpStream

An async TCP connection. Implements `AsyncRead + AsyncWrite`.

### `TcpStream::connect(addr)`

Opens a TCP connection to a remote host. The address is resolved asynchronously using `ToSocketAddrs`. Returns when the TCP handshake completes.

### `split()` vs `into_split()`

Both split the stream into read and write halves for concurrent use:

- `split(&mut self)` — borrows the stream, returns `ReadHalf` and `WriteHalf` with a lifetime tied to the borrow. No overhead. Cannot be sent to another task.
- `into_split(self)` — consumes the stream, returns `OwnedReadHalf` and `OwnedWriteHalf` that are `Send + 'static`. Can be sent to separate tasks. Slight overhead from `Arc`.

### `peek(buf)`

Reads data from the stream without consuming it. Subsequent `read()` calls will return the same data. Useful for protocol detection.

### `readable()` / `writable()` / `ready(interest)`

Waits until the socket is ready for reading, writing, or both. Use with `try_read()` / `try_write()` for manual readiness-based I/O. This is lower-level than the `AsyncRead`/`AsyncWrite` interface.

```rust
loop {
    stream.readable().await?;
    match stream.try_read(&mut buf) {
        Ok(0) => break, // EOF
        Ok(n) => { /* process n bytes */ }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
        Err(e) => return Err(e),
    }
}
```

### `set_nodelay(nodelay)` / `nodelay()`

Controls TCP Nagle's algorithm. Setting `nodelay(true)` disables Nagle, sending data immediately without waiting to batch small writes. Essential for latency-sensitive protocols (RPC, gaming, interactive shells).

### `set_linger(dur)` / `linger()`

Controls the socket linger option. `Some(Duration::from_secs(0))` causes the connection to be reset (RST) on close instead of performing a graceful shutdown.

---

## TcpSocket

A pre-connection TCP socket for configuring socket options before connecting or listening. Use when you need to set options like `SO_REUSEADDR` or buffer sizes before the socket is connected.

### Connecting with Custom Options

```rust
use tokio::net::TcpSocket;
use std::net::SocketAddr;

let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
let socket = TcpSocket::new_v4()?;
socket.set_reuseaddr(true)?;
socket.set_send_buffer_size(65536)?;
socket.set_recv_buffer_size(65536)?;
socket.set_nodelay(true)?;
socket.bind("0.0.0.0:0".parse().unwrap())?;
let stream = socket.connect(addr).await?;
```

### Listening with Custom Options

```rust
use tokio::net::TcpSocket;

let socket = TcpSocket::new_v4()?;
socket.set_reuseaddr(true)?;
socket.set_reuseport(true)?; // Linux: allows multiple listeners on same port
socket.bind("0.0.0.0:8080".parse().unwrap())?;
let listener = socket.listen(1024)?; // backlog of 1024
```

### `set_reuseaddr(true)`

Allows the socket to bind to an address that is in the `TIME_WAIT` state. Essential for server restarts.

### `set_reuseport(true)`

(Linux/macOS) Allows multiple sockets to bind to the same port. The kernel distributes incoming connections across them. Used for multi-process servers.

### `set_keepalive(true)`

Enables TCP keepalive probes. The OS periodically sends probes to detect dead connections.

---

## UdpSocket

An async UDP socket for connectionless datagram communication.

### Unconnected Usage (send_to / recv_from)

```rust
use tokio::net::UdpSocket;

let socket = UdpSocket::bind("0.0.0.0:0").await?;
socket.send_to(b"Hello", "127.0.0.1:9000").await?;

let mut buf = [0u8; 1024];
let (len, addr) = socket.recv_from(&mut buf).await?;
println!("Received {} bytes from {}", len, addr);
```

### Connected Usage (send / recv)

Calling `connect()` on a UDP socket fixes the remote address. After connecting, use `send()` and `recv()` instead of `send_to()` and `recv_from()`. The kernel filters out datagrams from other addresses.

```rust
let socket = UdpSocket::bind("0.0.0.0:0").await?;
socket.connect("127.0.0.1:9000").await?;

socket.send(b"Hello").await?;

let mut buf = [0u8; 1024];
let len = socket.recv(&mut buf).await?;
```

### `set_broadcast(true)`

Enables sending to the broadcast address (`255.255.255.255` or subnet broadcast). Required before sending broadcast datagrams.

### `peek_from(buf)`

Reads a datagram without consuming it. The next `recv_from()` call returns the same datagram.

---

## Unix Domain Sockets

Unix domain sockets provide local inter-process communication. They are faster than TCP loopback because they bypass the network stack. Available on Unix-like systems only (`#[cfg(unix)]`).

### UnixListener

Listens for incoming Unix stream connections on a filesystem path.

```rust
use tokio::net::UnixListener;

let listener = UnixListener::bind("/tmp/my-app.sock")?;

loop {
    let (stream, _addr) = listener.accept().await?;
    tokio::spawn(async move {
        handle_connection(stream).await;
    });
}
```

The socket file is not automatically deleted when the listener is dropped. Clean it up manually with `std::fs::remove_file`.

### UnixStream

An async Unix stream connection. Implements `AsyncRead + AsyncWrite`. API is similar to `TcpStream`.

```rust
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

let mut stream = UnixStream::connect("/tmp/my-app.sock").await?;
stream.write_all(b"Hello, Unix!").await?;

let mut buf = vec![0u8; 1024];
let n = stream.read(&mut buf).await?;
println!("Received: {}", String::from_utf8_lossy(&buf[..n]));
```

### `UnixStream::pair()`

Creates a connected pair of Unix streams without a filesystem path. Useful for parent-child process communication or in-process testing.

```rust
let (mut a, mut b) = UnixStream::pair()?;

tokio::spawn(async move {
    a.write_all(b"message from a").await.unwrap();
});

let mut buf = vec![0u8; 1024];
let n = b.read(&mut buf).await?;
assert_eq!(&buf[..n], b"message from a");
```

### UnixDatagram

An async Unix datagram socket for connectionless local communication.

```rust
use tokio::net::UnixDatagram;

let (a, b) = UnixDatagram::pair()?;

a.send(b"hello").await?;
let mut buf = [0u8; 1024];
let n = b.recv(&mut buf).await?;
assert_eq!(&buf[..n], b"hello");
```

---

## ToSocketAddrs

The `ToSocketAddrs` trait enables flexible address specification. Tokio's version performs DNS resolution asynchronously (on a blocking thread internally).

Types that implement `ToSocketAddrs`:

| Type | Example |
|------|---------|
| `SocketAddr` | `SocketAddr::from(([127, 0, 0, 1], 8080))` |
| `(IpAddr, u16)` | `(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080)` |
| `(&str, u16)` | `("localhost", 8080)` |
| `&str` (with port) | `"127.0.0.1:8080"` |
| `String` (with port) | `"localhost:8080".to_string()` |
| `&[SocketAddr]` | Slice of resolved addresses |

---

## Examples

### TCP Echo Server

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on {}", listener.local_addr()?);

    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);

        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Read error: {}", e);
                        return;
                    }
                };

                if let Err(e) = socket.write_all(&buf[..n]).await {
                    eprintln!("Write error: {}", e);
                    return;
                }
            }
        });
    }
}
```

### TCP Echo Server with Split

```rust
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            loop {
                line.clear();
                let n = reader.read_line(&mut line).await.unwrap();
                if n == 0 { break; }
                writer.write_all(line.as_bytes()).await.unwrap();
            }
        });
    }
}
```

### UDP Echo

```rust
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:9000").await?;
    println!("UDP listening on {}", socket.local_addr()?);

    let mut buf = [0u8; 65535];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        println!("Received {} bytes from {}", len, addr);
        socket.send_to(&buf[..len], addr).await?;
    }
}
```

### Unix Socket Communication

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let socket_path = "/tmp/tokio-example.sock";

    // Clean up leftover socket file
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    println!("Unix socket listening on {}", socket_path);

    // Spawn client
    tokio::spawn(async move {
        let mut stream = UnixStream::connect(socket_path).await.unwrap();
        stream.write_all(b"Hello via Unix socket!").await.unwrap();
        stream.shutdown().await.unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        println!("Client received: {}", response);
    });

    // Accept connection
    let (mut stream, _addr) = listener.accept().await?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf).await?;
    println!("Server received: {}", buf);

    stream.write_all(b"Acknowledged").await?;
    stream.shutdown().await?;

    // Clean up
    std::fs::remove_file(socket_path)?;
    Ok(())
}
```

### TCP Proxy with copy_bidirectional

```rust
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    let upstream_addr = "127.0.0.1:8080";

    loop {
        let (mut client, client_addr) = listener.accept().await?;
        println!("Proxying connection from {}", client_addr);

        tokio::spawn(async move {
            let mut upstream = match TcpStream::connect(upstream_addr).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to connect to upstream: {}", e);
                    return;
                }
            };

            match io::copy_bidirectional(&mut client, &mut upstream).await {
                Ok((c_to_u, u_to_c)) => {
                    println!(
                        "Connection closed. Client->upstream: {} bytes, upstream->client: {} bytes",
                        c_to_u, u_to_c
                    );
                }
                Err(e) => eprintln!("Proxy error: {}", e),
            }
        });
    }
}
```

### TcpSocket with Pre-Connection Configuration

```rust
use tokio::net::TcpSocket;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();

    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.set_keepalive(true)?;
    socket.set_send_buffer_size(128 * 1024)?;
    socket.set_recv_buffer_size(128 * 1024)?;
    socket.bind(addr)?;

    let listener = socket.listen(1024)?;
    println!("Listening on {} with custom socket options", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Accepted connection from {}", addr);
        tokio::spawn(async move {
            handle_connection(stream).await;
        });
    }
}

async fn handle_connection(mut stream: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => return,
            Ok(n) => { stream.write_all(&buf[..n]).await.unwrap(); }
            Err(_) => return,
        }
    }
}
```

---

## Thread Safety

### TCP Types

| Type | `Send` | `Sync` | `Clone` | Notes |
|------|--------|--------|---------|-------|
| `TcpListener` | Yes | Yes | No | Shared via `Arc` for multi-task accept |
| `TcpStream` | Yes | Yes | No | Split into halves for concurrent I/O |
| `TcpSocket` | Yes | Yes | No | Consumed on connect/listen |
| `OwnedReadHalf` | Yes | Yes | No | From `into_split()` |
| `OwnedWriteHalf` | Yes | Yes | No | From `into_split()` |

### UDP Types

| Type | `Send` | `Sync` | `Clone` | Notes |
|------|--------|--------|---------|-------|
| `UdpSocket` | Yes | Yes | No | Shared via `Arc` for concurrent send/recv |

### Unix Types (Unix-only)

| Type | `Send` | `Sync` | `Clone` | Notes |
|------|--------|--------|---------|-------|
| `UnixListener` | Yes | Yes | No | Local IPC only |
| `UnixStream` | Yes | Yes | No | Supports `split()` and `into_split()` |
| `UnixDatagram` | Yes | Yes | No | Connectionless local datagrams |

---

## See Also

- [The Tokio Runtime](01-runtime.md) — runtime configuration, I/O driver must be enabled
- [Tasks and Spawning](02-tasks.md) — spawning connection handler tasks
- [I/O Traits and Utilities](03-io.md) — `AsyncRead`, `AsyncWrite`, `copy()`, `split()`, and buffered I/O
