# Fact Extraction

Fact extraction is the process of identifying and extracting meaningful information from conversations. Cortex Memory uses sophisticated LLM-based extraction to identify facts about users, assistants, and procedures.

---

## Overview

The fact extraction system:
- **Analyzes conversations** to identify important information
- **Categorizes facts** by type (personal, preference, factual, procedural)
- **Filters noise** to focus on meaningful content
- **Handles multiple languages** with automatic detection
- **Prevents duplication** through intelligent filtering

---

## Extraction Strategies

### Strategy Selection

```rust
enum ExtractionStrategy {
    DualChannel,      // Extract both user and assistant facts
    UserOnly,         // Extract user facts only
    AssistantOnly,    // Extract assistant facts only
    ProceduralMemory, // Extract step-by-step procedures
}
```

The system automatically selects the best strategy based on conversation context:

```rust
fn analyze_conversation_context(&self, messages: &[Message]) -> ExtractionStrategy {
    let mut has_user = false;
    let mut has_assistant = false;
    let is_procedural = self.detect_procedural_pattern(messages);
    
    if is_procedural {
        ExtractionStrategy::ProceduralMemory
    } else if has_user && has_assistant {
        ExtractionStrategy::DualChannel
    } else if has_user {
        ExtractionStrategy::UserOnly
    } else if has_assistant {
        ExtractionStrategy::AssistantOnly
    } else {
        ExtractionStrategy::UserOnly
    }
}
```

---

## Extraction Prompts

### User Memory Extraction

Focuses on extracting personal information about the user:

```rust
const USER_MEMORY_EXTRACTION_PROMPT: &str = r#"
You are a Personal Information Organizer, specialized in accurately storing facts, user memories, and preferences.

Types of Information to Remember:
1. Store Personal Preferences: Keep track of likes, dislikes, and specific preferences
2. Maintain Important Personal Details: Remember significant personal information like names, relationships, important dates
3. Track Plans and Intentions: Note upcoming events, trips, goals, and any plans
4. Remember Activity and Service Preferences: Recall preferences for dining, travel, hobbies
5. Monitor Health and Wellness Preferences: Keep a record of dietary restrictions, fitness routines
6. Store Professional Details: Remember job titles, work habits, career goals
7. Miscellaneous Information Management: Keep track of favorite books, movies, brands

CRITICAL RULES:
- [IMPORTANT]: GENERATE FACTS SOLELY BASED ON THE USER'S MESSAGES
- [IMPORTANT]: DO NOT INCLUDE INFORMATION FROM ASSISTANT OR SYSTEM MESSAGES
- You will be PENALIZED if you include information from assistant or system messages
- Create facts based on user messages only
- Make sure to return valid JSON only

Return format:
{
  "facts": ["fact 1", "fact 2", "fact 3"]
}
"#;
```

**Example Input**:
```
User: I just got a promotion at work! I'm now a senior developer.
User: I've been learning Rust for the past 6 months.
Assistant: Congratulations! That's a great achievement.
```

**Example Output**:
```json
{
  "facts": [
    "User got a promotion to senior developer",
    "User has been learning Rust for 6 months"
  ]
}
```

### Assistant Memory Extraction

Focuses on extracting information about the AI assistant:

```rust
const AGENT_MEMORY_EXTRACTION_PROMPT: &str = r#"
You are an Assistant Information Organizer, specialized in accurately storing facts about the AI assistant.

Types of Information to Remember:
1. Assistant's Preferences: Track likes, dislikes, and specific preferences
2. Assistant's Capabilities: Note specific skills, knowledge areas, or tasks
3. Assistant's Hypothetical Plans: Record any hypothetical activities or plans
4. Assistant's Personality Traits: Identify personality traits or characteristics
5. Assistant's Approach to Tasks: Remember how the assistant approaches tasks
6. Assistant's Knowledge Areas: Keep track of subjects the assistant knows
7. Miscellaneous Information: Record any other interesting details

CRITICAL RULES:
- [IMPORTANT]: GENERATE FACTS SOLELY BASED ON THE ASSISTANT'S MESSAGES
- [IMPORTANT]: DO NOT INCLUDE INFORMATION FROM USER OR SYSTEM MESSAGES
- Create facts based on assistant messages only
- Make sure to return valid JSON only

Return format:
{
  "facts": ["fact 1", "fact 2", "fact 3"]
}
"#;
```

### Procedural Memory Extraction

Extracts step-by-step procedures:

```rust
const PROCEDURAL_MEMORY_SYSTEM_PROMPT: &str = r#"
You are a Procedural Memory System specialized in creating comprehensive step-by-step action records.

For each interaction, create a detailed procedural memory that includes:
1. Action Description: What action was performed
2. Context: Why this action was taken
3. Steps: Detailed breakdown of the procedure
4. Results: Outcomes and observations
5. Dependencies: Prerequisites and requirements

Format as a clear, step-by-step guide that can be followed later.
"#;
```

