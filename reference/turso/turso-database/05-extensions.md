# Extensions

## Overview

libSQL extends SQLite with additional functionality for modern application needs. These extensions are built-in and don't require external dependencies.

## JSON Extension

### Parsing and Querying JSON
```sql
-- Parse JSON string
SELECT json('{"name": "Alice", "age": 30}');

-- Extract values
SELECT json_extract('{"name": "Alice", "age": 30}', '$.name');
-- Returns: "Alice"

-- Nested extraction
SELECT json_extract(data, '$.address.city') FROM users;

-- Array operations
SELECT json_array_length('[1, 2, 3, 4]');
-- Returns: 4

-- Array element access
SELECT json_extract('["a", "b", "c"]', '$[1]');
-- Returns: "b"
```

### JSON Functions
```sql
-- Check if valid JSON
SELECT json_valid('{"key": "value"}');
-- Returns: 1 (true)

-- Type detection
SELECT json_type('{"name": "Alice"}', '$.name');
-- Returns: "text"

-- Create JSON objects
SELECT json_object('name', 'Alice', 'age', 30);
-- Returns: {"name":"Alice","age":30}

-- Create JSON arrays
SELECT json_array('a', 'b', 'c');
-- Returns: ["a","b","c"]

-- Modify JSON
SELECT json_set('{"name": "Alice"}', '$.age', 30);
-- Returns: {"name":"Alice","age":30}

-- Remove keys
SELECT json_remove('{"a": 1, "b": 2}', '$.a');
-- Returns: {"b":2}

-- Merge JSON
SELECT json_patch('{"a": 1}', '{"b": 2}');
-- Returns: {"a":1,"b":2}
```

### JSON Table Functions
```sql
-- Convert JSON array to table rows
SELECT * FROM json_each('[1, 2, 3]');
-- Returns rows: 1, 2, 3

-- With object keys
SELECT key, value FROM json_each('{"a": 1, "b": 2}');
-- Returns: (a, 1), (b, 2)

-- Complex example
SELECT 
    json_extract(value, '$.id') as user_id,
    json_extract(value, '$.name') as user_name
FROM json_each('[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]');
```

### Storing JSON in libSQL
```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    event_type TEXT,
    payload JSON,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert JSON data
INSERT INTO events (event_type, payload) 
VALUES ('user_signup', '{"user_id": 123, "email": "alice@example.com"}');

-- Query with JSON extraction
SELECT 
    event_type,
    json_extract(payload, '$.user_id') as user_id,
    json_extract(payload, '$.email') as email
FROM events
WHERE event_type = 'user_signup';
```

## Full-Text Search

### Setting Up FTS5
```sql
-- Create FTS5 virtual table
CREATE VIRTUAL TABLE articles USING fts5(
    title,
    content,
    tokenize='porter'  -- Stemming support
);

-- Insert documents
INSERT INTO articles (title, content) 
VALUES ('SQLite Tutorial', 'This is a comprehensive guide to SQLite.');

-- Search documents
SELECT * FROM articles WHERE articles MATCH 'sqlite';

-- Phrase search
SELECT * FROM articles WHERE articles MATCH '"comprehensive guide"';

-- Boolean queries
SELECT * FROM articles WHERE articles MATCH 'sqlite AND tutorial';
SELECT * FROM articles WHERE articles MATCH 'sqlite OR postgresql';
SELECT * FROM articles WHERE articles MATCH 'database NOT oracle';
```

### FTS5 Ranking
```sql
-- Rank results by relevance
SELECT 
    title,
    content,
    rank
FROM articles
WHERE articles MATCH 'sqlite tutorial'
ORDER BY rank;

-- Highlight matches
SELECT 
    highlight(articles, 0, '<b>', '</b>') as title_highlighted,
    highlight(articles, 1, '<b>', '</b>') as content_highlighted
FROM articles
WHERE articles MATCH 'sqlite';
```

### FTS5 Auxiliary Functions
```sql
-- Get snippet around match
SELECT snippet(articles, 1, '<b>', '</b>', '...', 10) 
FROM articles 
WHERE articles MATCH 'tutorial';

-- Count matches
SELECT matchinfo(articles) FROM articles WHERE articles MATCH 'sqlite';

-- Get offsets of matches
SELECT offsets(articles) FROM articles WHERE articles MATCH 'sqlite';
```

## Cryptographic Extension

### Hash Functions
```sql
-- SHA-256 hash
SELECT hex(sha256('hello world'));
-- Returns: b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9

-- SHA-512 hash
SELECT hex(sha512('hello world'));

-- MD5 hash
SELECT hex(md5('hello world'));
```

### HMAC
```sql
-- HMAC-SHA256
SELECT hex(hmac_sha256('secret_key', 'message'));

-- HMAC-SHA512
SELECT hex(hmac_sha512('secret_key', 'message'));
```

