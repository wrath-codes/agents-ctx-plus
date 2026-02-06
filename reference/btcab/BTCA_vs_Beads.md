# Better Context (BTCA) vs Beads

A comprehensive comparison of two complementary tools for AI agent enhancement.

## 🎯 Different Purposes

### Beads: Project Management

**Focus**: Track work, manage tasks, coordinate agents

**What it does**:
- Issue tracking and dependency management
- Multi-agent coordination
- Git-backed task persistence
- Workflow orchestration

**Best for**:
- Managing development projects
- Coordinating multiple agents on tasks
- Tracking work progress
- Maintaining project context

### BTCA: Knowledge Retrieval

**Focus**: Query libraries and frameworks for accurate context

**What it does**:
- Search actual source code
- Query documentation at source
- Get up-to-date library information
- Answer questions about technologies

**Best for**:
- Learning new libraries
- Understanding framework internals
- Getting accurate API information
- Researching technologies

## 📊 Feature Comparison

| Feature | Beads | BTCA |
|---------|-------|------|
| **Primary Use** | Task/Issue tracking | Knowledge retrieval |
| **Data Source** | Your project's issues/tasks | External codebases |
| **Storage** | Git (JSONL) | Local files or cloud |
| **Queries** | Task dependencies, status | Natural language |
| **Integration** | CLI + Hooks + MCP | CLI + MCP + API |
| **Scope** | Project-specific | Library/framework agnostic |
| **Persistence** | Git-backed | File/Cloud storage |
| **Multi-Agent** | Yes (coordination) | Yes (context sharing) |
| **Workflows** | Yes (formulas/molecules) | No (direct queries) |

## 🔄 Complementary Workflows

### Scenario 1: New Feature Development

**Using Beads**:
```bash
# Track the work
bd create "Add user authentication" -t epic
bd create "Design auth flow" --parent bd-epic-001
bd create "Implement JWT" --parent bd-epic-001

# Coordinate agents
bd pin bd-epic-001.1 --for agent-1
```

**Using BTCA**:
```bash
# Get library context
btca ask -r svelte -q "How do I create protected routes?"
btca ask -r lucia-auth -q "Show JWT implementation example"

# Understand APIs
btca ask -r nextjs -q "How does middleware work for auth?"
```

### Scenario 2: Debugging

**Using Beads**:
```bash
# Track bug
bd create "Login fails on mobile" -t bug -p 0

# Document progress
bd comment add bd-001 "Found issue in auth middleware"
```

**Using BTCA**:
```bash
# Understand the code
btca ask -r nextjs -q "How does auth middleware handle mobile?"

# Find similar issues
btca ask -r nextjs -q "Common mobile auth issues and solutions"
```

### Scenario 3: Multi-Agent Coordination

**Using Beads**:
```bash
# Assign work
bd pin bd-task-001 --for frontend-agent
bd pin bd-task-002 --for backend-agent

# Track dependencies
bd dep add bd-task-002 bd-task-001
```

**Using BTCA**:
```bash
# Share context between agents
btca ask -r svelte -q "Store implementation patterns"

# Backend agent queries
btca ask -r express -q "Middleware patterns for auth"
```

## 🎛️ Integration Opportunities

### Combined Context Enhancement

```python
# Context manager using both tools
class ContextManager:
    def __init__(self):
        self.beads = BeadsClient()
        self.btca = BTCAClient()
    
    def get_full_context(self):
        # Get project status from Beads
        ready_work = self.beads.get_ready_work()
        
        # Get technical context from BTCA
        if ready_work:
            task = ready_work[0]
            # Query relevant libraries
            tech_context = self.btca.ask(
                question=f"Context for: {task['title']}",
                resources=self.get_relevant_resources(task)
            )
        
        return {
            "tasks": ready_work,
            "tech_context": tech_context
        }
```

### Unified CLI

```bash
# Combined command
ctx init          # Initialize both Beads and BTCA
ctx status        # Show Beads tasks + BTCA resources
ctx work          # Get Beads ready work + BTCA context
ctx ask           # Beads task tracking + BTCA queries
```

### Smart Task-Resource Mapping

```python
# Automatically suggest BTCA resources based on Beads tasks
def suggest_resources(task):
    # Parse task for keywords
    keywords = extract_keywords(task['title'])
    
    # Map to BTCA resources
    resource_map = {
        'svelte': ['svelte', 'svelte-kit'],
        'react': ['react', 'nextjs'],
        'auth': ['lucia-auth', 'next-auth'],
        'database': ['prisma', 'drizzle']
    }
    
    suggested = []
    for keyword in keywords:
        if keyword in resource_map:
            suggested.extend(resource_map[keyword])
    
    return suggested
```

## 🏗️ Architecture Comparison

### Beads Architecture

```
Git Repo
  ↓
JSONL Files (issues)
  ↓
SQLite Database
  ↓
CLI/API
```

**Characteristics**:
- Three-layer persistence
- Git-native
- Conflict-resistant (hash IDs)
- Append-only log

### BTCA Architecture

```
CLI/TUI
  ↓
Local Server (optional)
  ↓
Resources (git/local)
  ↓
AI Provider
```

