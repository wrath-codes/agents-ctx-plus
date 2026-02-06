# Optimization System

Cortex Memory includes a sophisticated optimization system that automatically maintains memory quality, removes duplicates, and improves retrieval performance over time.

---

## Overview

The optimization system periodically:
- **Detects duplicates** and merges similar memories
- **Improves quality** by refining and consolidating content
- **Optimizes relevance** by removing outdated information
- **Manages storage** by archiving or deleting old memories

---

## Optimization Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Optimization Flow                     │
├─────────────┬─────────────┬──────────────┬───────────────┤
│  Detection  │   Analysis  │   Execution  │    Reporting  │
├─────────────┼─────────────┼──────────────┼───────────────┤
│ Find        │ Create      │ Execute      │ Generate      │
│ Issues      │ Plan        │ Actions      │ Report        │
├─────────────┼─────────────┼──────────────┼───────────────┤
│ Duplicates  │ Strategy    │ Merge        │ Statistics    │
│ Low Quality │ Selection   │ Delete       │ Metrics       │
│ Outdated    │ Priority    │ Update       │ History       │
└─────────────┴─────────────┴──────────────┴───────────────┘
```

---

## Optimization Components

### 1. Optimization Detector

Identifies issues in the memory store:

```rust
pub struct OptimizationDetector {
    memory_manager: Arc<MemoryManager>,
}

impl OptimizationDetector {
    pub async fn detect_issues(&self, filters: &Filters) -> Result<Vec<OptimizationIssue>> {
        // Detect various issues:
        // - Duplicate memories
        // - Low quality content
        // - Outdated information
        // - Poor classification
        // - Space inefficiency
    }
}
```

### 2. Optimization Analyzer

Creates optimization plans:

```rust
pub struct OptimizationAnalyzer {
    memory_manager: Arc<MemoryManager>,
}

impl OptimizationAnalyzer {
    pub async fn create_optimization_plan(
        &self,
        issues: &[OptimizationIssue],
        strategy: &OptimizationStrategy,
        filters: &Filters,
    ) -> Result<OptimizationPlan> {
        // Analyze issues and create plan
        // - Select appropriate actions
        // - Estimate duration
        // - Prioritize operations
    }
}
```

### 3. Execution Engine

Executes optimization actions:

```rust
pub struct ExecutionEngine {
    memory_manager: Arc<MemoryManager>,
}

impl ExecutionEngine {
    pub async fn execute_plan(
        &self,
        optimization_id: &str,
        plan: OptimizationPlan,
    ) -> Result<OptimizationResult> {
        // Execute actions:
        // - Merge duplicates
        // - Delete outdated
        // - Update classifications
        // - Archive old memories
    }
}
```

### 4. Result Reporter

Generates optimization reports:

```rust
pub struct ResultReporter;

impl ResultReporter {
    pub async fn report_optimization_result(
        &self,
        result: &OptimizationResult,
    ) -> Result<()> {
        // Log results
        // Update statistics
        // Notify subscribers
    }
}
```

---

## Issue Types

### IssueKind Enum

```rust
pub enum IssueKind {
    Duplicate,          // Similar or identical memories
    LowQuality,         // Poor quality content
    Outdated,           // Old, potentially stale information
    PoorClassification, // Misclassified memories
    SpaceInefficient,   // Storage optimization needed
}
```

### IssueSeverity

```rust
pub enum IssueSeverity {
    Low,      // Minor improvement possible
    Medium,   // Noticeable impact
    High,     // Significant problem
    Critical, // Urgent attention needed
}
```

---

## Optimization Strategies

### Strategy Types

```rust
pub enum OptimizationStrategy {
    Full,           // Comprehensive optimization
    Incremental,    // Small, frequent updates
    Batch,          // Process in batches
    Deduplication,  // Focus on duplicates only
    Relevance,      // Focus on relevance only
    Quality,        // Focus on quality only
    Space,          // Focus on storage only
}
```

### Strategy Details

#### Full Optimization

```rust
// Comprehensive optimization covering all aspects
OptimizationStrategy::Full

// Actions:
// - Detect and merge duplicates
// - Improve quality of low-scoring memories
// - Remove outdated information
// - Reclassify misclassified memories
// - Archive old memories
```

#### Deduplication Only

```rust
// Focus only on removing duplicates
OptimizationStrategy::Deduplication

// Actions:
// - Find similar memories (semantic similarity > 0.85)
// - Merge related duplicates
// - Delete exact duplicates
```

#### Quality Optimization

```rust
// Focus on improving memory quality
OptimizationStrategy::Quality

// Actions:
// - Find low-quality memories (length < 10 chars)
// - Consolidate fragmented memories
// - Refine unclear content using LLM
```

#### Relevance Optimization

```rust
// Focus on maintaining relevance
OptimizationStrategy::Relevance

