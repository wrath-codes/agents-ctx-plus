# Testing Guide

## Testing Levels

### Unit Tests

Unit tests verify individual functions and components in isolation.

**Run unit tests:**

```bash
go test ./... -v -race
```

**Example unit test:**

```go
// internal/bridge/workflow_test.go
package bridge

import (
    "testing"
    "github.com/stretchr/testify/assert"
    "github.com/stretchr/testify/mock"
)

func TestWorkflow_Start(t *testing.T) {
    // Setup mocks
    mockBeads := new(MockBeadsClient)
    mockTempolite := new(MockTempoliteClient)

    bridge := NewCoordinationBridge(mockBeads, mockTempolite, nil, nil)

    // Setup expectations
    mockBeads.On("CreateIssue", mock.Anything, mock.Anything).Return(&beads.Issue{
        ID: "bd-test-001",
    }, nil)

    mockTempolite.On("StartWorkflow", mock.Anything, mock.Anything, mock.Anything).Return(nil)

    // Execute
    workflow, err := bridge.StartWorkflow(context.Background(), &StartWorkflowRequest{
        IssueTitle: "Test workflow",
        WorkflowType: "research",
    })

    // Assert
    assert.NoError(t, err)
    assert.NotNil(t, workflow)
    assert.Equal(t, "bd-test-001", workflow.BeadsIssueID)

    // Verify mocks
    mockBeads.AssertExpectations(t)
    mockTempolite.AssertExpectations(t)
}
```

### Integration Tests

Integration tests verify component interactions.

**Run integration tests:**

```bash
go test ./tests/integration/... -v -tags=integration
```

**Example integration test:**

```go
// tests/integration/workflow_test.go
package integration

import (
    "context"
    "testing"
    "github.com/stretchr/testify/require"
    "github.com/ory/dockertest/v3"
)

func TestWorkflow_Lifecycle(t *testing.T) {
    // Setup test database
    pool, err := dockertest.NewPool("")
    require.NoError(t, err)

    resource, err := pool.Run("alpine", "latest", []string{})
    require.NoError(t, err)
    defer pool.Purge(resource)

    // Create bridge with test database
    bridge := createTestBridge(t)

    // Test workflow creation
    workflow, err := bridge.StartWorkflow(context.Background(), &StartWorkflowRequest{
        IssueTitle: "Integration test workflow",
        WorkflowType: "research",
        AgentType: "research",
    })

    require.NoError(t, err)
    require.NotNil(t, workflow)

    // Test workflow retrieval
    retrieved, err := bridge.GetWorkflow(context.Background(), workflow.ID)
    require.NoError(t, err)
    require.Equal(t, workflow.ID, retrieved.ID)
}
```

### End-to-End Tests

E2E tests verify complete workflows from CLI to database.

**Run E2E tests:**

```bash
make e2e-tests
```

**Example E2E test:**

```go
// tests/e2e/workflow_test.go
package e2e

import (
    "os/exec"
    "strings"
    "testing"
    "github.com/stretchr/testify/assert"
)

func TestE2E_StartAndCancelWorkflow(t *testing.T) {
    // Start workflow
    cmd := exec.Command("workflow", "start", "research", "E2E test workflow")
    output, err := cmd.CombinedOutput()

    assert.NoError(t, err)
    assert.Contains(t, string(output), "Workflow started")

    // Extract workflow ID
    lines := strings.Split(string(output), "\n")
    workflowID := extractWorkflowID(lines)

    // Cancel workflow
    cmd = exec.Command("workflow", "cancel", workflowID, "--reason", "Test cleanup")
    output, err = cmd.CombinedOutput()

    assert.NoError(t, err)
    assert.Contains(t, string(output), "cancelled")
}
```

## Test Coverage

**View coverage report:**

```bash
go test ./... -coverprofile=coverage.out
go tool cover -html=coverage.out -o coverage.html
```

**Coverage targets:**

- Unit tests: >80%
- Integration tests: >60%
- Overall: >70%

## Load Testing

**Install k6:**

```bash
brew install k6
```

**Run load test:**

```bash
k6 run tests/load/workflow_load.js
```

**Load test script:**

```javascript
// tests/load/workflow_load.js
import http from "k6/http";
import { check } from "k6";

export const options = {
  stages: [
    { duration: "2m", target: 100 },
    { duration: "5m", target: 100 },
    { duration: "2m", target: 200 },
    { duration: "5m", target: 200 },
    { duration: "2m", target: 0 },
  ],
};

export default function () {
  const res = http.post(
    "http://localhost:8080/api/v1/workflows",
    JSON.stringify({
      issue_title: "Load test workflow",
      workflow_type: "research",
      agent_type: "research",
    }),
    {
      headers: {
        "Content-Type": "application/json",
        Authorization: "Bearer " + __ENV.API_TOKEN,
      },
    },
  );

  check(res, {
    "status is 201": (r) => r.status === 201,
    "response time < 500ms": (r) => r.timings.duration < 500,
  });
}
```

## Test Fixtures

**Creating test fixtures:**

```go
// tests/fixtures/workflows.go
package fixtures

import (
    "github.com/your-org/beads-workflow-system/pkg/types"
    "time"
)

func NewTestWorkflow() *types.Workflow {
    return &types.Workflow{
        ID: "wf-test-001",
        BeadsIssueID: "bd-test-001",
        Type: "research",
        Status: "active",
        Priority: 1,
        StartedAt: time.Now(),
        Variables: map[string]interface{}{
            "query": "test",
        },
    }
}

func NewTestAgent() *types.Agent {
    return &types.Agent{
        ID: "test-agent-01",
        Type: "research",
        Status: "active",
        MaxWorkload: 5,
        CurrentWorkload: 0,
    }
}
```

## Mocking

**Mock generation with mockery:**

```bash
mockery --name=BeadsClient --dir=pkg/beads --output=mocks
mockery --name=TempoliteClient --dir=pkg/tempolite --output=mocks
```

**Using mocks:**

```go
import (
    "github.com/stretchr/testify/mock"
    "mocks"
)

func TestWithMocks(t *testing.T) {
    mockBeads := new(mocks.BeadsClient)
    mockTempolite := new(mocks.TempoliteClient)

    mockBeads.On("CreateIssue", mock.Anything, mock.Anything).
        Return(&beads.Issue{ID: "test"}, nil)

    // Use mocks in test
}
```

