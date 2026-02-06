# Database Branching

## Overview

Turso Cloud supports database branching with Copy-on-Write semantics, allowing you to create isolated copies of databases for testing, development, and experimentation without duplicating storage costs.

## How Branching Works

```
┌─────────────────────────────────────────────────────────────┐
│              Copy-on-Write Branching                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────────────────────────────┐             │
│  │              Parent Database               │             │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐   │             │
│  │  │  Page 1 │  │  Page 2 │  │  Page 3 │   │             │
│  │  │ (Shared)│  │ (Shared)│  │ (Shared)│   │             │
│  │  └────┬────┘  └────┬────┘  └────┬────┘   │             │
│  │       │            │            │         │             │
│  │       └────────────┼────────────┘         │             │
│  │                    │                      │             │
│  └────────────────────┼──────────────────────┘             │
│                       │                                     │
│                       │ (Shared until modified)             │
│                       │                                     │
│  ┌────────────────────┼──────────────────────┐             │
│  │        Branch Database                     │             │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐   │             │
│  │  │  Page 1 │  │  Page 2 │  │Page 3*  │   │             │
│  │  │ (Shared)│  │ (Shared)│  │(Copied) │   │             │
│  │  └─────────┘  └─────────┘  └────┬────┘   │             │
│  │                                 │         │             │
│  │                          (Modified)        │             │
│  └───────────────────────────────────────────┘             │
│                                                             │
│  Storage: Only changed pages are duplicated                │
│  Cost: Minimal until significant changes                   │
│  Time: Instant (no data copying)                           │
└─────────────────────────────────────────────────────────────┘
```

## Creating Branches

### Basic Branching
```bash
# Create branch from existing database
turso db branch production-db staging-db

# Create branch with specific name
turso db branch mydb mydb-feature-x

# Branch to different group
turso db branch prod/mydb staging/mydb-copy
```

### Branch from Specific Point
```bash
# Branch from current state
turso db branch mydb mydb-backup

# Branch from specific timestamp
turso db branch mydb mydb-yesterday \
  --from "2024-01-14T00:00:00Z"

# Branch from transaction (if WAL archiving enabled)
turso db branch mydb mydb-stable \
  --from-transaction tx-abc123
```

## Branch Management

### Listing Branches
```bash
# List all branches
turso db branch list

# List branches of specific database
turso db branch list mydb

# Show branch hierarchy
turso db branch list --tree
```

### Inspecting Branches
```bash
# Show branch details
turso db branch show mydb-branch

# Compare with parent
turso db branch diff mydb-branch

# Show storage usage
turso db branch show mydb-branch --storage
```

### Deleting Branches
```bash
# Delete branch
turso db branch delete mydb-branch

# Delete without confirmation
turso db branch delete mydb-branch --yes

# Delete all branches of database
turso db branch delete --all --db mydb
```

## Workflows

### Development Workflow
```bash
# 1. Production database
turso db create production-db

# 2. Create feature branch
turso db branch production-db feature-login

# 3. Test schema changes
turso db shell feature-login
# > ALTER TABLE users ADD COLUMN last_login DATETIME;

# 4. Test with data
turso db shell feature-login < seed-data.sql

# 5. If tests pass, apply to production
turso db shell production-db < migration.sql

# 6. Clean up branch
turso db branch delete feature-login
```

### Testing Workflow
```bash
# Create test branch before deployment
turso db branch production-db pre-deploy-test

# Run integration tests
npm test -- --db-url=$(turso db show pre-deploy-test --url)

# If tests pass, proceed with deployment
# If tests fail, debug without affecting production

# Clean up
turso db branch delete pre-deploy-test
```

### Blue-Green Deployment
```bash
# 1. Current: production-db-v1 (serving traffic)

# 2. Create new version
turso db branch production-db-v1 production-db-v2

# 3. Apply migrations to v2
turso db shell production-db-v2 < v2-migrations.sql

# 4. Verify v2
turso db shell production-db-v2 < verify-queries.sql

# 5. Switch traffic (update connection strings)
#    Point apps to production-db-v2

# 6. Keep v1 as rollback option

# 7. After successful period, delete v1
turso db destroy production-db-v1
```

### Experimentation
```bash
# Create experimental branch
turso db branch production-db experiment-vector-search

# Try new schema
turso db shell experiment-vector-search <<EOF
CREATE TABLE embeddings (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding F32_BLOB(1536)
);
CREATE INDEX idx_embedding ON embeddings(libsql_vector_idx(embedding));
EOF

# Populate with test data
python generate-embeddings.py | turso db shell experiment-vector-search

# Test queries
turso db shell experiment-vector-search <<EOF
SELECT * FROM vector_top_k('idx_embedding', vector('[...]'), 10);
EOF

# If successful, apply to production
# If not, just delete
turso db branch delete experiment-vector-search
```

## Copy-on-Write Mechanics

### Storage Efficiency
```
Scenario: 10GB database, branch changes 100MB

Traditional Copy:
- Original: 10GB
- Copy: 10GB
- Total: 20GB

Copy-on-Write:
- Original: 10GB
- Branch: 100MB (changed pages only)
- Total: 10.1GB
- Savings: 95%
```