// Actions:
// - Apply time decay to old memories
// - Remove memories below importance threshold
// - Update based on access frequency
```

---

## Configuration

### OptimizationConfig

```rust
pub struct OptimizationConfig {
    pub auto_optimize: bool,
    pub trigger_config: TriggerConfig,
    pub strategy_configs: StrategyConfigs,
    pub execution_config: ExecutionConfig,
    pub safety_config: SafetyConfig,
}
```

### Trigger Configuration

```toml
[memory.optimization.trigger_config]

# Auto-trigger when thresholds are exceeded
[[auto_trigger]]
name = "weekly_full_optimize"
enabled = true
strategy = "Full"

[auto_trigger.thresholds]
max_memory_count = 10000
max_storage_size_mb = 1024
duplicate_ratio_threshold = 0.2
search_latency_ms = 1000

# Schedule-based triggers
[schedule_config]
default_cron = "0 2 * * 0"  # Every Sunday at 2 AM
time_zone = "UTC"

# Manual trigger settings
[manual_config]
confirm_required = true
preview_enabled = true
```

### Strategy Configurations

```toml
[memory.optimization.strategy_configs]

# Deduplication settings
[deduplication]
semantic_threshold = 0.85      # Semantic similarity threshold
content_threshold = 0.70       # Content similarity threshold
metadata_threshold = 0.80      # Metadata similarity threshold
merge_threshold = 0.90         # Threshold for auto-merge
max_batch_size = 1000          # Process in batches

# Relevance settings
[relevance]
time_decay_days = 30           # Decay factor for age
min_access_frequency = 0.05    # Minimum access rate
importance_threshold = 0.3     # Minimum importance

# Quality settings
[quality]
min_content_length = 10        # Minimum characters
quality_score_threshold = 0.4  # Minimum quality score

# Space settings
[space]
max_memory_per_type = 5000     # Limit per memory type
archive_after_days = 90        # Archive after 90 days
```

### Execution Configuration

```toml
[memory.optimization.execution_config]
batch_size = 100               # Memories per batch
max_concurrent_tasks = 4       # Parallel operations
timeout_minutes = 30           # Max optimization time
retry_attempts = 3             # Retry failed operations
```

### Safety Configuration

```toml
[memory.optimization.safety_config]
auto_backup = true             # Backup before optimization
backup_retention_days = 7      # Keep backups for 7 days
max_optimization_duration_hours = 2
```

---

## Using the Optimization System

### Via CLI

```bash
# Start optimization manually
cortex-mem-cli optimize start

# Check optimization status
cortex-mem-cli optimize-status --job-id <uuid>

# View optimization configuration
cortex-mem-cli optimize-config --get

# Update configuration
cortex-mem-cli optimize-config --set \
  --schedule "0 0 * * 0" \
  --enabled true

# Cancel running optimization
cortex-mem-cli optimize cancel --job-id <uuid>

# Analyze without executing (dry run)
cortex-mem-cli optimize start --dry-run
```

### Via REST API

```bash
# Start optimization
curl -X POST http://localhost:8000/optimization \
  -H "Content-Type: application/json" \
  -d '{
    "strategy": "Full",
    "filters": {
      "user_id": "user123"
    },
    "dry_run": false
  }'

# Check status
curl http://localhost:8000/optimization/<job-id>

# Get optimization history
curl http://localhost:8000/optimization/history

# Get statistics
curl http://localhost:8000/optimization/statistics

# Cleanup old history
curl -X POST http://localhost:8000/optimization/cleanup
```

### Via Library

```rust
use cortex_mem_core::memory::optimizer::MemoryOptimizer;
use cortex_mem_core::types::{
    OptimizationRequest, OptimizationStrategy, OptimizationFilters
};

// Create optimizer
let optimizer = DefaultMemoryOptimizer::new(
    memory_manager.clone(),
    optimization_config
);

// Create optimization request
let request = OptimizationRequest {
    optimization_id: Some("opt-001".to_string()),
    strategy: OptimizationStrategy::Full,
    filters: OptimizationFilters {
        user_id: Some("user123".to_string()),
        ..Default::default()
    },
    aggressive: false,
    dry_run: false,
    timeout_minutes: Some(30),
};

// Execute optimization
let result = optimizer.optimize(&request).await?;

println!("Optimization completed!");
println!("Issues found: {}", result.issues_found.len());
println!("Actions performed: {}", result.actions_performed.len());
println!("Duration: {:?}", result.end_time - result.start_time);
```

---

## Optimization Actions

### Action Types

```rust
pub enum OptimizationAction {
    Merge { 
        memories: Vec<String>  // IDs to merge
    },
    Delete { 
        memory_id: String 
    },
    Update { 
        memory_id: String,
        updates: MemoryUpdates 
    },
    Reclassify { 
        memory_id: String 
    },
    Archive { 
        memory_id: String 
    },
}

