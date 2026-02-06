# MCP Server

## Overview

Turso Database includes a built-in MCP (Model Context Protocol) server, enabling AI agents to interact with databases through standardized tools.

## What is MCP?

MCP (Model Context Protocol) is a protocol for connecting AI assistants with external tools and data sources:
- **Standardized Interface**: Common protocol for tool invocation
- **Type Safety**: JSON-RPC based with schema validation
- **Security**: Controlled access to resources
- **Composability**: Mix multiple MCP servers

## MCP Server Features

### Available Tools

#### 1. query
Execute read-only SQL queries

**Schema:**
```json
{
  "name": "query",
  "description": "Execute a read-only SQL query",
  "inputSchema": {
    "type": "object",
    "properties": {
      "sql": {
        "type": "string",
        "description": "SQL SELECT statement"
      }
    },
    "required": ["sql"]
  }
}
```

**Example:**
```json
{
  "tool": "query",
  "arguments": {
    "sql": "SELECT * FROM users WHERE active = 1 LIMIT 10"
  }
}
```

**Response:**
```json
{
  "columns": ["id", "name", "email"],
  "rows": [
    [1, "Alice", "alice@example.com"],
    [2, "Bob", "bob@example.com"]
  ]
}
```

#### 2. execute
Execute SQL statements (INSERT, UPDATE, DELETE, DDL)

**Schema:**
```json
{
  "name": "execute",
  "description": "Execute a SQL statement",
  "inputSchema": {
    "type": "object",
    "properties": {
      "sql": {
        "type": "string",
        "description": "SQL statement to execute"
      }
    },
    "required": ["sql"]
  }
}
```

**Example:**
```json
{
  "tool": "execute",
  "arguments": {
    "sql": "INSERT INTO users (name, email) VALUES ('Charlie', 'charlie@example.com')"
  }
}
```

**Response:**
```json
{
  "lastInsertRowId": 3,
  "rowsAffected": 1
}
```

#### 3. schema
Get database schema information

**Schema:**
```json
{
  "name": "schema",
  "description": "Get database schema",
  "inputSchema": {
    "type": "object",
    "properties": {
      "table": {
        "type": "string",
        "description": "Optional table name to get specific schema"
      }
    }
  }
}
```

**Example:**
```json
{
  "tool": "schema",
  "arguments": {
    "table": "users"
  }
}
```

**Response:**
```json
{
  "tables": [
    {
      "name": "users",
      "columns": [
        {"name": "id", "type": "INTEGER", "notNull": true, "primaryKey": true},
        {"name": "name", "type": "TEXT", "notNull": true},
        {"name": "email", "type": "TEXT", "notNull": true},
        {"name": "created_at", "type": "DATETIME", "default": "CURRENT_TIMESTAMP"}
      ],
      "indexes": [
        {"name": "idx_users_email", "columns": ["email"], "unique": true}
      ]
    }
  ]
}
```

#### 4. vector_search
Search using vector similarity

**Schema:**
```json
{
  "name": "vector_search",
  "description": "Search for similar vectors",
  "inputSchema": {
    "type": "object",
    "properties": {
      "table": {
        "type": "string",
        "description": "Table name"
      },
      "column": {
        "type": "string",
        "description": "Vector column name"
      },
      "vector": {
        "type": "array",
        "items": {"type": "number"},
        "description": "Query vector"
      },
      "limit": {
        "type": "integer",
        "description": "Maximum results",
        "default": 10
      }
    },
    "required": ["table", "column", "vector"]
  }
}
```

**Example:**
```json
{
  "tool": "vector_search",
  "arguments": {
    "table": "documents",
    "column": "embedding",
    "vector": [0.1, 0.2, 0.3, 0.4],
    "limit": 5
  }
}
```

#### 5. transaction
Manage database transactions

**Schema:**
```json
{
  "name": "transaction",
  "description": "Start, commit, or rollback a transaction",
  "inputSchema": {
    "type": "object",
    "properties": {
      "action": {
        "type": "string",
        "enum": ["begin", "commit", "rollback"],
        "description": "Transaction action"
      },
      "isolation": {
        "type": "string",
        "enum": ["deferred", "immediate", "exclusive"],
        "description": "Isolation level for BEGIN"
      }
    },
    "required": ["action"]
  }
}
```

## Setting Up MCP Server

### 1. Start MCP Server

```bash
# Start MCP server on local database
turso mcp --db ./mydb.db --port 8080

# With authentication
turso mcp --db ./mydb.db --port 8080 --token my-secret-token
```

### 2. Configure MCP Client

#### Claude Desktop
```json
// claude_desktop_config.json
{
  "mcpServers": {
    "turso": {
      "command": "turso",
      "args": ["mcp", "--db", "./mydb.db", "--port", "8080"]
    }
  }
}
```

#### Cursor
```json
// .cursor/mcp.json
{
  "mcpServers": [
    {
      "name": "turso",
      "command": "turso mcp --db ./mydb.db --port 8080"
    }
  ]
}
```

#### Generic MCP Client
```python
from mcp import ClientSession, StdioServerParameters

server_params = StdioServerParameters(
    command="turso",
    args=["mcp", "--db", "./mydb.db"]
)

async with ClientSession(server_params) as session:
    # List available tools
    tools = await session.list_tools()
    
    # Execute query
    result = await session.call_tool("query", {
        "sql": "SELECT * FROM users LIMIT 5"
    })
```

## Security Considerations

### Access Control
```bash
# Use tokens for authentication
turso mcp --db ./mydb.db --token $(cat ~/.turso_mcp_token)

# Restrict to read-only mode
turso mcp --db ./mydb.db --read-only
```

### Query Validation
The MCP server validates all queries:
- **query tool**: Only SELECT statements allowed
- **execute tool**: No DROP DATABASE or dangerous operations
- **All tools**: SQL injection prevention via parameterized queries

## Integration Examples

### AI Agent Workflow
```python
# AI agent using Turso MCP
async def analyze_data(session):
    # Get schema
    schema = await session.call_tool("schema", {})
    
    # Query data
    users = await session.call_tool("query", {
        "sql": "SELECT COUNT(*) as count FROM users"
    })
    
    # Vector search for similar items
    similar = await session.call_tool("vector_search", {
        "table": "documents",
        "column": "embedding",
        "vector": embedding_model.encode("query text"),
        "limit": 5
    })
    
    return {
        "schema": schema,
        "user_count": users["rows"][0][0],
        "similar_documents": similar
    }
```

### Multi-Agent Coordination
```python
# Multiple agents sharing database via MCP
async def agent_1_task(session):
    await session.call_tool("transaction", {"action": "begin"})
    await session.call_tool("execute", {
        "sql": "INSERT INTO tasks (agent, status) VALUES ('agent-1', 'working')"
    })
    # ... do work ...
    await session.call_tool("transaction", {"action": "commit"})

async def agent_2_task(session):
    # Can see agent-1's changes immediately
    tasks = await session.call_tool("query", {
        "sql": "SELECT * FROM tasks WHERE agent = 'agent-1'"
    })
```

## Next Steps

- **Turso Cloud**: [../turso-cloud/01-overview.md](../turso-cloud/01-overview.md)
- **AgentFS**: [../agentfs/01-overview.md](../agentfs/01-overview.md)