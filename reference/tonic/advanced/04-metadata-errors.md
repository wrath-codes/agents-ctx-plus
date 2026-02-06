# Metadata and Error Handling

gRPC metadata (custom headers) and the `Status` error type for communicating errors.

---

## Request and Response

```rust
impl<T> Request<T> {
    pub fn new(message: T) -> Self
    pub fn into_inner(self) -> T
    pub fn get_ref(&self) -> &T
    pub fn get_mut(&mut self) -> &mut T
    pub fn metadata(&self) -> &MetadataMap
    pub fn metadata_mut(&mut self) -> &mut MetadataMap
    pub fn extensions(&self) -> &Extensions
    pub fn extensions_mut(&mut self) -> &mut Extensions
    pub fn into_parts(self) -> (MetadataMap, Extensions, T)
    pub fn from_parts(metadata: MetadataMap, extensions: Extensions, message: T) -> Self
    pub fn remote_addr(&self) -> Option<SocketAddr>
    pub fn local_addr(&self) -> Option<SocketAddr>
}

impl<T> Response<T> {
    pub fn new(message: T) -> Self
    pub fn into_inner(self) -> T
    pub fn get_ref(&self) -> &T
    pub fn get_mut(&mut self) -> &mut T
    pub fn metadata(&self) -> &MetadataMap
    pub fn metadata_mut(&mut self) -> &mut MetadataMap
    pub fn extensions(&self) -> &Extensions
    pub fn extensions_mut(&mut self) -> &mut Extensions
    pub fn into_parts(self) -> (MetadataMap, Extensions, T)
    pub fn from_parts(metadata: MetadataMap, extensions: Extensions, message: T) -> Self
}
```

---

## MetadataMap

gRPC metadata is similar to HTTP headers — key-value pairs sent with requests and responses.

```rust
use tonic::metadata::MetadataMap;

// Create metadata
let mut metadata = MetadataMap::new();
metadata.insert("x-request-id", "req-123".parse().unwrap());
metadata.insert("authorization", "Bearer token".parse().unwrap());

// Binary metadata (keys ending in "-bin")
metadata.insert_bin("x-data-bin", tonic::metadata::MetadataValue::from_bytes(b"binary data"));

// Read metadata
let request_id = metadata.get("x-request-id")
    .and_then(|v| v.to_str().ok());
```

### In Handlers

```rust
async fn say_hello(
    &self,
    request: Request<HelloRequest>,
) -> Result<Response<HelloReply>, Status> {
    // Read request metadata
    let request_id = request.metadata().get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let reply = HelloReply {
        message: format!("Hello {}!", request.into_inner().name),
    };

    // Add response metadata
    let mut response = Response::new(reply);
    response.metadata_mut().insert(
        "x-served-by",
        "server-1".parse().unwrap(),
    );

    Ok(response)
}
```

---

## Status (Error Type)

`Status` represents a gRPC error with a code, message, and optional details/metadata.

### Creating Status

```rust
use tonic::{Status, Code};

// Using convenience methods
let status = Status::not_found("User not found");
let status = Status::invalid_argument("Name is required");
let status = Status::internal("Database error");
let status = Status::unauthenticated("Invalid credentials");
let status = Status::permission_denied("Insufficient permissions");

// Using new()
let status = Status::new(Code::Unavailable, "Service temporarily unavailable");

// With binary details
let status = Status::with_details(
    Code::InvalidArgument,
    "Validation failed",
    bytes::Bytes::from("detailed error info"),
);

// With metadata
let mut metadata = MetadataMap::new();
metadata.insert("x-retry-after", "5".parse().unwrap());
let status = Status::with_metadata(
    Code::ResourceExhausted,
    "Rate limited",
    metadata,
);
```

### Status Methods

| Method | Description |
|--------|-------------|
| `code()` | Get the gRPC `Code` |
| `message()` | Get the error message |
| `details()` | Get binary error details |
| `metadata()` | Get associated metadata |
| `set_source(err)` | Attach a source error |

---

## Code Enum

| Code | Value | Description |
|------|-------|-------------|
| `Ok` | 0 | Success |
| `Cancelled` | 1 | Operation cancelled by caller |
| `Unknown` | 2 | Unknown error |
| `InvalidArgument` | 3 | Client sent invalid argument |
| `DeadlineExceeded` | 4 | Operation timed out |
| `NotFound` | 5 | Requested entity not found |
| `AlreadyExists` | 6 | Entity already exists |
| `PermissionDenied` | 7 | Caller lacks permission |
| `ResourceExhausted` | 8 | Resource limit reached |
| `FailedPrecondition` | 9 | System not in required state |
| `Aborted` | 10 | Operation aborted (retry at higher level) |
| `OutOfRange` | 11 | Operation outside valid range |
| `Unimplemented` | 12 | Method not implemented |
| `Internal` | 13 | Internal server error |
| `Unavailable` | 14 | Service temporarily unavailable (retry) |
| `DataLoss` | 15 | Unrecoverable data loss |
| `Unauthenticated` | 16 | Request not authenticated |

### Choosing the Right Code

| Scenario | Code |
|----------|------|
| Input validation failed | `InvalidArgument` |
| Entity not found in database | `NotFound` |
| Auth token missing or expired | `Unauthenticated` |
| User lacks access rights | `PermissionDenied` |
| Rate limit exceeded | `ResourceExhausted` |
| Request timed out | `DeadlineExceeded` |
| Concurrent modification conflict | `Aborted` |
| Server bug | `Internal` |
| Temporary outage (retry safe) | `Unavailable` |
| Feature not built yet | `Unimplemented` |

---

## Error Handling Patterns

### Converting Rust Errors to Status

```rust
async fn get_user(
    &self,
    request: Request<GetUserRequest>,
) -> Result<Response<User>, Status> {
    let id = request.into_inner().id;

    let user = self.db.find_user(id).await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::not_found(format!("User {} not found", id)))?;

    Ok(Response::new(user))
}
```

### Matching Status on Client

```rust
match client.get_user(request).await {
    Ok(response) => {
        println!("User: {:?}", response.into_inner());
    }
    Err(status) => match status.code() {
        Code::NotFound => println!("User not found"),
        Code::Unauthenticated => println!("Please log in"),
        Code::PermissionDenied => println!("Access denied"),
        code => println!("Error ({}): {}", code, status.message()),
    },
}
```

---

## See Also

- [Interceptors](01-interceptors.md) — adding metadata via interceptors
- [Server](../core/01-server.md) — server implementation
- [Client](../core/02-client.md) — client error handling
- [Streaming Patterns](../streaming/01-patterns.md) — error handling in streams
