# Python Binding

## Installation

```bash
pip install libsql-client
```

Requirements:
- Python 3.7+
- asyncio support

## Quick Start

```python
import asyncio
from libsql_client import create_client

async def main():
    # Local database
    async with create_client("file:mydb.db") as client:
        # Create table
        await client.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT
            )
        """)
        
        # Insert
        await client.execute(
            "INSERT INTO users (name) VALUES (?)",
            ["Alice"]
        )
        
        # Query
        result = await client.execute("SELECT * FROM users")
        for row in result.rows:
            print(f"{row[0]}: {row[1]}")

asyncio.run(main())
```

## Database Connection

### Local Database

```python
from libsql_client import create_client

# File-based database
async with create_client("file:./mydb.db") as client:
    pass

# In-memory database
async with create_client(":memory:") as client:
    pass
```

### Remote Database (Turso Cloud)

```python
async with create_client(
    "libsql://mydb-org.turso.io",
    auth_token="your-auth-token"
) as client:
    pass

# Or as context manager
client = create_client(
    "libsql://mydb-org.turso.io",
    auth_token="your-auth-token"
)
await client.open()
# ... use client ...
await client.close()
```

### Embedded Replica

```python
from libsql_client import create_client_sync

# Synchronous client for local replica
client = create_client_sync(
    "file:./local-replica.db",
    sync_url="libsql://mydb-org.turso.io",
    auth_token="your-auth-token"
)

# Manual sync
client.sync()
```

## CRUD Operations

### Create

```python
# Insert single row
result = await client.execute(
    "INSERT INTO users (name, email) VALUES (?, ?)",
    ["Alice", "alice@example.com"]
)
print(f"Inserted row: {result.last_insert_rowid}")

# Insert many
await client.batch([
    ("INSERT INTO users (name, email) VALUES (?, ?)", ["Bob", "bob@example.com"]),
    ("INSERT INTO users (name, email) VALUES (?, ?)", ["Charlie", "charlie@example.com"]),
])
```

### Read

```python
# Query single row
result = await client.execute(
    "SELECT name FROM users WHERE id = ?",
    [1]
)
if result.rows:
    print(result.rows[0][0])

# Query multiple rows
result = await client.execute("SELECT id, name, email FROM users")
for row in result.rows:
    id, name, email = row
    print(f"{id}: {name} ({email})")

# Named parameters
result = await client.execute(
    "SELECT * FROM users WHERE name = :name",
    {"name": "Alice"}
)
```

### Update

```python
result = await client.execute(
    "UPDATE users SET name = ? WHERE id = ?",
    ["Alice Smith", 1]
)
print(f"Updated {result.rows_affected} rows")
```

### Delete

```python
result = await client.execute(
    "DELETE FROM users WHERE id = ?",
    [1]
)
print(f"Deleted {result.rows_affected} rows")
```

## Transactions

```python
# Using transaction context manager
async with client.transaction() as tx:
    await tx.execute(
        "INSERT INTO users (name) VALUES (?)",
        ["Bob"]
    )
    await tx.execute(
        "INSERT INTO logs (msg) VALUES (?)",
        ["Added Bob"]
    )
    # Auto-committed on success, rolled back on exception

# Manual transaction
tx = await client.transaction()
try:
    await tx.execute("INSERT INTO users (name) VALUES (?)", ["Bob"])
    await tx.execute("INSERT INTO logs (msg) VALUES (?)", ["Added Bob"])
    await tx.commit()
except Exception as e:
    await tx.rollback()
    raise
```

## Vector Operations

### Storing Vectors

```python
import numpy as np

# Create table
await client.execute("""
    CREATE TABLE documents (
        id INTEGER PRIMARY KEY,
        content TEXT,
        embedding F32_BLOB(384)
    )
""")

# Insert with embedding
embedding = np.array([0.1, 0.2, 0.3], dtype=np.float32)
await client.execute(
    "INSERT INTO documents (content, embedding) VALUES (?, ?)",
    ["Hello world", embedding.tobytes()]
)
```