pub struct MemoryUpdates {
    pub content: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub importance_score: Option<f32>,
    pub entities: Option<Vec<String>>,
    pub topics: Option<Vec<String>>,
    pub custom_metadata: Option<HashMap<String, serde_json::Value>>,
}
```

### Action Examples

#### Merge Duplicates

```rust
OptimizationAction::Merge {
    memories: vec![
        "mem-001".to_string(),
        "mem-002".to_string(),
        "mem-003".to_string(),
    ],
}
// Result: Creates merged memory, deletes originals
```

#### Delete Outdated

```rust
OptimizationAction::Delete {
    memory_id: "mem-old".to_string(),
}
// Result: Removes memory from store
```

#### Update Classification

```rust
OptimizationAction::Update {
    memory_id: "mem-123".to_string(),
    updates: MemoryUpdates {
        memory_type: Some(MemoryType::Personal),
        importance_score: Some(0.9),
        ..Default::default()
    },
}
```

---

## Optimization Results

### OptimizationResult Structure

```rust
pub struct OptimizationResult {
    pub optimization_id: String,
    pub strategy: OptimizationStrategy,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub issues_found: Vec<OptimizationIssue>,
    pub actions_performed: Vec<OptimizationAction>,
    pub metrics: Option<OptimizationMetrics>,
    pub success: bool,
    pub error_message: Option<String>,
}
```

### OptimizationMetrics

```rust
pub struct OptimizationMetrics {
    pub total_optimizations: u64,
    pub last_optimization: Option<DateTime<Utc>>,
    pub memory_count_before: usize,
    pub memory_count_after: usize,
    pub saved_space_mb: f64,
    pub deduplication_rate: f32,
    pub quality_improvement: f32,
    pub performance_improvement: f32,
}
```

---

## Best Practices

### 1. Schedule Regular Optimization

```toml
# Weekly full optimization
[memory.optimization.schedule_config]
default_cron = "0 2 * * 0"  # Sunday 2 AM

# Daily incremental optimization
[[auto_trigger]]
name = "daily_incremental"
enabled = true
strategy = "Incremental"
cron = "0 3 * * *"
```

### 2. Use Dry Run First

```rust
// Test optimization before executing
let request = OptimizationRequest {
    dry_run: true,  // Preview only
    ..Default::default()
};

let preview = optimizer.optimize(&request).await?;
println!("Would perform {} actions", preview.actions_performed.len());
```

### 3. Monitor Optimization Metrics

```rust
// Track optimization effectiveness
let stats = optimizer.get_optimization_statistics().await?;

println!("Total optimizations: {}", stats.total_optimizations);
println!("Deduplication rate: {:.1}%", stats.deduplication_rate * 100.0);
println!("Space saved: {:.1} MB", stats.saved_space_mb);
```

### 4. Set Appropriate Thresholds

```toml
# Conservative settings for production
[deduplication]
semantic_threshold = 0.90  # High threshold = fewer false positives
merge_threshold = 0.95     # Only merge very similar memories

# Aggressive settings for cleanup
[deduplication]
semantic_threshold = 0.75  # Lower threshold = more aggressive
merge_threshold = 0.85
```

### 5. Use Scoped Optimization

```rust
// Optimize specific user only
let filters = OptimizationFilters {
    user_id: Some("user123".to_string()),
    ..Default::default()
};

// Optimize specific time range
let filters = OptimizationFilters {
    date_range: Some(DateRange {
        start: Some(Utc::now() - Duration::days(30)),
        end: Some(Utc::now()),
    }),
    ..Default::default()
};
```

---

## Troubleshooting

### Common Issues

#### Optimization Taking Too Long

```rust
// Reduce batch size
[memory.optimization.execution_config]
batch_size = 50  // Smaller batches

// Limit scope
let filters = OptimizationFilters {
    user_id: Some("specific-user".to_string()),  // Single user only
    ..Default::default()
};
```

#### Too Many Duplicates Detected

```rust
// Increase thresholds
[deduplication]
semantic_threshold = 0.90  // Require higher similarity
```

#### Memories Deleted Unexpectedly

```rust
// Enable safety features
[memory.optimization.safety_config]
auto_backup = true
confirm_required = true  // For manual triggers

// Use dry run first
let request = OptimizationRequest {
    dry_run: true,
    ..Default::default()
};
```

---

## Next Steps

- [Memory Pipeline](./memory-pipeline.md) - How memories flow through the system
- [Architecture Overview](./architecture.md) - System architecture
- [Configuration](../config/memory.md) - Memory configuration options
