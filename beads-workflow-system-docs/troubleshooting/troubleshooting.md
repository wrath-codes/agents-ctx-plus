# Troubleshooting Guide

## Common Issues

### 1. Database is Locked

**Symptoms:**
- Error: "database is locked"
- Timeouts on database operations
- Slow response times

**Root Causes:**
- Concurrent write operations
- Long-running transactions
- WAL mode not enabled

**Solutions:**

```bash
# Enable WAL mode
sqlite3 coordination.db "PRAGMA journal_mode=WAL;"

# Check busy timeout
sqlite3 coordination.db "PRAGMA busy_timeout;"

# Increase busy timeout
sqlite3 coordination.db "PRAGMA busy_timeout=30000;"
```

**Prevention:**
```yaml
# config/production.yaml
database:
  coordination_db:
    max_open_conns: 1  # Single writer
    busy_timeout: "30s"
```

### 2. Workflow Stuck in Progress

**Symptoms:**
- Workflow status remains "in_progress" indefinitely
- Agent not making progress
- No activity logs

**Diagnosis:**
```bash
# Check agent health
workflow agent status <agent-id>

# View workflow logs
workflow logs <workflow-id> --follow

# Check database
sqlite3 coordination.db "SELECT * FROM agent_assignments WHERE workflow_id = '<id>';"
```

**Solutions:**

**Option 1: Restart agent**
```bash
# If agent is unresponsive
workflow agent restart <agent-id>
```

**Option 2: Manual fail**
```bash
# Force workflow to failed state
workflow cancel <workflow-id> --reason "Agent timeout"
```

**Option 3: Reassign to different agent**
```bash
workflow agent assign <workflow-id> <new-agent-id>
```

### 3. Beads Sync Conflicts

**Symptoms:**
- Git merge conflicts in issues.jsonl
- Sync failures
- Duplicate issues

**Resolution:**
```bash
# Automatic resolution (keep all operations)
cd .beads
git checkout --ours issues.jsonl
git add issues.jsonl
git rebase --continue

# Rebuild database
rm beads.db*
bd sync --import-only
```

**Prevention:**
- Use automatic sync with debouncing
- Configure conflict resolution strategy
- Enable pre-sync hooks

### 4. High Memory Usage

**Symptoms:**
- Out of memory errors
- Slow performance
- System instability

**Diagnosis:**
```bash
# Check memory usage
ps aux | grep workflow-server

# Check database size
du -sh data/*.db

# Monitor with pprof
curl http://localhost:6060/debug/pprof/heap > heap.prof
go tool pprof heap.prof
```

**Solutions:**

```yaml
# config/production.yaml
# Limit cache size
cache:
  memory:
    max_cost: 536870912  # 512MB
    
# Limit connection pools
database:
  coordination_db:
    max_open_conns: 1
    max_idle_conns: 1
```

**Code fix:**
```go
// Add memory limits to cache
cache, err := ristretto.NewCache(&ristretto.Config{
    NumCounters: 1e7,
    MaxCost:     512 * 1024 * 1024,  // 512MB
    BufferItems: 64,
})
```

### 5. API Rate Limiting

**Symptoms:**
- HTTP 429 Too Many Requests
- Request throttling
- Slow client responses

**Diagnosis:**
```bash
# Check rate limit status
curl -H "Authorization: Bearer <token>" \
     http://localhost:8080/api/v1/rate-limit-status

# View logs
tail -f /var/log/workflow/app.log | grep "rate limit"
```

**Solutions:**

```yaml
# Increase rate limits
server:
  rate_limit:
    requests_per_minute: 200
    burst: 50
```

**Client-side:**
```go
// Implement exponential backoff
func makeRequestWithRetry() (*Response, error) {
    var lastErr error
    backoff := time.Second
    
    for attempt := 0; attempt < 5; attempt++ {
        resp, err := makeRequest()
        if err == nil {
            return resp, nil
        }
        
        if resp.StatusCode == 429 {
            // Rate limited, wait and retry
            time.Sleep(backoff)
            backoff *= 2
            lastErr = err
            continue
        }
        
        return nil, err
    }
    
    return nil, lastErr
}
```

