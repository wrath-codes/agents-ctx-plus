# Community Extensions Security

> Signing, verification, and hardening for third-party DuckDB extensions

## Threat Model

```
Threats to Community Extensions:
┌─────────────────────────────────────────────┐
│ 1. Malicious extension code                 │
│    → CI checks, static analysis, review     │
├─────────────────────────────────────────────┤
│ 2. Compromised extension repo               │
│    → Multi-party review for updates         │
├─────────────────────────────────────────────┤
│ 3. Compromised CI/build system              │
│    → Reproducible builds, transparency log  │
├─────────────────────────────────────────────┤
│ 4. Supply chain (dependencies)            │
│    → Dependency pinning, audit              │
├─────────────────────────────────────────────┤
│ 5. Runtime exploits                         │
│    → Sandboxing, permission limits          │
└─────────────────────────────────────────────┘
```

## Signing Architecture

### Key Hierarchy

```
DuckDB Labs (Root Trust)
         │
         ├─► Core Extension Signing Key
         │
         └─► Community CI Signing Key ──► Signs Community Extensions
```

### Signature Verification Flow

```
INSTALL myext FROM community
         │
         ▼
┌─────────────────┐
│ Download        │
│ myext.duckdb_ext│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Verify Signature│
│ (CI Public Key) │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
  Valid    Invalid
    │         │
    ▼         ▼
  LOAD    Reject
  ALLOW   DENY
```

## Security Controls

### 1. Build-time Controls

| Control | Implementation |
|---------|---------------|
| Code Review | PR review by DuckDB team |
| Static Analysis | Automated scanning |
| Dependency Audit | Known vulnerability checking |
| Reproducible Builds | Deterministic build process |
| Test Suite | SQLLogicTest mandatory |

### 2. Distribution Controls

| Control | Implementation |
|---------|---------------|
| HTTPS Only | All downloads over TLS |
| Signature Verification | Ed25519 signatures |
| Manifest | Signed extension list |
| Transparency | Build logs public |

### 3. Runtime Controls

| Setting | Default | Effect |
|---------|---------|--------|
| `allow_unsigned_extensions` | `false` | Reject unsigned extensions |
| `allow_community_extensions` | `true` | Allow community extensions |
| `allow_extensions_metadata_mismatch` | `false` | Strict metadata check |

## Configuration

### Disable Community Extensions (Enterprise)

```sql
-- Permanent disable
SET allow_community_extensions = false;

-- Verify
SELECT current_setting('allow_community_extensions');
-- Result: false
```

```bash
# Environment variable (all clients)
export DUCKDB_ALLOW_COMMUNITY_EXTENSIONS=false

# CLI flag
duckdb -c "SET allow_community_extensions = false"
```

### Allow Unsigned Extensions (Development Only)

```sql
-- DANGER: Only for development
SET allow_unsigned_extensions = true;

-- Load local extension
LOAD '/path/to/unsigned_extension.duckdb_extension';
```

⚠️ **Never enable in production** — bypasses all signature checks.

### Strict Mode

```sql
-- Maximum security (default settings)
SET allow_unsigned_extensions = false;
SET allow_community_extensions = false;
SET allow_extensions_metadata_mismatch = false;

-- Only core extensions from core repo
INSTALL httpfs FROM core;
```

## Verification Commands

### Check Extension Signature

```bash
# DuckDB validates on LOAD, but we can inspect

# List extension metadata including signature status
duckdb -json -c "
    SELECT 
        extension_name,
        loaded,
        installed_from,
        extension_version,
        CASE installed_from 
            WHEN 'core' THEN 'DuckDB Labs'
            WHEN 'community' THEN 'Community CI'
            ELSE 'Unknown/Unsigned'
        END AS signed_by
    FROM duckdb_extensions()
    WHERE installed
" | jq '.[]'
```

### Verify Specific Extension

