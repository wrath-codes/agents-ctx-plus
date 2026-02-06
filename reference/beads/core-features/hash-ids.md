# Hash-Based IDs

Beads uses a sophisticated hash-based ID system that prevents collisions when multiple agents or branches work concurrently, enabling zero-conflict multi-agent workflows.

## 🔑 ID System Overview

### ID Format

```
bd-[hash-prefix]

Examples:
bd-a1b2    # Short hash (4 chars)
bd-f14c8d9 # Long hash (7 chars)  
bd-epic-001 # Hierarchical ID
bd-epic-001.1 # Child issue
```

### Collision Prevention

The hash-based system ensures unique IDs even when:
- Multiple agents create issues simultaneously
- Multiple branches create issues without syncing
- Issues are created offline and later merged
- Large teams work in parallel

## 🎯 How Hash-Based IDs Work

### Generation Algorithm

```python
# Simplified ID generation
def generate_id(title, timestamp, random_component):
    # Combine multiple sources of entropy
    data = f"{title}:{timestamp}:{random_component}:{uuid4()}"
    
    # Hash the data
    hash_bytes = sha256(data.encode()).digest()
    
    # Encode as base32 for readability
    encoded = base32_encode(hash_bytes)
    
    # Take first 4 characters for short ID
    short_id = encoded[:4].lower()
    
    return f"bd-{short_id}"

# Example:
# Input: "Set up database", "2026-02-06T10:00:00Z", 12345
# Output: "bd-a1b2"
```

### Collision Probability

```
ID Space: 32^4 = 1,048,576 possible 4-char IDs

Collision Probability:
- 100 issues:  0.5% chance
- 1,000 issues: 40% chance  
- 10,000 issues: 99.9% chance

Mitigation:
- Use 6-char IDs for large repositories (32^6 = 1 billion)
- Automatic re-generation on collision
- Timestamp component reduces simultaneous collision
```

### Real-World Collision Handling

```go
// Beads collision resolution
type IDGenerator struct {
    attempts int
    maxAttempts int
}

func (g *IDGenerator) GenerateID(issue Issue) (string, error) {
    for i := 0; i < g.maxAttempts; i++ {
        id := g.generateHash(issue, i)
        
        // Check if ID exists
        exists, err := g.idExists(id)
        if err != nil {
            return "", err
        }
        
        if !exists {
            return id, nil  // Success!
        }
        
        // Collision - try again with different nonce
        g.attempts++
    }
    
    return "", fmt.Errorf("failed to generate unique ID after %d attempts", g.maxAttempts)
}
```

## 🌳 Hierarchical IDs

### Parent-Child ID Structure

When issues have parent-child relationships, IDs reflect the hierarchy:

```
Parent:  bd-epic-001                    # Epic
├── Child: bd-epic-001.1                # First child
├── Child: bd-epic-001.2                # Second child
└── Child: bd-epic-001.3                # Third child

Grandchild: bd-epic-001.1.1            # Child of first child
```

### Creating Hierarchical Issues

```bash
# Create parent epic
bd create "User authentication system" -t epic
# → ID: bd-epic-001

# Create child issues
bd create "Design auth flow" --parent bd-epic-001
# → ID: bd-epic-001.1

bd create "Implement login API" --parent bd-epic-001
# → ID: bd-epic-001.2

# Create grandchild
bd create "Add OAuth support" --parent bd-epic-001.2
# → ID: bd-epic-001.2.1
```

### Hierarchical ID Benefits

**Visual Organization**:
```
bd-epic-001.1      # Clearly shows hierarchy
bd-epic-001.1.1    # Multiple levels supported
```

**Query Efficiency**:
```bash
# Find all children of epic
bd list --parent bd-epic-001

# Find all descendants
bd dep tree bd-epic-001

# Find siblings
bd list --siblings-of bd-epic-001.1
```

**Workflow Logic**:
```bash
# Epic completion requires all children
bd close bd-epic-001  # Only allowed when all children closed

# Child inherits parent's metadata
bd create "Subtask" --parent bd-epic-001
# → Inherits labels, priority from parent
```

## 🔄 Multi-Agent Collision Prevention

### Simultaneous Creation Scenario

```
Agent A (Branch: feature-a)          Agent B (Branch: feature-b)
    │                                    │
    ├── Create issue                     ├── Create issue (simultaneous)
    │   Title: "Fix bug"                 │   Title: "Add feature"  
    │   Timestamp: 10:00:00.100         │   Timestamp: 10:00:00.150
    │   Random: 12345                    │   Random: 67890
    │                                    │
    ├── ID: bd-a1b2                      ├── ID: bd-c3d4
    │   (Different due to random)        │   (Different due to random)
    │                                    │
    ├── Append to JSONL                  ├── Append to JSONL
    │                                    │
    ├── Git commit                       ├── Git commit
         (No conflict - different lines)      (No conflict - different lines)
```

### Merge Scenario

```bash
# Both agents push their branches
git push origin feature-a  # Contains bd-a1b2
git push origin feature-b  # Contains bd-c3d4

# Merge to main - no conflicts!
git checkout main
git merge feature-a    # Adds line with bd-a1b2
git merge feature-b    # Adds line with bd-c3d4

# JSONL result:
{"id": "bd-a1b2", ...}  # From Agent A
{"id": "bd-c3d4", ...}  # From Agent B

# Both issues coexist without conflict
```

