# Language Bindings

## Overview

Turso provides official language bindings for several programming languages, enabling you to use Turso Database and Turso Cloud from your preferred language.

## Available Bindings

| Language | Package | Status | Documentation |
|----------|---------|--------|---------------|
| Go | `github.com/tursodatabase/go-libsql` | Stable | [Go Binding](./go-binding.md) |
| JavaScript | `@libsql/client` | Stable | [JavaScript Binding](./javascript-binding.md) |
| Python | `libsql-client` | Stable | [Python Binding](./python-binding.md) |
| Java | `tech.turso.libsql` | Beta | [Java Binding](./java-binding.md) |
| WebAssembly | `libsql-wasm` | Beta | [WASM Binding](./wasm-binding.md) |

## Common Features

All language bindings support:

- ✅ Local SQLite databases
- ✅ Remote Turso Cloud databases
- ✅ Embedded replicas with sync
- ✅ Vector search operations
- ✅ Transactions
- ✅ Prepared statements
- ✅ Batch operations
- ✅ Async/await patterns (where applicable)

## Quick Comparison

### Connection Example

**Go:**
```go
import "github.com/tursodatabase/go-libsql"

db, err := libsql.Open("file:data.db")
```

**JavaScript:**
```javascript
import { createClient } from '@libsql/client';

const client = createClient({
  url: 'file:data.db'
});
```

**Python:**
```python
from libsql_client import create_client

client = create_client(url="file:data.db")
```

**Java:**
```java
import tech.turso.libsql.Database;

Database db = Database.open("file:data.db");
```

### Query Example

**Go:**
```go
rows, err := db.Query("SELECT * FROM users WHERE id = ?", 1)
```

**JavaScript:**
```javascript
const result = await client.execute({
  sql: "SELECT * FROM users WHERE id = ?",
  args: [1]
});
```

**Python:**
```python
result = await client.execute(
    "SELECT * FROM users WHERE id = ?",
    [1]
)
```

**Java:**
```java
ResultSet rs = db.query("SELECT * FROM users WHERE id = ?", 1);
```

## Feature Matrix

| Feature | Go | JS | Python | Java | WASM |
|---------|-----|-----|--------|------|------|
| Local DB | ✅ | ✅ | ✅ | ✅ | ✅ |
| Remote DB | ✅ | ✅ | ✅ | ✅ | ✅ |
| Embedded Replica | ✅ | ✅ | ✅ | ✅ | ❌ |
| Vector Search | ✅ | ✅ | ✅ | ⚠️ | ⚠️ |
| Transactions | ✅ | ✅ | ✅ | ✅ | ✅ |
| Batch | ✅ | ✅ | ✅ | ✅ | ✅ |
| Async | ✅ | ✅ | ✅ | ✅ | ✅ |
| Streams | ⚠️ | ✅ | ✅ | ❌ | ❌ |
| Encryption | ✅ | ✅ | ✅ | ⚠️ | ❌ |
| CDC | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ |

Legend: ✅ Full support, ⚠️ Partial support, ❌ Not supported

## Installation

### Go
```bash
go get github.com/tursodatabase/go-libsql
```

### JavaScript
```bash
npm install @libsql/client
# or
yarn add @libsql/client
```

### Python
```bash
pip install libsql-client
```

### Java
```xml
<dependency>
    <groupId>tech.turso</groupId>
    <artifactId>libsql</artifactId>
    <version>0.1.0</version>
</dependency>
```

### WebAssembly
```bash
npm install @libsql/wasm
```

## Next Steps

- [Go Binding](./go-binding.md)
- [JavaScript Binding](./javascript-binding.md)
- [Python Binding](./python-binding.md)
- [Java Binding](./java-binding.md)
- [WASM Binding](./wasm-binding.md)