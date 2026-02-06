# Community Extensions Overview

> Third-party DuckDB extensions — distributed and signed via centralized CI

## What Are Community Extensions?

Community Extensions are DuckDB extensions **not maintained by the DuckDB team**. They follow the same architecture as core extensions but are contributed by external developers.

```
┌─────────────────────────────────────────────────────┐
│                 Extension Ecosystem                 │
├─────────────────────────────────────────────────────┤
│  Core Extensions     │  Community Extensions       │
│  ──────────────────  │  ─────────────────────────  │
│  • Maintained by     │  • Maintained by community  │
│    DuckDB Labs       │  • Distributed via          │
│  • Primary/Secondary │    community-extensions.    │
│    support tiers     │    duckdb.org               │
│  • Signed by DuckDB  │  • Signed by CI             │
│  • In core repos     │  • In community repo        │
└─────────────────────────────────────────────────────┘
```

## Architecture

Community extensions use the same build and distribution pipeline:

```
Contributor Repo      Community Extensions CI        Distribution
────────────────      ─────────────────────────    ─────────────
[Extension Code]  →   [Build · Test · Sign]    →   [community-
     ↑                    ↓                        extensions.
  Developer          • Multi-platform builds        duckdb.org]
  maintains          • Security scanning
                     • Signing with CI key
                     • Distribution manifest
```

## Usage

### Basic Installation

```sql
-- Install from community repository
INSTALL quack FROM community;
LOAD quack;

-- Use the extension
SELECT quack('world');  -- Returns '🐍 Quack world 🐍'
```

### Bash One-Liners

```bash
# List available community extensions
curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq -r '.extensions[] | "\(.name): \(.description)"' | \
    sort

# Search for an extension
curl -s https://community-extensions.duckdb.org/extensions.json | \
    jq -r '.extensions[] | select(.name | contains("crypto")) | .name'

# Check if extension exists
function duckdb-community-check() {
    local name=$1
    curl -s https://community-extensions.duckdb.org/extensions.json | \
        jq -e --arg name "$name" '.extensions[] | select(.name == $name)' > /dev/null
    if [ $? -eq 0 ]; then
        echo "✓ $name available"
    else
        echo "✗ $name not found"
    fi
}
```

## Finding Extensions

### Web Interface