**Characteristics**:
- Direct AI integration
- Resource caching
- Multiple providers
- Cloud or local

## 💡 When to Use Which

### Use Beads When:

✅ Managing a project with multiple tasks
✅ Coordinating multiple agents
✅ Tracking work progress
✅ Need dependency management
✅ Want Git-backed task history
✅ Building complex workflows

**Example**:
```bash
# Starting a new project
bd init
bd create "Build e-commerce site" -t epic
bd create "Setup database" --parent bd-epic-001
bd create "Build API" --parent bd-epic-001
```

### Use BTCA When:

✅ Learning a new library
✅ Need accurate API documentation
✅ Want to understand framework internals
✅ Researching best practices
✅ Debugging unfamiliar code
✅ Need up-to-date library info

**Example**:
```bash
# Learning Svelte
btca add https://github.com/sveltejs/svelte -n svelte
btca ask -r svelte -q "How do runes work?"
```

### Use Both When:

✅ Managing complex projects with new technologies
✅ Coordinating agents on unfamiliar codebases
✅ Need both task tracking and knowledge retrieval

**Example**:
```bash
# Start project
bd init
btca init

# Track work
bd create "Implement auth with SvelteKit" -t task

# Get context
btca ask -r svelte-kit -q "Auth implementation patterns"

# Continue work
bd update bd-001 --status in_progress
```

## 🔧 Integration Strategies

### Strategy 1: Side-by-Side

Use both tools independently:

```bash
# Terminal 1: Beads for project management
bd ready
bd update bd-001 --status in_progress

# Terminal 2: BTCA for knowledge
btca ask -r svelte -q "How do I..."
```

### Strategy 2: Agent Orchestration

Agent uses both based on context:

```python
if task_requires_library_research(task):
    # Use BTCA
    context = btca.ask(question, resources)
else:
    # Use Beads
    context = beads.get_task_context(task_id)
```

### Strategy 3: Unified Context

Build a context manager that combines both:

```python
class UnifiedContext:
    def get_context(self):
        # Beads: What needs to be done
        tasks = self.beads.get_ready_work()
        
        # BTCA: How to do it
        if tasks:
            resources = self.map_tasks_to_resources(tasks)
            knowledge = self.btca.query(tasks[0], resources)
        
        return {
            "work": tasks,
            "knowledge": knowledge
        }
```

## 📈 Benefits of Using Both

### Complete Context

**Beads provides**:
- What to work on
- Task dependencies
- Work status
- Agent assignments

**BTCA provides**:
- How to implement
- Library documentation
- Code examples
- Best practices

**Together**:
- Complete project + technical context
- Coordinated agents with knowledge
- Efficient workflows
- Better outcomes

### Example: Full Context Window

```
[Beads Context - 1k tokens]
Ready work:
- bd-001: Implement auth system (P1)
  Depends on: bd-002 (DB schema)
  Parent: Epic-001

[BTCA Context - 1k tokens]  
Technical context:
- SvelteKit auth patterns from source
- Lucia auth implementation examples
- Database schema best practices

[Combined - 2k tokens]
Complete context for agent to work effectively
```

## 🎯 Recommendations

### For Individual Developers

**Start with BTCA** if:
- Learning new technologies
- Working on small projects
- Need quick answers about libraries

**Add Beads when**:
- Project grows
- Multiple features/tasks
- Need organization

### For Teams

**Use Beads for**:
- Task coordination
- Work assignment
- Progress tracking

**Use BTCA for**:
- Onboarding (learn codebase)
- Shared knowledge base
- Consistent implementation

### For AI Agents

**Use Beads when**:
- Managing long-running tasks
- Coordinating with other agents
- Tracking discovered work

**Use BTCA when**:
- Encountering unfamiliar libraries
- Need accurate API info
- Researching solutions

## 🚀 Future Integration Possibilities

### Automatic Resource Suggestion

Beads could suggest BTCA resources based on task labels:

```yaml
# In Beads formula
formula: "feature-workflow"
resources:
  svelte:
    - "https://github.com/sveltejs/svelte"
  auth:
    - "https://github.com/lucia-auth/lucia"
```

### Task-Aware Queries

BTCA could read Beads context to provide better answers:

```python
# BTCA knows current task from Beads
context = btca.ask(
    question="How to implement?",
    resources=[...],
    task_context=beads.get_current_task()
)
```

### Unified Dashboard

Web interface showing:
- Beads: Task status, ready work
- BTCA: Available resources, recent queries
- Combined: Suggested resources for ready tasks

## 🔗 Related Documentation

- [Beads Documentation](../beads/) - Complete Beads reference
- [BTCA Documentation](./) - Complete BTCA reference
- [Context Enhancement](../btcab/context-enhancement/) - Integration patterns

## 📚 Summary

**Beads and BTCA are complementary, not competing**:

- **Beads** = *Project management and coordination*
- **BTCA** = *Knowledge retrieval and learning*

**Use Beads** to track *what* needs to be done.
**Use BTCA** to learn *how* to do it.

**Together** they provide complete context for AI agents to work effectively on complex projects with unfamiliar technologies.