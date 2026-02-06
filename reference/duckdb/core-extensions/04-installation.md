# Installing Extensions — Advanced Methods

> Repository management, versioning, and troubleshooting

## Repository Configuration

### Built-in Repositories

| Alias | URL | Purpose |
|-------|-----|---------|
| `core` | `http://extensions.duckdb.org` | Stable releases |
| `core_nightly` | `http://nightly-extensions.duckdb.org` | Latest builds |
| `community` | `http://community-extensions.duckdb.org` | Third-party |
| `local_build_debug` | `./build/debug/repository` | Dev builds |
| `local_build_release` | `./build/release/repository` | Dev builds |

### Switching Repositories

```sql
-- Set default repository for all installs
SET custom_extension_repository = 'http://nightly-extensions.duckdb.org';

-- Per-install specification (recommended)
INSTALL spatial FROM core;
INSTALL aws FROM core_nightly;

-- Force reinstall from different repo
FORCE INSTALL httpfs FROM core_nightly;
```

### Multi-Repository Workflow

```sql
-- Core for stability
INSTALL httpfs FROM core;

-- Nightly for bleeding-edge features
INSTALL aws FROM core_nightly;

-- Verify origins
SELECT extension_name, extension_version, installed_from
FROM duckdb_extensions()
WHERE installed;

-- Result:
-- httpfs │ 62d61a417f │ core
-- aws    │ 42c78d3    │ core_nightly
```

## Versioning

### Extension Version Format

Extensions use Git commit hashes for versioning:
```
extension_version = "62d61a417f"  -- Short commit hash
```

### Compatibility Matrix

| DuckDB | Extension ABI | Can Load |
|--------|---------------|----------|
| v1.4.1 | v1.4.1 | Extensions built for v1.4.1 |
| v1.4.2 | v1.4.2 | Extensions built for v1.4.2 |
| nightly | nightly | Same commit only |

Extensions **must** match DuckDB version exactly. Cannot mix versions.

### Updating Extensions

```sql
-- Update all installed extensions
UPDATE EXTENSIONS;

-- Restart required after update
-- Extensions cannot be reloaded in-process
```

### Checking Versions

```bash
# Bash one-liner with jq
function duckdb-ext-versions() {
    duckdb -json -c "
        SELECT 
            extension_name,
            extension_version,
            installed_from,
            loaded
        FROM duckdb_extensions()
        WHERE installed
        ORDER BY installed_from, extension_name
    " | jq -r '.[] | "\(.installed_from)/\(.extension_name)@\(.extension_version) (loaded:\(.loaded))"'
}

# Usage
duckdb-ext-versions
# core/httpfs@62d61a417f (loaded:true)
# core/parquet@1.2.3 (loaded:true)
# core_nightly/aws@42c78d3 (loaded:false)
```

## Installation Paths

### Default Location

```
Linux/macOS: ~/.duckdb/extensions/<version>/<platform>/
Windows:     %USERPROFILE%\.duckdb\extensions\<version>\<platform>\

Examples:
~/.duckdb/extensions/v1.4.1/osx_arm64/
~/.duckdb/extensions/v1.4.1/linux_amd64/
~/.duckdb/extensions/fc2e4b26a6/osx_arm64/  -- nightly
```

### Platforms

| Platform | Architecture | Extension |
|----------|--------------|-----------|
| `osx_arm64` | Apple Silicon | `.duckdb_extension` |
| `osx_amd64` | Intel Mac | `.duckdb_extension` |
| `linux_arm64` | ARM64 Linux | `.duckdb_extension` |
| `linux_amd64` | x86_64 Linux | `.duckdb_extension` |
| `windows_amd64` | x86_64 Windows | `.duckdb_extension` |

### Custom Directory

```sql
-- Change extension storage location
SET extension_directory = '/path/to/extensions';

-- Verify
SELECT current_setting('extension_directory');
```

### Sharing Extensions

Extensions can be shared between clients of same version:

```
DuckDB CLI (v1.4.1)  ──┐
Python (v1.4.1)        ├──► ~/.duckdb/extensions/v1.4.1/  (shared)
Rust (v1.4.1)         ──┘
```

Requirements for sharing:
- Same DuckDB version
- Same platform (architecture)
- Access to extension directory

