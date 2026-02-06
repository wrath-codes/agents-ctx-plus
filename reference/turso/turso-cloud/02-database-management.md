# Database Management

## Creating Databases

### Basic Creation
```bash
# Create database with random name
turso db create

# Create with specific name
turso db create mydb

# Create in specific location
turso db create mydb --location lhr

# Create from existing database
turso db create mydb-copy --from-db mydb

# Create from dump file
turso db create mydb --from-dump ./backup.sql
```

### Database Groups
```bash
# Create database group
turso group create mygroup

# Create database in group
turso db create mydb --group mygroup

# List databases in group
turso db list --group mygroup
```

## Database Operations

### Listing Databases
```bash
# List all databases
turso db list

# List with details
turso db list --verbose

# Filter by group
turso db list --group production
```

### Inspecting Databases
```bash
# Show database details
turso db show mydb

# Show connection URL
turso db show mydb --url

# Show HTTP URL
turso db show mydb --http-url

# Show statistics
turso db show mydb --stats
```

### Destroying Databases
```bash
# Destroy database (with confirmation)
turso db destroy mydb

# Destroy without confirmation
turso db destroy mydb --yes

# Destroy all in group
turso db destroy --group mygroup --yes
```

## Database Settings

### Viewing Settings
```bash
# Get all settings
turso db settings get mydb

# Get specific setting
turso db settings get mydb --setting size_limit
turso db settings get mydb --setting allow_attach
```

### Updating Settings
```bash
# Set database size limit
turso db settings update mydb --size-limit 10GB

# Enable/disable extensions
turso db settings update mydb --allow-attach true

# Update multiple settings
turso db settings update mydb \
  --size-limit 5GB \
  --allow-attach false
```

### Available Settings

| Setting | Description | Default |
|---------|-------------|---------|
| size_limit | Maximum database size | Plan limit |
| allow_attach | Allow ATTACH DATABASE | false |
| is_schema | Schema database for multi-tenant | false |
| encryption | Enable encryption at rest | false |

## Database Lifecycle

### Backup and Restore
```bash
# Create backup
turso db backup mydb

# List backups
turso db backup list mydb

# Restore from backup
turso db restore mydb --from-backup backup-id

# Download backup
turso db backup download mydb backup-id --output ./backup.sql
```

### Exporting Data
```bash
# Export to SQL dump
turso db dump mydb --output ./mydb-export.sql

# Export specific tables
turso db dump mydb --tables users,orders --output ./export.sql

# Export with data only
turso db dump mydb --data-only --output ./data.sql
```

### Importing Data
```bash
# Import from SQL file
turso db shell mydb < ./import.sql

# Import with turso CLI
turso db create mydb --from-dump ./import.sql
```

## Database Migration

### Zero-Downtime Migration
```bash
# 1. Create new database
turso db create mydb-v2

# 2. Copy schema
turso db dump mydb --schema-only | turso db shell mydb-v2

# 3. Sync data (ongoing replication)
# Setup replication from mydb to mydb-v2

# 4. Test new database

# 5. Switch traffic (update connection strings)

# 6. Decommission old database
turso db destroy mydb
```

## Monitoring and Maintenance

### Database Statistics
```bash
# Get database stats
turso db stats mydb

# Get usage metrics
turso db usage mydb --from 2024-01-01 --to 2024-01-31

# Get real-time metrics
turso db stats mydb --watch
```

### Health Checks
```bash
# Check database health
turso db inspect mydb

# Check for issues
turso db inspect mydb --verbose
```

### Maintenance Windows
```bash
# Schedule maintenance
turso db maintenance schedule mydb \
  --start "2024-01-15T02:00:00Z" \
  --duration 30m

# Cancel maintenance
turso db maintenance cancel mydb
```

## Multi-tenant Databases

### Schema Databases
```bash
# Create schema database
turso db create schema-template --is-schema

# Create child databases
turso db create tenant-1 --schema schema-template
turso db create tenant-2 --schema schema-template

# Update all children
turso db schema migrate schema-template --all
```

### Tenant Isolation
```rust
// Each tenant gets own database
struct TenantDatabase {
    db_name: String,
    connection: libsql::Connection,
}

impl TenantDatabase {
    async fn new(tenant_id: &str) -> Result<Self> {
        let db_name = format!("tenant-{}", tenant_id);
        let db = Builder::new_remote(
            &format!("libsql://{}-org.turso.io", db_name),
            token
        ).build().await?;
        
        Ok(Self {
            db_name,
            connection: db.connect()?,
        })
    }
}
```

## CLI Reference

### Complete Command List
```bash
# Database operations
turso db create <name> [options]
turso db list [options]
turso db show <name> [options]
turso db destroy <name> [options]
turso db shell <name> [options]

# Replication
turso db replicate <name> <location>
turso db unreplicate <name> <location>

# Branching
turso db branch <name> <new-name>
turso db fork <name> <new-name>

# Backup
turso db backup <name>
turso db restore <name> [options]

# Import/Export
turso db dump <name> [options]
turso db import <name> <file>

# Settings
turso db settings get <name> [options]
turso db settings update <name> [options]

# Monitoring
turso db stats <name> [options]
turso db usage <name> [options]
turso db inspect <name> [options]
```

## Best Practices

### Naming Conventions
```bash
# Environment prefix
turso db create prod-myapp
turso db create staging-myapp
turso db create dev-myapp

# Version suffix
turso db create myapp-v1
turso db create myapp-v2

# Group organization
turso group create prod
turso group create staging
turso db create myapp --group prod
```

### Backup Strategy
```bash
# Daily automated backups
# (Turso provides continuous backups)

# Weekly manual backup for critical databases
turso db backup critical-db --name "weekly-$(date +%Y%m%d)"

# Before major changes
turso db backup mydb --name "pre-migration-$(date +%s)"
```

### Performance Optimization
```bash
# Monitor query performance
turso db stats mydb --queries

# Check for missing indexes
turso db inspect mydb --indexes

# Optimize with ANALYZE
turso db shell mydb "ANALYZE"
```

## Next Steps

- **Organizations**: [03-organizations.md](./03-organizations.md)
- **Locations**: [04-locations-regions.md](./04-locations-regions.md)
- **Authentication**: [05-authentication.md](./05-authentication.md)