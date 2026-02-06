# Context Enhancement Opportunities

Leveraging Beads to build context enhancement tools for AI agents, providing persistent structured memory and intelligent context management.

## 🎯 Overview

Beads provides an excellent foundation for context enhancement CLIs:

- **Persistent structured memory** across sessions
- **Dependency-aware task selection** - always know what to work on
- **Git-integrated knowledge base** - context travels with code
- **Multi-agent context sharing** - collaborate across agents
- **Workflow-driven context injection** - intelligent context loading
- **Rich metadata and labels** - categorize and filter context

## 🚀 Key Opportunities

### 1. Persistent Context Memory

**Problem**: AI agents lose context between sessions

**Beads Solution**:
```bash
# End of session: Save context
bd sync

# Next session: Restore context  
bd prime  # Loads ~1-2k tokens of context

# Agent always knows:
# - What was being worked on
# - Current state of all issues
# - Dependencies and blockers
# - Recent decisions and discoveries
```

**Implementation**:
```python
# Context manager for agents
class ContextManager:
    def save_context(self, session_data):
        # Sync beads data
        subprocess.run(["bd", "sync"])
        
    def load_context(self):
        # Get prime context
        result = subprocess.run(
            ["bd", "prime", "--json"],
            capture_output=True
        )
        return json.loads(result.stdout)
```

### 2. Intelligent Task Selection

**Problem**: Agents waste time deciding what to work on

**Beads Solution**:
```bash
# Get only unblocked work
bd ready --json

# Filtered for specific agent
bd ready --agent agent-1 --json

# Prioritized by urgency
bd ready --priority 0,1 --json
```

**Implementation**:
```python
# Smart task selector
def get_next_task(agent_id):
    result = subprocess.run(
        ["bd", "ready", "--agent", agent_id, "--json"],
        capture_output=True
    )
    tasks = json.loads(result.stdout)
    
    if tasks["issues"]:
        return tasks["issues"][0]  # Highest priority ready task
    return None
```

### 3. Context-Aware Code Understanding

**Problem**: Agents need to understand codebase context

**Beads Solution**:
```bash
# Find related issues for current file
grep -l "auth.go" .beads/issues.jsonl

# Get context for specific area
bd list --label backend --search "authentication"

# Track file-issue relationships
bd create "Refactor auth" -l "file:auth.go"
```

**Implementation**:
```python
# File context extractor
def get_file_context(filepath):
    # Find related issues
    result = subprocess.run(
        ["bd", "list", "--search", filepath, "--json"],
        capture_output=True
    )
    return json.loads(result.stdout)
```

### 4. Knowledge Extraction

**Problem**: Extract insights from development history

**Beads Solution**:
```bash
# Query patterns
bd label stats                    # Most common labels
bd dep bottlenecks               # Workflow bottlenecks
bd comment search "decision"     # Decision history

# Historical context
bd list --closed-after 2026-01-01 --type bug
```

**Implementation**:
```python
# Knowledge mining
class KnowledgeExtractor:
    def extract_patterns(self):
        # Common issues
        result = subprocess.run(
            ["bd", "label", "stats"],
            capture_output=True
        )
        return self._parse_stats(result.stdout)
    
    def find_similar_issues(self, title):
        result = subprocess.run(
            ["bd", "list", "--search", title, "--json"],
            capture_output=True
        )
        return json.loads(result.stdout)
```

### 5. Multi-Agent Context Sharing

**Problem**: Multiple agents need shared context

**Beads Solution**:
```bash
# Agent A documents context
bd comment add bd-001 "Key insight: Use Redis for caching"

# Agent B reads context
bd show bd-001 --full

# Cross-repo context
bd hydrate --from backend-repo
```

**Implementation**:
```python
# Shared context protocol
class SharedContext:
    def share_insight(self, issue_id, insight):
        subprocess.run([
            "bd", "comment", "add", issue_id,
            f"[AGENT-CONTEXT] {insight}"
        ])
    
    def get_shared_context(self, issue_id):
        result = subprocess.run(
            ["bd", "show", issue_id, "--json"],
            capture_output=True
        )
        return json.loads(result.stdout)
```