### Vector Search

```python
import numpy as np

query_vector = np.array([0.1, 0.2, 0.3], dtype=np.float32)

result = await client.execute("""
    SELECT content, vector_distance_cosine(embedding, vector(?)) as distance
    FROM documents
    ORDER BY distance
    LIMIT 5
""", [query_vector.tobytes()])

for row in result.rows:
    content, distance = row
    print(f"{content} (distance: {distance})")
```

## Prepared Statements

```python
# Prepare statement
stmt = await client.prepare("SELECT * FROM users WHERE id = ?")

# Execute multiple times
for i in range(1, 101):
    result = await stmt.execute([i])
    if result.rows:
        print(result.rows[0])

# Close statement
await stmt.close()
```

## Batch Operations

```python
# Batch insert
statements = [
    ("INSERT INTO logs (msg) VALUES (?)", [f"Log {i}"])
    for i in range(1000)
]

await client.batch(statements)

# Batch with transaction
async with client.transaction() as tx:
    for i in range(1000):
        await tx.execute(
            "INSERT INTO logs (msg) VALUES (?)",
            [f"Log {i}"]
        )
```

## Error Handling

```python
from libsql_client import LibsqlError

try:
    await client.execute("INVALID SQL")
except LibsqlError as e:
    print(f"SQLite error: {e.code} - {e.message}")
except Exception as e:
    print(f"Other error: {e}")
```

## Type Conversion

| Python Type | SQLite Type |
|-------------|-------------|
| `int` | INTEGER |
| `float` | REAL |
| `str` | TEXT |
| `bytes` | BLOB |
| `bool` | INTEGER (0 or 1) |
| `datetime.datetime` | TEXT (ISO 8601) |
| `None` | NULL |

```python
from datetime import datetime

await client.execute(
    "INSERT INTO events (name, created_at) VALUES (?, ?)",
    ["Test", datetime.now()]
)
```

## Async Context

```python
import asyncio

async def main():
    async with create_client("file:mydb.db") as client:
        # Multiple concurrent operations
        results = await asyncio.gather(
            client.execute("SELECT * FROM users WHERE id = 1"),
            client.execute("SELECT * FROM users WHERE id = 2"),
            client.execute("SELECT * FROM users WHERE id = 3"),
        )
        
        for result in results:
            print(result.rows)

asyncio.run(main())
```

## Migrations

```python
async def migrate(client):
    result = await client.execute("PRAGMA user_version")
    version = result.rows[0][0]
    
    if version < 1:
        await client.execute("""
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT
            )
        """)
        await client.execute("PRAGMA user_version = 1")
    
    if version < 2:
        await client.execute("ALTER TABLE users ADD COLUMN email TEXT")
        await client.execute("PRAGMA user_version = 2")
```

## FastAPI Integration

```python
from fastapi import FastAPI, Depends
from libsql_client import create_client

app = FastAPI()

async def get_db():
    async with create_client("libsql://mydb-org.turso.io") as client:
        yield client

@app.get("/users/{user_id}")
async def get_user(user_id: int, db=Depends(get_db)):
    result = await db.execute(
        "SELECT * FROM users WHERE id = ?",
        [user_id]
    )
    if not result.rows:
        raise HTTPException(status_code=404, detail="User not found")
    return {"id": result.rows[0][0], "name": result.rows[0][1]}
```

## Best Practices

1. **Use async context managers** for automatic cleanup
2. **Use transactions** for multi-statement operations
3. **Batch inserts** for better performance
4. **Use prepared statements** for repeated queries
5. **Handle LibsqlError** for SQLite-specific errors
6. **Close statements** when done

## API Reference

See [libsql-client-py documentation](https://github.com/tursodatabase/libsql-client-py) for complete API reference.

## Next Steps

- [Go Binding](./go-binding.md)
- [JavaScript Binding](./javascript-binding.md)
- [Rust Crate](../rust-crate/01-overview.md)