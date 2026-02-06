# Advanced Features

## Overview

Turso Cloud includes several advanced features for production workloads: Point-in-Time Recovery, Analytics queries, Per-database Encryption, and more.

## Point-in-Time Recovery (PITR)

PITR allows you to restore your database to any point in time within the retention window.

### How PITR Works
```
┌────────────────────────────────────────────────────────────┐
│              Point-in-Time Recovery                        │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Time ───────────────────────────────────────────────►    │
│                                                            │
│  Now        6h ago      12h ago      24h ago      30d     │
│   │           │           │            │          (limit)  │
│   ▼           ▼           ▼            ▼                   │
│ ┌───┐       ┌───┐       ┌───┐       ┌───┐                 │
│ │ W │──────►│ W │──────►│ W │──────►│ W │  WAL Segments   │
│ │ A │       │ A │       │ A │       │ A │                 │
│ │ L │       │ L │       │ L │       │ L │                 │
│ └───┘       └───┘       └───┘       └───┘                 │
│   │           │           │            │                   │
│   └───────────┴───────────┴────────────┘                   │
│              Can restore to any point                      │
│              within retention window                       │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Using PITR

```bash
# View available restore points
turso db restore-points mydb

# Restore to specific time
turso db restore mydb \
  --to "2024-01-15T14:30:00Z" \
  --new-name mydb-restored

# Restore to transaction
turso db restore mydb \
  --to-transaction tx-abc123 \
  --new-name mydb-restored

# Restore in-place (dangerous!)
turso db restore mydb \
  --to "2024-01-15T14:30:00Z" \
  --in-place \
  --yes
```

### Retention Configuration
```bash
# View current retention
turso db settings get mydb --setting pitr-retention

# Update retention (Scaler plan+)
turso db settings update mydb \
  --pitr-retention 30d  # 7d, 14d, 30d available
```

### Use Cases

#### Recovering from Accidental Deletes
```bash
# User accidentally deleted data at 2:30 PM
# Restore to 2:29 PM
turso db restore mydb \
  --to "2024-01-15T14:29:00Z" \
  --new-name mydb-pre-delete

# Export deleted data
turso db dump mydb-pre-delete --tables deleted_table > recovery.sql

# Import to production
turso db shell mydb < recovery.sql

# Clean up
turso db destroy mydb-pre-delete
```

#### Testing Historical Data
```bash
# See database state as of last week
turso db restore mydb \
  --to "2024-01-08T00:00:00Z" \
  --new-name mydb-week-ago

# Run analysis queries
turso db shell mydb-week-ago < analysis.sql

# Clean up
turso db destroy mydb-week-ago
```

## Analytics Queries

Turso Cloud supports analytical queries for large-scale data analysis.

### Enabling Analytics
```bash
# Enable analytics mode for database
turso db settings update mydb --enable-analytics true

# Analytics uses separate compute resources
# Doesn't affect production query performance
```

### Running Analytics Queries
```bash
# Connect to analytics endpoint
turso db shell mydb --analytics

# Run analytical queries
SELECT 
    DATE(created_at) as date,
    COUNT(*) as signups,
    AVG(order_value) as avg_order
FROM users
JOIN orders ON users.id = orders.user_id
GROUP BY DATE(created_at)
ORDER BY date DESC;
```

### Analytics Features

#### Columnar Storage
```sql
-- Analytics queries benefit from columnar storage
-- Optimized for aggregations and scans

SELECT 
    product_category,
    SUM(revenue) as total_revenue,
    COUNT(DISTINCT customer_id) as unique_customers
FROM sales
WHERE date >= '2024-01-01'
GROUP BY product_category;
```

#### Parallel Query Execution
```sql
-- Large queries automatically parallelized
SELECT COUNT(*) FROM events;  -- Uses multiple cores

-- Complex aggregations
SELECT 
    region,
    COUNT(*) as events,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY response_time) as p95_latency
FROM api_logs
GROUP BY region;
```

### Analytics vs Production

| Feature | Production | Analytics |
|---------|------------|-----------|
| Query type | OLTP (transactions) | OLAP (analytics) |
| Latency | < 10ms | < 10s acceptable |
| Concurrency | High | Low |
| Resource isolation | Shared | Dedicated |
| Cost | Included | Usage-based |

## Per-Database Encryption

Encrypt individual databases with separate keys.

### Enabling Encryption
```bash
# Enable encryption for database
turso db settings update mydb --encryption true

