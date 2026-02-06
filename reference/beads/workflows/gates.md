# Gates

Gates are async coordination primitives that block step progression until specific conditions are met. They enable sophisticated workflow orchestration including approvals, timers, and external event waiting.

## 🚪 What are Gates?

Gates block workflow step progression until a condition is satisfied:

- **Human gates**: Wait for human approval
- **Timer gates**: Wait for duration
- **GitHub gates**: Wait for external events (PR merge, CI, etc.)
- **Custom gates**: Wait for custom conditions

## 🎯 Gate Types

### Human Gate

Waits for human approval before proceeding.

```toml
[[steps]]
id = "deploy-approval"
title = "Approval for production deploy"
type = "human"
description = "Requires approval before deploying to production"

[steps.gate]
type = "human"
approvers = ["team-lead", "security-team"]
require_all = false  # Any approver can approve
```

**Usage**:
```bash
# Gate is created in "pending" state
bd pour production-deploy --var version="1.0.0"

# Gate blocks progression
bd ready  # Deploy step not shown

# Approver approves
bd gate approve bd-mol-001.4 --approver "team-lead"

# Gate now "open", can proceed
bd ready  # Deploy step now shown
```

### Timer Gate

Waits for a specified duration.

```toml
[[steps]]
id = "cooldown"
title = "Wait for cooldown period"
description = "Cool down period between deployments"

[steps.gate]
type = "timer"
duration = "24h"  # 30m, 2h, 24h, 7d
```

**Duration Formats**:
- `30m` - 30 minutes
- `2h` - 2 hours  
- `24h` - 24 hours
- `7d` - 7 days
- `1w` - 1 week

**Usage**:
```bash
# Gate starts timer when step is reached
bd update bd-mol-001.3 --status in_progress
# Timer starts automatically

# Check remaining time
bd gate status bd-mol-001.3

# Output:
Gate: Timer
Status: pending
Started: 2026-02-06 10:00
Expires: 2026-02-07 10:00
Remaining: 23h 45m

# After duration expires, gate opens automatically
```

### GitHub Gate

Waits for GitHub events.

```toml
# Wait for CI to pass
[[steps]]
id = "wait-ci"
title = "Wait for CI to pass"

[steps.gate]
type = "github"
event = "check_suite"
status = "success"
```

```toml
# Wait for PR merge
[[steps]]
id = "wait-merge"
title = "Wait for PR merge"

[steps.gate]
type = "github"
event = "pull_request"
action = "closed"
merged = true
```

**Supported Events**:
- `check_suite` - CI check completion
- `pull_request` - PR events (open, close, merge)
- `push` - Git push events
- `release` - Release creation
- `workflow_run` - GitHub Actions workflow

### Composite Gate

Combines multiple conditions.

```toml
[[steps]]
id = "complex-gate"
title = "Complex approval"

[steps.gate]
type = "composite"

[[steps.gate.conditions]]
type = "human"
approvers = ["manager"]

[[steps.gate.conditions]]
type = "timer"
duration = "2h"

[steps.gate.logic]
operator = "AND"  # All conditions must be met
```

## 🔄 Gate States

### State Machine

```
[pending] → [open] → [closed]
    ↑         │
   reset    skip (emergency)
```

### State Descriptions

| State | Description | Can Proceed? |
|-------|-------------|--------------|
| **pending** | Waiting for condition | No |
| **open** | Condition met, can proceed | Yes |
| **closed** | Step completed | N/A |

### State Transitions

```bash
# pending → open (normal flow)
bd gate approve bd-001     # Human gate
# Timer expires            # Timer gate
# GitHub event received    # GitHub gate

# pending → closed (skip)
bd gate skip bd-001 --reason "Emergency"  # Emergency bypass

# open → pending (reset)
bd gate reset bd-001       # Reset gate to pending
```

## 🎛️ Gate Operations

### Checking Gate Status

```bash
# Show gate status
bd show bd-mol-001.3

# Gate-specific status
bd gate status bd-mol-001.3

# JSON output
bd gate status bd-mol-001.3 --json

# Output:
{
  "gate": {
    "type": "human",
    "status": "pending",
    "created_at": "2026-02-06T10:00:00Z",
    "approvers": ["team-lead", "security-team"],
    "approvals_received": 0,
    "require_all": false
  }
}
```

### Manual Gate Override

