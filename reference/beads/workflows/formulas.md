# Formulas

Formulas are declarative workflow templates that define reusable, multi-step processes. They serve as the "Proto" (solid) phase in Beads' chemistry metaphor.

## 📝 Formula Structure

### Basic Formula

Formulas can be written in **TOML** (preferred) or **JSON**:

```toml
# feature-workflow.formula.toml
formula = "feature-workflow"
description = "Standard feature development workflow"
version = 1
type = "workflow"

[[steps]]
id = "design"
title = "Design {{feature_name}}"
type = "human"
description = "Create technical design document"

[[steps]]
id = "implement"
title = "Implement {{feature_name}}"
needs = ["design"]

[[steps]]
id = "test"
title = "Test {{feature_name}}"
needs = ["implement"]
```

### JSON Alternative

```json
{
  "formula": "feature-workflow",
  "description": "Standard feature development workflow",
  "version": 1,
  "type": "workflow",
  "steps": [
    {
      "id": "design",
      "title": "Design {{feature_name}}",
      "type": "human",
      "description": "Create technical design document"
    },
    {
      "id": "implement",
      "title": "Implement {{feature_name}}",
      "needs": ["design"]
    },
    {
      "id": "test",
      "title": "Test {{feature_name}}",
      "needs": ["implement"]
    }
  ]
}
```

## 🎯 Formula Types

### Workflow Type

Standard step-by-step workflows.

```toml
type = "workflow"

[[steps]]
id = "step1"
title = "First step"

[[steps]]
id = "step2"  
title = "Second step"
needs = ["step1"]
```

### Expansion Type

Template for generating multiple similar steps.

```toml
type = "expansion"

[[steps]]
id = "test-{{item}}"
title = "Test {{item}}"
items = ["unit", "integration", "e2e"]
```

**Expands to**:
- test-unit
- test-integration  
- test-e2e

### Aspect Type

Cross-cutting concerns applied to other formulas.

```toml
type = "aspect"

[[advice]]
target = "*.deploy"
[advice.before]
id = "security-scan"
title = "Security scan"
```

## 📋 Step Definition

### Step Fields

| Field | Type | Required | Description | Example |
|-------|------|----------|-------------|---------|
| `id` | string | ✓ | Unique step identifier | `"design"` |
| `title` | string | ✓ | Human-readable title | `"Design {{feature}}"` |
| `description` | string | ✗ | Detailed description | `"Create design doc"` |
| `type` | string | ✗ | Step type | `"task"`, `"human"`, `"gate"` |
| `needs` | array | ✗ | Dependencies | `["design"]` |
| `waits_for` | array | ✗ | Fan-in dependencies | `["test-a", "test-b"]` |
| `priority` | int | ✗ | Step priority | `1` |
| `labels` | array | ✗ | Step labels | `["backend"]` |

### Step Types

#### Task Step

Default type for standard work.

```toml
[[steps]]
id = "implement"
title = "Implement feature"
type = "task"  # or omit (default)
```

#### Human Step

Requires human action/approval.

```toml
[[steps]]
id = "review"
title = "Code review"
type = "human"
description = "Review implementation"
```

#### Gate Step

Async coordination point.

```toml
[[steps]]
id = "approval"
title = "Manager approval"
type = "gate"

[steps.gate]
type = "human"
approvers = ["manager"]
```

## 🔄 Step Dependencies

### Sequential Dependencies

```toml
[[steps]]
id = "design"
title = "Design"

[[steps]]
id = "implement"
title = "Implement"
needs = ["design"]

[[steps]]
id = "test"
title = "Test"
needs = ["implement"]

# Execution order:
# design → implement → test
```

### Parallel Dependencies

```toml
[[steps]]
id = "test-a"
title = "Test suite A"

[[steps]]
id = "test-b"
title = "Test suite B"

[[steps]]
id = "report"
title = "Generate report"
waits_for = ["test-a", "test-b"]

# Execution order:
# test-a ─┐
#         ├→ report
# test-b ─┘
```

### Complex Dependencies

