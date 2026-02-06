# Authentication

## Overview

Turso Cloud provides multiple authentication mechanisms to secure your databases and manage access. Understanding these mechanisms is crucial for building secure applications.

## Authentication Types

### 1. User Authentication
For CLI and web dashboard access:
- GitHub OAuth
- Email/password
- SSO (Enterprise)

### 2. Database Tokens
For application connections to databases:
- Full-access tokens
- Read-only tokens
- Scoped tokens

### 3. Platform API Tokens
For automation and CI/CD:
- Organization-level tokens
- Fine-grained permissions
- Expiration control

## User Authentication

### CLI Authentication
```bash
# Login via browser (OAuth)
turso auth login

# Login with token (CI/CD)
turso auth login --token $TURSO_TOKEN

# Check authentication status
turso auth status

# Logout
turso auth logout
```

### Web Dashboard
```
1. Go to https://app.turso.tech
2. Click "Sign in with GitHub"
3. Authorize Turso application
4. Access your organizations
```

### Session Management
```bash
# List active sessions
turso auth sessions

# Revoke specific session
turso auth revoke-session <session-id>

# Revoke all other sessions
turso auth revoke-other-sessions
```

## Database Tokens

### Creating Tokens
```bash
# Full-access token
turso db tokens create mydb

# Read-only token
turso db tokens create mydb --read-only

# Named token (for tracking)
turso db tokens create mydb --name "Production App"

# Token with expiration
turso db tokens create mydb --expiration 30d
```

### Token Scopes
```bash
# Full access (default)
turso db tokens create mydb --permission full

# Read-only
turso db tokens create mydb --permission read

# Write-only
turso db tokens create mydb --permission write

# Custom permissions
turso db tokens create mydb \
  --permission read \
  --permission "write:users" \
  --permission "write:orders"
```

### Token Management
```bash
# List tokens
turso db tokens list mydb

# Token details
turso db tokens show mydb token-id

# Revoke token
turso db tokens revoke mydb token-id

# Revoke all tokens
turso db tokens revoke-all mydb
```

### Using Database Tokens

#### In Connection Strings
```rust
// libSQL client
let db = Builder::new_remote(
    "libsql://mydb-org.turso.io",
    "your-database-token"
).build().await?;
```

#### Environment Variables
```bash
# Store token securely
export TURSO_DATABASE_URL="libsql://mydb-org.turso.io"
export TURSO_AUTH_TOKEN="your-token-here"

# Application reads from env
let db = Builder::new_remote(
    std::env::var("TURSO_DATABASE_URL")?,
    std::env::var("TURSO_AUTH_TOKEN")?
).build().await?;
```

#### Configuration Files
```bash
# .env file (add to .gitignore!)
TURSO_DATABASE_URL=libsql://mydb-org.turso.io
TURSO_AUTH_TOKEN=your-token
```

## Platform API Tokens

### Creating API Tokens
```bash
# Organization-level token
turso org api-tokens create myorg --name "CI/CD Pipeline"

# With specific permissions
turso org api-tokens create myorg \
  --name "Deploy Bot" \
  --permission "db:read" \
  --permission "db:write" \
  --permission "db:create"

# With expiration
turso org api-tokens create myorg \
  --name "Temporary Access" \
  --expiration 7d
```

### API Token Permissions

| Permission | Description |
|------------|-------------|
| `org:read` | Read organization details |
| `org:write` | Modify organization |
| `db:read` | Read database metadata |
| `db:write` | Modify databases |
| `db:create` | Create new databases |
| `db:delete` | Delete databases |
| `member:read` | List members |
| `member:write` | Manage members |
| `billing:read` | View billing info |

### Managing API Tokens
```bash
# List tokens
turso org api-tokens list myorg

# Show token details
turso org api-tokens show myorg token-id

# Update token
turso org api-tokens update myorg token-id \
  --name "Updated Name"

# Revoke token
turso org api-tokens revoke myorg token-id
```

### Using API Tokens

#### With Turso CLI
```bash
# Login with API token
turso auth login --token $TURSO_API_TOKEN

# Or set for single command
TURSO_API_TOKEN=xxx turso db list
```

#### With Platform API
```bash
# Direct API call
curl -H "Authorization: Bearer $TURSO_API_TOKEN" \
  https://api.turso.tech/v1/organizations/myorg/databases
```

## Token Security Best Practices

### Token Rotation
```bash
# Create new token
turso db tokens create mydb --name "New Token"

# Update application to use new token
# Verify application works

# Revoke old token
turso db tokens revoke mydb old-token-id
```

