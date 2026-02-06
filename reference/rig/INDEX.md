# rig — Sub-Index

> Rust LLM agent framework with RAG and vector stores (17 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Why Rig?](README.md#why-rig) · [Quick Start](README.md#quick-start) · [Installation](README.md#installation) · [Documentation Map](README.md#documentation-map) · [Key Concepts](README.md#key-concepts) · [Model Providers](README.md#model-providers) · [Vector Stores](README.md#vector-stores) · [Next Steps](README.md#next-steps)|

### [core](core/)

|file|description|
|---|---|
|[getting-started.md](core/getting-started.md)|Getting started — setup, first agent|
| |↳ [Installation](core/getting-started.md#installation) · [Your First Agent](core/getting-started.md#your-first-agent) · [Agent Configuration](core/getting-started.md#agent-configuration) · [Working with Different Providers](core/getting-started.md#working-with-different-providers) · [Error Handling](core/getting-started.md#error-handling) · [Project Structure](core/getting-started.md#project-structure) · [Testing Your Agent](core/getting-started.md#testing-your-agent) · [Next Steps](core/getting-started.md#next-steps)|
|[agents.md](core/agents.md)|Agents — Agent trait, completion, preamble|
| |↳ [Creating Agents](core/agents.md#creating-agents) · [Agent Configuration](core/agents.md#agent-configuration) · [Prompting](core/agents.md#prompting) · [Streaming Responses](core/agents.md#streaming-responses) · [Multi-Turn Conversations](core/agents.md#multi-turn-conversations) · [Agent Types](core/agents.md#agent-types) · [Advanced Patterns](core/agents.md#advanced-patterns) · [Best Practices](core/agents.md#best-practices) · +1 more|
|[tools.md](core/tools.md)|Tools — Tool trait, function calling|
| |↳ [Basic Tool](core/tools.md#basic-tool) · [Tool with Description](core/tools.md#tool-with-description) · [Async Tools](core/tools.md#async-tools) · [Tool with State](core/tools.md#tool-with-state) · [Multiple Tools](core/tools.md#multiple-tools) · [Tool Execution](core/tools.md#tool-execution) · [Error Handling](core/tools.md#error-handling) · [Best Practices](core/tools.md#best-practices) · +1 more|

### [providers](providers/)

|file|description|
|---|---|
|[openai.md](providers/openai.md)|OpenAI — GPT integration|
| |↳ [OpenAI](providers/openai.md#openai) · [Anthropic](providers/openai.md#anthropic) · [Gemini (Google)](providers/openai.md#gemini-google) · [Ollama (Local Models)](providers/openai.md#ollama-local-models) · [Cohere](providers/openai.md#cohere) · [Perplexity](providers/openai.md#perplexity) · [Hugging Face](providers/openai.md#hugging-face) · [DeepSeek](providers/openai.md#deepseek) · +5 more|

### [integrations](integrations/)

|file|description|
|---|---|
|[index.md](integrations/index.md)|Integrations — overview of all integrations|
| |↳ [Available Integrations](integrations/index.md#available-integrations) · [Provider Integrations](integrations/index.md#provider-integrations) · [Choosing Integrations](integrations/index.md#choosing-integrations) · [Next Steps](integrations/index.md#next-steps)|
|[fastembed.md](integrations/fastembed.md)|FastEmbed — embedding model integration|
| |↳ [Setup](integrations/fastembed.md#setup) · [Basic Usage](integrations/fastembed.md#basic-usage) · [Available Models](integrations/fastembed.md#available-models) · [Usage with Vector Stores](integrations/fastembed.md#usage-with-vector-stores) · [Benefits](integrations/fastembed.md#benefits) · [Performance](integrations/fastembed.md#performance) · [Next Steps](integrations/fastembed.md#next-steps)|
|[bedrock.md](integrations/bedrock.md)|AWS Bedrock — Claude/Titan integration|
| |↳ [Setup](integrations/bedrock.md#setup) · [Basic Usage](integrations/bedrock.md#basic-usage) · [Advanced Configuration](integrations/bedrock.md#advanced-configuration) · [IAM Permissions](integrations/bedrock.md#iam-permissions) · [Use Cases](integrations/bedrock.md#use-cases) · [Next Steps](integrations/bedrock.md#next-steps)|

### [advanced](advanced/)

|file|description|
|---|---|
|[custom-providers.md](advanced/custom-providers.md)|Custom providers — implementing new LLM backends|
| |↳ [Implementing a Provider](advanced/custom-providers.md#implementing-a-provider) · [Testing](advanced/custom-providers.md#testing) · [Best Practices](advanced/custom-providers.md#best-practices) · [Next Steps](advanced/custom-providers.md#next-steps)|

### [examples](examples/)

|file|description|
|---|---|
|[basic-agent.md](examples/basic-agent.md)|Basic agent — simple agent example|
| |↳ [Code](examples/basic-agent.md#code) · [Running the Example](examples/basic-agent.md#running-the-example) · [Sample Interaction](examples/basic-agent.md#sample-interaction) · [Key Concepts](examples/basic-agent.md#key-concepts) · [Variations](examples/basic-agent.md#variations)|
|[rag-system.md](examples/rag-system.md)|RAG system — retrieval-augmented generation|
| |↳ [Architecture](examples/rag-system.md#architecture) · [Complete Implementation](examples/rag-system.md#complete-implementation) · [Running the Example](examples/rag-system.md#running-the-example) · [Expected Output](examples/rag-system.md#expected-output) · [Key Components](examples/rag-system.md#key-components) · [Advanced Features](examples/rag-system.md#advanced-features) · [Next Steps](examples/rag-system.md#next-steps)|

### [vector-stores](vector-stores/)

|file|description|
|---|---|
|[index.md](vector-stores/index.md)|Vector stores — overview, trait interface|
| |↳ [Available Vector Store Integrations](vector-stores/index.md#available-vector-store-integrations) · [Additional Integrations](vector-stores/index.md#additional-integrations) · [Choosing a Vector Store](vector-stores/index.md#choosing-a-vector-store) · [Quick Comparison](vector-stores/index.md#quick-comparison) · [Implementation Status](vector-stores/index.md#implementation-status) · [Next Steps](vector-stores/index.md#next-steps)|
|[lancedb.md](vector-stores/lancedb.md)|LanceDB — embedded vector DB|
| |↳ [Setup](vector-stores/lancedb.md#setup) · [Basic Usage](vector-stores/lancedb.md#basic-usage) · [Features](vector-stores/lancedb.md#features) · [Use Cases](vector-stores/lancedb.md#use-cases)|
|[milvus.md](vector-stores/milvus.md)|Milvus — distributed vector DB|
| |↳ [Setup](vector-stores/milvus.md#setup) · [Basic Usage](vector-stores/milvus.md#basic-usage) · [Advanced Features](vector-stores/milvus.md#advanced-features) · [Production Deployment](vector-stores/milvus.md#production-deployment) · [Complete Example](vector-stores/milvus.md#complete-example) · [Use Cases](vector-stores/milvus.md#use-cases) · [Next Steps](vector-stores/milvus.md#next-steps)|
|[mongodb.md](vector-stores/mongodb.md)|MongoDB — Atlas vector search|
| |↳ [Setup](vector-stores/mongodb.md#setup) · [Basic Usage](vector-stores/mongodb.md#basic-usage) · [RAG Implementation](vector-stores/mongodb.md#rag-implementation) · [Advanced Features](vector-stores/mongodb.md#advanced-features) · [Best Practices](vector-stores/mongodb.md#best-practices) · [Complete Example](vector-stores/mongodb.md#complete-example) · [Next Steps](vector-stores/mongodb.md#next-steps)|
|[neo4j.md](vector-stores/neo4j.md)|Neo4j — graph + vector|
| |↳ [Setup](vector-stores/neo4j.md#setup) · [Basic Usage](vector-stores/neo4j.md#basic-usage) · [Graph-Enhanced RAG](vector-stores/neo4j.md#graph-enhanced-rag) · [Advanced Features](vector-stores/neo4j.md#advanced-features) · [Best Practices](vector-stores/neo4j.md#best-practices) · [Complete Example: Knowledge Base](vector-stores/neo4j.md#complete-example-knowledge-base) · [Use Cases](vector-stores/neo4j.md#use-cases) · [Next Steps](vector-stores/neo4j.md#next-steps)|
|[qdrant.md](vector-stores/qdrant.md)|Qdrant — purpose-built vector DB|
| |↳ [Setup](vector-stores/qdrant.md#setup) · [Basic Usage](vector-stores/qdrant.md#basic-usage) · [Advanced Features](vector-stores/qdrant.md#advanced-features) · [Collection Management](vector-stores/qdrant.md#collection-management) · [Production Deployment](vector-stores/qdrant.md#production-deployment) · [Complete Example](vector-stores/qdrant.md#complete-example) · [Use Cases](vector-stores/qdrant.md#use-cases) · [Next Steps](vector-stores/qdrant.md#next-steps)|
|[sqlite.md](vector-stores/sqlite.md)|SQLite — lightweight vector store|
| |↳ [Setup](vector-stores/sqlite.md#setup) · [Basic Usage](vector-stores/sqlite.md#basic-usage) · [Features](vector-stores/sqlite.md#features) · [Best Practices](vector-stores/sqlite.md#best-practices) · [Use Cases](vector-stores/sqlite.md#use-cases) · [Next Steps](vector-stores/sqlite.md#next-steps)|
|[surrealdb.md](vector-stores/surrealdb.md)|SurrealDB — multi-model DB|
| |↳ [Setup](vector-stores/surrealdb.md#setup) · [Basic Usage](vector-stores/surrealdb.md#basic-usage) · [Advanced Features](vector-stores/surrealdb.md#advanced-features) · [Complete Example](vector-stores/surrealdb.md#complete-example) · [Use Cases](vector-stores/surrealdb.md#use-cases) · [Next Steps](vector-stores/surrealdb.md#next-steps)|

---
*17 files · Related: [fastembed](../fastembed/INDEX.md), [turso](../turso/INDEX.md)*
