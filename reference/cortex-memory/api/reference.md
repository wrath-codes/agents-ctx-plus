# API Reference

Complete API reference for Cortex Memory.

---

## Rust Library API

### Core Types

#### Memory

```rust
pub struct Memory {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: MemoryMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Memory {
    pub fn new(content: String, embedding: Vec<f32>, metadata: MemoryMetadata) -> Self;
    pub fn update_content(&mut self, content: String, embedding: Vec<f32>);
    pub fn compute_hash(content: &str) -> String;
}
```

#### MemoryMetadata

```rust
pub struct MemoryMetadata {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub role: Option<String>,
    pub memory_type: MemoryType,
    pub hash: String,
    pub importance_score: f32,
    pub entities: Vec<String>,
    pub topics: Vec<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

impl MemoryMetadata {
    pub fn new(memory_type: MemoryType) -> Self;
    pub fn with_user_id(self, user_id: String) -> Self;
    pub fn with_agent_id(self, agent_id: String) -> Self;
    pub fn with_importance_score(self, score: f32) -> Self;
    pub fn with_entities(self, entities: Vec<String>) -> Self;
    pub fn with_topics(self, topics: Vec<String>) -> Self;
}
```

#### MemoryType

```rust
pub enum MemoryType {
    Conversational,
    Procedural,
    Factual,
    Semantic,
    Episodic,
    Personal,
}

impl MemoryType {
    pub fn parse(s: &str) -> Self;
    pub fn parse_with_result(s: &str) -> Result<Self, String>;
}
```

#### ScoredMemory

```rust
pub struct ScoredMemory {
    pub memory: Memory,
    pub score: f32,
}
```

#### Filters

```rust
pub struct Filters {
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub run_id: Option<String>,
    pub actor_id: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub min_importance: Option<f32>,
    pub max_importance: Option<f32>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub entities: Option<Vec<String>>,
    pub topics: Option<Vec<String>>,
    pub custom: HashMap<String, serde_json::Value>,
}

impl Filters {
    pub fn new() -> Self;
    pub fn for_user(user_id: &str) -> Self;
    pub fn for_agent(agent_id: &str) -> Self;
    pub fn for_run(run_id: &str) -> Self;
    pub fn with_memory_type(self, memory_type: MemoryType) -> Self;
}
```

### MemoryManager

```rust
pub struct MemoryManager;

impl MemoryManager {
    // Creation
    pub fn new(
        vector_store: Box<dyn VectorStore>,
        llm_client: Box<dyn LLMClient>,
        config: MemoryConfig,
    ) -> Self;
    
    // Storage
    pub async fn store(
        &self,
        content: String,
        metadata: MemoryMetadata,
    ) -> Result<String>;
    
    pub async fn add_memory(
        &self,
        messages: &[Message],
        metadata: MemoryMetadata,
    ) -> Result<Vec<MemoryResult>>;
    
    // Retrieval
    pub async fn search(
        &self,
        query: &str,
        filters: &Filters,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>>;
    
    pub async fn search_with_threshold(
        &self,
        query: &str,
        filters: &Filters,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<ScoredMemory>>;
    
    pub async fn get(&self, id: &str) -> Result<Option<Memory>>;
    
    pub async fn list(
        &self,
        filters: &Filters,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>>;
    
    // Updates
    pub async fn update(&self, id: &str, content: String) -> Result<()>;
    
    pub async fn smart_update(&self, id: &str, new_content: String) -> Result<()>;
    
    pub async fn update_metadata(
        &self,
        id: &str,
        new_memory_type: MemoryType,
    ) -> Result<()>;
    
    pub async fn update_complete_memory(
        &self,
        id: &str,
        new_content: Option<String>,
        new_memory_type: Option<MemoryType>,
        new_importance: Option<f32>,
        new_entities: Option<Vec<String>>,
        new_topics: Option<Vec<String>>,
        new_custom: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()>;
    
    // Deletion
    pub async fn delete(&self, id: &str) -> Result<()>;
    
    // Utilities
    pub async fn health_check(&self) -> Result<HealthStatus>;
    
    pub async fn get_stats(&self, filters: &Filters) -> Result<MemoryStats>;
    
    pub fn llm_client(&self) -> &dyn LLMClient;
}
```

### Initialization

```rust
use cortex_mem_core::init::initialize_memory_system;
use cortex_mem_config::Config;

pub async fn initialize_memory_system(
    config: &Config,
) -> Result<(Box<dyn VectorStore>, Box<dyn LLMClient>)>;
```

---

## REST API Endpoints

### Base URL

```
http://localhost:8000
```

### Health Endpoints

#### GET /health
Health check for the service.