## Log Analysis

### Enable Debug Logging

```yaml
# config/debug.yaml
logging:
  level: debug
  format: console
```

### Common Log Patterns

**Successful workflow start:**
```
INFO[2026-02-07T10:30:00Z] Starting workflow 
  workflow_id=wf-research-001 
  workflow_type=research
INFO[2026-02-07T10:30:01Z] Created beads issue 
  issue_id=bd-a1b2
INFO[2026-02-07T10:30:01Z] Workflow started successfully
```

**Database lock detected:**
```
WARN[2026-02-07T10:30:02Z] Database busy, retrying
  error="database is locked"
  attempt=1
```

**Agent timeout:**
```
ERROR[2026-02-07T10:45:00Z] Agent timeout
  agent_id=research-agent-01
  workflow_id=wf-research-001
  elapsed=15m0s
```

## Debugging Tools

### 1. Database Inspection

```bash
# List all workflows
sqlite3 coordination.db "SELECT * FROM workflow_mappings LIMIT 10;"

# Check agent assignments
sqlite3 coordination.db "SELECT * FROM agent_assignments WHERE status = 'assigned';"

# View performance metrics
sqlite3 coordination.db "SELECT * FROM workflow_performance ORDER BY duration_ms DESC LIMIT 10;"
```

### 2. Health Check

```bash
# Check system health
curl http://localhost:8080/health

# Detailed health status
curl http://localhost:8080/health/detailed
```

### 3. Profiling

```bash
# CPU profile
curl http://localhost:6060/debug/pprof/profile > cpu.prof
go tool pprof cpu.prof

# Memory profile
curl http://localhost:6060/debug/pprof/heap > heap.prof
go tool pprof heap.prof

# Goroutines
curl http://localhost:6060/debug/pprof/goroutine?debug=1
```

### 4. Metrics

```bash
# View Prometheus metrics
curl http://localhost:9090/metrics

# Check specific metric
curl http://localhost:9090/metrics | grep workflow_system_workflows_started_total
```

## Recovery Procedures

### Database Corruption Recovery

```bash
# 1. Stop the application
systemctl stop workflow-server

# 2. Backup corrupted database
cp coordination.db coordination.db.corrupted.$(date +%Y%m%d_%H%M%S)

# 3. Check database integrity
sqlite3 coordination.db "PRAGMA integrity_check;"

# 4. Attempt recovery
sqlite3 coordination.db ".recover" | sqlite3 coordination.db.recovered

# 5. Verify recovered database
sqlite3 coordination.db.recovered "PRAGMA integrity_check;"

# 6. Replace database
mv coordination.db.recovered coordination.db

# 7. Restart application
systemctl start workflow-server
```

### Complete System Recovery

```bash
# 1. Stop all services
systemctl stop workflow-server
systemctl stop workflow-agents

# 2. Backup current state
tar czvf workflow-backup-$(date +%Y%m%d_%H%M%S).tar.gz data/ .beads/

# 3. Restore from backup
tar xzvf workflow-backup-<date>.tar.gz

# 4. Rebuild databases
rm data/*.db*
bd sync --import-only

# 5. Start services
systemctl start workflow-server
systemctl start workflow-agents
```

## Getting Help

### Before Asking for Help

1. Check logs for error messages
2. Verify configuration is correct
3. Check system resources (CPU, memory, disk)
4. Review recent changes
5. Try basic troubleshooting steps

### Information to Provide

When reporting issues:
- Error messages and stack traces
- Configuration (sanitized)
- Log files (relevant sections)
- System information (OS, version)
- Steps to reproduce

### Support Channels

- **Documentation:** docs.workflow.yourdomain.com
- **GitHub Issues:** github.com/your-org/beads-workflow-system/issues
- **Slack:** #workflow-support
- **Email:** support@yourdomain.com

**Response Times:**
- Critical issues: 4 hours
- High priority: 24 hours
- Normal priority: 72 hours
- Low priority: 1 week