## Uninstalling Extensions

```bash
# No SQL command — remove files directly
rm ~/.duckdb/extensions/v1.4.1/osx_arm64/spatial.duckdb_extension

# Bash helper
function duckdb-ext-remove() {
    local ext=$1
    local version=$(duckdb -csv -c "SELECT version()" | tail -1)
    local platform="osx_arm64"  # detect or hardcode
    rm ~/.duckdb/extensions/${version}/${platform}/${ext}.duckdb_extension
}

# Remove all extensions
rm -rf ~/.duckdb/extensions/
```

## Network & Proxy

### Environment Variables

```bash
# Standard proxy support
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
export NO_PROXY=localhost,127.0.0.1

# DuckDB respects these
```

### Offline Installation

```bash
# Download manually
curl -O http://extensions.duckdb.org/v1.4.1/osx_arm64/httpfs.duckdb_extension

# Copy to extension directory
mkdir -p ~/.duckdb/extensions/v1.4.1/osx_arm64/
cp httpfs.duckdb_extension ~/.duckdb/extensions/v1.4.1/osx_arm64/

# DuckDB will find it
LOAD httpfs;  -- works without network
```

### Custom Repository (Enterprise)

Host internal extension repository:

```bash
# 1. Mirror DuckDB extensions
mkdir -p /var/duckdb-extensions/v1.4.1/
cd /var/duckdb-extensions/v1.4.1/

# Download all platforms
curl -O http://extensions.duckdb.org/v1.4.1/osx_arm64/httpfs.duckdb_extension
curl -O http://extensions.duckdb.org/v1.4.1/linux_amd64/httpfs.duckdb_extension
# ... etc

# 2. Serve via HTTP
python -m http.server 8080 --directory /var/duckdb-extensions/
```

```sql
-- Client configuration
SET custom_extension_repository = 'http://internal-repo:8080';

-- Or per-install
INSTALL httpfs FROM 'http://internal-repo:8080';
```

## Rust Integration

### Installation via Rust

```rust
// Install and load in one batch
conn.execute_batch(r#"
    INSTALL httpfs;
    LOAD httpfs;
    
    INSTALL spatial;
    LOAD spatial;
"#)?;
```

### Checking Installation Status

```rust
let mut stmt = conn.prepare("
    SELECT extension_name, installed, loaded 
    FROM duckdb_extensions()
    WHERE extension_name = ?
")?;

let (name, installed, loaded): (String, bool, bool) = 
    stmt.query_row(["httpfs"], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
        ))
    })?;

println!("{}: installed={}, loaded={}", name, installed, loaded);
```

### Handling Extension Errors

```rust
use duckdb::Error;

// Extension not found
match conn.execute("LOAD nonexistent", []) {
    Err(Error::DuckDBFailure(err, _)) => {
        if err.message.contains("not found") {
            println!("Extension not installed, installing...");
            conn.execute("INSTALL nonexistent", [])?;
            conn.execute("LOAD nonexistent", [])?;
        }
    }
    Ok(_) => {}
    Err(e) => return Err(e),
}
```

## Troubleshooting

### Common Issues

| Symptom | Cause | Solution |
|---------|-------|----------|
| `Extension not found` | Not installed | `INSTALL name` |
| `Extension version mismatch` | Wrong DuckDB version | `UPDATE EXTENSIONS` |
| `Network error` | Proxy/firewall | Check proxy, use offline install |
| `Platform not found` | Unsupported platform | Build from source |
| `Signature check failed` | Corrupted/unsigned | `FORCE INSTALL` |

### Debugging

```sql
-- Check extension directory
SELECT current_setting('extension_directory');

-- List all extensions with metadata
SELECT * FROM duckdb_extensions();

-- Check settings
SELECT * FROM duckdb_settings() 
WHERE name LIKE '%extension%';
```

### Signature Verification

Extensions are signed by DuckDB team. To disable (not recommended):

```sql
SET allow_unsigned_extensions = true;
SET allow_community_extensions = false;  -- disable community
```

### Build from Source (Last Resort)

```bash
# Clone extension template
git clone https://github.com/duckdb/extension-template.git
cd extension-template

# Build
make configure
make build

# Install locally
LOAD 'build/release/extension/myext/myext.duckdb_extension';
```
