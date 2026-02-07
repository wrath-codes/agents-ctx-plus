# Quick Start Guide

## Prerequisites

- Go 1.21 or higher
- SQLite 3.35 or higher
- Git 2.30 or higher

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/your-org/beads-workflow-system.git
cd beads-workflow-system
```

### 2. Install Dependencies

```bash
# Download Go modules
go mod download

# Or use make
make deps
```

### 3. Build

```bash
# Build CLI tool
go build -o workflow ./cmd/workflow

# Build server
go build -o workflow-server ./cmd/server

# Or use make
make build
```

### 4. Initialize

```bash
# Initialize beads
bd init

# Create data directory
mkdir -p data

# Copy default config
cp configs/default.yaml data/config.yaml
```

## First Workflow

### 1. Start the Server

```bash
./workflow-server --config data/config.yaml
```

### 2. Start a Workflow

```bash
./workflow start research "My first workflow" \
  --agent research \
  --priority 1
```

**Expected output:**
```
✅ Workflow started successfully!

📋 Workflow ID: wf-research-001
🔗 Beads Issue: bd-a1b2
🤖 Agent: research-agent-01
⏰ Started: 2026-02-07T10:30:00Z

💡 Check status with: workflow status wf-research-001
```

### 3. Check Status

```bash
./workflow status wf-research-001
```

**Output:**
```
📋 Workflow: wf-research-001
Status:       in_progress
Agent:        research-agent-01
Progress:     25%
Current Step: library_discovery
Started:      2026-02-07T10:30:00Z
Estimated:    2026-02-07T10:45:00Z
```

### 4. View Results

```bash
./workflow results wf-research-001
```

### 5. List All Workflows

```bash
./workflow list
```

## Docker Quick Start

### 1. Build Image

```bash
docker build -t beads-workflow-system .
```

### 2. Run Container

```bash
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/data:/data \
  beads-workflow-system
```

### 3. Use CLI

```bash
docker exec -it <container> workflow list
```

## API Quick Start

### 1. Start Workflow via API

```bash
curl -X POST http://localhost:8080/api/v1/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "issue_title": "API test workflow",
    "workflow_type": "research",
    "agent_type": "research"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "wf-research-002",
    "status": "active"
  }
}
```

### 2. Get Status

```bash
curl http://localhost:8080/api/v1/workflows/wf-research-002
```

### 3. Cancel Workflow

```bash
curl -X DELETE http://localhost:8080/api/v1/workflows/wf-research-002 \
  -H "Content-Type: application/json" \
  -d '{"reason": "Testing"}'
```

## Configuration

### Basic Config

Edit `data/config.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8080

database:
  coordination_db:
    path: "./data/coordination.db"

logging:
  level: "debug"
```

### Environment Variables

```bash
export WORKFLOW_LOGGING_LEVEL=debug
export WORKFLOW_SERVER_PORT=8080
```

## Common Commands

```bash
# Start workflow
workflow start research "Research task"

# Check status
workflow status <workflow-id>

# List workflows
workflow list --status active

# Cancel workflow
workflow cancel <workflow-id> --reason "Done"

# View results
workflow results <workflow-id>

# View logs
workflow logs <workflow-id> --follow

# Register agent
workflow agent register --config agent.yaml

# View analytics
workflow analytics performance --period 7d
```

## Troubleshooting

### Port Already in Use

```bash
# Kill process using port 8080
lsof -ti:8080 | xargs kill -9

# Or use different port
workflow-server --port 8081
```

### Database Locked

```bash
# Enable WAL mode
sqlite3 data/coordination.db "PRAGMA journal_mode=WAL;"
```

### Permission Denied

```bash
# Fix permissions
chmod +x workflow workflow-server
```

## Next Steps

1. **Read Full Documentation**
   - [Architecture](../architecture/system-architecture.md)
   - [CLI Reference](../cli-reference/commands.md)
   - [API Reference](../api-reference/rest-api.md)

2. **Explore Examples**
   ```bash
   # See example workflows
   ls examples/
   
   # Run example
   workflow start research examples/simple-research.yaml
   ```

3. **Configure for Production**
   - [Production Deployment](../deployment/production.md)
   - [Security Configuration](../security/security.md)

## Getting Help

- **Documentation:** See README.md in docs/
- **Issues:** GitHub Issues
- **Slack:** #workflow-support

## Quick Reference Card

```
┌─────────────────────────────────────────────┐
│          QUICK REFERENCE                    │
├─────────────────────────────────────────────┤
│ START:  workflow start <type> <title>      │
│ STATUS: workflow status <id>               │
│ LIST:   workflow list                      │
│ CANCEL: workflow cancel <id>               │
│ RESULTS: workflow results <id>             │
│ LOGS:   workflow logs <id>                 │
├─────────────────────────────────────────────┤
│ API:    curl http://localhost:8080/api/v1  │
│ HEALTH: curl http://localhost:8080/health  │
└─────────────────────────────────────────────┘
```