**Response**:
```json
{
  "status": "healthy",
  "vector_store": true,
  "llm_service": true,
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### GET /llm/status
Detailed LLM service status.

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

### Memory Endpoints

#### POST /memories
Create a new memory.

**Request**:
```json
{
  "content": "User prefers dark mode",
  "user_id": "user123",
  "agent_id": "assistant",
  "memory_type": "Personal",
  "metadata": {}
}
```

**Response** (201):
```json
{
  "message": "Memory created successfully",
  "id": "uuid"
}
```

#### GET /memories/{id}
Retrieve a memory by ID.

**Response** (200):
```json
{
  "id": "uuid",
  "content": "User prefers dark mode",
  "metadata": {
    "user_id": "user123",
    "agent_id": "assistant",
    "memory_type": "Personal",
    "hash": "...",
    "importance_score": 0.9,
    "custom": {}
  },
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### PUT /memories/{id}
Update a memory.

**Request**:
```json
{
  "content": "Updated content"
}
```

**Response** (200):
```json
{
  "message": "Memory updated successfully",
  "id": "uuid"
}
```

#### DELETE /memories/{id}
Delete a memory.

**Response** (200):
```json
{
  "message": "Memory deleted successfully",
  "id": "uuid"
}
```

#### GET /memories
List memories with filters.

**Query Parameters**:
- `user_id` - Filter by user
- `agent_id` - Filter by agent
- `run_id` - Filter by run
- `memory_type` - Filter by type
- `limit` - Maximum results

**Response** (200):
```json
{
  "total": 15,
  "memories": [...]
}
```

#### POST /memories/search
Search memories semantically.

**Request**:
```json
{
  "query": "What does the user prefer?",
  "filters": {
    "user_id": "user123"
  },
  "limit": 10,
  "similarity_threshold": 0.70
}
```

**Response** (200):
```json
{
  "total": 5,
  "results": [
    {
      "memory": {...},
      "score": 0.92
    }
  ]
}
```

#### POST /memories/batch/delete
Delete multiple memories.

**Request**:
```json
{
  "ids": ["id1", "id2", "id3"]
}
```

**Response** (200 or 207):
```json
{
  "success_count": 2,
  "failure_count": 1,
  "errors": [...],
  "message": "Batch delete completed: 2 succeeded, 1 failed"
}
```

#### POST /memories/batch/update
Update multiple memories.

**Request**:
```json
{
  "updates": [
    {"id": "id1", "content": "new content 1"},
    {"id": "id2", "content": "new content 2"}
  ]
}
```

### Optimization Endpoints

#### POST /optimization
Start optimization.

**Request**:
```json
{
  "strategy": "Full",
  "filters": {},
  "dry_run": false
}
```

**Response** (202):
```json
{
  "optimization_id": "opt-uuid",
  "status": "started"
}
```

#### GET /optimization/{job_id}
Get optimization status.

**Response** (200):
```json
{
  "optimization_id": "opt-uuid",
  "status": "running",
  "progress": 45,
  "current_phase": "detecting_duplicates"
}
```

#### POST /optimization/{job_id}/cancel
Cancel optimization.

**Response** (200):
```json
{
  "message": "Optimization cancelled"
}
```

---

## MCP Tools

### Tool Definitions

#### store_memory

Store a new memory.

```json
{
  "name": "store_memory",
  "description": "Store a new memory in the system",
  "inputSchema": {
    "type": "object",
    "properties": {
      "content": {
        "type": "string",
        "description": "The content to store"
      },
      "user_id": {
        "type": "string",
        "description": "User identifier"
      },
      "agent_id": {
        "type": "string",
        "description": "Agent identifier"
      },
      "memory_type": {
        "type": "string",
        "enum": ["Conversational", "Personal", "Factual", "Procedural", "Semantic", "Episodic"],
        "description": "Type of memory"
      },
      "topics": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Topics associated with this memory"
      }
    },
    "required": ["content"]
  }
}
```

#### query_memory

Query memories using semantic search.

```json
{
  "name": "query_memory",
  "description": "Search for memories using natural language",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Natural language search query"
      },
      "k": {
        "type": "integer",
        "description": "Number of results to return",
        "default": 5
      },
      "memory_type": {
        "type": "string",
        "description": "Filter by memory type"
      },
      "user_id": {
        "type": "string",
        "description": "Filter by user"
      },
      "agent_id": {
        "type": "string",
        "description": "Filter by agent"
      }
    },
    "required": ["query"]
  }
}
```

#### list_memories

List memories with metadata filters.

```json
{
  "name": "list_memories",
  "description": "List memories filtered by metadata",
  "inputSchema": {
    "type": "object",
    "properties": {
      "limit": {
        "type": "integer",
        "description": "Maximum number of memories to return",
        "default": 20
      },
      "memory_type": {
        "type": "string",
        "description": "Filter by memory type"
      },
      "user_id": {
        "type": "string",
        "description": "Filter by user"
      },
      "agent_id": {
        "type": "string",
        "description": "Filter by agent"
      }
    }
  }
}
```

#### get_memory

Get a specific memory by ID.

```json
{
  "name": "get_memory",
  "description": "Retrieve a specific memory by its ID",
  "inputSchema": {
    "type": "object",
    "properties": {
      "memory_id": {
        "type": "string",
        "description": "The unique identifier of the memory"
      }
    },
    "required": ["memory_id"]
  }
}
```

---

## Error Codes

### HTTP Status Codes

| Code | Description |
|------|-------------|
| 200 | Success |
| 201 | Created |
| 202 | Accepted |
| 400 | Bad Request |
| 404 | Not Found |
| 500 | Internal Server Error |
| 207 | Multi-Status |

### Error Response Format

```json
{
  "error": "Human-readable message",
  "code": "ERROR_CODE"
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `MEMORY_NOT_FOUND` | Memory ID doesn't exist |
| `MEMORY_CREATION_FAILED` | Failed to create memory |
| `MEMORY_UPDATE_FAILED` | Failed to update memory |
| `MEMORY_DELETION_FAILED` | Failed to delete memory |
| `MEMORY_SEARCH_FAILED` | Search operation failed |
| `HEALTH_CHECK_FAILED` | System health check failed |
| `OPTIMIZATION_FAILED` | Optimization failed |
| `VALIDATION_ERROR` | Invalid input data |
| `CONFIG_ERROR` | Configuration error |

---

## TypeScript Definitions

```typescript
// Core Types
interface Memory {
  id: string;
  content: string;
  metadata: MemoryMetadata;
  created_at: string;
  updated_at: string;
}

interface MemoryMetadata {
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  role?: string;
  memory_type: MemoryType;
  hash: string;
  importance_score?: number;
  entities: string[];
  topics: string[];
  custom: Record<string, any>;
}

type MemoryType = 
  | "Conversational" 
  | "Procedural" 
  | "Factual" 
  | "Semantic" 
  | "Episodic" 
  | "Personal";

interface ScoredMemory {
  memory: Memory;
  score: number;
}

interface Filters {
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  memory_type?: MemoryType;
  min_importance?: number;
  max_importance?: number;
  created_after?: string;
  created_before?: string;
  updated_after?: string;
  updated_before?: string;
  entities?: string[];
  topics?: string[];
  custom?: Record<string, any>;
}

// API Request/Response Types
interface CreateMemoryRequest {
  content: string;
  user_id?: string;
  agent_id?: string;
  run_id?: string;
  actor_id?: string;
  role?: string;
  memory_type?: MemoryType;
  metadata?: Record<string, any>;
}

interface SearchMemoryRequest {
  query: string;
  filters?: Filters;
  limit?: number;
  similarity_threshold?: number;
}

interface SearchResponse {
  total: number;
  results: ScoredMemory[];
}

interface ListResponse {
  total: number;
  memories: Memory[];
}
```

---

## Python Types

```python
from typing import Optional, List, Dict, Any
from datetime import datetime
from enum import Enum

class MemoryType(str, Enum):
    CONVERSATIONAL = "Conversational"
    PROCEDURAL = "Procedural"
    FACTUAL = "Factual"
    SEMANTIC = "Semantic"
    EPISODIC = "Episodic"
    PERSONAL = "Personal"

class MemoryMetadata:
    user_id: Optional[str]
    agent_id: Optional[str]
    run_id: Optional[str]
    actor_id: Optional[str]
    role: Optional[str]
    memory_type: MemoryType
    hash: str
    importance_score: Optional[float]
    entities: List[str]
    topics: List[str]
    custom: Dict[str, Any]

class Memory:
    id: str
    content: str
    metadata: MemoryMetadata
    created_at: datetime
    updated_at: datetime

class ScoredMemory:
    memory: Memory
    score: float

class Filters:
    user_id: Optional[str] = None
    agent_id: Optional[str] = None
    run_id: Optional[str] = None
    actor_id: Optional[str] = None
    memory_type: Optional[MemoryType] = None
    min_importance: Optional[float] = None
    max_importance: Optional[float] = None
    created_after: Optional[datetime] = None
    created_before: Optional[datetime] = None
    entities: Optional[List[str]] = None
    topics: Optional[List[str]] = None
    custom: Dict[str, Any] = {}
```

---

## CLI Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Configuration error |
| 4 | Connection error |
| 5 | Not found |

---

## Rate Limits

### Default Limits

| Operation | Limit |
|-----------|-------|
| Memory creation | 100/minute |
| Memory search | 1000/minute |
| Batch operations | 10/minute |
| Health checks | Unlimited |

**Note**: Actual limits depend on your LLM provider's rate limits.

---

## Version Compatibility

| API Version | cortex-mem-core | Status |
|-------------|-----------------|--------|
| 1.0.x | 1.0.x | Current |

---

## Next Steps

- [Rust API](../core/memory-manager.md) - Using the Rust library
- [REST API](../service/overview.md) - HTTP API details
- [MCP Integration](../mcp/overview.md) - Model Context Protocol
