# REST API Service

The Cortex Memory Service provides a comprehensive REST API for integrating memory capabilities into any application, regardless of programming language.

---

## Overview

- **Framework**: Built with [Axum](https://github.com/tokio-rs/axum)
- **Protocol**: HTTP/1.1 and HTTP/2
- **Format**: JSON
- **Authentication**: API key-based (configured externally)
- **CORS**: Configurable cross-origin support

---

## Starting the Service

### Command Line

```bash
# Start with default config (config.toml)
cortex-mem-service

# Start with custom config
cortex-mem-service --config /path/to/config.toml

# With verbose logging
RUST_LOG=debug cortex-mem-service
```

### Service Configuration

```toml
[server]
host = "0.0.0.0"           # Bind address
port = 8000                # Port number
cors_origins = ["*"]       # CORS origins (use ["*"] for development)
```

---

## API Endpoints

### Health & Status

#### Health Check
```http
GET /health
```

**Response**:
```json
{
  "status": "healthy",
  "vector_store": true,
  "llm_service": true,
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### LLM Status
```http
GET /llm/status
```

**Response**:
```json
{
  "overall_status": "healthy",
  "completion_model": {
    "available": true,
    "provider": "openai",
    "model_name": "gpt-4o-mini",
    "latency_ms": 150,
    "last_check": "2024-01-15T10:30:00Z"
  },
  "embedding_model": {
    "available": true,
    "provider": "openai",
    "model_name": "text-embedding-3-small",
    "latency_ms": 80,
    "last_check": "2024-01-15T10:30:00Z"
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### Simple LLM Health Check
```http
GET /llm/health-check
```

**Response**:
```json
{
  "completion_model_available": true,
  "embedding_model_available": true,
  "timestamp": "2024-01-15T10:30:00Z"
}
```

---

### Memory Management

#### Create Memory
```http
POST /memories
Content-Type: application/json

{
  "content": "User prefers dark mode in all applications",
  "user_id": "user123",
  "agent_id": "assistant",
  "memory_type": "Personal",
  "metadata": {
    "importance": 0.9
  }
}
```

**Response** (201 Created):
```json
{
  "message": "Memory created successfully",
  "id": "uuid-of-created-memory"
}
```

**Complex Example with Conversation**:
```http
POST /memories
Content-Type: application/json

{
  "content": "User: I love hiking in the mountains\nAssistant: That sounds wonderful! Do you have a favorite trail?\nUser: Yes, I love the Appalachian Trail",
  "user_id": "user123",
  "agent_id": "outdoor-assistant",
  "memory_type": "Conversational"
}
```

#### Get Memory
```http
GET /memories/{id}
```

**Response** (200 OK):
```json
{
  "id": "memory-uuid",
  "content": "User prefers dark mode in all applications",
  "metadata": {
    "user_id": "user123",
    "agent_id": "assistant",
    "memory_type": "Personal",
    "hash": "sha256-hash",
    "importance_score": 0.9,
    "custom": {}
  },
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### Update Memory
```http
PUT /memories/{id}
Content-Type: application/json

{
  "content": "Updated content here"
}
```

**Response** (200 OK):
```json
{
  "message": "Memory updated successfully",
  "id": "memory-uuid"
}
```

#### Delete Memory
```http
DELETE /memories/{id}
```

**Response** (200 OK):
```json
{
  "message": "Memory deleted successfully",
  "id": "memory-uuid"
}
```

#### List Memories
```http
GET /memories?user_id=user123&memory_type=Personal&limit=20
```

**Query Parameters**:
- `user_id` - Filter by user
- `agent_id` - Filter by agent
- `run_id` - Filter by run/session
- `memory_type` - Filter by type (Conversational, Personal, Factual, etc.)
- `limit` - Maximum results (default: 100)

**Response** (200 OK):
```json
{
  "total": 15,
  "memories": [
    {
      "id": "memory-uuid-1",
      "content": "...",
      "metadata": {...},
      "created_at": "...",
      "updated_at": "..."
    }
  ]
}
```

#### Search Memories
```http
POST /memories/search
Content-Type: application/json

{
  "query": "What are the user's preferences?",
  "filters": {
    "user_id": "user123",
    "memory_type": "Personal"
  },
  "limit": 10,
  "similarity_threshold": 0.70
}
```

**Request Fields**:
- `query` (required) - Natural language search query
- `filters` (optional) - Metadata filters
- `limit` (optional) - Max results (default: 10)
- `similarity_threshold` (optional) - Minimum similarity score

**Response** (200 OK):
```json
{
  "total": 5,
  "results": [
    {
      "memory": {
        "id": "memory-uuid",
        "content": "User prefers dark mode in all applications",
        "metadata": {...},
        "created_at": "...",
        "updated_at": "..."
      },
      "score": 0.92
    }
  ]
}
```

#### Batch Delete
```http
POST /memories/batch/delete
Content-Type: application/json

{
  "ids": ["memory-uuid-1", "memory-uuid-2", "memory-uuid-3"]
}
```

**Response** (200 OK or 207 Multi-Status):
```json
{
  "success_count": 2,
  "failure_count": 1,
  "errors": ["Failed to delete memory-uuid-2: not found"],
  "message": "Batch delete completed: 2 succeeded, 1 failed"
}
```

#### Batch Update
```http
POST /memories/batch/update
Content-Type: application/json

{
  "updates": [
    {
      "id": "memory-uuid-1",
      "content": "Updated content 1"
    },
    {
      "id": "memory-uuid-2",
      "content": "Updated content 2"
    }
  ]
}
```

**Response** (200 OK or 207 Multi-Status):
```json
{
  "success_count": 2,
  "failure_count": 0,
  "errors": [],
  "message": "Batch update completed: 2 succeeded, 0 failed"
}
```

---

### Optimization

#### Start Optimization
```http
POST /optimization
Content-Type: application/json

{
  "strategy": "Full",
  "filters": {
    "user_id": "user123"
  },
  "dry_run": false,
  "timeout_minutes": 30
}
```

**Request Fields**:
- `strategy` - Optimization strategy (Full, Deduplication, Quality, Relevance, Space)
- `filters` - Scope the optimization to specific memories
- `dry_run` - Preview without executing (default: false)
- `timeout_minutes` - Maximum optimization time

**Response** (202 Accepted):
```json
{
  "optimization_id": "opt-uuid",
  "status": "started",
  "estimated_duration": "15 minutes"
}
```

#### Get Optimization Status
```http
GET /optimization/{job_id}
```

**Response** (200 OK):
```json
{
  "optimization_id": "opt-uuid",
  "status": "running",
  "progress": 45,
  "current_phase": "detecting_duplicates",
  "started_at": "2024-01-15T10:30:00Z",
  "estimated_completion": "2024-01-15T10:45:00Z"
}
```

#### Cancel Optimization
```http
POST /optimization/{job_id}/cancel
```

**Response** (200 OK):
```json
{
  "message": "Optimization cancelled",
  "optimization_id": "opt-uuid"
}
```

#### Get Optimization History
```http
GET /optimization/history
```

**Response** (200 OK):
```json
{
  "optimizations": [
    {
      "optimization_id": "opt-uuid-1",
      "strategy": "Full",
      "status": "completed",
      "started_at": "2024-01-14T02:00:00Z",
      "end_time": "2024-01-14T02:15:00Z",
      "issues_found": 12,
      "actions_performed": 8
    }
  ]
}
```

#### Get Optimization Statistics
```http
GET /optimization/statistics
```

**Response** (200 OK):
```json
{
  "total_optimizations": 10,
  "last_optimization": "2024-01-14T02:15:00Z",
  "memory_count_before": 1000,
  "memory_count_after": 920,
  "saved_space_mb": 15.5,
  "deduplication_rate": 0.08,
  "quality_improvement": 0.12
}
```

#### Analyze Optimization (Dry Run)
```http
POST /optimization/analyze
Content-Type: application/json

{
  "strategy": "Full",
  "filters": {
    "user_id": "user123"
  }
}
```

**Response** (200 OK):
```json
{
  "optimization_id": "preview-uuid",
  "strategy": "Full",
  "estimated_duration_minutes": 15,
  "issues_found": [
    {
      "id": "issue-1",
      "kind": "Duplicate",
      "severity": "Medium",
      "description": "Found 3 similar memories about user preferences",
      "affected_memories": ["mem-1", "mem-2", "mem-3"],
      "recommendation": "Merge into single memory"
    }
  ],
  "actions_planned": [
    {
      "type": "Merge",
      "memories": ["mem-1", "mem-2", "mem-3"]
    }
  ]
}
```

#### Cleanup History
```http
POST /optimization/cleanup
Content-Type: application/json

{
  "keep_last": 10,
  "older_than_days": 30
}
```

---

## Error Responses

### Error Format

```json
{
  "error": "Human-readable error message",
  "code": "ERROR_CODE"
}
```

### HTTP Status Codes

| Status | Meaning |
|--------|---------|
| 200 OK | Request successful |
| 201 Created | Resource created successfully |
| 202 Accepted | Request accepted, processing async |
| 400 Bad Request | Invalid request format or parameters |
| 404 Not Found | Resource not found |
| 500 Internal Server Error | Server error |
| 207 Multi-Status | Batch operation partially successful |

### Common Error Codes

| Code | Description |
|------|-------------|
| `MEMORY_NOT_FOUND` | The requested memory doesn't exist |
| `MEMORY_CREATION_FAILED` | Failed to create memory |
| `MEMORY_UPDATE_FAILED` | Failed to update memory |
| `MEMORY_DELETION_FAILED` | Failed to delete memory |
| `MEMORY_SEARCH_FAILED` | Search operation failed |
| `HEALTH_CHECK_FAILED` | System health check failed |
| `OPTIMIZATION_FAILED` | Optimization operation failed |
| `BATCH_DELETE_PARTIAL_FAILURE` | Some deletions failed |
| `BATCH_UPDATE_PARTIAL_FAILURE` | Some updates failed |

---

## Request/Response Models

### CreateMemoryRequest

```json
{
  "content": "string (required)",
  "user_id": "string (optional)",
  "agent_id": "string (optional)",
  "run_id": "string (optional)",
  "actor_id": "string (optional)",
  "role": "string (optional)",
  "memory_type": "string (optional, default: Conversational)",
  "metadata": "object (optional)"
}
```

### SearchMemoryRequest

```json
{
  "query": "string (required)",
  "filters": {
    "user_id": "string (optional)",
    "agent_id": "string (optional)",
    "run_id": "string (optional)",
    "actor_id": "string (optional)",
    "memory_type": "string (optional)",
    "min_importance": "number (optional)",
    "max_importance": "number (optional)",
    "created_after": "ISO8601 datetime (optional)",
    "created_before": "ISO8601 datetime (optional)"
  },
  "limit": "number (optional, default: 10)",
  "similarity_threshold": "number (optional)"
}
```

### MemoryResponse

```json
{
  "id": "string",
  "content": "string",
  "metadata": {
    "user_id": "string | null",
    "agent_id": "string | null",
    "run_id": "string | null",
    "actor_id": "string | null",
    "role": "string | null",
    "memory_type": "string",
    "hash": "string",
    "importance_score": "number | null",
    "entities": ["string"],
    "topics": ["string"],
    "custom": "object"
  },
  "created_at": "ISO8601 datetime",
  "updated_at": "ISO8601 datetime"
}
```

---

## Client Examples

### cURL Examples

```bash
# Store a memory
curl -X POST http://localhost:8000/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User is learning Rust programming",
    "user_id": "user123",
    "memory_type": "Personal"
  }'

# Search memories
curl -X POST http://localhost:8000/memories/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is the user learning?",
    "filters": {
      "user_id": "user123"
    },
    "limit": 5
  }'

# List memories
curl "http://localhost:8000/memories?user_id=user123&limit=10"
```

### Python Example

```python
import requests

BASE_URL = "http://localhost:8000"

# Store memory
response = requests.post(f"{BASE_URL}/memories", json={
    "content": "User prefers tea over coffee",
    "user_id": "user123",
    "memory_type": "Personal"
})
memory_id = response.json()["id"]

# Search memories
response = requests.post(f"{BASE_URL}/memories/search", json={
    "query": "What does the user drink?",
    "filters": {"user_id": "user123"},
    "limit": 5
})
results = response.json()["results"]

for result in results:
    print(f"Score: {result['score']:.2f}")
    print(f"Content: {result['memory']['content']}")
```

### JavaScript/TypeScript Example

```typescript
const BASE_URL = "http://localhost:8000";

// Store memory
const storeMemory = async (content: string, userId: string) => {
  const response = await fetch(`${BASE_URL}/memories`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      content,
      user_id: userId,
      memory_type: "Personal"
    })
  });
  return response.json();
};

