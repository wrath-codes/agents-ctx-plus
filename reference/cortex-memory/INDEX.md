# cortex-memory — Sub-Index

> LLM memory management service with fact extraction and vector store (14 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [🧠 The Production-Ready Memory System for Intelligent Agents](README.md#the-production-ready-memory-system-for-intelligent-agents) · [What is Cortex Memory?](README.md#what-is-cortex-memory) · [Documentation Structure](README.md#documentation-structure) · [The Cortex Memory Ecosystem](README.md#the-cortex-memory-ecosystem) · [Key Features](README.md#key-features) · [Benchmarks](README.md#benchmarks) · [Installation](README.md#installation) · [Quick Example](README.md#quick-example) · +4 more|

### [getting-started](getting-started/)

|file|description|
|---|---|
|[installation.md](getting-started/installation.md)|Installation — setup and dependencies|
| |↳ [Installation](getting-started/installation.md#installation) · [Installing Cortex Memory Components](getting-started/installation.md#installing-cortex-memory-components) · [Configuration](getting-started/installation.md#configuration) · [Verifying Installation](getting-started/installation.md#verifying-installation) · [Next Steps](getting-started/installation.md#next-steps)|
|[quickstart.md](getting-started/quickstart.md)|Quickstart — first memory operations|
| |↳ [Scenario: Personal AI Assistant](getting-started/quickstart.md#scenario-personal-ai-assistant) · [Advanced Example: Conversation Memory](getting-started/quickstart.md#advanced-example-conversation-memory) · [Using the REST API](getting-started/quickstart.md#using-the-rest-api) · [Using the CLI](getting-started/quickstart.md#using-the-cli) · [Next Steps](getting-started/quickstart.md#next-steps)|

### [concepts](concepts/)

|file|description|
|---|---|
|[architecture.md](concepts/architecture.md)|Architecture — system design and components|
| |↳ [System Architecture](concepts/architecture.md#system-architecture) · [Component Details](concepts/architecture.md#component-details) · [Data Flow](concepts/architecture.md#data-flow) · [Key Design Principles](concepts/architecture.md#key-design-principles) · [Performance Characteristics](concepts/architecture.md#performance-characteristics) · [Scalability Considerations](concepts/architecture.md#scalability-considerations) · [Security Architecture](concepts/architecture.md#security-architecture) · [Next Steps](concepts/architecture.md#next-steps)|
|[memory-types.md](concepts/memory-types.md)|Memory types — episodic, semantic, procedural|
| |↳ [Memory Type Details](concepts/memory-types.md#memory-type-details) · [Memory Type Selection Guidelines](concepts/memory-types.md#memory-type-selection-guidelines) · [Memory Type Filtering](concepts/memory-types.md#memory-type-filtering) · [Memory Type Statistics](concepts/memory-types.md#memory-type-statistics) · [Best Practices](concepts/memory-types.md#best-practices) · [Next Steps](concepts/memory-types.md#next-steps)|
|[memory-pipeline.md](concepts/memory-pipeline.md)|Pipeline — ingestion, extraction, storage flow|
| |↳ [Pipeline Overview](concepts/memory-pipeline.md#pipeline-overview) · [Stage 1: Input Processing](concepts/memory-pipeline.md#stage-1-input-processing) · [Stage 2: Fact Extraction](concepts/memory-pipeline.md#stage-2-fact-extraction) · [Stage 3: Memory Enhancement](concepts/memory-pipeline.md#stage-3-memory-enhancement) · [Stage 4: Storage](concepts/memory-pipeline.md#stage-4-storage) · [Stage 5: Retrieval Pipeline](concepts/memory-pipeline.md#stage-5-retrieval-pipeline) · [Stage 6: Memory Update Pipeline](concepts/memory-pipeline.md#stage-6-memory-update-pipeline) · [Pipeline Configuration](concepts/memory-pipeline.md#pipeline-configuration) · +4 more|
|[vector-store.md](concepts/vector-store.md)|Vector store — embedding storage and retrieval|
| |↳ [Architecture](concepts/vector-store.md#architecture) · [Qdrant Implementation](concepts/vector-store.md#qdrant-implementation) · [Memory Storage Format](concepts/vector-store.md#memory-storage-format) · [Search Operations](concepts/vector-store.md#search-operations) · [Filter Types](concepts/vector-store.md#filter-types) · [CRUD Operations](concepts/vector-store.md#crud-operations) · [Similarity Metrics](concepts/vector-store.md#similarity-metrics) · [Performance Optimization](concepts/vector-store.md#performance-optimization) · +6 more|
|[optimization.md](concepts/optimization.md)|Optimization — performance tuning|
| |↳ [Optimization Architecture](concepts/optimization.md#optimization-architecture) · [Optimization Components](concepts/optimization.md#optimization-components) · [Issue Types](concepts/optimization.md#issue-types) · [Optimization Strategies](concepts/optimization.md#optimization-strategies) · [Configuration](concepts/optimization.md#configuration) · [Using the Optimization System](concepts/optimization.md#using-the-optimization-system) · [Optimization Actions](concepts/optimization.md#optimization-actions) · [Optimization Results](concepts/optimization.md#optimization-results) · +3 more|

### [core](core/)

|file|description|
|---|---|
|[fact-extraction.md](core/fact-extraction.md)|Fact extraction — LLM-based fact parsing|
| |↳ [Extraction Strategies](core/fact-extraction.md#extraction-strategies) · [Extraction Prompts](core/fact-extraction.md#extraction-prompts) · [Extraction Process](core/fact-extraction.md#extraction-process) · [Fact Categories](core/fact-extraction.md#fact-categories) · [Intelligent Filtering](core/fact-extraction.md#intelligent-filtering) · [Usage Examples](core/fact-extraction.md#usage-examples) · [Configuration](core/fact-extraction.md#configuration) · [Best Practices](core/fact-extraction.md#best-practices) · +2 more|
|[memory-manager.md](core/memory-manager.md)|Memory manager — CRUD operations, lifecycle|
| |↳ [Architecture](core/memory-manager.md#architecture) · [Creating a Memory Manager](core/memory-manager.md#creating-a-memory-manager) · [Core Operations](core/memory-manager.md#core-operations) · [Advanced Operations](core/memory-manager.md#advanced-operations) · [MemoryManager Structure](core/memory-manager.md#memorymanager-structure) · [Configuration Options](core/memory-manager.md#configuration-options) · [Error Handling](core/memory-manager.md#error-handling) · [Best Practices](core/memory-manager.md#best-practices) · +2 more|

### [config](config/)

|file|description|
|---|---|
|[file.md](config/file.md)|Configuration — config file format and options|
| |↳ [Configuration File Structure](config/file.md#configuration-file-structure) · [Server Configuration](config/file.md#server-configuration) · [Qdrant Configuration](config/file.md#qdrant-configuration) · [LLM Configuration](config/file.md#llm-configuration) · [Embedding Configuration](config/file.md#embedding-configuration) · [Memory Management Configuration](config/file.md#memory-management-configuration) · [Logging Configuration](config/file.md#logging-configuration) · [Complete Example Configurations](config/file.md#complete-example-configurations) · +4 more|

### [cli](cli/)

|file|description|
|---|---|
|[commands.md](cli/commands.md)|CLI — command reference|
| |↳ [Installation](cli/commands.md#installation) · [Global Options](cli/commands.md#global-options) · [Commands](cli/commands.md#commands) · [Configuration File](cli/commands.md#configuration-file) · [Environment Variables](cli/commands.md#environment-variables) · [Common Workflows](cli/commands.md#common-workflows) · [Exit Codes](cli/commands.md#exit-codes) · [Tips and Best Practices](cli/commands.md#tips-and-best-practices) · +2 more|

### [api](api/)

|file|description|
|---|---|
|[reference.md](api/reference.md)|API — HTTP/programmatic interface|
| |↳ [Rust Library API](api/reference.md#rust-library-api) · [REST API Endpoints](api/reference.md#rest-api-endpoints) · [MCP Tools](api/reference.md#mcp-tools) · [Error Codes](api/reference.md#error-codes) · [TypeScript Definitions](api/reference.md#typescript-definitions) · [Python Types](api/reference.md#python-types) · [CLI Exit Codes](api/reference.md#cli-exit-codes) · [Rate Limits](api/reference.md#rate-limits) · +2 more|

### [service](service/)

|file|description|
|---|---|
|[overview.md](service/overview.md)|Service — deployment and runtime|
| |↳ [Starting the Service](service/overview.md#starting-the-service) · [API Endpoints](service/overview.md#api-endpoints) · [Error Responses](service/overview.md#error-responses) · [Request/Response Models](service/overview.md#requestresponse-models) · [Client Examples](service/overview.md#client-examples) · [Best Practices](service/overview.md#best-practices) · [Next Steps](service/overview.md#next-steps)|

---
*14 files · Related: [fastembed](../fastembed/INDEX.md), [rig](../rig/INDEX.md)*