### Principle of Least Privilege
```bash
# Don't use full-access tokens everywhere

# Application only needs read? Use read-only
turso db tokens create mydb --read-only

# Service only writes to specific tables? Scope it
turso db tokens create mydb \
  --permission "read" \
  --permission "write:logs"
```

### Token Storage
```
✅ DO:
- Store in environment variables
- Use secret management (AWS Secrets Manager, etc.)
- Rotate tokens regularly
- Use short expiration for temporary tokens

❌ DON'T:
- Hardcode tokens in source code
- Commit tokens to version control
- Share tokens in logs or messages
- Use same token for all environments
```

### Token Monitoring
```bash
# View token usage
turso db tokens usage mydb token-id

# Check for suspicious activity
turso db tokens audit mydb --last 30d
```

## Authentication Patterns

### Multi-Environment Setup
```
Production:
  Database: prod-app
  Token: prod-full-access (restricted to production IP)

Staging:
  Database: staging-app  
  Token: staging-read-write

Development:
  Database: dev-app
  Token: dev-full-access (local only)
```

### Microservices Architecture
```
service-auth:
  Token: read-write users, sessions

service-orders:
  Token: read-write orders

service-analytics:
  Token: read-only all tables

service-migrations:
  Token: schema modification (temporary)
```

### CI/CD Integration
```yaml
# .github/workflows/deploy.yml
name: Deploy

on: [push]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Turso
        uses: turso/turso-action@v1
        with:
          api-token: ${{ secrets.TURSO_API_TOKEN }}
      
      - name: Run Migrations
        run: turso db shell prod-app < migrations.sql
        env:
          TURSO_API_TOKEN: ${{ secrets.TURSO_API_TOKEN }}
```

## Troubleshooting Authentication

### Common Issues

#### "Authentication failed"
```bash
# Check token validity
turso db tokens show mydb token-id

# If expired, create new token
turso db tokens create mydb

# Verify connection string
# Should be: libsql://db-org.turso.io
```

#### "Insufficient permissions"
```bash
# Check token permissions
turso db tokens show mydb token-id --permissions

# Create token with correct permissions
turso db tokens create mydb --permission full
```

#### "Token revoked"
```bash
# Check if token is revoked
turso db tokens list mydb --show-revoked

# Create new token
turso db tokens create mydb
```

### Debugging Authentication
```bash
# Enable verbose logging
turso db shell mydb --verbose

# Test with curl
curl -v -H "Authorization: Bearer $TOKEN" \
  https://mydb-org.turso.io/v2/pipeline
```

## Migration from SQLite

### Connection String Changes
```python
# SQLite
conn = sqlite3.connect("./mydb.db")

# Turso Cloud
import libsql_client
conn = libsql_client.create_client_sync(
    url="libsql://mydb-org.turso.io",
    auth_token="your-token"
)
```

### Authentication Code Changes
```javascript
// Before: No authentication needed
const db = new Database('./mydb.db');

// After: Token-based auth
import { createClient } from '@libsql/client';
const db = createClient({
  url: process.env.TURSO_DATABASE_URL,
  authToken: process.env.TURSO_AUTH_TOKEN
});
```

## Advanced Authentication

### Custom JWT Claims
```bash
# Create token with custom claims (Enterprise)
turso db tokens create mydb \
  --claim "user_id:12345" \
  --claim "role:admin" \
  --claim "tenant:acme"
```

### IP Whitelisting
```bash
# Restrict token to specific IPs
turso db tokens create mydb \
  --allow-ip "203.0.113.0/24" \
  --allow-ip "198.51.100.10"
```

### Time-Based Restrictions
```bash
# Token only valid during business hours
turso db tokens create mydb \
  --valid-hours "09:00-17:00" \
  --valid-days "mon,tue,wed,thu,fri"
```

## CLI Reference

```bash
# User authentication
turso auth login [options]
turso auth logout
turso auth status
turso auth sessions
turso auth revoke-session <id>

# Database tokens
turso db tokens create <db> [options]
turso db tokens list <db> [options]
turso db tokens show <db> <id>
turso db tokens revoke <db> <id>
turso db tokens revoke-all <db>

# API tokens
turso org api-tokens create <org> [options]
turso org api-tokens list <org>
turso org api-tokens show <org> <id>
turso org api-tokens revoke <org> <id>
```

## Next Steps

- **Embedded Replicas**: [06-embedded-replicas.md](./06-embedded-replicas.md)
- **Branching**: [07-branching.md](./07-branching.md)
- **Platform API**: [09-platform-api.md](./09-platform-api.md)