# Storage Backends

GraphFlow provides pluggable storage backends for session persistence.

---

## Storage Trait

All storage backends implement `SessionStorage`:

```rust
#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save(&self, session: Session) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Session>>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

---

## In-Memory Storage

Fast, non-persistent storage for development and testing.

### Usage

```rust
use graph_flow::InMemorySessionStorage;

let storage = Arc::new(InMemorySessionStorage::new());
```

### Characteristics

- **Speed**: ~1μs per operation
- **Persistence**: None (data lost on restart)
- **Concurrency**: Thread-safe via DashMap
- **Use Case**: Development, testing, demos

### Example

```rust
#[tokio::main]
async fn main() -> graph_flow::Result<()> {
    let storage = Arc::new(InMemorySessionStorage::new());
    
    // Create session
    let session = Session::new_from_task(
        "session_001".to_string(),
        "start_task"
    );
    
    // Save
    storage.save(session.clone()).await?;
    
    // Retrieve
    let retrieved = storage.get("session_001").await?;
    assert!(retrieved.is_some());
    
    // Delete
    storage.delete("session_001").await?;
    
    Ok(())
}
```

---

## PostgreSQL Storage

Persistent, scalable storage for production.

### Setup

1. **Database URL**

```bash
export DATABASE_URL="postgresql://user:password@localhost/dbname"
```

2. **Required Schema**

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    graph_id TEXT NOT NULL,
    current_task_id TEXT NOT NULL,
    status_message TEXT,
    context JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_sessions_id ON sessions(id);
```

### Usage

```rust
use graph_flow::PostgresSessionStorage;

let storage = Arc::new(
    PostgresSessionStorage::connect(&database_url).await?
);
```

### Characteristics

- **Speed**: ~5-10ms per operation
- **Persistence**: Full durability
- **Scalability**: Horizontal scaling support
- **Use Case**: Production applications

### Example

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    
    let storage = Arc::new(
        PostgresSessionStorage::connect(&database_url).await?
    );
    
    // Use like any storage
    let session = Session::new_from_task(
        "session_001".to_string(),
        "start_task"
    );
    
    storage.save(session).await?;
    
    let retrieved = storage.get("session_001").await?;
    println!("Retrieved: {:?}", retrieved);
    
    Ok(())
}
```

---

## Custom Storage

Implement your own storage backend:

```rust
use async_trait::async_trait;
use graph_flow::{SessionStorage, Session, Result, GraphError};

pub struct RedisStorage {
    client: redis::Client,
}

#[async_trait]
impl SessionStorage for RedisStorage {
    async fn save(&self, session: Session) -> Result<()> {
        let json = serde_json::to_string(&session)
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        redis::cmd("SET")
            .arg(&session.id)
            .arg(json)
            .query_async(&mut conn)
            .await
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get(&self, id: &str) -> Result<Option<Session>> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        let json: Option<String> = redis::cmd("GET")
            .arg(id)
            .query_async(&mut conn)
            .await
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        match json {
            Some(json) => {
                let session = serde_json::from_str(&json)
                    .map_err(|e| GraphError::StorageError(e.to_string()))?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        redis::cmd("DEL")
            .arg(id)
            .query_async(&mut conn)
            .await
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        Ok(())
    }
}
```

---

## Storage Selection

### Development

```rust
let storage: Arc<dyn SessionStorage> = Arc::new(
    InMemorySessionStorage::new()
);
```

### Production

```rust
let storage: Arc<dyn SessionStorage> = if let Ok(db_url) = std::env::var("DATABASE_URL") {
    match PostgresSessionStorage::connect(&db_url).await {
        Ok(pg) => Arc::new(pg),
        Err(e) => {
            tracing::error!("Failed to connect to PostgreSQL: {}", e);
            Arc::new(InMemorySessionStorage::new())
        }
    }
} else {
    tracing::warn!("DATABASE_URL not set, using in-memory storage");
    Arc::new(InMemorySessionStorage::new())
};
```

---

## Best Practices

### 1. Use Trait Objects for Flexibility

```rust
// Good: Can switch backends
let storage: Arc<dyn SessionStorage> = Arc::new(
    InMemorySessionStorage::new()
);

// Later switch to:
let storage: Arc<dyn SessionStorage> = Arc::new(
    PostgresSessionStorage::connect(&url).await?
);
```

### 2. Handle Storage Errors

```rust
match storage.get(session_id).await {
    Ok(Some(session)) => session,
    Ok(None) => {
        // Create new session
        Session::new_from_task(session_id.to_string(), start_task)
    }
    Err(e) => {
        tracing::error!("Storage error: {}", e);
        // Fallback or error
        return Err(e);
    }
}
```

### 3. Implement Fallback

```rust
async fn get_storage() -> Arc<dyn SessionStorage> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if let Ok(pg) = PostgresSessionStorage::connect(&url).await {
            return Arc::new(pg);
        }
    }
    
    tracing::warn!("Using in-memory storage");
    Arc::new(InMemorySessionStorage::new())
}
```

### 4. Monitor Performance

```rust
use std::time::Instant;

let start = Instant::now();
storage.save(session).await?;
let duration = start.elapsed();

tracing::info!(
    "Session saved in {:?}",
    duration
);
```

---

## Next Steps

- [Context and State](./context.md) - Managing workflow state
- [Graph Execution](./graph-execution.md) - Executing workflows
- [Examples](../examples/simple.md) - Real-world usage
