# Java Binding

## Installation

### Maven

```xml
<dependency>
    <groupId>tech.turso</groupId>
    <artifactId>libsql</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Gradle

```groovy
dependencies {
    implementation 'tech.turso:libsql:0.1.0'
}
```

## Quick Start

```java
import tech.turso.libsql.Database;
import tech.turso.libsql.Connection;
import tech.turso.libsql.ResultSet;
import tech.turso.libsql.Row;

public class Main {
    public static void main(String[] args) {
        // Local database
        Database db = Database.open("file:mydb.db");
        
        // Remote database
        Database db = Database.open(
            "libsql://mydb-org.turso.io",
            "your-auth-token"
        );
        
        Connection conn = db.connect();
        
        // Create table
        conn.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT
            )
        """);
        
        // Insert
        conn.execute("INSERT INTO users (name) VALUES (?)", "Alice");
        
        // Query
        ResultSet rs = conn.query("SELECT * FROM users");
        for (Row row : rs) {
            System.out.println(row.getLong(0) + ": " + row.getString(1));
        }
        
        conn.close();
        db.close();
    }
}
```

## Database Connection

### Local Database

```java
// File-based database
Database db = Database.open("file:./mydb.db");

// In-memory database
Database db = Database.open(":memory:");

// With options
Database db = Database.open("file:./mydb.db");
db.setJournalMode("WAL");
db.setCacheSize(10000);
```

### Remote Database

```java
Database db = Database.open(
    "libsql://mydb-org.turso.io",
    "your-auth-token"
);

// With custom HTTP client
HttpClient httpClient = HttpClient.newBuilder()
    .connectTimeout(Duration.ofSeconds(30))
    .build();

Database db = Database.open(
    "libsql://mydb-org.turso.io",
    "your-auth-token",
    httpClient
);
```

### Embedded Replica

```java
Database db = Database.open(
    "file:./local-replica.db",
    "libsql://mydb-org.turso.io",
    "your-auth-token"
);

// Manual sync
db.sync();
```

## CRUD Operations

### Create

```java
// Insert single row
long lastId = conn.execute(
    "INSERT INTO users (name, email) VALUES (?, ?)",
    "Alice", "alice@example.com"
);
System.out.println("Inserted row: " + lastId);

// Insert with auto-generated ID
conn.execute("INSERT INTO users (name) VALUES (?)", "Bob");
```

### Read

```java
// Query single row
Optional<Row> row = conn.queryRow(
    "SELECT name FROM users WHERE id = ?",
    1
);
row.ifPresent(r -> System.out.println(r.getString(0)));

// Query multiple rows
ResultSet rs = conn.query("SELECT id, name, email FROM users");
while (rs.next()) {
    long id = rs.getLong(0);
    String name = rs.getString(1);
    String email = rs.getString(2);
    System.out.println(id + ": " + name + " (" + email + ")");
}
```

### Update

```java
int rowsAffected = conn.execute(
    "UPDATE users SET name = ? WHERE id = ?",
    "Alice Smith", 1
);
System.out.println("Updated " + rowsAffected + " rows");
```

### Delete

```java
int rowsAffected = conn.execute(
    "DELETE FROM users WHERE id = ?",
    1
);
System.out.println("Deleted " + rowsAffected + " rows");
```

## Transactions

```java
// Standard transaction
Transaction tx = conn.beginTransaction();
try {
    tx.execute("INSERT INTO users (name) VALUES (?)", "Bob");
    tx.execute("INSERT INTO logs (msg) VALUES (?)", "Added Bob");
    tx.commit();
} catch (Exception e) {
    tx.rollback();
    throw e;
}

// Try-with-resources (auto rollback on failure)
try (Transaction tx = conn.beginTransaction()) {
    tx.execute("INSERT INTO users (name) VALUES (?)", "Bob");
    tx.execute("INSERT INTO logs (msg) VALUES (?)", "Added Bob");
    tx.commit();
}
```

## Prepared Statements

```java
PreparedStatement stmt = conn.prepare(
    "SELECT * FROM users WHERE id = ?"
);

try {
    for (int i = 1; i <= 100; i++) {
        ResultSet rs = stmt.query(i);
        while (rs.next()) {
            System.out.println(rs.getString("name"));
        }
    }
} finally {
    stmt.close();
}
```

## Batch Operations

```java
// Batch insert
try (Batch batch = conn.createBatch()) {
    for (int i = 0; i < 1000; i++) {
        batch.add(
            "INSERT INTO logs (msg) VALUES (?)",
            "Log entry " + i
        );
    }
    int[] results = batch.execute();
    System.out.println("Inserted " + results.length + " rows");
}
```

## Error Handling

```java
try {
    conn.execute("INVALID SQL");
} catch (LibsqlException e) {
    System.err.println("SQLite error: " + e.getCode() + " - " + e.getMessage());
} catch (Exception e) {
    System.err.println("Other error: " + e.getMessage());
}
```

## Type Mappings

| Java Type | SQLite Type |
|-----------|-------------|
| `Long`, `Integer`, `Short`, `Byte` | INTEGER |
| `Double`, `Float` | REAL |
| `String` | TEXT |
| `byte[]` | BLOB |
| `Boolean` | INTEGER (0 or 1) |
| `LocalDateTime` | TEXT (ISO 8601) |
| `null` | NULL |

## Spring Boot Integration

```java
@Configuration
public class DatabaseConfig {
    
    @Bean
    public Database libsqlDatabase(
        @Value("${libsql.url}") String url,
        @Value("${libsql.token}") String token
    ) {
        return Database.open(url, token);
    }
    
    @Bean
    public Connection libsqlConnection(Database db) {
        return db.connect();
    }
}

@Service
public class UserService {
    @Autowired
    private Connection conn;
    
    public User getUser(long id) {
        Optional<Row> row = conn.queryRow(
            "SELECT * FROM users WHERE id = ?",
            id
        );
        return row.map(this::mapToUser).orElse(null);
    }
    
    private User mapToUser(Row row) {
        User user = new User();
        user.setId(row.getLong("id"));
        user.setName(row.getString("name"));
        return user;
    }
}
```

## Best Practices

1. **Always close resources** - Use try-with-resources
2. **Use transactions** for multi-statement operations
3. **Use prepared statements** for repeated queries
4. **Handle LibsqlException** for SQLite errors
5. **Configure connection pooling** for multi-threaded apps

## API Reference

See [Java documentation](https://github.com/tursodatabase/libsql-java) for complete API reference.

## Next Steps

- [Go Binding](./go-binding.md)
- [JavaScript Binding](./javascript-binding.md)
- [Python Binding](./python-binding.md)