```toml
[[steps]]
id = "foundation"
title = "Build foundation"

[[steps]]
id = "wall-a"
title = "Build wall A"
needs = ["foundation"]

[[steps]]
id = "wall-b"
title = "Build wall B"
needs = ["foundation"]

[[steps]]
id = "roof"
title = "Add roof"
needs = ["wall-a", "wall-b"]

# Execution order:
# foundation ─┬→ wall-a ─┐
#             └→ wall-b ─┴→ roof
```

## 📊 Variables

### Variable Definition

```toml
[vars.feature_name]
description = "Name of the feature"
required = true
pattern = "^[a-z0-9-]+$"  # Validation regex

[vars.priority]
description = "Feature priority"
default = 2
enum = [0, 1, 2, 3]  # Allowed values

[vars.environment]
description = "Target environment"
default = "staging"
enum = ["staging", "production"]
```

### Variable Usage

```toml
[[steps]]
title = "Deploy {{feature_name}} to {{environment}}"

[[steps]]
title = "Set priority to {{priority}}"
```

### Variable Constraints

```toml
[vars.version]
description = "Release version"
required = true
pattern = "^\\d+\\.\\d+\\.\\d+$"  # Semantic versioning

[vars.team]
description = "Responsible team"
default = "backend"
enum = ["backend", "frontend", "devops", "qa"]

[vars.complexity]
description = "Implementation complexity"
default = "medium"
enum = ["low", "medium", "high"]
```

## 🚪 Gates

### Human Gate

```toml
[[steps]]
id = "approval"
title = "Manager approval"
type = "gate"

[steps.gate]
type = "human"
approvers = ["manager", "team-lead"]
require_all = false  # Any approver can approve
```

### Timer Gate

```toml
[[steps]]
id = "cooldown"
title = "Wait for cooldown"
type = "gate"

[steps.gate]
type = "timer"
duration = "24h"  # 30m, 2h, 24h, 7d
```

### GitHub Gate

```toml
[[steps]]
id = "wait-ci"
title = "Wait for CI"
type = "gate"

[steps.gate]
type = "github"
event = "check_suite"
status = "success"
```

## 🔗 Bond Points

### Composition Points

```toml
[compose]
[[compose.bond_points]]
id = "entry"
step = "design"
position = "before"

[[compose.bond_points]]
id = "exit"
step = "deploy"
position = "after"
```

### Formula Composition

```bash
# Compose multiple formulas
bd pour feature-workflow --compose security-aspect

# Results in combined workflow with security checks
```

## 🎣 Hooks

### Step Completion Hooks

```toml
[[steps]]
id = "build"
title = "Build project"

[steps.on_complete]
run = "make build"

[steps.on_complete.notify]
type = "slack"
channel = "#builds"
```

### Step Start Hooks

```toml
[[steps]]
id = "deploy"
title = "Deploy to production"

[steps.on_start]
run = "scripts/pre-deploy.sh"
```

## 📁 Formula Locations

### Search Order

Beads searches for formulas in this order:

1. **Project-level**: `.beads/formulas/`
2. **User-level**: `~/.beads/formulas/`
3. **Built-in**: Bundled with Beads installation

### Project Formulas

```bash
# Create project-specific formula
mkdir -p .beads/formulas
cat > .beads/formulas/release.formula.toml << 'EOF'
formula = "release"
[[steps]]
id = "version"
title = "Bump version"
EOF

# Use immediately
bd pour release
```

### User Formulas

```bash
# Create user-level formula (available in all projects)
mkdir -p ~/.beads/formulas
cat > ~/.beads/formulas/standup.formula.toml << 'EOF'
formula = "standup"
[[steps]]
id = "update"
title = "Daily standup update"
EOF

# Use in any project
bd pour standup
```

### Built-in Formulas

```bash
# List built-in formulas
bd mol list --built-in

# Examples:
# - feature-workflow
# - bug-fix-workflow
# - release-workflow
# - hotfix-workflow
```

## 🛠️ Formula Commands

### Creating Formulas

