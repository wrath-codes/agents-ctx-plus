# Go Binding

## Installation

```bash
go get github.com/tursodatabase/go-libsql
```

## Quick Start

```go
package main

import (
    "database/sql"
    "fmt"
    "log"
    
    _ "github.com/tursodatabase/go-libsql"
)

func main() {
    // Open local database
    db, err := sql.Open("libsql", "file:./data.db")
    if err != nil {
        log.Fatal(err)
    }
    defer db.Close()
    
    // Create table
    _, err = db.Exec(`CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY,
        name TEXT
    )`)
    if err != nil {
        log.Fatal(err)
    }
    
    // Insert
    _, err = db.Exec("INSERT INTO users (name) VALUES (?)", "Alice")
    if err != nil {
        log.Fatal(err)
    }
    
    // Query
    rows, err := db.Query("SELECT id, name FROM users")
    if err != nil {
        log.Fatal(err)
    }
    defer rows.Close()
    
    for rows.Next() {
        var id int64
        var name string
        if err := rows.Scan(&id, &name); err != nil {
            log.Fatal(err)
        }
        fmt.Printf("%d: %s\n", id, name)
    }
}
```

## Database Connection

### Local Database

```go
// File-based database
db, err := sql.Open("libsql", "file:./mydb.db")

// In-memory database
db, err := sql.Open("libsql", ":memory:")

// With options
db, err := sql.Open("libsql", "file:./mydb.db?_journal_mode=WAL&_cache_size=10000")
```

### Remote Database (Turso Cloud)

```go
import (
    "github.com/tursodatabase/go-libsql"
)

// Direct connection
connector, err := libsql.NewConnector(
    "libsql://mydb-org.turso.io",
    libsql.WithAuthToken("your-auth-token"),
)
if err != nil {
    log.Fatal(err)
}
db := sql.OpenDB(connector)

// With HTTP client options
connector, err := libsql.NewConnector(
    "libsql://mydb-org.turso.io",
    libsql.WithAuthToken("your-auth-token"),
    libsql.WithHttpClient(&http.Client{
        Timeout: 30 * time.Second,
    }),
)
```

### Embedded Replica

```go
// Local replica with cloud sync
connector, err := libsql.NewEmbeddedReplicaConnector(
    "./local-replica.db",
    "libsql://mydb-org.turso.io",
    libsql.WithAuthToken("your-auth-token"),
    libsql.WithSyncInterval(5 * time.Second),
)
if err != nil {
    log.Fatal(err)
}
db := sql.OpenDB(connector)
```

## CRUD Operations

### Create

```go
// Insert single row
result, err := db.Exec(
    "INSERT INTO users (name, email) VALUES (?, ?)",
    "Alice", "alice@example.com",
)
if err != nil {
    log.Fatal(err)
}

lastID, _ := result.LastInsertId()
rowsAffected, _ := result.RowsAffected()
```

### Read

```go
// Query single row
var name string
err := db.QueryRow(
    "SELECT name FROM users WHERE id = ?",
    1,
).Scan(&name)

if err == sql.ErrNoRows {
    // No results
} else if err != nil {
    log.Fatal(err)
}

// Query multiple rows
rows, err := db.Query("SELECT id, name, email FROM users")
if err != nil {
    log.Fatal(err)
}
defer rows.Close()

for rows.Next() {
    var id int64
    var name, email string
    if err := rows.Scan(&id, &name, &email); err != nil {
        log.Fatal(err)
    }
    fmt.Printf("%d: %s (%s)\n", id, name, email)
}

if err := rows.Err(); err != nil {
    log.Fatal(err)
}
```

### Update

```go
result, err := db.Exec(
    "UPDATE users SET name = ? WHERE id = ?",
    "Alice Smith", 1,
)
if err != nil {
    log.Fatal(err)
}

rowsAffected, _ := result.RowsAffected()
fmt.Printf("Updated %d rows\n", rowsAffected)
```

### Delete

```go
result, err := db.Exec("DELETE FROM users WHERE id = ?", 1)
if err != nil {
    log.Fatal(err)
}

rowsAffected, _ := result.RowsAffected()
fmt.Printf("Deleted %d rows\n", rowsAffected)
```

## Transactions

### Standard Transaction

