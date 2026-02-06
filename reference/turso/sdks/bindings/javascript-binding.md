# JavaScript Binding

## Installation

```bash
npm install @libsql/client
# or
yarn add @libsql/client
# or
pnpm add @libsql/client
```

## Quick Start

```javascript
import { createClient } from '@libsql/client';

// Local database
const client = createClient({
  url: 'file:mydb.db',
});

// Turso Cloud
const client = createClient({
  url: 'libsql://mydb-org.turso.io',
  authToken: 'your-auth-token',
});

// Execute SQL
await client.execute(`
  CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    name TEXT
  )
`);

await client.execute({
  sql: 'INSERT INTO users (name) VALUES (?)',
  args: ['Alice'],
});

// Query
const result = await client.execute('SELECT * FROM users');
for (const row of result.rows) {
  console.log(`${row.id}: ${row.name}`);
}
```

## Database Connection

### Local Database

```javascript
import { createClient } from '@libsql/client';

// File-based
const client = createClient({
  url: 'file:./mydb.db',
});

// In-memory
const client = createClient({
  url: ':memory:',
});

// With options
const client = createClient({
  url: 'file:./mydb.db',
  syncUrl: 'libsql://mydb-org.turso.io',
  authToken: 'your-auth-token',
});
```

### Remote Database

```javascript
const client = createClient({
  url: 'libsql://mydb-org.turso.io',
  authToken: 'your-auth-token',
});

// With custom fetch
const client = createClient({
  url: 'libsql://mydb-org.turso.io',
  authToken: 'your-auth-token',
  fetch: (request) => {
    // Custom fetch implementation
    return fetch(request);
  },
});
```

### Embedded Replica

```javascript
const client = createClient({
  url: 'file:./local-replica.db',
  syncUrl: 'libsql://mydb-org.turso.io',
  authToken: 'your-auth-token',
});

// Manual sync
await client.sync();
```

## CRUD Operations

### Create

```javascript
// Insert single row
const result = await client.execute({
  sql: 'INSERT INTO users (name, email) VALUES (?, ?)',
  args: ['Alice', 'alice@example.com'],
});

console.log(`Inserted row: ${result.lastInsertRowid}`);

// Insert multiple
await client.batch([
  {
    sql: 'INSERT INTO users (name, email) VALUES (?, ?)',
    args: ['Bob', 'bob@example.com'],
  },
  {
    sql: 'INSERT INTO users (name, email) VALUES (?, ?)',
    args: ['Charlie', 'charlie@example.com'],
  },
], 'write');
```

### Read

```javascript
// Query single row
const result = await client.execute({
  sql: 'SELECT name FROM users WHERE id = ?',
  args: [1],
});

if (result.rows.length > 0) {
  console.log(result.rows[0].name);
}

// Query multiple rows
const result = await client.execute('SELECT id, name, email FROM users');
for (const row of result.rows) {
  console.log(`${row.id}: ${row.name} (${row.email})`);
}

// With typed results
const result = await client.execute('SELECT * FROM users');
/** @type {{id: number, name: string, email: string}[]} */
const users = result.rows;
```

### Update

```javascript
const result = await client.execute({
  sql: 'UPDATE users SET name = ? WHERE id = ?',
  args: ['Alice Smith', 1],
});

console.log(`Updated ${result.rowsAffected} rows`);
```

### Delete

```javascript
const result = await client.execute({
  sql: 'DELETE FROM users WHERE id = ?',
  args: [1],
});

console.log(`Deleted ${result.rowsAffected} rows`);
```

## Transactions

### Interactive Transactions

```javascript
const transaction = await client.transaction();

try {
  await transaction.execute({
    sql: 'INSERT INTO users (name) VALUES (?)',
    args: ['Bob'],
  });
  
  await transaction.execute({
    sql: 'INSERT INTO logs (msg) VALUES (?)',
    args: ['Added Bob'],
  });
  
  await transaction.commit();
} catch (e) {
  await transaction.rollback();
  throw e;
}
```

### Batch Transactions

