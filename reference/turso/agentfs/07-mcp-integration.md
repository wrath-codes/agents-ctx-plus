# MCP Integration

## Overview

AgentFS includes a built-in MCP (Model Context Protocol) server, enabling AI agents to interact with filesystem operations through standardized tools. This allows AI agents to safely work with files in isolated workspaces.

## MCP Server Features

### Available Tools

#### 1. workspace_create
Create a new isolated workspace.

**Schema:**
```json
{
  "name": "workspace_create",
  "description": "Create a new workspace for isolated file operations",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Workspace name"
      },
      "description": {
        "type": "string",
        "description": "Optional workspace description"
      },
      "base_path": {
        "type": "string",
        "description": "Base directory (optional)"
      }
    },
    "required": ["name"]
  }
}
```

**Example:**
```json
{
  "tool": "workspace_create",
  "arguments": {
    "name": "refactor-task",
    "description": "Workspace for code refactoring"
  }
}
```

#### 2. workspace_list
List all available workspaces.

**Schema:**
```json
{
  "name": "workspace_list",
  "description": "List all workspaces",
  "inputSchema": {
    "type": "object",
    "properties": {
      "base_path": {
        "type": "string",
        "description": "Filter by base directory"
      }
    }
  }
}
```

#### 3. workspace_delete
Delete a workspace.

**Schema:**
```json
{
  "name": "workspace_delete",
  "description": "Delete a workspace and all its data",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Workspace name"
      },
      "force": {
        "type": "boolean",
        "description": "Force delete even with uncommitted changes",
        "default": false
      }
    },
    "required": ["name"]
  }
}
```

#### 4. file_read
Read a file from a workspace.

**Schema:**
```json
{
  "name": "file_read",
  "description": "Read file contents from a workspace",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workspace": {
        "type": "string",
        "description": "Workspace name"
      },
      "path": {
        "type": "string",
        "description": "File path within workspace"
      },
      "offset": {
        "type": "integer",
        "description": "Start reading from offset",
        "default": 0
      },
      "limit": {
        "type": "integer",
        "description": "Maximum bytes to read",
        "default": 1048576
      }
    },
    "required": ["workspace", "path"]
  }
}
```

#### 5. file_write
Write content to a file in a workspace.

**Schema:**
```json
{
  "name": "file_write",
  "description": "Write content to a file in a workspace",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workspace": {
        "type": "string",
        "description": "Workspace name"
      },
      "path": {
        "type": "string",
        "description": "File path within workspace"
      },
      "content": {
        "type": "string",
        "description": "File content"
      },
      "append": {
        "type": "boolean",
        "description": "Append to existing file",
        "default": false
      }
    },
    "required": ["workspace", "path", "content"]
  }
}
```

#### 6. file_delete
Delete a file from a workspace.

**Schema:**
```json
{
  "name": "file_delete",
  "description": "Delete a file from a workspace",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workspace": {
        "type": "string",
        "description": "Workspace name"
      },
      "path": {
        "type": "string",
        "description": "File path within workspace"
      }
    },
    "required": ["workspace", "path"]
  }
}
```

#### 7. directory_list
List directory contents.

**Schema:**
```json
{
  "name": "directory_list",
  "description": "List files and directories in a workspace",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workspace": {
        "type": "string",
        "description": "Workspace name"
      },
      "path": {
        "type": "string",
        "description": "Directory path",
        "default": "/"
      },
      "recursive": {
        "type": "boolean",
        "description": "List recursively",
        "default": false
      }
    },
    "required": ["workspace"]
  }
}
```

#### 8. workspace_status
Get workspace status and changes.

**Schema:**
```json
{
  "name": "workspace_status",
  "description": "Get workspace status showing modified files",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workspace": {
        "type": "string",
        "description": "Workspace name"
      }
    },
    "required": ["workspace"]
  }
}
```

#### 9. workspace_commit
Commit workspace changes to base.

**Schema:**
```json
{
  "name": "workspace_commit",
  "description": "Commit workspace changes to the base filesystem",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workspace": {
        "type": "string",
        "description": "Workspace name"
      },
      "message": {
        "type": "string",
        "description": "Commit message"
      },
      "include": {
        "type": "array",
        "items": { "type": "string" },
        "description": "File patterns to include"
      },
      "exclude": {
        "type": "array",
        "items": { "type": "string" },
        "description": "File patterns to exclude"
      }
    },
    "required": ["workspace", "message"]
  }
}
```

## Setting Up MCP Server

### 1. Start MCP Server

```bash
# Start MCP server
agentfs mcp --port 8080

# With specific base directory
agentfs mcp --port 8080 --base /path/to/project

# With authentication
agentfs mcp --port 8080 --token my-secret-token
```

### 2. Configure MCP Client

#### Claude Desktop
```json
// claude_desktop_config.json
{
  "mcpServers": {
    "agentfs": {
      "command": "agentfs",
      "args": ["mcp", "--port", "8080", "--base", "/path/to/project"]
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
      "name": "agentfs",
      "command": "agentfs mcp --port 8080 --base /path/to/project"
    }
  ]
}
```