---

## Extraction Process

### 1. Message Filtering

```rust
// Filter by role
let user_messages = filter_messages_by_role(messages, "user");
let assistant_messages = filter_messages_by_role(messages, "assistant");

// Filter by multiple roles
let relevant_messages = filter_messages_by_roles(messages, &["user", "assistant"]);
```

### 2. Language Detection

```rust
pub struct LanguageInfo {
    pub language: String,      // e.g., "en", "zh", "es"
    pub confidence: f32,       // 0.0 - 1.0
}

fn detect_language(text: &str) -> LanguageInfo {
    // Detect language based on character patterns
    // Return language code and confidence
}
```

### 3. Prompt Construction

```rust
fn build_user_memory_prompt(&self, messages: &[Message]) -> String {
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let conversation = parse_messages(messages);
    
    format!(
        r#"{}...
        
Today's date is {}.

Conversation:
{}

JSON Response:"#,
        USER_MEMORY_EXTRACTION_PROMPT,
        current_date,
        conversation
    )
}
```

### 4. LLM Processing

```rust
// Use rig's structured extractor
let extractor = llm_client
    .extractor_completions_api::<StructuredFactExtraction>(&model_name)
    .preamble(&prompt)
    .build();

let result = extractor.extract("").await?;
```

### 5. Result Parsing

```rust
fn parse_structured_facts(&self, structured: StructuredFactExtraction) -> Vec<ExtractedFact> {
    let mut facts = Vec::new();
    for fact_str in structured.facts {
        let language = detect_language(&fact_str);
        facts.push(ExtractedFact {
            content: fact_str,
            importance: 0.7,
            category: FactCategory::Personal,
            entities: vec![],
            language: Some(language),
            source_role: "unknown".to_string(),
        });
    }
    facts
}
```

---

## Fact Categories

### Category Types

```rust
pub enum FactCategory {
    Personal,    // Personal information about users
    Preference,  // User preferences and likes/dislikes
    Factual,     // General factual information
    Procedural,  // How-to information and procedures
    Contextual,  // Context about ongoing conversations
}
```

### Category Examples

| Category | Examples |
|----------|----------|
| **Personal** | "User's name is Alice", "User has a dog named Max" |
| **Preference** | "User prefers dark mode", "User likes Italian food" |
| **Factual** | "Rust is a systems programming language" |
| **Procedural** | "Steps to reset password: 1. Go to login page..." |
| **Contextual** | "User is currently working on Project X" |

---

## Intelligent Filtering

### Duplicate Detection

```rust
async fn intelligent_fact_filtering(
    &self,
    facts: Vec<ExtractedFact>,
) -> Result<Vec<ExtractedFact>> {
    let mut filtered_facts: Vec<ExtractedFact> = Vec::new();
    let mut seen_contents: HashSet<String> = HashSet::new();
    
    for fact in &facts {
        // Normalize for comparison
        let content_normalized = fact.content.to_lowercase().trim().to_string();
        
        // Skip exact duplicates
        if seen_contents.contains(&content_normalized) {
            continue;
        }
        
        // Check semantic similarity
        let mut is_duplicate = false;
        for existing in &filtered_facts {
            if self.are_facts_semantically_similar(&fact.content, &existing.content) {
                is_duplicate = true;
                break;
            }
        }
        
        if !is_duplicate && fact.importance >= 0.5 {
            seen_contents.insert(content_normalized);
            filtered_facts.push(fact.clone());
        }
    }
    
    // Sort by importance and category priority
    filtered_facts.sort_by(|a, b| {
        let category_order = |cat: &FactCategory| match cat {
            FactCategory::Personal => 4,
            FactCategory::Preference => 3,
            FactCategory::Factual => 2,
            FactCategory::Procedural => 1,
            FactCategory::Contextual => 0,
        };
        
        let category_cmp = category_order(&a.category).cmp(&category_order(&b.category));
        if category_cmp != std::cmp::Ordering::Equal {
            return category_cmp.reverse();
        }
        
        b.importance.partial_cmp(&a.importance).unwrap()
    });
    
    Ok(filtered_facts)
}
```

### Semantic Similarity Check

