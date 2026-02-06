# WebAssembly Binding

## Overview

The WebAssembly binding allows you to use libSQL/Turso in browser environments and WebAssembly runtimes like Deno and Cloudflare Workers.

## Installation

```bash
npm install @libsql/wasm
```

## Quick Start

```javascript
import { createClient } from '@libsql/wasm';

async function main() {
    // In-memory database (browser)
    const client = createClient({ url: ':memory:' });
    
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
    
    const result = await client.execute('SELECT * FROM users');
    for (const row of result.rows) {
        console.log(`${row.id}: ${row.name}`);
    }
}

main();
```

## Browser Usage

### Vanilla JavaScript

```html
<!DOCTYPE html>
<html>
<head>
    <script type="module">
        import { createClient } from 'https://esm.sh/@libsql/wasm';
        
        async function init() {
            const client = createClient({ url: ':memory:' });
            
            await client.execute(`
                CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)
            `);
            
            await client.execute({
                sql: 'INSERT INTO test (value) VALUES (?)',
                args: ['Hello from browser!']
            });
            
            const result = await client.execute('SELECT * FROM test');
            document.getElementById('output').textContent = 
                JSON.stringify(result.rows, null, 2);
        }
        
        init();
    </script>
</head>
<body>
    <pre id="output"></pre>
</body>
</html>
```

### React

```jsx
import { useEffect, useState } from 'react';
import { createClient } from '@libsql/wasm';

function DatabaseComponent() {
    const [client, setClient] = useState(null);
    const [data, setData] = useState([]);
    
    useEffect(() => {
        const init = async () => {
            const db = createClient({ url: ':memory:' });
            
            await db.execute(`
                CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)
            `);
            
            await db.execute({
                sql: 'INSERT INTO items (name) VALUES (?)',
                args: ['Item 1']
            });
            
            setClient(db);
        };
        
        init();
    }, []);
    
    const loadData = async () => {
        if (!client) return;
        
        const result = await client.execute('SELECT * FROM items');
        setData(result.rows);
    };
    
    return (
        <div>
            <button onClick={loadData}>Load Data</button>
            <pre>{JSON.stringify(data, null, 2)}</pre>
        </div>
    );
}
```

## Cloudflare Workers

```javascript
// worker.js
import { createClient } from '@libsql/wasm';

export default {
    async fetch(request, env, ctx) {
        // Create in-memory database per request
        const client = createClient({ url: ':memory:' });
        
        await client.execute(`
            CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY,
                path TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        `);
        
        await client.execute({
            sql: 'INSERT INTO requests (path) VALUES (?)',
            args: [new URL(request.url).pathname]
        });
        
        const result = await client.execute(
            'SELECT COUNT(*) as count FROM requests'
        );
        
        return new Response(JSON.stringify({
            totalRequests: result.rows[0].count
        }), {
            headers: { 'Content-Type': 'application/json' }
        });
    }
};
```

## Deno

```typescript
import { createClient } from "https://esm.sh/@libsql/wasm";

const client = createClient({ url: ":memory:" });

await client.execute(`
    CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)
`);

await client.execute({
    sql: "INSERT INTO users (name) VALUES (?)",
    args: ["Alice"]
});

const result = await client.execute("SELECT * FROM users");
console.log(result.rows);
```

## Node.js with WASM

```javascript
import { createClient } from '@libsql/wasm';

// Use WASM build instead of native
const client = createClient({ url: ':memory:' });

// Same API as regular client
await client.execute(`
    CREATE TABLE test (id INTEGER PRIMARY KEY)
`);
```

## Limitations

WebAssembly binding has some limitations compared to native builds:

| Feature | WASM | Native |
|---------|------|--------|
| In-memory databases | ✅ | ✅ |
| File-based databases | ❌ | ✅ |
| Remote databases | ✅ | ✅ |
| Embedded replicas | ❌ | ✅ |
| Vector search | ⚠️ | ✅ |
| Encryption | ❌ | ✅ |
| io_uring | ❌ | ✅ |

## Best Practices

1. **Use in-memory databases** - File system access not available
2. **Persist data remotely** - Use Turso Cloud for persistence
3. **Handle initialization** - WASM module loading is async
4. **Consider bundle size** - WASM file is ~2MB

## Example: Offline-First PWA

```javascript
// db.js
import { createClient } from '@libsql/wasm';

class OfflineDatabase {
    constructor() {
        this.client = null;
        this.syncUrl = 'libsql://mydb-org.turso.io';
    }
    
    async init() {
        this.client = createClient({ url: ':memory:' });
        
        await this.client.execute(`
            CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY,
                text TEXT,
                completed BOOLEAN DEFAULT 0,
                synced BOOLEAN DEFAULT 0
            )
        `);
    }
    
    async addTodo(text) {
        await this.client.execute({
            sql: 'INSERT INTO todos (text, synced) VALUES (?, 0)',
            args: [text]
        });
        
        // Try to sync if online
        if (navigator.onLine) {
            await this.sync();
        }
    }
    
    async sync() {
        // Sync with Turso Cloud
        const unsynced = await this.client.execute(
            'SELECT * FROM todos WHERE synced = 0'
        );
        
        // Send to server...
        
        await this.client.execute(
            'UPDATE todos SET synced = 1 WHERE synced = 0'
        );
    }
    
    async getTodos() {
        const result = await this.client.execute(
            'SELECT * FROM todos ORDER BY id DESC'
        );
        return result.rows;
    }
}

export const db = new OfflineDatabase();
```

## API Reference

The WASM binding uses the same API as the regular JavaScript client. See [JavaScript Binding](./javascript-binding.md) for full documentation.

## Next Steps

- [JavaScript Binding](./javascript-binding.md)
- [Browser Integration Guide](https://docs.turso.tech/libsql/wasm)
- [Turso Cloud Setup](../../turso-cloud/01-overview.md)