#### Generic MCP Client
```python
from mcp import ClientSession, StdioServerParameters

server_params = StdioServerParameters(
    command="agentfs",
    args=["mcp", "--port", "8080", "--base", "/path/to/project"]
)

async with ClientSession(server_params) as session:
    # List available tools
    tools = await session.list_tools()
    
    # Create workspace
    result = await session.call_tool("workspace_create", {
        "name": "ai-task-1",
        "description": "Workspace for AI agent"
    })
```

## AI Agent Workflow Example

```python
# AI agent using AgentFS MCP
async def refactor_code(session):
    # 1. Create isolated workspace
    await session.call_tool("workspace_create", {
        "name": "refactor-session",
        "description": "Code refactoring workspace"
    })
    
    # 2. Read existing code
    code = await session.call_tool("file_read", {
        "workspace": "refactor-session",
        "path": "/src/main.py"
    })
    
    # 3. Analyze and refactor (AI processing)
    refactored_code = ai_refactor(code.content)
    
    # 4. Write changes
    await session.call_tool("file_write", {
        "workspace": "refactor-session",
        "path": "/src/main.py",
        "content": refactored_code
    })
    
    # 5. Check status
    status = await session.call_tool("workspace_status", {
        "workspace": "refactor-session"
    })
    
    # 6. If confident, commit; else review
    if ai_confidence(refactored_code) > 0.9:
        await session.call_tool("workspace_commit", {
            "workspace": "refactor-session",
            "message": "AI refactoring: improved function structure"
        })
        
        # Clean up workspace
        await session.call_tool("workspace_delete", {
            "name": "refactor-session"
        })
    else:
        # Leave for human review
        return {
            "status": "needs_review",
            "workspace": "refactor-session",
            "changes": status.modified
        }
```

## Multi-Agent Coordination

```python
# Multiple agents working in parallel
async def multi_agent_workflow(session):
    # Agent A: Frontend changes
    await session.call_tool("workspace_create", {
        "name": "agent-frontend"
    })
    
    # Agent B: Backend changes
    await session.call_tool("workspace_create", {
        "name": "agent-backend"
    })
    
    # Agent C: Test updates
    await session.call_tool("workspace_create", {
        "name": "agent-tests"
    })
    
    # Each agent works independently
    # ... (agents do their work) ...
    
    # Check all statuses
    frontend_status = await session.call_tool("workspace_status", {
        "workspace": "agent-frontend"
    })
    backend_status = await session.call_tool("workspace_status", {
        "workspace": "agent-backend"
    })
    tests_status = await session.call_tool("workspace_status", {
        "workspace": "agent-tests"
    })
    
    # Commit successful work
    if frontend_status.success:
        await session.call_tool("workspace_commit", {
            "workspace": "agent-frontend",
            "message": "Frontend updates"
        })
    
    if backend_status.success:
        await session.call_tool("workspace_commit", {
            "workspace": "agent-backend",
            "message": "Backend updates"
        })
    
    if tests_status.success:
        await session.call_tool("workspace_commit", {
            "workspace": "agent-tests",
            "message": "Test updates"
        })
```

## Security Considerations

### Access Control
```bash
# Use tokens for authentication
agentfs mcp --port 8080 --token $(cat ~/.agentfs_mcp_token)

# Restrict to read-only mode
agentfs mcp --port 8080 --read-only

# Restrict base directory
agentfs mcp --port 8080 --base /allowed/path --no-escape
```

### Workspace Isolation
Each workspace is completely isolated:
- Files in one workspace don't affect others
- Workspace can only access files within its base
- Audit log tracks all operations

### Validation
All MCP operations validate:
- Workspace names (no traversal)
- File paths (within workspace)
- Content size limits
- Rate limiting

## Integration with Beads

```python
# Combine Beads (task tracking) with AgentFS (execution)
async def execute_bead_task(session, beads_client, gate_id, molecule_id):
    # Get task from Beads
    task = await beads_client.get_task(gate_id, molecule_id)
    
    # Create workspace for task
    workspace_name = f"task-{molecule_id}"
    await session.call_tool("workspace_create", {
        "name": workspace_name,
        "description": task.description
    })
    
    # Execute task in workspace
    for step in task.steps:
        if step.type == "file_edit":
            await session.call_tool("file_write", {
                "workspace": workspace_name,
                "path": step.path,
                "content": step.content
            })
        elif step.type == "command":
            # Execute command via AgentFS run
            pass
    
    # Commit changes
    await session.call_tool("workspace_commit", {
        "workspace": workspace_name,
        "message": f"Completed task: {task.title}"
    })
    
    # Update Beads
    await beads_client.complete_molecule(gate_id, molecule_id)
```

## Next Steps

- [Cloud Sync](./08-cloud-sync.md)
- [NFS Export](./09-nfs-export.md)
- [Security](./10-security.md)