```bash
# Approve human gate
bd gate approve bd-mol-001.3 --approver "team-lead"

# Reject human gate
bd gate reject bd-mol-001.3 --reason "Issues found"

# Skip gate (emergency)
bd gate skip bd-mol-001.3 --reason "Critical hotfix"

# Reset gate
bd gate reset bd-mol-001.3 --reason "Re-evaluation needed"
```

### Gate Notifications

```bash
# Configure notifications
bd gate notify bd-mol-001.3 --channel slack --target "#deployments"

# Notify on gate open
bd gate notify bd-mol-001.3 --on open --email "team@example.com"

# Escalation
bd gate escalate bd-mol-001.3 --after "4h" --to "manager"
```

## 📋 Gate Configuration

### Approval Rules

```toml
# Single approver
[steps.gate]
approvers = ["team-lead"]

# Multiple approvers (any can approve)
[steps.gate]
approvers = ["team-lead", "security-team", "manager"]
require_all = false

# Multiple approvers (all must approve)
[steps.gate]
approvers = ["security-team", "compliance-team"]
require_all = true

# Minimum approvals
[steps.gate]
approvers = ["team-lead", "senior-dev-1", "senior-dev-2"]
min_approvals = 2
```

### Timer Configuration

```toml
# Simple timer
[steps.gate]
type = "timer"
duration = "24h"

# Timer with warning
[steps.gate]
type = "timer"
duration = "24h"
warning_at = "2h"  # Notify when 2 hours remain

# Business hours timer
[steps.gate]
type = "timer"
duration = "24h"
business_hours_only = true  # Only count business hours
```

### GitHub Event Configuration

```toml
# CI check
[steps.gate]
type = "github"
event = "check_suite"
status = "success"
check_name = "tests"  # Optional: specific check

# PR merge
[steps.gate]
type = "github"
event = "pull_request"
action = "closed"
merged = true
base_branch = "main"

# Workflow completion
[steps.gate]
type = "github"
event = "workflow_run"
workflow_name = "deploy"
conclusion = "success"
```

## 🔄 waits-for Dependency

### Fan-In Pattern

The `waits-for` dependency creates fan-in patterns where a step waits for multiple predecessors:

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
waits_for = ["test-a", "test-b"]  # Waits for ALL
```

**Execution Flow**:
```
Step 1: Test A ─┐
                ├→ Step 3: Report (waits for both)
Step 2: Test B ─┘
```

### vs needs Dependency

```toml
# 'needs' - Sequential dependency
[[steps]]
id = "step2"
title = "Step 2"
needs = ["step1"]  # Just step1 must complete

# 'waits_for' - Fan-in dependency  
[[steps]]
id = "step3"
title = "Step 3"
waits_for = ["step1", "step2"]  # Both must complete
```

### Complex Fan-In

```toml
# Multiple parallel tracks converging
[[steps]]
id = "backend-tests"

[[steps]]
id = "frontend-tests"

[[steps]]
id = "integration-tests"

[[steps]]
id = "e2e-tests"

[[steps]]
id = "deploy"
waits_for = ["backend-tests", "frontend-tests", "integration-tests", "e2e-tests"]
```

## 🎯 Gate Examples

### Approval Flow

```toml
formula = "production-deploy"

[[steps]]
id = "build"
title = "Build production artifacts"

[[steps]]
id = "staging"
title = "Deploy to staging"
needs = ["build"]

[[steps]]
id = "qa-approval"
title = "QA sign-off"
needs = ["staging"]
type = "gate"

[steps.gate]
type = "human"
approvers = ["qa-team"]

[[steps]]
id = "production"
title = "Deploy to production"
needs = ["qa-approval"]
```

### Scheduled Release

```toml
formula = "scheduled-release"

[[steps]]
id = "prepare"
title = "Prepare release"

[[steps]]
id = "wait-window"
title = "Wait for release window"
needs = ["prepare"]
type = "gate"

[steps.gate]
type = "timer"
duration = "2h"

[[steps]]
id = "deploy"
title = "Deploy release"
needs = ["wait-window"]
```

### CI-Gated Deploy

```toml
formula = "ci-gated-deploy"

[[steps]]
id = "create-pr"
title = "Create pull request"

[[steps]]
id = "wait-ci"
title = "Wait for CI"
needs = ["create-pr"]
type = "gate"

[steps.gate]
type = "github"
event = "check_suite"
status = "success"

[[steps]]
id = "merge"
title = "Merge PR"
needs = ["wait-ci"]
type = "human"
```

### Multi-Level Approval

```toml
formula = "security-release"

