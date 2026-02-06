# CLI Commands Reference

The Cortex Memory CLI (`cortex-mem-cli`) provides a powerful command-line interface for interacting with the memory system directly.

---

## Installation

```bash
cargo install cortex-mem-cli
```

Verify installation:

```bash
cortex-mem-cli --version
```

---

## Global Options

```bash
cortex-mem-cli [OPTIONS] <COMMAND>

Options:
  -c, --config <PATH>    Path to configuration file [default: config.toml]
  -h, --help             Print help
  -V, --version          Print version
```

---

## Commands

### Add Memory

Add a new memory to the store.

```bash
cortex-mem-cli add [OPTIONS] --content <CONTENT>
```

**Options**:
- `--content <TEXT>` - Memory content (required)
- `--user-id <ID>` - Associate with user
- `--agent-id <ID>` - Associate with agent
- `--memory-type <TYPE>` - Type: conversational, personal, factual, procedural, semantic, episodic

**Examples**:

```bash
# Simple memory
cortex-mem-cli add --content "User prefers dark mode"

# With metadata
cortex-mem-cli add \
  --content "Alice is a software engineer" \
  --user-id "alice" \
  --memory-type "personal"

# Multi-line content
cortex-mem-cli add \
  --content "User has been learning Rust for 6 months and loves the ownership system" \
  --user-id "bob" \
  --agent-id "tutor-bot"
```

---

### Search Memories

Perform semantic search on memories.

```bash
cortex-mem-cli search [OPTIONS]
```

**Options**:
- `--query <TEXT>` - Search query (optional - if not provided, uses only metadata filters)
- `--user-id <ID>` - Filter by user
- `--agent-id <ID>` - Filter by agent
- `--topics <T1,T2>` - Filter by topics (comma-separated)
- `--keywords <K1,K2>` - Filter by keywords (comma-separated)
- `--limit <N>` - Maximum results [default: 10]

**Examples**:

```bash
# Basic search
cortex-mem-cli search --query "What does the user like?"

# With filters
cortex-mem-cli search \
  --query "programming languages" \
  --user-id "alice" \
  --limit 5

# Metadata-only search (no semantic query)
cortex-mem-cli search \
  --user-id "bob" \
  --topics "rust,programming" \
  --limit 20
```

**Output**:
```
+----+------------------------------------------+-------+
| ID | Content                                  | Score |
+----+------------------------------------------+-------+
| 1  | User has been learning Rust...          | 0.92  |
| 2  | User prefers systems programming...      | 0.85  |
+----+------------------------------------------+-------+
```

---

### List Memories

List memories with metadata filters (no semantic search).

```bash
cortex-mem-cli list [OPTIONS]
```

**Options**:
- `--user-id <ID>` - Filter by user
- `--agent-id <ID>` - Filter by agent
- `--memory-type <TYPE>` - Filter by memory type
- `--topics <T1,T2>` - Filter by topics
- `--keywords <K1,K2>` - Filter by keywords
- `--limit <N>` - Maximum results [default: 20]

**Examples**:

```bash
# List all memories for a user
cortex-mem-cli list --user-id "alice"

# Filter by type
cortex-mem-cli list \
  --user-id "bob" \
  --memory-type "personal" \
  --limit 50

# Filter by topics
cortex-mem-cli list \
  --user-id "alice" \
  --topics "preferences,settings"
```

**Output**:
```
+--------------------------------------+----------+-----------+---------------------+
| ID                                   | Type     | User      | Created             |
+--------------------------------------+----------+-----------+---------------------+
| 550e8400-e29b-41d4-a716-446655440000 | Personal | alice     | 2024-01-15 10:30:00 |
| 550e8400-e29b-41d4-a716-446655440001 | Factual  | alice     | 2024-01-15 10:35:00 |
+--------------------------------------+----------+-----------+---------------------+
```

---

### Delete Memory

Remove a memory by ID.

```bash
cortex-mem-cli delete <MEMORY_ID>
```

**Example**:

```bash
cortex-mem-cli delete 550e8400-e29b-41d4-a716-446655440000
```

**Output**:
```
✓ Memory deleted successfully
ID: 550e8400-e29b-41d4-a716-446655440000
```

---

### Optimize

Manage memory optimization.

#### Start Optimization

```bash
cortex-mem-cli optimize [OPTIONS]
```

**Options**:
- `--strategy <STRATEGY>` - Optimization strategy: full, incremental, deduplication, relevance, quality, space
- `--dry-run` - Preview without executing
- `--user-id <ID>` - Scope to specific user

