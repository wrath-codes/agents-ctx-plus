# Getting Started with Cortex Memory

## Installation

### Prerequisites

Before installing Cortex Memory, ensure you have:

- **Rust** (version 1.70 or later) - [Install Rust](https://www.rust-lang.org/tools/install)
- **Qdrant** - Vector database for storing memories
- **OpenAI-compatible LLM API** - For text generation and embeddings

### Installing Qdrant

#### Using Docker (Recommended)

```bash
# Run Qdrant with Docker
docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant

# Or with persistent storage
docker run -p 6333:6333 -p 6334:6334 \
  -v $(pwd)/qdrant_storage:/qdrant/storage \
  qdrant/qdrant
```

#### Using Homebrew (macOS)

```bash
brew install qdrant
qdrant
```

#### Local Installation

```bash
# Download from https://github.com/qdrant/qdrant/releases
# Extract and run:
./qdrant
```

Verify Qdrant is running:

```bash
curl http://localhost:6333/health
```

---

## Installing Cortex Memory Components

### Option 1: Install Binaries

```bash
# Install the CLI
cargo install cortex-mem-cli

# Install the REST API Service
cargo install cortex-mem-service

# Install the MCP Server
cargo install cortex-mem-mcp
```

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/sopaco/cortex-mem.git
cd cortex-mem

# Build all components
cargo build --release

# Binaries will be in target/release/
```

### Option 3: Library Integration

Add to your `Cargo.toml`:

```toml
[dependencies]
cortex-mem-core = "1.0"
cortex-mem-config = "1.0"

# Optional: for Rig framework integration
cortex-mem-rig = "1.0"

# Optional: for MCP server
cortex-mem-mcp = "1.0"
```

---

## Configuration

Create a `config.toml` file in your project directory:

```toml
# ------------------------------------------------------------------------------
# HTTP Server Configuration (cortex-mem-service only)
# ------------------------------------------------------------------------------
[server]
host = "0.0.0.0"
port = 8000
cors_origins = ["*"]

# ------------------------------------------------------------------------------
# Qdrant Vector Database Configuration
# ------------------------------------------------------------------------------
[qdrant]
url = "http://localhost:6333"
collection_name = "cortex-memory"
timeout_secs = 5

# ------------------------------------------------------------------------------
# LLM Configuration for text generation and reasoning
# ------------------------------------------------------------------------------
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-api-key"
model_efficient = "gpt-4o-mini"
temperature = 0.7
max_tokens = 8192

# ------------------------------------------------------------------------------
# Embedding Service Configuration
# ------------------------------------------------------------------------------
[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-api-key"
model_name = "text-embedding-3-small"
batch_size = 16
timeout_secs = 10

# ------------------------------------------------------------------------------
# Memory Management Configuration
# ------------------------------------------------------------------------------
[memory]
max_memories = 10000
similarity_threshold = 0.65
max_search_results = 50
auto_summary_threshold = 32768
auto_enhance = true
deduplicate = true
merge_threshold = 0.75
search_similarity_threshold = 0.50

# ------------------------------------------------------------------------------
# Logging Configuration
# ------------------------------------------------------------------------------
[logging]
enabled = true
log_directory = "logs"
level = "info"
```

### Environment Variables

You can also configure via environment variables:

```bash
export QDRANT_URL="http://localhost:6333"
export LLM_API_KEY="sk-your-openai-api-key"
export EMBEDDING_API_KEY="sk-your-openai-api-key"
```

---

## Verifying Installation

### Test CLI

```bash
# Add a test memory
cortex-mem-cli add --content "Testing Cortex Memory installation" --user-id "test"

# Search for memories
cortex-mem-cli search --query "installation test" --user-id "test"

# List all memories
cortex-mem-cli list --user-id "test"
```

### Test Service

```bash
# Start the service
cortex-mem-service

# In another terminal, test the API
curl -X POST http://localhost:8000/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Testing the REST API",
    "metadata": {
      "user_id": "test-user"
    }
  }'

# Search via API
curl -X POST http://localhost:8000/memories/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "REST API test",
    "filters": {
      "user_id": "test-user"
    }
  }'
```

---

## Next Steps

- [Quick Start Guide](./quickstart.md) - Build your first memory-enabled application
- [Configuration Guide](./configuration.md) - Deep dive into all configuration options
- [Architecture Overview](../concepts/architecture.md) - Understand how Cortex Memory works
