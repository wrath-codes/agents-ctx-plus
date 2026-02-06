# Experimental Setup

## Overview

The study conducted rigorous experiments comparing context management strategies across diverse model configurations and agent scaffolds. This section details the experimental design, benchmarks, models, and statistical methodology.

## Research Design

### Experimental Goals

1. **Compare effectiveness** - Do context management strategies change solve rates?
2. **Compare efficiency** - How much do strategies reduce cost?
3. **Test generality** - Do findings hold across models and scaffolds?
4. **Identify tradeoffs** - What are the efficiency-effectiveness tradeoffs?

### Independent Variables

| Variable | Levels | Description |
|----------|--------|-------------|
| **Context Strategy** | 3 | Raw Agent, Observation Masking, LLM Summary |
| **Model Family** | 2 | Qwen3, Gemini |
| **Model Size** | 3 | 32B, 480B, Proprietary |
| **License** | 2 | Open-weight, Proprietary |
| **Reasoning** | 2 | Non-thinking, Thinking |

### Dependent Variables

| Variable | Measurement | Priority |
|----------|-------------|----------|
| **Solve Rate** | % tasks successfully completed | Effectiveness ↑ |
| **Instance Cost** | Total API cost per task ($) | Efficiency ↓ |
| **Trajectory Length** | Number of turns until termination | Secondary |
| **Token Count** | Total tokens processed | Diagnostic |

## Benchmark: SWE-bench Verified

### Dataset

**SWE-bench Verified** - Industry-standard benchmark for software engineering agents:

| Attribute | Value |
|-----------|-------|
| Source | Real GitHub issues |
| Language | Python |
| Instances | 500 |
| Task Type | Bug fixing |
| Difficulty | Production-level |

### Instance Structure

Each instance contains:
```
{
  "repo": "python/cpython",
  "issue_url": "https://github.com/python/cpython/issues/...",
  "base_commit": "abc123...",
  "test_patch": "...",
  "patch": "...",
  "problem_statement": "Description of bug to fix"
}
```

### Why SWE-bench Verified?

1. **Real-world relevance** - Actual production bugs
2. **Verifiable solutions** - Test suite validates fixes
3. **Diverse complexity** - Simple to challenging issues
4. **Industry standard** - Used by leading agent research
5. **Long trajectories** - Requires multiple tool invocations

### Task Requirements

Typical successful trajectory:
- Read relevant source files (5-10 file reads)
- Understand codebase structure
- Identify bug location
- Implement fix
- Run tests to verify
- Submit solution

## Models Tested

### Configuration Matrix

| Model | Size | License | Context Window | Reasoning Modes |
|-------|------|---------|----------------|-----------------|
| Qwen3-32B | 32B | Open-weight | 122K tokens | Thinking, Non-thinking |
| Qwen3-Coder 480B | 480B (35B active) | Open-weight | 256K tokens | Non-thinking |
| Gemini 2.5 Flash | — | Proprietary | 1M tokens | Thinking, Non-thinking |

### Model Details

#### Qwen3-32B

```
Architecture: Dense transformer
Parameters: 32.5B
Context: 122K (via YaRN extension)
Inference: Self-hosted on 2× H200 GPUs
Framework: vLLM
Temperature: 0.8 (agent), 0.0 (summary)
```

#### Qwen3-Coder 480B

```
Architecture: Mixture of Experts (MoE)
Total Parameters: 480B
Active Parameters: 35B
Context: 256K tokens (native)
Inference: Self-hosted on 8× H200 GPUs
Workers: 35 inference workers
```

#### Gemini 2.5 Flash

```
Access: Vertex AI API
Context: 1M tokens (advertised)
Effective: ~100K (estimated)
Version: gemini-2.5-flash
Thinking budget: 0 or 800 tokens
```

### Reasoning Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Non-thinking** | Direct response generation | Fast, cheaper inference |
| **Thinking** | Chain-of-thought reasoning | Complex problem solving |

### Why These Models?

Diverse configurations test generality:
- **Sizes**: 32B to 480B
- **Licenses**: Open vs. proprietary
- **Reasoning**: Both modes
- **Best performers**: Qwen3-Coder 480B achieved highest solve rates