```rust
fn are_facts_semantically_similar(&self, fact1: &str, fact2: &str) -> bool {
    let fact1_lower = fact1.to_lowercase();
    let fact2_lower = fact2.to_lowercase();
    
    // Exact match
    if fact1_lower.trim() == fact2_lower.trim() {
        return true;
    }
    
    // Word overlap (Jaccard similarity)
    let words1: HashSet<&str> = fact1_lower.split_whitespace().collect();
    let words2: HashSet<&str> = fact2_lower.split_whitespace().collect();
    
    let intersection: HashSet<_> = words1.intersection(&words2).collect();
    let union_size = words1.len().max(words2.len());
    let jaccard = intersection.len() as f64 / union_size as f64;
    
    if jaccard > 0.7 {
        return true;
    }
    
    // Technical term overlap
    let technical_terms = ["rust", "tokio", "async", "wasm", ...];
    let shared_terms: Vec<_> = technical_terms
        .iter()
        .filter(|term| {
            fact1_lower.contains(*term) && fact2_lower.contains(*term)
        })
        .collect();
    
    shared_terms.len() >= 2
}
```

---

## Usage Examples

### Extract from Single Text

```rust
use cortex_mem_core::memory::FactExtractor;

let extractor = create_fact_extractor(llm_client);

let facts = extractor.extract_facts_from_text(
    "I've been living in Tokyo for 5 years and I love the food scene here."
).await?;

for fact in facts {
    println!("Extracted: {}", fact.content);
    println!("Category: {:?}", fact.category);
    println!("Importance: {}", fact.importance);
}
```

### Extract from Conversation

```rust
let messages = vec![
    Message::user("I just started learning guitar"),
    Message::assistant("That's wonderful! What style are you interested in?"),
    Message::user("I want to learn classical guitar. I practice 30 minutes every day."),
];

let facts = extractor.extract_facts(&messages).await?;

// Output might include:
// - "User started learning guitar"
// - "User is interested in classical guitar"
// - "User practices 30 minutes daily"
```

### Extract User-Only Facts

```rust
let messages = vec![
    Message::user("My favorite color is blue"),
    Message::assistant("Blue is a calming color. Many people find it peaceful."),
];

// Extract only user facts (ignores assistant's general statement)
let user_facts = extractor.extract_user_facts(&messages).await?;

// Output: ["User's favorite color is blue"]
```

---

## Configuration

### Extraction Settings

```toml
[memory]
# Enable automatic fact extraction
auto_enhance = true

# Importance threshold for filtering
importance_threshold = 0.5

# Language detection
auto_detect_language = true
```

### Prompt Customization

You can customize extraction behavior by modifying the prompts in the configuration or extending the `FactExtractor` trait.

---

## Best Practices

### 1. Process Conversations as a Whole

```rust
// Good: Process entire conversation for context
let results = memory_manager.add_memory(&messages, metadata).await?;

// Avoid: Processing messages individually (loses context)
for msg in messages {
    memory_manager.store(msg.content, metadata.clone()).await?;
}
```

### 2. Handle Low-Quality Extractions

```rust
let facts = extractor.extract_facts(&messages).await?;

if facts.is_empty() {
    println!("No significant facts found in conversation");
} else {
    println!("Extracted {} facts", facts.len());
}
```

### 3. Validate Extracted Facts

```rust
for fact in &facts {
    // Check minimum length
    if fact.content.len() < 10 {
        continue;
    }
    
    // Check importance
    if fact.importance < 0.5 {
        continue;
    }
    
    // Store valid fact
    memory_manager.store(fact.content.clone(), metadata.clone()).await?;
}
```

### 4. Use Appropriate Strategy

```rust
// For learning user preferences - use UserOnly
let facts = extractor.extract_user_facts(&messages).await?;

// For documenting agent capabilities - use AssistantOnly
let facts = extractor.extract_assistant_facts(&messages).await?;

// For procedures - use ProceduralMemory
if detect_procedural_pattern(&messages) {
    let facts = extractor.extract_procedural_facts(&messages).await?;
}
```

---

## Troubleshooting

### No Facts Extracted

**Symptoms**: Empty result from extraction

**Solutions**:
- Check if conversation has meaningful content
- Verify LLM API is working
- Reduce importance threshold
- Check for language detection issues

### Too Many Facts

**Symptoms**: Extraction returns too many low-quality facts

**Solutions**:
- Increase importance threshold
- Enable intelligent filtering
- Check for conversational noise
- Use more specific prompts

### Duplicate Facts

**Symptoms**: Similar facts extracted multiple times

**Solutions**:
- Enable semantic deduplication
- Adjust merge threshold
- Check duplicate detector configuration
- Use hash-based filtering

### Language Issues

**Symptoms**: Facts in wrong language or garbled

**Solutions**:
- Verify language detection
- Check LLM supports input language
- Use language-specific prompts
- Set explicit language if known

---

## Next Steps

- [Memory Manager](./memory-manager.md) - Using extracted facts with the manager
- [Memory Types](../concepts/memory-types.md) - Understanding memory categories
- [Memory Pipeline](../concepts/memory-pipeline.md) - Complete memory processing flow
