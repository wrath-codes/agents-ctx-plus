# Core Extensions Overview

> Extensions maintained by the DuckDB team — built-in, installable, with autoload support

## Extension Types

| Type | Availability | Installation Required | Example |
|------|--------------|----------------------|---------|
| **Built-in** | Always available | No | `json`, `parquet` |
| **Autoloadable** | Auto-installed on use | No (auto) | `httpfs`, `icu` |
| **Explicit** | Must install & load | Yes | `spatial`, `delta` |

## Repository Sources

```
┌─────────────────────────────────────────┐
│           core (default)                │
│    http://extensions.duckdb.org         │
│    — Stable releases                    │
├─────────────────────────────────────────┤
│           core_nightly                  │
│    http://nightly-extensions.duckdb.org   │
│    — Bleeding edge                      │
├─────────────────────────────────────────┤
│           community                     │
│    http://community-extensions.duckdb.org │
│    — Third-party (see ../community-extensions) │
└─────────────────────────────────────────┘
```

## Installation Methods

### SQL
```sql
-- Default (core repo)
INSTALL httpfs;
LOAD httpfs;

-- Explicit repository
INSTALL spatial FROM core;
INSTALL aws FROM core_nightly;

-- Force reinstall
FORCE INSTALL httpfs FROM core_nightly;

-- Update all
UPDATE EXTENSIONS;
```

### Rust
```rust
conn.execute("INSTALL httpfs", [])?;
conn.execute("LOAD httpfs", [])?;
```

### Python
```python
con.install_extension("httpfs")
con.load_extension("httpfs")
```

## Extension Metadata

```sql
-- List all extensions
SELECT extension_name, installed, loaded, 
       extension_version, installed_from
FROM duckdb_extensions()
ORDER BY extension_name;
```

## Support Tiers

| Tier | Coverage | Extensions |
|------|----------|------------|
| **Primary** | Community support | `json`, `parquet`, `icu`, `httpfs` |
| **Secondary** | Best-effort | `spatial`, `iceberg`, `delta`, `aws`, etc. |

## Listing Extensions

### Bash (one-liners)

```bash
# List installed extensions via CLI
duckdb -c "SELECT extension_name, loaded FROM duckdb_extensions() WHERE installed"

# Pretty-print with bat
duckdb -json -c "SELECT extension_name, description FROM duckdb_extensions() WHERE installed" | bat -l json

# Filter by loaded status
duckdb -csv -c "SELECT extension_name, extension_version FROM duckdb_extensions() WHERE loaded"
```

### Rust

```rust
let mut stmt = conn.prepare("
    SELECT extension_name, loaded, extension_version 
    FROM duckdb_extensions()
")?;

for row in stmt.query_map([], |row| {
    Ok((
        row.get::<_, String>(0)?,
        row.get::<_, bool>(1)?,
        row.get::<_, Option<String>>(2)?,
    ))
})? {
    let (name, loaded, version) = row?;
    println!("{}: loaded={}, version={:?}", name, loaded, version);
}
```

## Extension Directory

Default location:
```
~/.duckdb/extensions/<duckdb_version>/<platform>/
# e.g., ~/.duckdb/extensions/v1.4.1/osx_arm64/
```

Customize:
```sql
SET extension_directory = '/path/to/extensions';
```

## Versioning

Extensions are version-locked to DuckDB releases:
- Extension ABI matches DuckDB version
- Cannot mix extensions from different DuckDB versions
- Update extensions with `UPDATE EXTENSIONS`

## Limitations

1. **Cannot unload** — Once loaded, extension stays until process exit
2. **Cannot reload** — Must restart to load updated extension
3. **Version locked** — Extensions must match DuckDB version

## Cross-References

- [Primary Extensions](02-primary.md) — json, parquet, icu, httpfs
- [Secondary Extensions](03-secondary.md) — spatial, iceberg, delta, mysql, postgres
- [Installation Details](04-installation.md) — Advanced installation methods