```bash
#!/bin/bash
verify_extension() {
    local name=$1
    
    result=$(duckdb -csv -c "
        SELECT installed_from, loaded 
        FROM duckdb_extensions() 
        WHERE extension_name='$name' AND installed
    " | tail -1)
    
    if [ -z "$result" ]; then
        echo "✗ $name not installed"
        return 1
    fi
    
    IFS=',' read -r source loaded <<< "$result"
    
    case $source in
        core)      echo "✓ $name (Core/DuckDB Labs)" ;;
        community) echo "⚠ $name (Community CI)" ;;
        *)         echo "⚠ $name (Source: $source)" ;;
    esac
    
    [ "$loaded" = "true" ] && echo "  Status: loaded" || echo "  Status: not loaded"
}

verify_extension "httpfs"
verify_extension "crypto"  # community extension
```

## Extension Trust Levels

| Level | Source | Verification | Risk |
|-------|--------|--------------|------|
| 🔵 Core | `core` repo | DuckDB Labs signed | Minimal |
| 🟡 Community | `community` repo | CI signed, reviewed | Low-Medium |
| 🟠 Custom URL | Custom HTTPS | TLS + manual audit | Medium |
| 🔴 Unsigned | Local file | None | High |

## Best Practices

### For End Users

1. **Prefer core extensions** when available
2. **Audit community extensions** before production use
   - Check source repository
   - Review recent commits
   - Understand what functions are exposed
3. **Disable in production** if not needed:
   ```sql
   SET allow_community_extensions = false;
   ```
4. **Pin versions** for reproducibility:
   ```sql
   FORCE INSTALL crypto FROM community;
   ```

### For Extension Developers

1. **Security-first design**
   - Minimize attack surface
   - Validate all inputs
   - Avoid unsafe code where possible
2. **Transparent builds**
   - Public CI logs
   - Reproducible build instructions
3. **Dependency hygiene**
   - Pin to specific versions
   - Regular audit of dependencies
4. **Prompt updates**
   - Respond to security issues
   - Maintain compatibility

### For Administrators

```bash
# Corporate policy: whitelist approach

# 1. Disable community by default
cat >> ~/.duckbrc << 'EOF'
SET allow_community_extensions = false;
EOF

# 2. Pre-install approved extensions
APPROVED="crypto ulid"
for ext in $APPROVED; do
    duckdb -c "INSTALL $ext FROM community"
done

# 3. Local extension repo for approved extensions
mkdir -p /usr/share/duckdb-extensions
# Mirror approved extensions from community
curl -o /usr/share/duckdb-extensions/crypto.duckdb_extension \
    https://community-extensions.duckdb.org/v1.4.1/linux_amd64/crypto.duckdb_extension

# 4. Configure DuckDB to use local repo
cat >> ~/.duckbrc << 'EOF'
SET custom_extension_repository = 'file:///usr/share/duckdb-extensions';
EOF
```

## Incident Response

### If Malicious Extension Detected

1. **Immediate**: Unload and block
   ```sql
   -- If extension is loaded
   -- (cannot unload, must restart)
   
   -- Prevent installation
   SET allow_community_extensions = false;
   ```

2. **Remove** from all systems
   ```bash
   # Remove extension file
   rm ~/.duckdb/extensions/v1.4.1/*/malicious.duckdb_extension
   
   # Purge from shared repos
   ```

3. **Report** to DuckDB team
   - Email: security@duckdblabs.com
   - Include extension name, version, threat details

4. **Audit** for compromise
   - Check query logs
   - Review data access
   - Scan for persistence mechanisms

## Compliance Considerations

| Requirement | DuckDB Feature |
|-------------|----------------|
| No unsigned code | `allow_unsigned_extensions = false` |
| Approved software only | Whitelist + local repository |
| Audit trail | Extension installation logging |
| Key management | Centralized signing by DuckDB |
| Supply chain | Dependency scanning in CI |

## Cross-References

- [Overview](01-overview.md) — Community extension basics
- [Installation](03-installation.md) — Installing from community
- [Core Extensions Security](/docs/stable/operations_manual/securing_duckdb/securing_extensions.html) — Official DuckDB docs