```go
tx, err := db.Begin()
if err != nil {
    log.Fatal(err)
}

_, err = tx.Exec("INSERT INTO users (name) VALUES (?)", "Bob")
if err != nil {
    tx.Rollback()
    log.Fatal(err)
}

_, err = tx.Exec("INSERT INTO logs (msg) VALUES (?)", "Added Bob")
if err != nil {
    tx.Rollback()
    log.Fatal(err)
}

if err := tx.Commit(); err != nil {
    log.Fatal(err)
}
```

### Concurrent Transaction (MVCC)

```go
tx, err := db.BeginTx(context.Background(), &sql.TxOptions{
    Isolation: sql.LevelSnapshot, // Uses libSQL MVCC
})
if err != nil {
    log.Fatal(err)
}
defer tx.Rollback()

// Multiple connections can write concurrently
// with BEGIN CONCURRENT
```

## Prepared Statements

```go
// Prepare statement
stmt, err := db.Prepare("SELECT * FROM users WHERE id = ?")
if err != nil {
    log.Fatal(err)
}
defer stmt.Close()

// Execute multiple times
for i := 1; i <= 100; i++ {
    var name string
    err := stmt.QueryRow(i).Scan(&name)
    if err != nil && err != sql.ErrNoRows {
        log.Fatal(err)
    }
}
```

## Vector Operations

### Storing Vectors

```go
// Create table with vector column
_, err = db.Exec(`CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding F32_BLOB(384)
)`)

// Insert vector
embedding := []float32{0.1, 0.2, 0.3 /* ... 384 values */}
_, err = db.Exec(
    "INSERT INTO documents (content, embedding) VALUES (?, ?)",
    "Hello world",
    libsql.Float32Array(embedding),
)
```

### Vector Search

```go
queryVector := []float32{0.1, 0.2, /* ... */}

rows, err := db.Query(
    `SELECT content, vector_distance_cosine(embedding, vector(?)) as distance
     FROM documents
     ORDER BY distance
     LIMIT 5`,
    libsql.Float32Array(queryVector),
)
if err != nil {
    log.Fatal(err)
}
defer rows.Close()

for rows.Next() {
    var content string
    var distance float64
    if err := rows.Scan(&content, &distance); err != nil {
        log.Fatal(err)
    }
    fmt.Printf("%s (distance: %f)\n", content, distance)
}
```

## Batch Operations

```go
// Prepare insert statement
stmt, err := db.Prepare("INSERT INTO logs (msg) VALUES (?)")
if err != nil {
    log.Fatal(err)
}
defer stmt.Close()

// Insert in transaction
tx, err := db.Begin()
if err != nil {
    log.Fatal(err)
}

txStmt := tx.Stmt(stmt)
for i := 0; i < 1000; i++ {
    _, err := txStmt.Exec(fmt.Sprintf("Log entry %d", i))
    if err != nil {
        tx.Rollback()
        log.Fatal(err)
    }
}

if err := tx.Commit(); err != nil {
    log.Fatal(err)
}
```

## Error Handling

```go
result, err := db.Exec("INVALID SQL")
if err != nil {
    if sqliteErr, ok := err.(*libsql.Error); ok {
        fmt.Printf("SQLite error code: %d\n", sqliteErr.Code)
        fmt.Printf("Error message: %s\n", sqliteErr.Message)
    } else {
        fmt.Printf("Other error: %v\n", err)
    }
}
```

## Context Support

```go
// With timeout
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()

row := db.QueryRowContext(ctx, "SELECT * FROM users WHERE id = ?", 1)

// With cancellation
ctx, cancel := context.WithCancel(context.Background())
go func() {
    time.Sleep(2 * time.Second)
    cancel()
}()

rows, err := db.QueryContext(ctx, "SELECT * FROM large_table")
```

## Best Practices

1. **Always use `sql.Null*` types** for nullable columns
2. **Defer `rows.Close()`** immediately after checking error
3. **Use prepared statements** for repeated queries
4. **Handle `sql.ErrNoRows`** explicitly
5. **Use transactions** for multi-statement operations
6. **Set connection pool limits**

```go
db.SetMaxOpenConns(25)
db.SetMaxIdleConns(25)
db.SetConnMaxLifetime(5 * time.Minute)
```

## API Reference

See [pkg.go.dev/github.com/tursodatabase/go-libsql](https://pkg.go.dev/github.com/tursodatabase/go-libsql) for complete API documentation.

## Next Steps

- [JavaScript Binding](./javascript-binding.md)
- [Python Binding](./python-binding.md)
- [Rust Crate](../rust-crate/01-overview.md)