## 🔧 Implementation Patterns

### Context Injection

```python
# Inject context into agent prompt
def build_context_prompt():
    # Get current state
    ready_work = get_ready_work()
    recent_activity = get_recent_activity()
    blockers = get_blocked_issues()
    
    context = f"""
    Current Project State:
    
    Ready to work ({len(ready_work)} issues):
    {format_issues(ready_work)}
    
    Recent Activity:
    {format_activity(recent_activity)}
    
    Blocked Issues:
    {format_issues(blockers)}
    
    Use 'bd ready' to see available work.
    Use 'bd show <id>' for issue details.
    """
    
    return context
```

### Workflow Automation

```python
# Automate context workflows
class ContextWorkflow:
    def start_session(self):
        # Load context
        context = self.load_context()
        
        # Get ready work
        tasks = self.get_ready_tasks()
        
        # Select best task
        task = self.select_task(tasks)
        
        return {
            "context": context,
            "current_task": task
        }
    
    def end_session(self):
        # Sync changes
        subprocess.run(["bd", "sync"])
        
        # Document progress
        self.document_session_summary()
```

### Context Pruning

```python
# Manage context window size
def prune_context(full_context, max_tokens=2000):
    # Priority order for context
    priority = [
        "current_task",
        "ready_work_high_priority", 
        "recent_decisions",
        "blockers",
        "ready_work_low_priority",
        "archived_work"
    ]
    
    pruned = {}
    current_tokens = 0
    
    for key in priority:
        content = full_context.get(key)
        tokens = estimate_tokens(content)
        
        if current_tokens + tokens <= max_tokens:
            pruned[key] = content
            current_tokens += tokens
        else:
            # Summarize or skip
            summary = summarize(content, 
                              max_tokens - current_tokens)
            if summary:
                pruned[f"{key}_summary"] = summary
            break
    
    return pruned
```

## 📊 Context Metrics

### Measuring Context Quality

```bash
# Context coverage
bd stats --coverage

# Context freshness  
bd stats --recency

# Context relevance
bd ready --relevance-score
```

### Context Analytics

```python
# Track context effectiveness
class ContextAnalytics:
    def measure_task_completion(self):
        # How often does agent pick right task?
        result = subprocess.run(
            ["bd", "stats", "--task-accuracy"],
            capture_output=True
        )
        return self._parse_stats(result.stdout)
    
    def measure_context_retention(self):
        # How well is context preserved?
        result = subprocess.run(
            ["bd", "stats", "--context-retention"],
            capture_output=True
        )
        return self._parse_stats(result.stdout)
```

## 🎛️ Advanced Features

### Context-Aware Search

```python
# Semantic search across issues
def semantic_search(query):
    # Get all issues
    result = subprocess.run(
        ["bd", "list", "--json"],
        capture_output=True
    )
    issues = json.loads(result.stdout)
    
    # Embed and search
    query_embedding = embed(query)
    matches = []
    
    for issue in issues:
        issue_embedding = embed(issue["title"] + issue["description"])
        similarity = cosine_similarity(query_embedding, issue_embedding)
        if similarity > 0.8:
            matches.append((issue, similarity))
    
    return sorted(matches, key=lambda x: x[1], reverse=True)
```

### Predictive Context

```python
# Predict what context will be needed
def predict_context_needs(current_task):
    # Analyze task type
    task_type = classify_task(current_task)
    
    # Get historical patterns
    similar_tasks = get_similar_tasks(task_type)
    
    # Predict related issues
    predictions = []
    for task in similar_tasks:
        related = get_related_issues(task)
        predictions.extend(related)
    
    return predictions
```

## 🔗 Integration Guide

### With Existing Agents

```python
# Wrapper for existing agents
class BeadsContextWrapper:
    def __init__(self, agent):
        self.agent = agent
        self.context_manager = ContextManager()
    
    def run(self, task):
        # Load context
        context = self.context_manager.load_context()
        
        # Enhance prompt with context
        enhanced_prompt = f"""
        {context}
        
        Task: {task}
        """
        
        # Run agent
        result = self.agent.run(enhanced_prompt)
        
        # Save context
        self.context_manager.save_context(result)
        
        return result
```