**Examples**:

```bash
# Full optimization
cortex-mem-cli optimize start

# Deduplication only
cortex-mem-cli optimize start --strategy deduplication

# Preview mode
cortex-mem-cli optimize start --dry-run

# Optimize specific user
cortex-mem-cli optimize start --user-id "alice"
```

**Output**:
```
Optimization started
ID: opt-uuid-123
Strategy: Full
Status: Running
```

#### Check Optimization Status

```bash
cortex-mem-cli optimize-status --job-id <JOB_ID>
```

**Example**:

```bash
cortex-mem-cli optimize-status --job-id opt-uuid-123
```

**Output**:
```
Optimization Status
------------------
ID: opt-uuid-123
Status: Running
Progress: 45%
Current Phase: detecting_duplicates
Started: 2024-01-15 10:30:00
Estimated Completion: 2024-01-15 10:45:00
```

#### Manage Configuration

```bash
# View configuration
cortex-mem-cli optimize-config --get

# Update configuration
cortex-mem-cli optimize-config --set \
  --schedule "0 2 * * 0" \
  --enabled true
```

---

## Configuration File

The CLI uses the same `config.toml` file as the service:

```toml
[qdrant]
url = "http://localhost:6333"
collection_name = "cortex-memory"

[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-api-key"
model_efficient = "gpt-4o-mini"

[memory]
auto_enhance = true
deduplicate = true
```

Specify a custom config file:

```bash
cortex-mem-cli --config /path/to/config.toml add --content "Test memory"
```

---

## Environment Variables

Override configuration with environment variables:

```bash
export QDRANT_URL="http://localhost:6333"
export LLM_API_KEY="sk-your-api-key"
export EMBEDDING_API_KEY="sk-your-api-key"
```

---

## Common Workflows

### Adding and Searching

```bash
# Add some memories
cortex-mem-cli add --content "Alice loves hiking" --user-id "alice"
cortex-mem-cli add --content "Alice has a dog named Max" --user-id "alice"
cortex-mem-cli add --content "Alice works as a software engineer" --user-id "alice"

# Search for memories
cortex-mem-cli search --query "What does Alice do?" --user-id "alice"
```

### Cleanup and Optimization

```bash
# Preview what would be optimized
cortex-mem-cli optimize start --dry-run

# Run optimization
cortex-mem-cli optimize start

# Check status
cortex-mem-cli optimize-status --job-id <job-id>
```

### Batch Operations

```bash
# List all memories for a user
cortex-mem-cli list --user-id "alice" --limit 100 > memories.txt

# Extract IDs and delete (using jq)
cortex-mem-cli list --user-id "alice" --limit 100 --format json | \
  jq -r '.[].id' | \
  while read id; do
    cortex-mem-cli delete "$id"
  done
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Configuration error |
| 4 | Connection error |
| 5 | Not found |

---

## Tips and Best Practices

### 1. Use Meaningful User IDs

```bash
# Good: Descriptive ID
cortex-mem-cli add --content "..." --user-id "alice.smith@example.com"

# Avoid: Generic IDs
cortex-mem-cli add --content "..." --user-id "user123"
```

### 2. Scope Operations

```bash
# Always filter by user when possible
cortex-mem-cli search --query "..." --user-id "alice"
```

### 3. Test Before Bulk Operations

```bash
# Preview optimization first
cortex-mem-cli optimize start --dry-run

# Then run for real
cortex-mem-cli optimize start
```

### 4. Use Appropriate Memory Types

```bash
cortex-mem-cli add \
  --content "User prefers dark mode" \
  --memory-type "personal"
```

---

## Troubleshooting

### Connection Errors

```
Error: Failed to connect to Qdrant

Solution:
- Check Qdrant is running: docker ps | grep qdrant
- Verify QDRANT_URL environment variable
- Check config.toml Qdrant settings
```

### Authentication Errors

```
Error: LLM API authentication failed

Solution:
- Verify LLM_API_KEY is set correctly
- Check API key has sufficient credits
- Ensure API endpoint URL is correct
```

### Memory Not Found

```
Error: Memory not found: <uuid>

Solution:
- Verify the memory ID is correct
- Check if memory belongs to different user
- List memories to find correct ID
```

---

## Next Steps

- [REST API](../service/overview.md) - HTTP API reference
- [Configuration](../config/file.md) - Configuration options
- [Memory Types](../concepts/memory-types.md) - Understanding memory categories
