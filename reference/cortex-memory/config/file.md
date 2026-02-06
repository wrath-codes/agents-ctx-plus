# Configuration Reference

Cortex Memory is configured through a TOML configuration file. This document covers all available configuration options.

---

## Configuration File Structure

```toml
# config.toml

# Server settings (cortex-mem-service only)
[server]
host = "0.0.0.0"
port = 8000
cors_origins = ["*"]

# Qdrant vector database
[qdrant]
url = "http://localhost:6333"
collection_name = "cortex-memory"
timeout_secs = 5

# LLM for text generation
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-api-key"
model_efficient = "gpt-4o-mini"
temperature = 0.7
max_tokens = 8192

# Embedding service
[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-api-key"
model_name = "text-embedding-3-small"
batch_size = 16
timeout_secs = 10

# Memory management
[memory]
max_memories = 10000
similarity_threshold = 0.65
max_search_results = 50
auto_summary_threshold = 32768
auto_enhance = true
deduplicate = true
merge_threshold = 0.75
search_similarity_threshold = 0.50

# Logging
[logging]
enabled = true
log_directory = "logs"
level = "info"
```

---

## Server Configuration

### `[server]` Section

Settings for the HTTP service (`cortex-mem-service`).

```toml
[server]
host = "0.0.0.0"           # IP address to bind
port = 8000                # Port number
cors_origins = ["*"]       # Allowed CORS origins
```

**host**: 
- `"0.0.0.0"` - Listen on all interfaces (production)
- `"127.0.0.1"` - Localhost only (development)

**port**: 
- Any valid port number
- Default: `8000`

**cors_origins**:
- `["*"]` - Allow all origins (development)
- `["https://example.com", "https://app.example.com"]` - Specific origins

---

## Qdrant Configuration

### `[qdrant]` Section

Vector database connection settings.

```toml
[qdrant]
url = "http://localhost:6333"
collection_name = "cortex-memory"
timeout_secs = 5
```

**url**: 
- Qdrant server URL
- Default: `"http://localhost:6333"`
- Examples:
  - `"http://localhost:6333"` - Local Qdrant
  - `"http://qdrant.example.com:6333"` - Remote Qdrant
  - `"https://qdrant.cloud:6333"` - Qdrant Cloud

**collection_name**:
- Name of the collection in Qdrant
- Default: `"cortex-memory"`
- Will be created automatically if doesn't exist

**timeout_secs**:
- Request timeout in seconds
- Default: `5`
- Increase for slow networks

### Environment Variables

```bash
export QDRANT_URL="http://localhost:6333"
export QDRANT_COLLECTION="cortex-memory"
```

---

## LLM Configuration

### `[llm]` Section