```javascript
await client.batch([
  {
    sql: 'INSERT INTO users (name) VALUES (?)',
    args: ['Bob'],
  },
  {
    sql: 'INSERT INTO logs (msg) VALUES (?)',
    args: ['Added Bob'],
  },
], 'write');
```

## Vector Operations

### Storing Vectors

```javascript
// Create table
await client.execute(`
  CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding F32_BLOB(384)
  )
`);

// Insert document with embedding
const embedding = new Float32Array([0.1, 0.2, 0.3 /* ... */]);

await client.execute({
  sql: 'INSERT INTO documents (content, embedding) VALUES (?, ?)',
  args: ['Hello world', embedding],
});
```

### Vector Search

```javascript
const queryVector = new Float32Array([0.1, 0.2, /* ... */]);

const result = await client.execute({
  sql: `
    SELECT content, vector_distance_cosine(embedding, vector(?)) as distance
    FROM documents
    ORDER BY distance
    LIMIT 5
  `,
  args: [queryVector],
});

for (const row of result.rows) {
  console.log(`${row.content} (distance: ${row.distance})`);
}
```

## Streaming Results

```javascript
// Stream large result sets
const stmt = await client.prepare('SELECT * FROM large_table');

for await (const row of stmt) {
  console.log(row);
}

stmt.finalize();
```

## Prepared Statements

```javascript
const stmt = await client.prepare('SELECT * FROM users WHERE id = ?');

for (let i = 1; i <= 100; i++) {
  const result = await stmt.execute(i);
  if (result.rows.length > 0) {
    console.log(result.rows[0]);
  }
}

await stmt.finalize();
```

## Migrations

```javascript
async function migrate(client) {
  const result = await client.execute('PRAGMA user_version');
  const version = result.rows[0].user_version;
  
  if (version < 1) {
    await client.execute(`
      CREATE TABLE users (
        id INTEGER PRIMARY KEY,
        name TEXT
      )
    `);
    await client.execute('PRAGMA user_version = 1');
  }
  
  if (version < 2) {
    await client.execute('ALTER TABLE users ADD COLUMN email TEXT');
    await client.execute('PRAGMA user_version = 2');
  }
}
```

## Error Handling

```javascript
import { LibsqlError } from '@libsql/client';

try {
  await client.execute('INVALID SQL');
} catch (e) {
  if (e instanceof LibsqlError) {
    console.error('SQLite error:', e.code, e.message);
  } else {
    console.error('Other error:', e);
  }
}
```

## TypeScript Support

```typescript
import { createClient, Client } from '@libsql/client';

interface User {
  id: number;
  name: string;
  email: string;
}

const client: Client = createClient({
  url: 'file:mydb.db',
});

async function getUsers(): Promise<User[]> {
  const result = await client.execute<User>('SELECT * FROM users');
  return result.rows;
}
```

## React Integration

```javascript
// hooks/useDatabase.js
import { useEffect, useState } from 'react';
import { createClient } from '@libsql/client';

export function useDatabase(url, authToken) {
  const [client, setClient] = useState(null);
  const [error, setError] = useState(null);
  
  useEffect(() => {
    try {
      const db = createClient({ url, authToken });
      setClient(db);
    } catch (e) {
      setError(e);
    }
  }, [url, authToken]);
  
  return { client, error };
}

// Usage
function App() {
  const { client, error } = useDatabase(
    'libsql://mydb-org.turso.io',
    'your-auth-token'
  );
  
  if (error) return <div>Error: {error.message}</div>;
  if (!client) return <div>Loading...</div>;
  
  return <UserList client={client} />;
}
```

## Best Practices

1. **Always await promises** - All operations are async
2. **Use prepared statements** for repeated queries
3. **Batch operations** when possible
4. **Handle errors** explicitly
5. **Close statements** when done
6. **Use transactions** for multi-statement operations

## API Reference

See [@libsql/client documentation](https://github.com/tursodatabase/libsql-client-ts) for complete API reference.

## Next Steps

- [Go Binding](./go-binding.md)
- [Python Binding](./python-binding.md)
- [Rust Crate](../rust-crate/01-overview.md)