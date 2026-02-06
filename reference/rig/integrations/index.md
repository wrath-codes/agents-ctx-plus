# Integrations Overview

## Available Integrations

Rig provides companion crates for extended functionality:

### Cloud Providers

| Integration | Crate | Purpose |
|-------------|-------|---------|
| **AWS Bedrock** | rig-bedrock | Access models through AWS |
| **Google Vertex** | rig-vertexai | Google Cloud AI Platform |

### Local/Edge

| Integration | Crate | Purpose |
|-------------|-------|---------|
| **FastEmbed** | rig-fastembed | Local embedding models |
| **Ollama** | rig-core | Local LLM inference |

### Blockchain

| Integration | Crate | Purpose |
|-------------|-------|---------|
| **Rig Onchain Kit** | rig-onchain-kit | Solana/EVM integration |
| **Eternal AI** | rig-eternalai | Decentralized AI |

## Provider Integrations

All major LLM providers are supported:

### Completion & Chat
- OpenAI (GPT-4, GPT-3.5)
- Anthropic (Claude 3, Claude 2)
- Google (Gemini 1.5)
- Cohere (Command, Command-R)
- Ollama (Local models)
- Perplexity (Search-augmented)
- Hugging Face (Various)
- XAI (Grok)
- DeepSeek

### Embeddings
- OpenAI (text-embedding-3)
- Cohere (embed models)
- Hugging Face (Various)
- FastEmbed (Local)
- Ollama (Local)

## Choosing Integrations

### For Cloud Deployment
- AWS Bedrock for AWS infrastructure
- Google Vertex for GCP workloads
- Azure OpenAI for Microsoft Azure

### For Local Development
- Ollama for local LLMs
- FastEmbed for local embeddings
- SQLite for local vector storage

### For Blockchain
- Rig Onchain Kit for Web3 integration
- Eternal AI for decentralized compute

## Next Steps

- **[AWS Bedrock](bedrock.md)** - AWS model hosting
- **[FastEmbed](fastembed.md)** - Local embeddings
- **[Vector Stores](../vector-stores/index.md)** - Store vectors