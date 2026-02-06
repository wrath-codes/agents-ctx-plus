# rig — Sub-Index

> Rust LLM agent framework with RAG and vector stores (20 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [core](core/)

|file|description|
|---|---|
|[getting-started.md](core/getting-started.md)|Getting started — setup, first agent|
|[agents.md](core/agents.md)|Agents — Agent trait, completion, preamble|
|[tools.md](core/tools.md)|Tools — Tool trait, function calling|

### [providers](providers/)

|file|description|
|---|---|
|[openai.md](providers/openai.md)|OpenAI — GPT integration|

### [integrations](integrations/)

|file|description|
|---|---|
|[index.md](integrations/index.md)|Integrations — overview of all integrations|
|[fastembed.md](integrations/fastembed.md)|FastEmbed — embedding model integration|
|[bedrock.md](integrations/bedrock.md)|AWS Bedrock — Claude/Titan integration|

### [advanced](advanced/)

|file|description|
|---|---|
|[custom-providers.md](advanced/custom-providers.md)|Custom providers — implementing new LLM backends|

### [examples](examples/)

|file|description|
|---|---|
|[basic-agent.md](examples/basic-agent.md)|Basic agent — simple agent example|
|[rag-system.md](examples/rag-system.md)|RAG system — retrieval-augmented generation|

### [vector-stores](vector-stores/)

|file|description|
|---|---|
|[index.md](vector-stores/index.md)|Vector stores — overview, trait interface|
|[lancedb.md](vector-stores/lancedb.md)|LanceDB — embedded vector DB|
|[milvus.md](vector-stores/milvus.md)|Milvus — distributed vector DB|
|[mongodb.md](vector-stores/mongodb.md)|MongoDB — Atlas vector search|
|[neo4j.md](vector-stores/neo4j.md)|Neo4j — graph + vector|
|[qdrant.md](vector-stores/qdrant.md)|Qdrant — purpose-built vector DB|
|[sqlite.md](vector-stores/sqlite.md)|SQLite — lightweight vector store|
|[surrealdb.md](vector-stores/surrealdb.md)|SurrealDB — multi-model DB|

---
*20 files · Related: [fastembed](../fastembed/INDEX.md), [turso](../turso/INDEX.md)*