// Search memories
const searchMemories = async (query: string, userId: string) => {
  const response = await fetch(`${BASE_URL}/memories/search`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      query,
      filters: { user_id: userId },
      limit: 5
    })
  });
  return response.json();
};
```

---

## Best Practices

### 1. Always Filter by User

```http
# Good: Filtered by user
GET /memories?user_id=user123

# Avoid: Unfiltered in multi-tenant systems
GET /memories
```

### 2. Use Appropriate Memory Types

```json
{
  "content": "User preference content",
  "memory_type": "Personal",
  "user_id": "user123"
}
```

### 3. Handle Errors Gracefully

```python
try:
    response = requests.post(f"{BASE_URL}/memories", json=data)
    response.raise_for_status()
    result = response.json()
except requests.exceptions.HTTPError as e:
    if e.response.status_code == 404:
        print("Memory not found")
    elif e.response.status_code == 500:
        print("Server error")
```

### 4. Set Reasonable Limits

```json
{
  "query": "user preferences",
  "limit": 10
}
```

### 5. Use Similarity Thresholds

```json
{
  "query": "What does the user like?",
  "similarity_threshold": 0.70
}
```

---

## Next Steps

- [CLI Usage](../cli/commands.md) - Command-line interface
- [MCP Integration](../mcp/overview.md) - Model Context Protocol
- [Rig Integration](../rig/overview.md) - Rig framework integration
- [Configuration](../config/server.md) - Server configuration
