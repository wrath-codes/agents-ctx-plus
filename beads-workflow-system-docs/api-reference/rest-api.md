# REST API Reference

## Base URL

```
Production: https://api.workflow.yourdomain.com/v1
Local:      http://localhost:8080/api/v1
```

## Authentication

All requests require authentication via Bearer token:

```http
Authorization: Bearer <jwt-token>
```

## Common Response Format

```json
{
  "success": true,
  "data": { },
  "error": null,
  "request_id": "req-123456",
  "timestamp": "2026-02-07T10:30:00Z"
}
```

## Workflows

### POST /workflows

Start a new workflow.

**Request:**
```json
{
  "issue_title": "Research async Rust frameworks",
  "workflow_type": "research",
  "agent_type": "research",
  "priority": 1,
  "variables": {
    "query": "tokio async-std",
    "focus": "performance"
  },
  "template_id": "research-v1"
}
```

**Response (201):**
```json
{
  "success": true,
  "data": {
    "id": "wf-research-001",
    "beads_issue_id": "bd-a1b2",
    "type": "research",
    "status": "active",
    "agent_id": "research-agent-01",
    "started_at": "2026-02-07T10:30:00Z"
  }
}
```

### GET /workflows/:id

Get workflow status.

**Response (200):**
```json
{
  "success": true,
  "data": {
    "id": "wf-research-001",
    "status": "in_progress",
    "progress_percent": 65,
    "current_step": "documentation_analysis",
    "agent_id": "research-agent-01",
    "started_at": "2026-02-07T10:30:00Z",
    "estimated_end": "2026-02-07T10:45:00Z"
  }
}
```

### GET /workflows

List workflows.

**Query Parameters:**
- `status` - Filter by status
- `type` - Filter by type
- `agent` - Filter by agent
- `limit` - Limit results (default: 50)
- `offset` - Offset for pagination

**Response (200):**
```json
{
  "success": true,
  "data": {
    "workflows": [
      {
        "id": "wf-research-001",
        "status": "in_progress",
        "type": "research"
      }
    ],
    "pagination": {
      "total": 127,
      "limit": 50,
      "offset": 0
    }
  }
}
```

### DELETE /workflows/:id

Cancel workflow.

**Request:**
```json
{
  "reason": "User request"
}
```

### GET /workflows/:id/results

Get workflow results.

**Response (200):**
```json
{
  "success": true,
  "data": {
    "results": [
      {
        "result_type": "findings",
        "confidence_score": 0.85,
        "data": { }
      }
    ]
  }
}
```

## Agents

### POST /agents

Register agent.

**Request:**
```json
{
  "id": "research-agent-01",
  "type": "research",
  "capabilities": ["library_discovery", "documentation_analysis"],
  "max_workload": 5,
  "endpoints": {
    "health_check": "http://localhost:9001/health"
  }
}
```

### GET /agents/:id/status

Get agent status.

**Response (200):**
```json
{
  "success": true,
  "data": {
    "id": "research-agent-01",
    "status": "active",
    "current_workload": 2,
    "max_workload": 5,
    "last_heartbeat": "2026-02-07T10:47:30Z"
  }
}
```

## Analytics

### GET /analytics/performance

Get performance metrics.

**Query Parameters:**
- `period` - Time period (default: 7d)
- `type` - Workflow type

**Response (200):**
```json
{
  "success": true,
  "data": {
    "workflow_metrics": {
      "total_workflows": 42,
      "success_rate": 90.5
    },
    "performance_metrics": {
      "avg_execution_time_ms": 165000
    }
  }
}
```

## Error Responses

### 400 Bad Request
```json
{
  "success": false,
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Invalid workflow type"
  }
}
```

### 404 Not Found
```json
{
  "success": false,
  "error": {
    "code": "WORKFLOW_NOT_FOUND",
    "message": "Workflow 'wf-123' not found"
  }
}
```

### 500 Internal Server Error
```json
{
  "success": false,
  "error": {
    "code": "INTERNAL_ERROR",
    "message": "An unexpected error occurred"
  }
}
```