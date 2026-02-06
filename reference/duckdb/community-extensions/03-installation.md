# Installing Community Extensions

> From discovery to production deployment

## Installation Methods

### SQL (Recommended)

```sql
-- From community repository
INSTALL quack FROM community;
LOAD quack;

-- Verify
SELECT extension_name, loaded, installed_from
FROM duckdb_extensions()
WHERE extension_name = 'quack';
```

### Rust

```rust
// Install and load
conn.execute_batch(r#"
    INSTALL crypto FROM community;
    LOAD crypto;
"#)?;

// Use extension functions
let hash: String = conn.query_row(
    "SELECT hex(md5('hello'))",
    [],
    |row| row.get(0)
)?;
```

### Python

```python
import duckdb

con = duckdb.connect()

# Install from community
con.execute("INSTALL crypto FROM community")
con.execute("LOAD crypto")

# Use
result = con.execute("SELECT hex(md5('hello'))").fetchone()
print(result)  # ('5d41402abc4b2a76b9719d911017c592',)
```

## Discovery

### Web Browser

Browse all extensions at:
[duckdb.org/community_extensions/list_of_extensions.html](https://duckdb.org/community_extensions/list_of_extensions.html)

### Command Line

```bash
# Fetch and list all community extensions
curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq -r '.extensions[] | "\(.name): \(.description)"' | \
    sort | \
    column -t -s ':'

# Search by keyword
curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq -r '.extensions[] | select(.description | contains("crypto")) | .name'

# Get detailed info
curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq -r '.extensions[] | select(.name == "crypto") | {name, description, repo, version}'
```

### Programmatic (Rust)

```rust
use reqwest;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Extension {
    name: String,
    description: String,
    repo: String,
    version: String,
}

#[derive(Deserialize)]
struct Manifest {
    extensions: Vec<Extension>,
}

async fn find_extension(query: &str) -> Result<Vec<Extension>, Box<dyn std::error::Error>> {
    let manifest: Manifest = reqwest::get(
        "https://community-extensions.duckdb.org/extensions.json"
    ).await?.json().await?;
    
    let matches: Vec<_> = manifest.extensions
        .into_iter()
        .filter(|e| {
            e.name.contains(query) || 
            e.description.to_lowercase().contains(&query.to_lowercase())
        })
        .collect();
    
    Ok(matches)
}
```

## Installation Workflow

### 1. Discovery Phase

```bash
# Find extensions for a use case
duckdb-community search "hash"      # hypothetical CLI

# Or browse web
echo "Visit: https://duckdb.org/community_extensions/list_of_extensions.html"
```

### 2. Evaluation Phase

```sql
-- Install in test environment
INSTALL crypto FROM community;
LOAD crypto;

-- Verify source
SELECT extension_name, installed_from, extension_version
FROM duckdb_extensions()
WHERE extension_name = 'crypto';

-- Test functionality
SELECT hex(md5('test'));

-- Check repository
-- Visit GitHub repo linked in extensions list
```

### 3. Approval Phase

```bash
# Security review checklist
REPO_URL=$(curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq -r '.extensions[] | select(.name == "crypto") | .repo')

echo "Review: $REPO_URL"
# Check:
# - Recent activity
# - Code quality
# - Dependencies
# - License
```

### 4. Production Phase

```bash
# Option A: Use community repo (with allow_community_extensions=true)
duckdb -c "INSTALL crypto FROM community; LOAD crypto"

# Option B: Mirror to internal repo (recommended for enterprise)
# See offline installation below
```

## Offline Installation

### Download for Air-Gapped Systems

```bash
# 1. Get manifest
MANIFEST_URL="https://community-extensions.duckdb.org/v1.4.1/osx_arm64/extensions.list"
curl -s "$MANIFEST_URL" > extensions.list

# 2. Download specific extension
EXTENSION="crypto"
VERSION="v1.4.1"
PLATFORM="osx_arm64"

URL="https://community-extensions.duckdb.org/${VERSION}/${PLATFORM}/${EXTENSION}.duckdb_extension"
curl -o "${EXTENSION}.duckdb_extension" "$URL"

# 3. Verify checksum (if available)
# (DuckDB verifies signature on LOAD)

# 4. Transfer to air-gapped system
scp "${EXTENSION}.duckdb_extension" airgapped:/tmp/

# 5. Install on air-gapped system
duckdb -c "LOAD '/tmp/${EXTENSION}.duckdb_extension'"
```

### Local Repository Mirror

```bash
#!/bin/bash
# mirror-community-exts.sh — Mirror approved community extensions

APPROVED="crypto ulid evalexpr"
VERSION="v1.4.1"
PLATFORM="osx_arm64"  # Adjust per target
MIRROR_DIR="/var/duckdb-community-extensions"

mkdir -p "$MIRROR_DIR/$VERSION/$PLATFORM"

for ext in $APPROVED; do
    url="https://community-extensions.duckdb.org/${VERSION}/${PLATFORM}/${ext}.duckdb_extension"
    echo "Downloading: $ext"
    curl -fsSL -o "$MIRROR_DIR/$VERSION/$PLATFORM/${ext}.duckdb_extension" "$url"
done

# Create simple index
ls "$MIRROR_DIR/$VERSION/$PLATFORM/" > "$MIRROR_DIR/extensions.list"

echo "Mirror complete: $MIRROR_DIR"
```

Serve via HTTP:
```bash
cd /var/duckdb-community-extensions && python -m http.server 8080
```

Configure DuckDB:
```sql
SET custom_extension_repository = 'http://internal-mirror:8080';
INSTALL crypto;  -- From internal mirror
```

## Version Pinning

### Pin to Specific Extension Version

```sql
-- Force reinstall from community
FORCE INSTALL crypto FROM community;

-- Check exact version
SELECT extension_name, extension_version
FROM duckdb_extensions()
WHERE extension_name = 'crypto';

-- Result: crypto | a1b2c3d4
```

### Pin in Production

```bash
# Document approved versions
cat > approved-extensions.txt << 'EOF'
# name | version | source
crypto | a1b2c3d4 | community
ulid | b2c3d4e5 | community
EOF

# Validation script
while IFS='|' read -r name version source; do
    # Skip comments
    [[ $name =~ ^# ]] && continue
    
    # Trim whitespace
    name=$(echo "$name" | xargs)
    version=$(echo "$version" | xargs)
    
    # Check installed version
    installed=$(duckdb -csv -c "
        SELECT extension_version 
        FROM duckdb_extensions() 
        WHERE extension_name='$name'
    " | tail -1)
    
    if [ "$installed" = "$version" ]; then
        echo "✓ $name @ $version"
    else
        echo "✗ $name: expected $version, got $installed"
    fi
done < approved-extensions.txt
```

## Multi-Platform Deployment

### Detect Platform

```bash
# DuckDB reports platform
PLATFORM=$(duckdb -csv -c "SELECT platform FROM pragma_platform()" | tail -1)
echo "Platform: $PLATFORM"  # e.g., osx_arm64
```

### Download All Platforms

```bash
#!/bin/bash
# download-all-platforms.sh

EXTENSION="crypto"
VERSION="v1.4.1"
PLATFORMS="osx_arm64 osx_amd64 linux_arm64 linux_amd64 windows_amd64"

for platform in $PLATFORMS; do
    mkdir -p "extensions/$VERSION/$platform"
    url="https://community-extensions.duckdb.org/${VERSION}/${platform}/${EXTENSION}.duckdb_extension"
    curl -fsSL -o "extensions/$VERSION/${platform}/${EXTENSION}.duckdb_extension" "$url"
    echo "Downloaded: $platform"
done
```

## Troubleshooting

### Extension Not Found

```bash
# Verify in manifest
curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq -r '.extensions[] | .name' | \
    grep -i "crypto" || echo "Not in community registry"

# Check core extensions (maybe it's core now)
duckdb -c "SELECT * FROM duckdb_extensions() WHERE extension_name='crypto'"
```

### Version Mismatch

```bash
# Check DuckDB version
duckdb -c "SELECT version()"

# Extension may not be built for this version yet
# Wait for update or use FORCE INSTALL (may not work)
```

### Signature Verification Failed

```sql
-- Extension may be corrupted or tampered
-- Re-download
FORCE INSTALL crypto FROM community;

-- Or temporarily allow (DANGER)
SET allow_extensions_metadata_mismatch = true;
```

### Network Issues

```bash
# Test connectivity
curl -I https://community-extensions.duckdb.org/extensions.json

# Use proxy
export HTTP_PROXY=http://proxy.example.com:8080
duckdb -c "INSTALL crypto FROM community"

# Or offline install (see above)
```

## Bash Utilities

```bash
#!/bin/bash
# duckdb-community-cli.sh

COMMUNITY_URL="https://community-extensions.duckdb.org/extensions.json"

list() {
    curl -s "$COMMUNITY_URL" | \
        jq -r '.extensions[] | "\(.name)|\(.description)|\(.repo)"' | \
        column -t -s '|'
}

info() {
    local name=$1
    curl -s "$COMMUNITY_URL" | \
        jq -r --arg n "$name" '.extensions[] | select(.name == $n) | 
        "Name: \(.name)
Version: \(.version)
Repo: \(.repo)
Description: \(.description)"'
}

install() {
    local name=$1
    shift
    duckdb "$@" -c "INSTALL $name FROM community; LOAD $name"
}

case $1 in
    list) list ;;
    info) info "$2" ;;
    install) install "$2" "${@:3}" ;;
    *) echo "Usage: $0 {list|info <name>|install <name> [duckdb-args]}" ;;
esac
```

## Integration with Core Extensions

### Common Workflows

```sql
-- Cloud analytics: httpfs + community crypto
INSTALL httpfs;
INSTALL crypto FROM community;

LOAD httpfs;
LOAD crypto;

-- Query S3 with encrypted columns
CREATE SECRET s3 (TYPE S3, ...);

SELECT 
    id,
    hex(aes_encrypt(secret_data, 'key')) as encrypted
FROM 's3://bucket/data.parquet';
```

### Feature Detection

```sql
-- Check both core and community
SELECT 
    extension_name,
    installed_from,
    loaded,
    CASE installed_from
        WHEN 'core' THEN 'Core'
        ELSE 'Community'
    END as type
FROM duckdb_extensions()
WHERE installed
ORDER BY type, extension_name;
```
