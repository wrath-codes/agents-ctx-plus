# Configuration Reference

## Configuration File Format

Configuration files use YAML format. Multiple configuration files can be merged.

## Default Configuration

```yaml
# config/default.yaml

# Server configuration
server:
  host: "0.0.0.0"
  port: 8080
  read_timeout: "60s"
  write_timeout: "60s"
  max_header_bytes: 1048576
  enable_tls: false
  cert_file: ""
  key_file: ""
  
  # CORS settings
  cors:
    allowed_origins:
      - "http://localhost:3000"
    allowed_methods:
      - "GET"
      - "POST"
      - "PUT"
      - "DELETE"
      - "OPTIONS"
    allowed_headers:
      - "Content-Type"
      - "Authorization"

# Database configuration
database:
  coordination_db:
    path: "./data/coordination.db"
    max_open_conns: 1
    max_idle_conns: 1
    conn_max_lifetime: "1h"
    busy_timeout: "30s"
    num_readers: 3
  
  beads_db:
    path: "./.beads/beads.db"

# Tempolite configuration
tempolite:
  db_path: "./data/tempolite.db"
  max_workflows: 1000
  checkpoint_interval: "1m"
  cleanup_interval: "10m"

# Agent configuration
agents:
  research:
    max_workload: 5
    timeout: "30m"
    retry_policy:
      max_retries: 3
      initial_backoff: "1s"
      max_backoff: "30s"
      multiplier: 2.0
    resource_limits:
      memory_mb: 2048
      cpu_percent: 80
      disk_io_mb_ps: 100
  
  poc:
    max_workload: 3
    timeout: "45m"
    retry_policy:
      max_retries: 2
      initial_backoff: "5s"
      max_backoff: "60s"
      multiplier: 1.5
    resource_limits:
      memory_mb: 4096
      cpu_percent: 90
      disk_io_mb_ps: 200

# Authentication configuration
auth:
  enabled: false
  jwt_secret: "change-me-in-production"
  token_expiry: "24h"
  refresh_token_expiry: "168h"
  
  # RBAC configuration
  roles:
    admin:
      permissions:
        - "workflow:*"
        - "agent:*"
        - "analytics:*"
    operator:
      permissions:
        - "workflow:read"
        - "workflow:write"
        - "agent:read"
        - "analytics:read"
    viewer:
      permissions:
        - "workflow:read"
        - "analytics:read"

# Logging configuration
logging:
  level: "info"
  format: "console"
  output: "stdout"
  file_path: ""
  max_size: 100
  max_backups: 3
  max_age: 7
  compress: true

# Analytics configuration
analytics:
  enabled: true
  retention_days: 30
  aggregation_interval: "1h"

# Monitoring configuration
monitoring:
  enabled: true
  metrics_port: 9090
  health_check_interval: "30s"
  
  # Prometheus configuration
  prometheus:
    enabled: true
    path: "/metrics"
  
  # Tracing configuration
  tracing:
    enabled: false
    endpoint: ""
    sample_rate: 0.1

# Cache configuration
cache:
  enabled: true
  
  # In-memory cache
  memory:
    enabled: true
    max_cost: 1073741824  # 1GB
    num_counters: 10000000
  
  # Redis cache
  redis:
    enabled: false
    address: "localhost:6379"
    password: ""
    db: 0
    pool_size: 10
```

## Environment Variables

All configuration options can be overridden using environment variables:

```bash
# Server
export WORKFLOW_SERVER_HOST=0.0.0.0
export WORKFLOW_SERVER_PORT=8080

# Database
export WORKFLOW_DATABASE_COORDINATION_DB_PATH=/data/coordination.db
export WORKFLOW_DATABASE_BEADS_DB_PATH=./.beads/beads.db

# Logging
export WORKFLOW_LOGGING_LEVEL=debug
export WORKFLOW_LOGGING_FORMAT=json

# Authentication
export WORKFLOW_AUTH_ENABLED=true
export WORKFLOW_AUTH_JWT_SECRET=your-secret-key

# Monitoring
export WORKFLOW_MONITORING_ENABLED=true
export WORKFLOW_MONITORING_METRICS_PORT=9090
```

## Configuration Precedence

Configuration is loaded in this order (later overrides earlier):

1. Default configuration embedded in binary
2. Configuration file (config/default.yaml)
3. Environment-specific file (config/production.yaml)
4. Environment variables
5. Command line flags

## Production Configuration

```yaml
# config/production.yaml

server:
  host: "0.0.0.0"
  port: 8080
  enable_tls: true
  cert_file: "/etc/ssl/certs/workflow.crt"
  key_file: "/etc/ssl/private/workflow.key"

database:
  coordination_db:
    path: "/data/coordination.db"
    max_open_conns: 1
    busy_timeout: "30s"
  
  beads_db:
    path: "/data/.beads/beads.db"

logging:
  level: "info"
  format: "json"
  output: "file"
  file_path: "/var/log/workflow/app.log"
  max_size: 500
  max_backups: 10
  max_age: 30

auth:
  enabled: true
  jwt_secret: "${JWT_SECRET}"  # Loaded from environment

cache:
  enabled: true
  redis:
    enabled: true
    address: "redis:6379"
    password: "${REDIS_PASSWORD}"
```

## Configuration Validation

```bash
# Validate configuration
workflow-server --validate-config --config production.yaml

# Output parsed configuration
workflow-server --dump-config --config production.yaml
```