### With Workflows

```python
# Context-aware workflow execution
class ContextWorkflow:
    def execute(self, molecule_id):
        # Get molecule context
        molecule = self.get_molecule(molecule_id)
        
        # Get ready steps
        ready_steps = self.get_ready_steps(molecule_id)
        
        # Execute with context
        for step in ready_steps:
            context = self.build_step_context(step)
            self.execute_step(step, context)
```

## 🎯 Best Practices

### Context Management

**DO**:
```python
# Sync regularly
subprocess.run(["bd", "sync"])

# Use JSON output for parsing
subprocess.run(["bd", "ready", "--json"])

# Document discoveries
subprocess.run([
    "bd", "comment", "add", issue_id,
    "Key finding: ..."
])

# Prune context to fit token limits
context = prune_context(full_context, max_tokens=2000)
```

**DON'T**:
```python
# Don't parse human-readable output
result = subprocess.run(["bd", "ready"])  # Without --json

# Don't forget to sync
# Lost work between sessions

# Don't include too much context
# Exceeds token limits, wastes resources
```

## 📚 Implementation Examples

### Complete Context CLI

```python
#!/usr/bin/env python3
"""Context enhancement CLI using Beads"""

import subprocess
import json
import sys

class BeadsContextCLI:
    def __init__(self):
        self.validate_installation()
    
    def validate_installation(self):
        """Ensure beads is installed"""
        try:
            subprocess.run(["bd", "version"], check=True)
        except:
            print("Error: beads not installed")
            sys.exit(1)
    
    def get_context(self):
        """Get current project context"""
        # Ready work
        ready = subprocess.run(
            ["bd", "ready", "--json"],
            capture_output=True, text=True
        )
        ready_work = json.loads(ready.stdout)
        
        # Recent activity
        recent = subprocess.run(
            ["bd", "list", "--updated-after", "2026-02-01", "--json"],
            capture_output=True, text=True
        )
        recent_activity = json.loads(recent.stdout)
        
        return {
            "ready_work": ready_work,
            "recent_activity": recent_activity
        }
    
    def format_context(self, context):
        """Format context for agent prompt"""
        output = []
        
        # Ready work
        output.append("## Ready Work")
        for issue in context["ready_work"].get("issues", [])[:5]:
            output.append(f"- {issue['id']}: {issue['title']} [P{issue['priority']}]")
        
        # Recent activity
        output.append("\n## Recent Activity")
        for issue in context["recent_activity"].get("issues", [])[:3]:
            output.append(f"- {issue['id']}: {issue['title']} ({issue['status']})")
        
        return "\n".join(output)
    
    def prime(self):
        """Get context for agent"""
        context = self.get_context()
        formatted = self.format_context(context)
        print(formatted)
    
    def sync(self):
        """Sync context to git"""
        subprocess.run(["bd", "sync"])
        print("Context synced")

if __name__ == "__main__":
    cli = BeadsContextCLI()
    
    if len(sys.argv) < 2:
        print("Usage: beads-context <prime|sync>")
        sys.exit(1)
    
    command = sys.argv[1]
    if command == "prime":
        cli.prime()
    elif command == "sync":
        cli.sync()
    else:
        print(f"Unknown command: {command}")
```

## 🔗 Related Documentation

- [Overview](../index.md) - Complete Beads reference
- [Architecture](../architecture/) - Technical implementation
- [Workflows](../workflows/) - Workflow patterns
- [Multi-Agent](../multi-agent/) - Multi-agent coordination
- [Integrations](../integrations/) - Integration methods

## 📚 See Also

- [Best Practices](../best-practices/ai-agents.md) - Agent-specific patterns
- [Data Extraction](data-extraction.md) - Mining issues for context
- [Workflow Automation](workflow-automation.md) - Automated context workflows
- [Knowledge Management](knowledge-management.md) - Long-term knowledge base