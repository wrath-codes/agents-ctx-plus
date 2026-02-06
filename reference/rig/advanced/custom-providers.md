# Custom Providers

## Overview

Rig's trait-based architecture allows you to implement custom providers for any LLM service.

## Implementing a Provider

### 1. Define the Client

```rust
use rig::completion::{CompletionModel, CompletionRequest, CompletionResponse};
use rig::embeddings::{EmbeddingModel, Embedding, EmbeddingRequest};

pub struct CustomClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl CustomClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.custom-llm.com/v1".to_string(),
            http_client: reqwest::Client::new(),
        }
    }
    
    pub fn from_env() -> Self {
        let api_key = std::env::var("CUSTOM_API_KEY")
            .expect("CUSTOM_API_KEY not set");
        Self::new(api_key)
    }
}
```

### 2. Implement CompletionModel

```rust
#[async_trait]
impl CompletionModel for CustomClient {
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, CompletionError> {
        let response = self.http_client
            .post(format!("{}/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": request.model,
                "prompt": request.prompt,
                "temperature": request.temperature,
                "max_tokens": request.max_tokens,
            }))
            .send()
            .await
            .map_err(|e| CompletionError::RequestError(e.to_string()))?;
        
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CompletionError::ResponseError(e.to_string()))?;
        
        Ok(CompletionResponse {
            text: body["choices"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            usage: Some(Usage {
                prompt_tokens: body["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: body["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            }),
        })
    }
}
```

### 3. Implement EmbeddingModel

```rust
#[async_trait]
impl EmbeddingModel for CustomClient {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let response = self.http_client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": "text-embedding-model",
                "input": text,
            }))
            .send()
            .await
            .map_err(|e| EmbeddingError::RequestError(e.to_string()))?;
        
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EmbeddingError::ResponseError(e.to_string()))?;
        
        let vec: Vec<f32> = body["data"][0]["embedding"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();
        
        Ok(Embedding {
            vec,
            dimensions: vec.len(),
        })
    }
    
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
}
```

### 4. Create Agent Builder

```rust
impl CustomClient {
    pub fn agent(&self, model: &str) -> AgentBuilder<Self> {
        AgentBuilder::new(self, model)
    }
    
    pub fn completion_model(&self, model: &str) -> CustomCompletionModel {
        CustomCompletionModel {
            client: self.clone(),
            model: model.to_string(),
        }
    }
    
    pub fn embedding_model(&self, model: &str) -> CustomEmbeddingModel {
        CustomEmbeddingModel {
            client: self.clone(),
            model: model.to_string(),
        }
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_completion() {
        let client = CustomClient::from_env();
        let model = client.completion_model("custom-model");
        
        let response = model.complete("Hello").await.unwrap();
        assert!(!response.is_empty());
    }
    
    #[tokio::test]
    async fn test_embedding() {
        let client = CustomClient::from_env();
        let model = client.embedding_model("embedding-model");
        
        let embedding = model.embed("Hello").await.unwrap();
        assert_eq!(embedding.dimensions, 1536);
    }
}
```

## Best Practices

1. **Error Handling**: Map provider errors to Rig's error types
2. **Rate Limiting**: Implement retry logic
3. **Streaming**: Support streaming responses
4. **Validation**: Validate API responses
5. **Documentation**: Document all public APIs

## Next Steps

- **[Tools](custom-tools.md)** - Build custom tools
- **[Vector Stores](../vector-stores/mongodb.md)** - Add vector storage