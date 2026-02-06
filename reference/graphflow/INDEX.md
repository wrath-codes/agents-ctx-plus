# graphflow — Sub-Index

> DAG-based task execution engine with context and storage (10 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [🔄 High-Performance Workflow Framework for Rust](README.md#high-performance-workflow-framework-for-rust) · [What is GraphFlow?](README.md#what-is-graphflow) · [Documentation Structure](README.md#documentation-structure) · [Quick Example](README.md#quick-example) · [Key Features](README.md#key-features) · [Installation](README.md#installation) · [Repository Structure](README.md#repository-structure) · [Core Concepts](README.md#core-concepts) · +4 more|

### [getting-started](getting-started/)

|file|description|
|---|---|
|[installation.md](getting-started/installation.md)|Installation — setup|
| |↳ [Prerequisites](getting-started/installation.md#prerequisites) · [Installation Methods](getting-started/installation.md#installation-methods) · [Feature Flags](getting-started/installation.md#feature-flags) · [Verify Installation](getting-started/installation.md#verify-installation) · [Environment Setup](getting-started/installation.md#environment-setup) · [IDE Setup](getting-started/installation.md#ide-setup) · [Troubleshooting](getting-started/installation.md#troubleshooting) · [Next Steps](getting-started/installation.md#next-steps)|
|[quickstart.md](getting-started/quickstart.md)|Quickstart — first graph execution|
| |↳ [Simple Greeting Workflow](getting-started/quickstart.md#simple-greeting-workflow) · [Step-by-Step Execution](getting-started/quickstart.md#step-by-step-execution) · [Adding User Input](getting-started/quickstart.md#adding-user-input) · [Using Conditional Routing](getting-started/quickstart.md#using-conditional-routing) · [Next Steps](getting-started/quickstart.md#next-steps)|

### [concepts](concepts/)

|file|description|
|---|---|
|[architecture.md](concepts/architecture.md)|Architecture — DAG design, components|
| |↳ [System Architecture](concepts/architecture.md#system-architecture) · [Core Components](concepts/architecture.md#core-components) · [Data Flow](concepts/architecture.md#data-flow) · [Design Principles](concepts/architecture.md#design-principles) · [Execution Models](concepts/architecture.md#execution-models) · [Performance Characteristics](concepts/architecture.md#performance-characteristics) · [Extension Points](concepts/architecture.md#extension-points) · [Next Steps](concepts/architecture.md#next-steps)|
|[tasks.md](concepts/tasks.md)|Tasks — node definitions, inputs/outputs|
| |↳ [What is a Task?](concepts/tasks.md#what-is-a-task) · [Creating Tasks](concepts/tasks.md#creating-tasks) · [TaskResult](concepts/tasks.md#taskresult) · [NextAction](concepts/tasks.md#nextaction) · [Context Operations](concepts/tasks.md#context-operations) · [Task Patterns](concepts/tasks.md#task-patterns) · [Error Handling](concepts/tasks.md#error-handling) · [Best Practices](concepts/tasks.md#best-practices) · +2 more|
|[context.md](concepts/context.md)|Context — shared execution context|
| |↳ [What is Context?](concepts/context.md#what-is-context) · [Creating Context](concepts/context.md#creating-context) · [Storing Data](concepts/context.md#storing-data) · [Retrieving Data](concepts/context.md#retrieving-data) · [Data Types](concepts/context.md#data-types) · [Chat History](concepts/context.md#chat-history) · [Serialization](concepts/context.md#serialization) · [Best Practices](concepts/context.md#best-practices) · +2 more|
|[graph-execution.md](concepts/graph-execution.md)|Graph execution — topological sort, parallel execution|
| |↳ [Graph Structure](concepts/graph-execution.md#graph-structure) · [Execution Models](concepts/graph-execution.md#execution-models) · [Execution Flow](concepts/graph-execution.md#execution-flow) · [ExecutionResult](concepts/graph-execution.md#executionresult) · [Session Management](concepts/graph-execution.md#session-management) · [Task Timeout](concepts/graph-execution.md#task-timeout) · [Error Handling](concepts/graph-execution.md#error-handling) · [Best Practices](concepts/graph-execution.md#best-practices) · +1 more|
|[storage.md](concepts/storage.md)|Storage — persistence layer|
| |↳ [Storage Trait](concepts/storage.md#storage-trait) · [In-Memory Storage](concepts/storage.md#in-memory-storage) · [PostgreSQL Storage](concepts/storage.md#postgresql-storage) · [Custom Storage](concepts/storage.md#custom-storage) · [Storage Selection](concepts/storage.md#storage-selection) · [Best Practices](concepts/storage.md#best-practices) · [Next Steps](concepts/storage.md#next-steps)|

### [core](core/)

|file|description|
|---|---|
|[flow-runner.md](core/flow-runner.md)|FlowRunner — execution engine|
| |↳ [What is FlowRunner?](core/flow-runner.md#what-is-flowrunner) · [When to Use FlowRunner](core/flow-runner.md#when-to-use-flowrunner) · [Creating a FlowRunner](core/flow-runner.md#creating-a-flowrunner) · [Executing Workflows](core/flow-runner.md#executing-workflows) · [Web Service Pattern](core/flow-runner.md#web-service-pattern) · [Performance](core/flow-runner.md#performance) · [Error Handling](core/flow-runner.md#error-handling) · [Best Practices](core/flow-runner.md#best-practices) · +2 more|

### [api](api/)

|file|description|
|---|---|
|[reference.md](api/reference.md)|API — programmatic interface|
| |↳ [Core Types](api/reference.md#core-types) · [Graph API](api/reference.md#graph-api) · [Execution API](api/reference.md#execution-api) · [Context API](api/reference.md#context-api) · [Storage API](api/reference.md#storage-api) · [FanOut API](api/reference.md#fanout-api) · [Error Types](api/reference.md#error-types) · [TypeScript Definitions](api/reference.md#typescript-definitions) · +5 more|

---
*10 files · Related: [tokio](../tokio/INDEX.md)*