### Page-Level Tracking
```rust
// Internally, branching works at page level
struct DatabasePage {
    page_number: u32,
    data: [u8; 4096],
    checksum: u64,
    modified_in_branch: Option<BranchId>,
}

// When page is modified in branch:
// 1. Copy original page to branch storage
// 2. Modify the copy
// 3. Original page remains shared
```

### Storage Costs
```
┌─────────────────────────────────────────────┐
│          Branch Storage Costs               │
├─────────────────────────────────────────────┤
│ Base cost (metadata)       │ ~1 MB         │
│ Per changed page           │ 4 KB          │
│ Typical branch overhead    │ 0.1-5%        │
│ Maximum (all pages changed)│ 100% (copy)   │
└─────────────────────────────────────────────┘
```

## Branch Operations

### Schema Changes
```sql
-- Safe schema experimentation on branch
-- Parent remains unchanged

-- Add new table
CREATE TABLE new_feature (
    id INTEGER PRIMARY KEY,
    data JSON
);

-- Modify existing table
ALTER TABLE users ADD COLUMN preferences JSON;

-- Create indexes
CREATE INDEX idx_new_feature ON new_feature(data);

-- All changes isolated to branch
```

### Data Operations
```sql
-- Insert test data
INSERT INTO test_table VALUES (...);

-- Run destructive queries safely
DELETE FROM users WHERE last_login < '2020-01-01';

-- Analyze results before applying to parent
SELECT COUNT(*) FROM users;
```

### Merging Changes
```bash
# Currently Turso doesn't support automatic merge
# Manual merge process:

# 1. Export changes from branch
turso db dump mydb-branch --data-only --tables modified_tables > changes.sql

# 2. Review changes
cat changes.sql

# 3. Apply to parent
turso db shell mydb < changes.sql

# 4. Or apply schema changes only
turso db dump mydb-branch --schema-only > schema-changes.sql
turso db shell mydb < schema-changes.sql
```

## Advanced Features

### Branch from Branch
```bash
# Create nested branches
turso db branch production-db staging-db
turso db branch staging-db staging-db-feature-x
turso db branch staging-db-feature-x staging-db-feature-x-test

# Hierarchy:
# production-db
#   └── staging-db
#       └── staging-db-feature-x
#           └── staging-db-feature-x-test
```

### Branch Permissions
```bash
# Restrict branch access
turso db branch create mydb mydb-restricted \
  --read-only \
  --allowed-users user1@example.com,user2@example.com

# Make branch private
turso db branch create mydb mydb-private \
  --private
```

### Branch Lifecycle Policies
```bash
# Auto-delete old branches
turso org settings update myorg \
  --auto-delete-branches-after 30d

# Exclude specific branches from auto-delete
turso db branch protect mydb-branch

# Archive branch instead of delete
turso db branch archive mydb-old-feature
```

## Best Practices

### Naming Conventions
```bash
# Environment prefixes
turso db branch production-db staging-db
turso db branch production-db dev-alice-feature-x

# Date suffixes
turso db branch production-db backup-2024-01-15
turso db branch production-db experiment-2024-01-15-vector

# Ticket references
turso db branch production-db ticket-123-fix-login
turso db branch production-db pr-456-schema-change
```

### Cleanup Strategy
```bash
# Regular cleanup of old branches
# Add to CI/CD pipeline:

# Delete branches older than 7 days (except protected)
turso db branch list --older-than 7d --not-protected \
  | xargs -I {} turso db branch delete {}

# Keep last 5 backups
turso db branch list --pattern 'backup-*' --sort date \
  | tail -n +6 | xargs turso db branch delete
```

### Cost Management
```bash
# Monitor branch storage
turso org usage myorg --by-branch

# Set branch limits
turso org settings update myorg \
  --max-branches-per-db 10 \
  --max-branch-storage 5GB
```

## CLI Reference

```bash
# Branch creation
turso db branch <source> <target> [options]
turso db branch create <source> <target> [options]

# Branch management
turso db branch list [options]
turso db branch show <branch> [options]
turso db branch delete <branch> [options]
turso db branch diff <branch> [options]

# Advanced operations
turso db branch protect <branch>
turso db branch unprotect <branch>
turso db branch archive <branch>
turso db branch restore <branch>

# Options:
#   --from <timestamp>     Branch from specific time
#   --from-transaction <id> Branch from transaction
#   --group <group>        Target group
#   --read-only            Create read-only branch
#   --private              Private branch
```

## Troubleshooting

### Branch Creation Fails
```bash
# Check database exists
turso db show mydb

# Check permissions
turso db tokens list mydb

# Check storage limits
turso org usage myorg
```

### Slow Branch Operations
```bash
# Branch with many changes may be slow
# Check branch size
turso db branch show mydb-branch --storage

# Consider archiving instead of many branches
turso db branch archive old-branch
```

## Next Steps

- **Advanced Features**: [08-advanced-features.md](./08-advanced-features.md)
- **Platform API**: [09-platform-api.md](./09-platform-api.md)
- **SDKs**: [10-sdks/](./10-sdks/)