```bash
# Interactive formula creation
bd mol create-formula my-workflow

# Create from template
bd mol create-formula my-workflow --template feature-workflow

# Edit existing formula
bd mol edit-formula my-workflow

# Validate formula syntax
bd mol validate-formula my-workflow
```

### Listing Formulas

```bash
# List all available formulas
bd mol list

# List with details
bd mol list --verbose

# List by location
bd mol list --project     # Project-level only
bd mol list --user        # User-level only
bd mol list --built-in    # Built-in only

# Search formulas
bd mol list --search "release"
```

### Using Formulas

```bash
# Pour formula (create molecule)
bd pour feature-workflow --var feature_name="dark-mode"

# Preview what would be created (dry run)
bd pour feature-workflow --var feature_name="test" --dry-run

# Pour with multiple variables
bd pour release-workflow \
  --var version="1.0.0" \
  --var environment="production"
```

## 📊 Advanced Formula Features

### Conditional Steps

```toml
[[steps]]
id = "security-scan"
title = "Security scan"
condition = "{{environment}} == 'production'"
```

### Loops

```toml
[[steps]]
id = "test-{{env}}"
title = "Test in {{env}}"
for = ["staging", "production"]
```

### Step Templates

```toml
[step_template.default]
priority = 2
type = "task"

[[steps]]
id = "step1"
title = "Step 1"
template = "default"
```

### Formula Inheritance

```toml
formula = "custom-feature"
extends = "feature-workflow"
version = 1

[[steps]]
id = "custom-step"
title = "Custom step"
needs = ["design"]
```

## 🎯 Formula Examples

### Release Formula

```toml
formula = "release"
description = "Standard release workflow"
version = 1

[vars.version]
required = true
pattern = "^\\d+\\.\\d+\\.\\d+$"

[[steps]]
id = "bump-version"
title = "Bump version to {{version}}"

[[steps]]
id = "changelog"
title = "Update CHANGELOG"
needs = ["bump-version"]

[[steps]]
id = "test"
title = "Run full test suite"
needs = ["changelog"]

[[steps]]
id = "build"
title = "Build release artifacts"
needs = ["test"]

[[steps]]
id = "tag"
title = "Create git tag v{{version}}"
needs = ["build"]

[[steps]]
id = "publish"
title = "Publish release"
needs = ["tag"]
type = "human"
```

### Bug Fix Formula

```toml
formula = "bug-fix"
description = "Bug investigation and fix workflow"
version = 1

[[steps]]
id = "reproduce"
title = "Reproduce the bug"
description = "Confirm bug exists and document reproduction steps"

[[steps]]
id = "investigate"
title = "Investigate root cause"
needs = ["reproduce"]

[[steps]]
id = "fix"
title = "Implement fix"
needs = ["investigate"]

[[steps]]
id = "test"
title = "Verify fix"
needs = ["fix"]

[[steps]]
id = "deploy"
title = "Deploy fix"
needs = ["test"]
type = "gate"

[steps.gate]
type = "human"
approvers = ["team-lead"]
```

### Multi-Environment Deployment

```toml
formula = "multi-deploy"
description = "Deploy to multiple environments"
version = 1

[vars.version]
required = true

[[steps]]
id = "deploy-staging"
title = "Deploy to staging"

[[steps]]
id = "test-staging"
title = "Test in staging"
needs = ["deploy-staging"]

[[steps]]
id = "deploy-prod"
title = "Deploy to production"
needs = ["test-staging"]
type = "gate"

[steps.gate]
type = "timer"
duration = "2h"  # Cooldown period
```

## 🔗 Related Documentation

- [Chemistry Metaphor](chemistry-metaphor.md) - Formula phase overview
- [Molecules](molecules.md) - Creating instances from formulas
- [Gates](gates.md) - Async coordination
- [Variables](variables.md) - Template variables
- [Aspects](aspects.md) - Cross-cutting concerns

## 📚 See Also

- [CLI Reference](../cli-reference/workflow-commands.md) - Formula commands
- [Multi-Agent](../multi-agent/) - Multi-agent formula usage
- [Context Enhancement](../context-enhancement/) - Formula automation