# Generate encryption key (or provide your own)
turso db encryption-key generate mydb

# Or provide your own key
turso db encryption-key set mydb \
  --key "base64-encoded-32-byte-key"
```

### Managing Encryption Keys

```bash
# Rotate encryption key
turso db encryption-key rotate mydb

# Key rotation is online operation
# No downtime required

# View encryption status
turso db show mydb --encryption-status

# Disable encryption
turso db settings update mydb --encryption false
```

### Encryption in Application

```rust
// Encrypted database works transparently
let db = Builder::new_remote(
    "libsql://mydb-org.turso.io",
    token
).build().await?;

// All data encrypted at rest
// Encryption/decryption happens server-side
```

## Database Insights

### Query Performance Analysis
```bash
# Get slow query log
turso db insights mydb --slow-queries

# Get query statistics
turso db insights mydb --query-stats

# Top queries by execution time
turso db insights mydb --top-queries --limit 20
```

### Index Recommendations
```bash
# Get index recommendations
turso db insights mydb --index-recommendations

# Example output:
# Table: users
#   Missing index on (email) - would improve query by 85%
#   Missing index on (created_at) - would improve query by 45%

# Apply recommendation
turso db shell mydb "CREATE INDEX idx_users_email ON users(email)"
```

### Storage Analysis
```bash
# Database storage breakdown
turso db insights mydb --storage

# Table sizes
turso db insights mydb --table-sizes

# Index sizes
turso db insights mydb --index-sizes
```

## Advanced Configuration

### Connection Pooling
```bash
# Configure connection pool size
turso db settings update mydb --max-connections 100

# Connection timeout
turso db settings update mydb --connection-timeout 30s

# Idle timeout
turso db settings update mydb --idle-timeout 10m
```

### Query Limits
```bash
# Set query timeout
turso db settings update mydb --query-timeout 30s

# Limit concurrent queries
turso db settings update mydb --max-concurrent-queries 50

# Limit result set size
turso db settings update mydb --max-result-size 10000
```

### Caching Configuration
```bash
# Enable query result caching
turso db settings update mydb --query-cache true

# Cache TTL
turso db settings update mydb --cache-ttl 5m

# Cache size
turso db settings update mydb --cache-size 100MB
```

## Monitoring and Alerting

### Setting Up Alerts
```bash
# Alert on high latency
turso db alert create mydb \
  --metric query_latency_p99 \
  --threshold 100ms \
  --duration 5m \
  --email ops@example.com

# Alert on error rate
turso db alert create mydb \
  --metric error_rate \
  --threshold 0.01 \
  --duration 2m

# Storage alert
turso db alert create mydb \
  --metric storage_usage \
  --threshold 80%
```

### Custom Metrics
```bash
# Export metrics to Datadog
turso db integration create mydb \
  --type datadog \
  --api-key $DATADOG_API_KEY

# Export to Prometheus
turso db integration create mydb \
  --type prometheus \
  --endpoint https://prometheus.example.com/api/v1/write
```

## Disaster Recovery

### Cross-Region Replication
```bash
# Set up cross-region replication
turso db replicate mydb iad  # Primary
turso db replicate mydb lhr  # Secondary (EU)
turso db replicate mydb nrt  # Secondary (APAC)

# Automatic failover configuration
turso db failover-config mydb \
  --auto-failover true \
  --health-check-interval 30s \
  --failover-threshold 3
```

### Backup Strategy
```bash
# Automated daily backups
turso db backup-policy mydb \
  --schedule "0 2 * * *" \
  --retention 30d

# On-demand backup before major changes
turso db backup mydb --name "pre-migration-$(date +%s)"

# Cross-region backup
turso db backup mydb --region eu-west-1
```

## CLI Reference

```bash
# Point-in-Time Recovery
turso db restore-points <db>
turso db restore <db> [options]

# Analytics
turso db shell <db> --analytics
turso db settings update <db> --enable-analytics true

# Encryption
turso db settings update <db> --encryption true
turso db encryption-key generate <db>
turso db encryption-key rotate <db>

# Insights
turso db insights <db> [options]

# Alerts
turso db alert create <db> [options]
turso db alert list <db>
turso db alert delete <db> <alert-id>

# Advanced settings
turso db settings update <db> [options]
```

## Next Steps

- **Platform API**: [09-platform-api.md](./09-platform-api.md)
- **SDKs**: [10-sdks/](./10-sdks/)