### Branch-Specific IDs

```bash
# Feature branch isolation
bd create "Feature A work"  # bd-001
bd sync
git checkout main
bd create "Main branch work"  # bd-002 (different ID even if simultaneous)

# IDs are unique across branches
# When branches merge, both IDs preserved
```

## 📊 ID Management

### ID Length Configuration

```yaml
# .beads/config.yaml
id:
  length: 6                    # Use 6-char IDs (default: 4)
  prefix: "project-"          # Custom prefix (default: "bd")
  collision_retries: 10       # Max retry attempts
```

### Custom ID Prefixes

```bash
# Default prefix
bd create "Issue"  # → bd-a1b2

# Custom prefix (if configured)
bd create "Issue"  # → project-a1b2
```

### ID Validation

```bash
# Validate ID format
bd check id bd-a1b2

# Check if ID exists
bd show bd-a1b2

# List used ID prefixes
bd id prefixes
```

## 🔍 ID Operations

### Finding Issues by ID

```bash
# Exact ID match
bd show bd-a1b2

# Partial ID match
bd list --id-prefix bd-a1

# ID pattern matching
bd list --id-pattern "bd-*epic*"
```

### ID Ranges

```bash
# List issues created in time range
bd list --created-after "2026-02-01" --created-before "2026-02-07"

# ID range (by creation order)
bd list --id-range "bd-0001,bd-0100"
```

### ID Statistics

```bash
# ID distribution
bd stats --ids

# Output:
ID Statistics:
Total issues: 1,247
ID space used: 0.12%
Collision attempts: 3
Average ID length: 4.0 chars
```

## 🎛️ Advanced ID Features

### Sequential Numbering (Optional)

While hash-based IDs are default, sequential numbering is available for specific use cases:

```bash
# Create with sequential number (if enabled)
bd create "Issue" --sequential
# → bd-1248

# Auto-numbering in formulas
bd pour release-workflow --sequential
# → release-001, release-002, etc.
```

### Scoped IDs

```bash
# Repository-scoped IDs (default)
bd create "Issue"  # bd-a1b2

# Formula-scoped IDs
bd pour feature-workflow --scoped
# → feature-001-a1b2
```

### ID Aliases

```bash
# Create alias for long ID
bd alias add login-issue bd-very-long-hash-123456789

# Use alias instead of ID
bd show login-issue  # Shows bd-very-long-hash-123456789
```

## 📈 ID Analytics

### Collision Metrics

```bash
# View collision statistics
bd stats --collisions

# Output:
Collision Statistics:
Total IDs generated: 12,458
Collisions encountered: 23
Collision rate: 0.18%
Average retries per collision: 1.2
Max retries for single ID: 3
```

### ID Space Utilization

```bash
# Check ID space usage
bd stats --id-space

# Output:
ID Space Analysis (4-char IDs):
Total possible IDs: 1,048,576
Used IDs: 12,458
Available IDs: 1,036,118
Utilization: 1.19%
Estimated time to exhaustion: 50+ years
```

## 🛡️ ID Integrity

### ID Uniqueness Guarantees

```bash
# Ensure uniqueness constraint
CREATE UNIQUE INDEX idx_issues_id ON issues(id);

# Database-level enforcement
# Attempt to insert duplicate ID will fail
```

### ID Repair

```bash
# Check for ID conflicts
bd doctor --ids

# Repair ID issues
bd doctor --fix-ids

# Regenerate corrupted IDs
bd id regenerate --invalid-only
```

## 🔄 ID Migration

### Changing ID Length

```bash
# Before: 4-char IDs
bd-001, bd-002, bd-a1b2

# Migrate to 6-char IDs
bd config set id.length 6
bd migrate ids

# After: 6-char IDs  
bd-000001, bd-000002, bd-a1b2cd
```

### Prefix Changes

```bash
# Change prefix from "bd" to "proj"
bd config set id.prefix "proj"
bd migrate ids

# Issues now have new prefix
# Old IDs remain valid for references
```

## 🎯 Best Practices

### ID Usage

**DO**:
```bash
# Use IDs in commands
bd update bd-a1b2 --status in_progress

# Reference IDs in comments
bd comment add bd-a1b2 "Related to bd-c3d4"

# Use IDs in dependencies
bd dep add bd-c3d4 bd-a1b2

# Copy-paste IDs to avoid typos
bd show bd-a1b2  # Copy ID from previous output
```

**DON'T**:
```bash
# Don't guess IDs
bd show bd-1234  # May not exist

# Don't rely on sequential IDs
bd show bd-100   # Hash IDs aren't sequential

# Don't hardcode IDs in scripts
# Use search or variables instead
```

### ID Communication

```bash
# In comments and descriptions
bd comment add bd-a1b2 "This depends on bd-c3d4 being completed first"

# In commit messages
git commit -m "Fix authentication (bd-a1b2)"

# In documentation
# See issue bd-a1b2 for implementation details
```

## 🔗 Related Documentation

- [Issue Management](issue-management.md) - Issue operations
- [Dependencies](dependencies.md) - Issue relationships
- [Multi-Agent](../multi-agent/) - Multi-agent coordination
- [CLI Reference](../cli-reference/) - ID-related commands

## 📚 See Also

- [Architecture](../architecture/) - ID system implementation
- [Workflows](../workflows/) - Hierarchical work patterns
- [Best Practices](../best-practices/) - ID usage guidelines