[[steps]]
id = "security-scan"
title = "Security scan"

[[steps]]
id = "security-approval"
title = "Security team approval"
needs = ["security-scan"]
type = "gate"

[steps.gate]
type = "human"
approvers = ["security-team"]
require_all = true  # All security team must approve

[[steps]]
id = "manager-approval"
title = "Manager approval"
needs = ["security-approval"]
type = "gate"

[steps.gate]
type = "human"
approvers = ["engineering-manager"]

[[steps]]
id = "deploy"
title = "Deploy"
needs = ["manager-approval"]
```

## 🔔 Gate Notifications

### Notification Channels

```toml
[steps.gate.notify]
type = "slack"
channel = "#deployments"
message = "Production deployment awaiting approval"

[[steps.gate.notify.on]]
event = "pending"
type = "slack"

[[steps.gate.notify.on]]
event = "approved"
type = "email"
to = "team@example.com"
```

### Escalation

```toml
[steps.gate]
type = "human"
approvers = ["team-lead"]

[steps.gate.escalation]
after = "4h"
to = ["engineering-manager"]
message = "Gate has been pending for 4 hours"

[[steps.gate.escalation]]
after = "8h"
to = ["cto"]
message = "Critical: Gate pending for 8 hours"
```

## 🛡️ Gate Security

### Audit Trail

```bash
# View gate history
bd gate log bd-mol-001.3

# Output:
Gate History for bd-mol-001.3:
2026-02-06 10:00:00 - Created (pending)
2026-02-06 14:30:00 - Approved by team-lead
2026-02-06 14:30:00 - State changed to open
```

### Approval Authentication

```toml
[steps.gate]
type = "human"
approvers = ["team-lead"]
require_sso = true  # Require SSO authentication
require_mfa = true  # Require MFA
```

### Emergency Overrides

```bash
# Emergency skip (requires elevated permissions)
bd gate skip bd-mol-001.3 --reason "Critical hotfix" --force

# Requires confirmation
Are you sure? This will bypass normal approval process. [y/N]

# Audit log entry
2026-02-06 15:00:00 - EMERGENCY SKIP by admin
  Reason: Critical hotfix
  Elevated permissions used
```

## 📊 Gate Analytics

### Gate Metrics

```bash
# Gate statistics
bd gate stats

# Output:
Gate Statistics:
Total gates: 45
  Human gates: 30
  Timer gates: 10
  GitHub gates: 5

Average approval time: 4.2 hours
Average timer duration: 24 hours
Gates skipped (emergency): 2
```

### Bottleneck Analysis

```bash
# Find gate bottlenecks
bd gate bottlenecks

# Output:
Gate Bottlenecks:
1. production-approval: Avg 8.5h wait time
2. security-review: Avg 6.2h wait time
3. manager-sign-off: Avg 4.1h wait time
```

## 🎯 Best Practices

### Gate Design

**DO**:
```toml
# Use clear, descriptive gate titles
[[steps]]
title = "Production deployment approval"
description = "Approve deployment to production environment"

# Set appropriate approvers
approvers = ["team-lead"]  # Not too many, not too few

# Include timeout for human gates
max_wait = "48h"
```

**DON'T**:
```toml
# Don't create unnecessary gates
[[steps]]
title = "Step 1"
type = "gate"  # Every step doesn't need a gate

# Don't require too many approvers
approvers = ["team-lead", "manager", "director", "vp", "cto"]
# Too many = slow process

# Don't skip gates without reason
bd gate skip bd-001  # Always provide reason
```

### Emergency Procedures

```bash
# Document emergency override process
# 1. Attempt normal approval first
bd gate approve bd-001

# 2. If not possible, escalate
bd gate escalate bd-001

# 3. Only as last resort, skip with full documentation
bd gate skip bd-001 --reason "Production outage - critical fix needed" --force
```

## 🔗 Related Documentation

- [Chemistry Metaphor](chemistry-metaphor.md) - Gate phase overview
- [Formulas](formulas.md) - Creating gates in formulas
- [Molecules](molecules.md) - Using gates in workflows
- [Multi-Agent](../multi-agent/) - Multi-agent gate coordination

## 📚 See Also

- [CLI Reference](../cli-reference/workflow-commands.md) - Gate commands
- [Best Practices](../best-practices/) - Gate usage patterns
- [Context Enhancement](../context-enhancement/) - Gate automation