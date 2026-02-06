# AWS Bedrock Integration

## Overview

AWS Bedrock provides access to foundation models from Amazon and leading AI companies through a unified API. Rig's Bedrock integration enables using these models in your applications.

## Setup

### Installation

```toml
[dependencies]
rig-core = "0.5"
rig-bedrock = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
aws-config = "1.0"
```

### AWS Configuration

```bash
# Configure AWS credentials
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export AWS_REGION="us-east-1"
```

Or use IAM roles when running on AWS infrastructure.

## Basic Usage

### Using Bedrock Models

```rust
use rig::{completion::Prompt, providers::bedrock};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create Bedrock client
    let client = bedrock::Client::new().await;
    
    // Create agent with Claude
    let agent = client
        .agent("anthropic.claude-3-sonnet-20240229-v1:0")
        .preamble("You are a helpful assistant.")
        .build();
    
    // Prompt the agent
    let response = agent
        .prompt("What is AWS Bedrock?")
        .await?;
    
    println!("{}", response);
    
    Ok(())
}
```

### Available Models

| Model | Model ID | Type |
|-------|----------|------|
| Claude 3 Opus | anthropic.claude-3-opus-20240229-v1:0 | Completion |
| Claude 3 Sonnet | anthropic.claude-3-sonnet-20240229-v1:0 | Completion |
| Claude 3 Haiku | anthropic.claude-3-haiku-20240307-v1:0 | Completion |
| Amazon Titan | amazon.titan-text-express-v1 | Completion |
| Llama 2 | meta.llama2-70b-chat-v1 | Completion |
| Cohere Command | cohere.command-text-v14 | Completion |
| Amazon Titan Embeddings | amazon.titan-embed-text-v1 | Embedding |
| Cohere Embed | cohere.embed-english-v3 | Embedding |

### Using Embeddings

```rust
use rig::embeddings::EmbeddingModel;

// Create embedding model
let embedding_model = client.embedding_model("amazon.titan-embed-text-v1");

// Generate embedding
let embedding = embedding_model.embed("Hello world").await?;

println!("Dimensions: {}", embedding.dimensions);
```

## Advanced Configuration

### Custom AWS Config

```rust
use aws_config::Region;

// Custom region and credentials
let config = aws_config::from_env()
    .region(Region::new("eu-west-1"))
    .load()
    .await;

let client = bedrock::Client::with_config(config).await;
```

### Cross-Region Inference

```rust
// Use inference profiles for cross-region
let agent = client
    .agent("us.anthropic.claude-3-sonnet-20240229-v1:0")
    .preamble("You are helpful.")
    .build();
```

## IAM Permissions

Required IAM permissions:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "bedrock:InvokeModel",
                "bedrock:InvokeModelWithResponseStream"
            ],
            "Resource": "arn:aws:bedrock:*::foundation-model/*"
        }
    ]
}
```

## Use Cases

AWS Bedrock is ideal for:
- **Enterprise deployments** requiring AWS infrastructure
- **Claude models** without direct Anthropic API access
- **AWS ecosystem integration** (Lambda, ECS, etc.)
- **Compliance requirements** for data residency

## Next Steps

- **[FastEmbed](fastembed.md)** - Local embedding models
- **[Vector Stores](../vector-stores/index.md)** - Store and search vectors
- **[Production Deployment](../deployment/production.md)** - AWS deployment guides