Browse at [duckdb.org/community_extensions/](https://duckdb.org/community_extensions/)

### Programmatic Discovery

```rust
// Fetch and parse extension manifest
use reqwest;
use serde::Deserialize;

#[derive(Deserialize)]
struct ExtensionManifest {
    extensions: Vec<Extension>,
}

#[derive(Deserialize)]
struct Extension {
    name: String,
    description: String,
    version: String,
    repo: String,
}

async fn list_community_extensions() -> Result<Vec<Extension>, Box<dyn std::error::Error>> {
    let manifest: ExtensionManifest = reqwest::get(
        "https://community-extensions.duckdb.org/extensions.json"
    ).await?.json().await?;
    
    Ok(manifest.extensions)
}
```

## Developing Community Extensions

### Repository Structure

```
my-duckdb-extension/
├── CMakeLists.txt          # Build configuration
├── extension_config.cmake  # Extension metadata
├── src/
│   ├── my_extension.cpp    # C++ implementation
│   └── include/
└── test/
    └── sql/
        └── my_test.test    # SQLLogicTest
```

### Submission Process

1. **Develop** extension using [extension-template](https://github.com/duckdb/extension-template)
2. **Test** locally with `make test`
3. **Submit** PR to [community-extensions](https://github.com/duckdb/community-extensions)
4. **Review** by DuckDB team (security, quality)
5. **Build** via centralized CI
6. **Distribute** to community-extensions.duckdb.org

### Requirements

| Requirement | Description |
|-------------|-------------|
| Open Source | Public repository with OSI-approved license |
| CI Passing | Tests pass on all platforms |
| Documentation | README with usage examples |
| Security | No malicious code, dependencies vetted |
| Maintenance | Responsive maintainer |

### Example: Extension Configuration

```cmake
# extension_config.cmake
extension_loadable_extension(
    my_extension                    # Extension name
    # Sources
    src/my_extension.cpp
    # Dependencies
    duckdb_static
    # Link libraries
    LINK_LIBS mydep
)

# Metadata
set(EXTENSION_NAME "my_extension")
set(EXTENSION_VERSION "0.1.0")
set(EXTENSION_DESCRIPTION "My awesome extension")
```

## Security Model

### Signing

Community extensions are signed by the Community Extensions CI:

```
[Extension Binary] + [CI Private Key] → [Signature]
```

### Verification on Load

```
[Extension Binary] + [CI Public Key] + [Signature] → ✓ Valid / ✗ Invalid
```

### Trust Levels

| Extension Type | Signing Key | Trust |
|----------------|-------------|-------|
| Core | DuckDB Labs | High |
| Community | Community CI | Medium (audited) |
| Unsigned | None | User discretion |

### Disabling Community Extensions

```sql
-- Disable loading of community extensions
SET allow_community_extensions = false;

-- This also locks the setting (cannot be changed back)
```

```bash
# Environment variable (set before starting DuckDB)
export DUCKDB_ALLOW_COMMUNITY_EXTENSIONS=false
```

## Limitations vs Core Extensions

| Aspect | Core Extensions | Community Extensions |
|--------|-----------------|---------------------|
| Distribution | DuckDB domains | community-extensions.duckdb.org |
| Signing | DuckDB Labs key | Community CI key |
| Autoload | Some | None |
| Support | Community/best-effort | Maintainer only |
| Updates | With DuckDB releases | On maintainer schedule |
| Security audit | DuckDB Labs | Community CI checks |

## Notable Community Extensions

| Extension | Description | Repo |
|-----------|-------------|------|
| `crypto` | Cryptographic functions | duckdb/community-extensions |
| `evalexpr` | Expression evaluation | duckdb/community-extensions |
| `sheetreader` | Excel reading alternative | duckdb/community-extensions |
| `ulid` | ULID generation | duckdb/community-extensions |

Check [full list](https://duckdb.org/community_extensions/list_of_extensions.html) for current extensions.

## When to Use Community Extensions

| Scenario | Recommendation |
|----------|----------------|
| Need specific function | Check if core covers it first |
| Core extension missing feature | Check community alternatives |
| Production use | Audit extension code first |
| Security-critical | Prefer core extensions |
| Edge cases | Community may have solution |

## Comparison: Extension Sources

```bash
# Core (most trusted)
INSTALL httpfs;  -- from core by default

# Community (third-party, signed)
INSTALL crypto FROM community;

# Custom / Self-hosted (your responsibility)
INSTALL myext FROM 'https://my-repo.example.com';

# Local file (development)
LOAD '/path/to/myext.duckdb_extension';
```

## Bash Utilities

```bash
#!/bin/bash
# duckdb-ext-community.sh — Community extension helpers

COMMUNITY_URL="https://community-extensions.duckdb.org/extensions.json"

# List all community extensions
list_community() {
    curl -s "$COMMUNITY_URL" | \
        jq -r '.extensions[] | "\(.name) | \(.description) | \(.repo)"' | \
        column -t -s '|'
}

# Get extension info
info_community() {
    local name=$1
    curl -s "$COMMUNITY_URL" | \
        jq -r --arg n "$name" '.extensions[] | select(.name == $n)'
}

# Check if installed locally
is_installed() {
    local name=$1
    duckdb -csv -c "SELECT installed FROM duckdb_extensions() WHERE extension_name='$name'" | tail -1
}

# Install with verification
install_safe() {
    local name=$1
    
    # Verify in manifest
    if ! info_community "$name" | grep -q name; then
        echo "✗ $name not in community registry"
        return 1
    fi
    
    # Install
    duckdb -c "INSTALL $name FROM community; LOAD $name;"
    
    echo "✓ $name installed and loaded"
}

# Main
command=$1
shift
case $command in
    list) list_community ;;
    info) info_community "$1" ;;
    check) is_installed "$1" ;;
    install) install_safe "$1" ;;
    *) echo "Usage: $0 {list|info|check|install} [name]" ;;
esac
```