Settings for text generation and reasoning.

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-api-key"
model_efficient = "gpt-4o-mini"
temperature = 0.7
max_tokens = 8192
```

**api_base_url**:
- LLM provider API endpoint
- Default: `"https://api.openai.com/v1"`
- Compatible with OpenAI API format

**api_key**:
- API key for authentication
- Required for cloud providers
- Keep secure and don't commit to version control

**model_efficient**:
- Model name for text generation
- Default: `"gpt-4o-mini"`
- Examples:
  - `"gpt-4o-mini"` - Fast, cost-effective
  - `"gpt-4o"` - Higher quality, more expensive
  - `"claude-3-haiku-20240307"` - Anthropic (via compatible endpoint)

**temperature**:
- Sampling temperature (0.0 - 2.0)
- Default: `0.7`
- Lower = more deterministic, higher = more creative

**max_tokens**:
- Maximum tokens in response
- Default: `8192`
- Increase for long-form content

### Supported Providers

#### OpenAI

```toml
[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-..."
model_efficient = "gpt-4o-mini"
```

#### Azure OpenAI

```toml
[llm]
api_base_url = "https://your-resource.openai.azure.com/openai/deployments/your-deployment"
api_key = "..."
model_efficient = "gpt-4"
```

#### Local (Ollama, LM Studio)

```toml
[llm]
api_base_url = "http://localhost:11434/v1"
api_key = "ollama"  # Or any string
model_efficient = "llama3.1"
```

---

## Embedding Configuration

### `[embedding]` Section

Settings for vector embeddings generation.

```toml
[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-your-api-key"
model_name = "text-embedding-3-small"
batch_size = 16
timeout_secs = 10
```

**api_base_url**:
- Embedding API endpoint
- Can be same or different from LLM endpoint
- Default: `"https://api.openai.com/v1"`

**api_key**:
- API key for embedding service
- Can be same or different from LLM key

**model_name**:
- Embedding model name
- Default: `"text-embedding-3-small"`
- Options:
  - `"text-embedding-3-small"` - 1536 dimensions, cost-effective
  - `"text-embedding-3-large"` - 3072 dimensions, higher quality
  - `"text-embedding-ada-002"` - Legacy, 1536 dimensions

**batch_size**:
- Number of texts to embed per batch
- Default: `16`
- Increase for better throughput
- Decrease if hitting rate limits

**timeout_secs**:
- Embedding request timeout
- Default: `10`
- Increase for large batches

### Embedding Dimensions

| Model | Dimensions | Best For |
|-------|------------|----------|
| text-embedding-3-small | 1536 | Cost-effective, good quality |
| text-embedding-3-large | 3072 | Maximum quality |
| text-embedding-ada-002 | 1536 | Legacy compatibility |

**Note**: Cortex Memory auto-detects embedding dimensions. The collection will be created with the correct size.

---

## Memory Management Configuration

### `[memory]` Section

Core memory management settings.

```toml
[memory]
max_memories = 10000
similarity_threshold = 0.65
max_search_results = 50
auto_summary_threshold = 32768
auto_enhance = true
deduplicate = true
merge_threshold = 0.75
search_similarity_threshold = 0.50
```

**max_memories**:
- Maximum memories to store per user (approximate)
- Default: `10000`
- Used for optimization decisions
- `0` = unlimited

**similarity_threshold**:
- Threshold for considering memories similar
- Default: `0.65`
- Range: `0.0` - `1.0`
- Higher = more strict matching

**max_search_results**:
- Default maximum search results
- Default: `50`
- Can be overridden per request

**auto_summary_threshold**:
- Content length threshold for auto-summarization (bytes)
- Default: `32768` (32KB)
- Memories longer than this get auto-summarized

**auto_enhance**:
- Enable automatic memory enhancement
- Default: `true`
- When enabled, memories are:
  - Classified by type
  - Extracted for entities/topics
  - Evaluated for importance
  - Checked for duplicates

**deduplicate**:
- Enable automatic deduplication
- Default: `true`
- Merges or skips duplicate memories

**merge_threshold**:
- Similarity threshold for merging duplicates
- Default: `0.75`
- Higher = more conservative merging

**search_similarity_threshold**:
- Minimum similarity for search results
- Default: `0.50`
- `None` = no threshold (return all)
- Higher = fewer but more relevant results

### Memory Type Defaults

When `auto_enhance = true`, these types are auto-detected:
- **Personal**: User preferences, characteristics
- **Factual**: Objective facts and knowledge
- **Procedural**: Step-by-step instructions
- **Conversational**: Dialogue history (default)
- **Semantic**: Concepts and abstractions
- **Episodic**: Specific events

---

## Logging Configuration

### `[logging]` Section

```toml
[logging]
enabled = true
log_directory = "logs"
level = "info"
```

**enabled**:
- Enable file logging
- Default: `true`
- Also logs to stdout

**log_directory**:
- Directory for log files
- Default: `"logs"`
- Created automatically if doesn't exist

**level**:
- Log level
- Default: `"info"`
- Options: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`

### Log Format

Logs include:
- Timestamp
- Log level
- Target module
- Message
- Context (when applicable)

Example:
```
2024-01-15T10:30:00.123Z INFO cortex_mem_core::memory Memory stored with ID: 550e8400-e29b-41d4-a716-446655440000
```

---

## Complete Example Configurations

### Development Setup

```toml
# Development - Local Qdrant, OpenAI
[server]
host = "127.0.0.1"
port = 8000
cors_origins = ["*"]

[qdrant]
url = "http://localhost:6333"
collection_name = "cortex-memory-dev"

[llm]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-dev-key"
model_efficient = "gpt-4o-mini"
temperature = 0.7

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "sk-dev-key"
model_name = "text-embedding-3-small"

[memory]
auto_enhance = true
deduplicate = true

[logging]
level = "debug"
```

### Production Setup

```toml
# Production - Remote Qdrant, Azure OpenAI
[server]
host = "0.0.0.0"
port = 8080
cors_origins = ["https://app.example.com"]

[qdrant]
url = "https://qdrant-prod.example.com:6333"
collection_name = "cortex-memory-prod"
timeout_secs = 10

[llm]
api_base_url = "https://your-resource.openai.azure.com/openai/deployments/gpt-4"
api_key = "${AZURE_OPENAI_KEY}"  # Use env var
model_efficient = "gpt-4"
temperature = 0.5
max_tokens = 4096

[embedding]
api_base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model_name = "text-embedding-3-large"
batch_size = 32

[memory]
max_memories = 50000
similarity_threshold = 0.70
auto_enhance = true
deduplicate = true
search_similarity_threshold = 0.60

[logging]
enabled = true
log_directory = "/var/log/cortex-memory"
level = "info"
```

### Local Development (No Cloud)

```toml
# Local - Ollama for both LLM and embeddings
[server]
host = "127.0.0.1"
port = 8000

[qdrant]
url = "http://localhost:6333"

[llm]
api_base_url = "http://localhost:11434/v1"
api_key = "ollama"
model_efficient = "llama3.1"
temperature = 0.7

[embedding]
api_base_url = "http://localhost:11434/v1"
api_key = "ollama"
model_name = "nomic-embed-text"
batch_size = 8

[memory]
auto_enhance = true
search_similarity_threshold = 0.55

[logging]
level = "debug"
```

---

## Environment Variable Overrides

All configuration values can be overridden via environment variables:

```bash
# Server
export CORTEX_SERVER_HOST="0.0.0.0"
export CORTEX_SERVER_PORT="8000"

# Qdrant
export QDRANT_URL="http://localhost:6333"
export QDRANT_COLLECTION="cortex-memory"

# LLM
export LLM_API_BASE_URL="https://api.openai.com/v1"
export LLM_API_KEY="sk-..."
export LLM_MODEL="gpt-4o-mini"
export LLM_TEMPERATURE="0.7"

# Embedding
export EMBEDDING_API_BASE_URL="https://api.openai.com/v1"
export EMBEDDING_API_KEY="sk-..."
export EMBEDDING_MODEL="text-embedding-3-small"

# Memory
export MEMORY_AUTO_ENHANCE="true"
export MEMORY_DEDUPLICATE="true"
```

**Priority**: Environment variables > Config file > Defaults

---

## Configuration Validation

### Check Configuration

```bash
# Start service with validation
cortex-mem-service --config config.toml

# Or use CLI
cortex-mem-cli --config config.toml add --content "test"
```

### Common Issues

#### Invalid TOML

```
Error: Failed to parse config file

Solution:
- Check TOML syntax
- Validate with: cargo install toml-cli && toml-check config.toml
```

#### Missing Required Fields

```
Error: Missing required configuration: llm.api_key

Solution:
- Add missing fields to config.toml
- Or set environment variable: export LLM_API_KEY="..."
```

#### Invalid Values

```
Error: Invalid value for memory.similarity_threshold: must be between 0.0 and 1.0

Solution:
- Check value ranges in documentation
- Use valid values
```

---

## Configuration for Multiple Environments

### Directory Structure

```
config/
├── development.toml
├── staging.toml
├── production.toml
└── local.toml
```

### Usage

```bash
# Development
cortex-mem-service --config config/development.toml

# Production
cortex-mem-service --config config/production.toml

# Using environment variable
export CORTEX_CONFIG="config/production.toml"
cortex-mem-service
```

---

## Next Steps

- [Getting Started](../getting-started/installation.md) - Installation guide
- [Memory Manager](../core/memory-manager.md) - Using the memory system
- [CLI Commands](../cli/commands.md) - Command-line interface