## Agent Scaffolds

### Primary: SWE-agent

| Attribute | Configuration |
|-----------|---------------|
| Framework | ReAct / CodeAct |
| Version | Latest (2024) |
| Turn Limit | 250 |
| Main Test | 500 instances |

**SWE-agent Features**:
- File read/write tools
- Test execution
- Code search (grep/find)
- Editor with line numbers
- View commands (directory listing)

### Secondary: OpenHands

| Attribute | Configuration |
|-----------|---------------|
| Framework | CodeAct |
| Version | v0.43.0 |
| Turn Limit | 250 |
| Probe Test | 50 instances |

**Purpose**: Test generality across different agent implementations.

**Key Difference**: OpenHands retains retry turns (syntax errors, etc.), requiring larger masking window (M=58 vs M=10).

## Context Management Configurations

### Raw Agent (Baseline)

```python
config = {
    'strategy': 'none',
    'description': 'No context management - full trajectory sent to LLM'
}
```

### Observation Masking

```python
config = {
    'strategy': 'observation_masking',
    'window_size_m': 10,  # Keep last 10 turns visible
    'placeholder': '[Observation omitted for brevity]'
}
```

**Rationale for M=10**:
- Tested M=5, 10, 20
- M=10 optimal balance
- See [sensitivity analysis](./02-performance-results.md#sensitivity-analysis)

### LLM Summarization

```python
config = {
    'strategy': 'llm_summary',
    'summarize_window_n': 21,  # Turns to accumulate
    'tail_window_m': 10,        # Recent turns to keep
    'summary_temperature': 0.0
}
```

**Rationale for N=21, M=10**:
- Aligned with OpenHands implementation
- M=10 matches masking for fair comparison
- Summarize more turns at once than OpenHands default (50-50 split)

### Hybrid Approach

```python
config = {
    'strategy': 'hybrid',
    'masking_window_w': 10,
    'summarize_at_n': 43,
    'tail_window_m': 10
}
```

**Rationale for N=43**:
- At turn 43 with masking, context ≈ 30K tokens
- Matches raw agent at N=21 ≈ 30K tokens
- Optimal deferral of summarization

## Infrastructure

### Compute Resources

| Component | Specification |
|-----------|---------------|
| GPUs | 8× NVIDIA H200 |
| GPU Memory | 141 GB HBM each |
| Storage | 8 TB local |
| Host | Shared cluster |

### Deployment

| Model | GPUs | Workers | Notes |
|-------|------|---------|-------|
| Qwen3-32B | 2 | 15 | Conservative for long contexts |
| Qwen3-Coder 480B | 8 | 35 | Full cluster utilization |
| Gemini 2.5 Flash | — | 8 SWE-agent / 5 OpenHands | API rate limits |

### Serving Stack

```
vLLM (for Qwen models)
├── PagedAttention for efficient KV cache
├── Continuous batching
└── Tensor parallelism (for 480B)

Vertex AI (for Gemini)
├── Managed API
├── Auto-scaling
└── Cache pricing (miss vs hit)
```

## Cost Calculation

### Self-Hosted Models (Qwen)

```
Cost = (input_tokens × input_price) + (output_tokens × output_price)

Prices from Alibaba Cloud API (reference):
- Qwen3-32B: $0.50/M input, $1.00/M output
- Qwen3-Coder 480B: $2.00/M input, $6.00/M output

Note: No cache hit/miss distinction in pricing
```

### API Models (Gemini)

```
Cost = (input_cache_miss × miss_price) 
     + (input_cache_hit × hit_price)
     + (output_tokens × output_price)

Vertex AI pricing:
- Gemini 2.5 Flash: $0.15/M input (miss), $0.015/M input (hit), $0.60/M output
```

### Cost Components Tracked

| Component | Included | Notes |
|-----------|----------|-------|
| Agent LLM calls | ✅ | All turns |
| Summary generation | ✅ | Additional for summary strategy |
| Failed attempts | ✅ | Part of total cost |
| Warm-up turns | ✅ | Before management activates |

## Statistical Methodology

### Confidence Intervals

**95% bootstrap confidence intervals**:
- B = 10,000 replicates
- Percentile method
- Asymmetric intervals reported

Example:
```
Solve Rate: 54.8% 
CI: [50.4%, 59.2%]  (asymmetric)
Reported: 54.8 ± 4.4%
```

### Significance Testing

**Paired nonparametric bootstrap**:
```
For each bootstrap replicate:
  1. Resample 500 instances with replacement
  2. Compute paired difference: Δ = strategy - raw
  3. Store Δ

p-value = 2 × min(Pr(Δ* ≥ 0), Pr(Δ* ≤ 0))

Significance: † when p < 0.05
```

**Preserves instance-level correlations**:
- Same instances compared across strategies
- Accounts for task difficulty variation

### Effect Size Reporting

| Metric | Units | Example |
|--------|-------|---------|
| Solve rate change | Percentage points | +2.6 pp |
| Cost change | Dollars per instance | -$0.68 |
| Cost reduction | Percentage | -52.7% |

### Multiple Comparisons

For 5 model configurations × 3 strategies:
- Family-wise error not controlled
- Focus on effect sizes and consistency
- Replication across models as validation

## Experimental Procedure

### Per-Instance Execution

```
FOR each instance in SWE-bench Verified:
  FOR each strategy in [Raw, Masking, Summary]:
    1. Reset agent to initial state
    2. Set context management strategy
    3. Run agent up to 250 turns
    4. Record:
       - Success/failure
       - Total turns
       - Token counts
       - API costs
       - Final trajectory
```

### Quality Control

| Check | Action |
|-------|--------|
| Configuration validation | Verify N, M, W parameters |
| Trajectory inspection | Sample 4% for qualitative analysis |
| Outlier investigation | Examine unexpected results |
| Reproducibility | Release code and data |

### Outlier: Qwen3-32B (Thinking)

Short median trajectory length (~50% shorter than non-thinking):

| Configuration | Median Turns | Implication |
|-------------|--------------|-------------|
| Qwen3-32B | ~30 | Normal |
| Qwen3-32B (thinking) | ~15 | Too short for management benefits |

**Investigation**:
- Verified configuration correct
- No suspicious exit patterns
- Valid but reduces observed savings

## Data Release

### Available Resources

| Resource | Location | Contents |
|----------|----------|----------|
| **Code** | [GitHub](https://github.com/JetBrains-Research/the-complexity-trap) | Implementations, experiments |
| **Data** | [HuggingFace](https://huggingface.co/datasets/JetBrains-Research/the-complexity-trap) | Trajectories, metrics, results |
| **Paper** | [arXiv](https://arxiv.org/pdf/2508.21433) | Full research paper |

### Dataset Contents

```
Dataset Structure:
├── trajectories/
│   ├── sweagent_qwen3_32b_raw/
│   ├── sweagent_qwen3_32b_masking/
│   ├── sweagent_qwen3_32b_summary/
│   └── ... (all configurations)
├── metrics/
│   ├── solve_rates.json
│   ├── costs.json
│   └── turn_lengths.json
├── analysis/
│   ├── bootstrap_results.json
│   └── significance_tests.json
└── metadata/
    ├── instance_info.json
    └── model_configs.json
```

## Limitations of Experimental Design

### Scope Constraints

| Constraint | Impact | Mitigation |
|------------|--------|------------|
| SE domain only | Findings may not generalize | Related work suggests generalization |
| SWE-bench only | Single benchmark | Industry-standard, diverse issues |
| Python only | Language-specific? | Core findings likely language-agnostic |

### Methodological Limitations

| Limitation | Description |
|------------|-------------|
| Fixed thresholds | 80% load factor, fixed N/M/W |
| Heuristic triggers | Turn-based rather than semantic |
| Single trajectory | No multi-attempt selection |
| No deletion | Records cannot be removed |

## Next Steps

- **[Performance Results](02-performance-results.md)** - Benchmark data with statistical analysis
- **[Trajectory Elongation](03-trajectory-elongation.md)** - The hidden cost phenomenon
- **[Observation Masking](../strategies/01-observation-masking.md)** - Strategy details
- **[Limitations](../challenges/01-limitations.md)** - Scope and constraints