### Password Hashing
```sql
-- Argon2 password hashing (if enabled)
SELECT argon2_hash('password', 'salt', 3, 65536, 1, 32);

-- Verify password
SELECT argon2_verify(hash, 'password');
```

## Math Extension

### Mathematical Functions
```sql
-- Trigonometric
SELECT sin(radians(30));    -- 0.5
SELECT cos(radians(60));    -- 0.5
SELECT tan(radians(45));    -- 1.0

-- Logarithms
SELECT log(100);            -- 2.0
SELECT log10(100);          -- 2.0
SELECT log2(8);             -- 3.0

-- Powers and roots
SELECT pow(2, 10);          -- 1024
SELECT sqrt(16);            -- 4.0
SELECT cbrt(27);            -- 3.0

-- Rounding
SELECT ceil(3.2);           -- 4
SELECT floor(3.8);          -- 3
SELECT round(3.14159, 2);   -- 3.14

-- Constants
SELECT pi();                -- 3.14159265358979
```

### Statistical Functions
```sql
-- Aggregate statistics
SELECT 
    avg(salary) as mean,
    median(salary) as median,
    stdev(salary) as std_dev,
    variance(salary) as variance
FROM employees;
```

## Fuzzy Search Extension

### Soundex
```sql
-- Phonetic matching
SELECT soundex('Smith');    -- S530
SELECT soundex('Smyth');    -- S530

-- Compare similar-sounding names
SELECT * FROM users WHERE soundex(name) = soundex('Jon');
-- Matches: John, Jon, Joan
```

### Levenshtein Distance
```sql
-- Edit distance calculation
SELECT editdist3('kitten', 'sitting');  -- 3

-- Fuzzy matching
SELECT * FROM products 
WHERE editdist3(name, 'ipone') <= 2;
-- Matches: "iPhone", "iphone", "i-Phone"
```

## UUID Extension

### UUID Generation
```sql
-- Generate random UUID v4
SELECT uuid();              -- a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11

-- Generate UUID from string
SELECT uuid_str('550e8400-e29b-41d4-a716-446655440000');

-- Validate UUID
SELECT uuid_valid('550e8400-e29b-41d4-a716-446655440000');
-- Returns: 1
```

## Date/Time Extension

### Extended Date Functions
```sql
-- Parse various date formats
SELECT strftime('%Y-%m-%d', '2024-01-15');

-- Date arithmetic
SELECT date('now', '+1 day');           -- Tomorrow
SELECT date('now', '-1 month');         -- Last month
SELECT date('now', '+3 hours');         -- In 3 hours

-- Formatting
SELECT datetime('now', 'localtime');    -- Local time
SELECT unixepoch('now');                -- Unix timestamp
SELECT julianday('now');                -- Julian day

-- Date parts
SELECT 
    strftime('%Y', 'now') as year,
    strftime('%m', 'now') as month,
    strftime('%d', 'now') as day,
    strftime('%W', 'now') as week_of_year;
```

## CSV Extension

### Importing CSV
```sql
-- Create virtual table from CSV file
CREATE VIRTUAL TABLE temp.users USING csv(
    filename='users.csv',
    header=true
);

-- Query CSV data
SELECT * FROM temp.users WHERE age > 25;

-- Import into real table
INSERT INTO real_users SELECT * FROM temp.users;
```

### Exporting to CSV
```sql
-- Export query results to CSV
.headers on
.mode csv
.output users_export.csv
SELECT * FROM users;
.output stdout
```

## Extension Usage in Rust

```rust
use libsql::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Builder::new_local("app.db").build().await?;
    let conn = db.connect()?;
    
    // Create FTS5 table
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
            title, 
            content,
            tokenize='porter'
        )",
        (),
    ).await?;
    
    // Insert with JSON
    conn.execute(
        "INSERT INTO events (type, data) VALUES (?, json(?))",
        ("user_action", r#"{"action": "click", "element": "button"}"#),
    ).await?;
    
    // Query with JSON
    let mut rows = conn.query(
        "SELECT json_extract(data, '$.action') as action FROM events",
        (),
    ).await?;
    
    while let Some(row) = rows.next().await? {
        let action: String = row.get(0)?;
        println!("Action: {}", action);
    }
    
    // Full-text search
    let mut rows = conn.query(
        "SELECT title, snippet(search_index, 0, '<b>', '</b>', '...', 10)
         FROM search_index 
         WHERE search_index MATCH ?",
        ["sqlite tutorial"],
    ).await?;
    
    Ok(())
}
```

## Next Steps

- **Advanced Features**: [06-advanced-features.md](./06-advanced-features.md)
- **MCP Server**: [07-mcp-server.md](./07-mcp-server.md)