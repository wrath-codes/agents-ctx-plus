# Limitations and Scope

## Overview

This research, while comprehensive, has specific scope limitations that are important to acknowledge for proper interpretation of results and for identifying future research directions.

## Scope Constraints

### 1. Domain Limitation: Software Engineering Only

**Constraint**: All experiments conducted exclusively on SWE-bench Verified (Python bug fixing).

**Characteristics of SE Domain**:
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SOFTWARE ENGINEERING DOMAIN                              │
│                                                                             │
│   Observation Characteristics:                                              │
│   - File reads: Very verbose (100s-1000s of lines)                          │
│   - Test outputs: Long stack traces, multiple failures                      │
│   - Search results: Multiple matches with context                           │
│   - Error messages: Detailed compiler/linter output                         │
│                                                                             │
│   Natural Favoring of Observation Masking:                                  │
│   - 84% of tokens are observations                                          │
│   - Masking removes bulk of verbosity                                        │
│   - Reasoning chain (16%) is critical, always preserved                     │
│                                                                             │
│   Other Domains May Differ:                                                 │
│   - Web navigation: HTML can be compressed or masked                        │
│   - Multi-hop QA: Observations are concise facts                           │
│   - Dialogue: All turns are equally important                               │
│   - Code generation: Output is the goal, not observation                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Implication**: Findings on observation masking superiority may be strongest in SE domain. Other domains may favor different strategies.

### 2. Benchmark Limitation: SWE-bench Only

**Constraint**: All experiments on single benchmark (SWE-bench Verified).

| Attribute | SWE-bench Verified | Other Potential Benchmarks |
|-----------|-------------------|---------------------------|
| Language | Python only | Java, JavaScript, C++, etc. |
| Task type | Bug fixing | Feature implementation, refactoring |
| Source | Real GitHub issues | Synthetic, competition problems |
| Verified | Yes (test patches) | Varies |

**Mitigation**: SWE-bench is industry-standard and diverse within its scope.

### 3. Tool Limitation: Specific Agent Scaffolds

**Constraint**: Only two agent frameworks tested (SWE-agent, OpenHands).

| Scaffold | Tools | Characteristics |
|----------|-------|----------------|
| SWE-agent | File read/write, test, search | Optimized for SE tasks |
| OpenHands | Broader tool set | More general purpose |

**Other scaffolds not tested**:
- AutoGPT
- BabyAGI
- LangChain agents
- Custom corporate agents

**Implication**: Findings may be scaffold-dependent. Different tool sets may change optimal strategies.

## Methodological Limitations

### 1. Fixed Thresholds

**Limitation**: All strategies use fixed, non-adaptive thresholds.

```
Current Implementation:
┌─────────────────────────────────────────────────────────────────┐
│ Observation Masking:                                            │
│   if turn_age > M:  # M = 10 (fixed)                           │
│       mask_observation()                                        │
│                                                                 │
│ LLM Summarization:                                              │
│   if turns_since_summary >= N:  # N = 21 (fixed)               │
│       create_summary()                                          │
└─────────────────────────────────────────────────────────────────┘

Limitation:
- M = 10 may be wrong for some tasks
- N = 21 may be wrong for some trajectories
- No adaptation to task difficulty
```

**What's missing**:
- Semantic-based triggering (summarize at subgoal boundaries)
- Load-based triggering (compress when context > threshold)
- Task-specific optimization
- Online learning of optimal thresholds

### 2. Heuristic-Based Triggers

**Limitation**: Triggers are turn-count-based, not semantic.

| Trigger Type | Current | Alternative |
|--------------|---------|-------------|
| Masking | Turn count | Staleness of information |
| Summarization | Turn count | Semantic boundary |
| Hybrid switch | Fixed turn | Context size threshold |

**Example of semantic trigger**:
```
Instead of: "Summarize every 21 turns"
Consider:   "Summarize when agent completes a subtask"
```

### 3. No Deletion Support

**Limitation**: Records cannot be removed from trajectory.

```
Current: Can only mask or summarize
Missing:  Cannot delete irrelevant turns

Example:
- Turn 5: Read wrong file (irrelevant)
- Turn 6: Read correct file
- Current: Both kept (one masked)
- Ideal: Delete turn 5 entirely
```

**Why deletion matters**:
- Irrelevant actions waste context space
- Mistakes need not be preserved
- Cleaner trajectories

### 4. No Multi-Trajectory Optimization

**Limitation**: Each trajectory managed independently.

**Missing**:
- Cross-trajectory learning
- Shared summary patterns
- Knowledge transfer between similar tasks

### 5. Single-Attempt Trajectories

**Limitation**: No multi-attempt selection or ensemble methods.

**Context**: Current agent research often includes:
- Multiple attempts per task
- Best-of-N selection
- Execution-free critics

**Impact**: These increase total cost further, making efficient context management even more critical.

## Statistical Limitations

### 1. Effect Size vs. Statistical Significance

| Comparison | Effect Size | Statistical Significance |
|------------|-------------|-------------------------|
| Masking vs Raw (solve rate) | +1-3pp | Not significant (p > 0.15) |
| Summary vs Raw (solve rate) | +0-2pp | Not significant (p > 0.09) |
| Hybrid vs Raw (solve rate) | +4pp | Significant (p = 0.0499) |

**Interpretation**: While individual comparisons may not reach statistical significance, the consistent pattern across models supports the conclusions.

### 2. Bootstrap Assumptions

| Assumption | Validation |
|------------|------------|
| Paired comparisons | ✓ Same instances across strategies |
| Sufficient replicates | ✓ 10,000 bootstrap samples |
| Effect size reporting | ✓ Both absolute and relative |

### 3. Generalization Bounds

| Aspect | Tested | Generalization |
|--------|--------|----------------|
| Models | 5 configurations | To other models: Likely |
| Scaffolds | 2 | To other scaffolds: With tuning |
| Tasks | 500 instances | To other SE tasks: Likely |
| Domains | 1 (SE) | To other domains: Uncertain |

## Technical Limitations

### 1. Implementation Constraints

| Component | Current | Limitation |
|-----------|---------|------------|
| Summary prompt | OpenHands-style | May not be optimal |
| Placeholder text | Fixed | Could be task-specific |
| KV cache | Managed by vLLM | No explicit optimization |
| Page size | 4KB | Fixed, not tuned |

### 2. API Pricing Assumptions

| Model | Pricing Source | Limitation |
|-------|---------------|------------|
| Qwen | Alibaba Cloud reference | Self-hosted, actual costs vary |
| Gemini | Vertex AI API | Cache hit/miss pricing complex |

**Real-world variance**: 
- Cache hit rates affect Gemini costs significantly
- Self-hosted costs depend on infrastructure
- Reserved capacity vs. on-demand pricing differs

### 3. Turn Limit Constraints

| Experiment | Turn Limit | Impact |
|------------|-----------|--------|
| Main | 250 | Caps maximum trajectory length |
| Some analyses | 50-150 | Subset may not represent full distribution |

**Implication**: Very long trajectories (> 250 turns) not studied.

## Evaluation Limitations

### 1. Single Metric Focus

**Primary metric**: Solve rate (pass/fail on test cases)

**Not evaluated**:
- Patch quality (elegance, maintainability)
- Solution efficiency (performance of fix)
- Explanation quality (reasoning transparency)
- User satisfaction (if human-verified)

### 2. Binary Success Definition

```
Current: Pass/fail based on test cases
  - Pass: All tests pass
  - Fail: Any test fails or timeout

Missing nuance:
  - Partial solutions
  - Progress toward solution
  - Quality of attempt
```

### 3. No Human Evaluation

| Aspect | Automated | Human |
|--------|------------|-------|
| Correctness | ✓ Test cases | ✗ Code review |
| Reasoning | ✗ | ✗ |
| Naturalness | ✗ | ✗ |
| User experience | ✗ | ✗ |

## Environmental and Economic Assumptions

### 1. Cost Calculations

| Assumption | Basis | Variance |
|------------|-------|----------|
| Token prices | Published API rates | Change over time |
| Token counts | Model tokenizer | Exact counts vary |
| Infrastructure | H200 cluster | Real deployments differ |

### 2. Environmental Impact

| Factor | Assumed | Actual |
|--------|---------|--------|
| Carbon per token | Averages | Varies by energy source |
| PUE | 1.2 (typical) | 1.1-1.5 range |
| Hardware efficiency | Measured | Degrades over time |

**Disclaimer**: Environmental calculations are estimates for illustration.

## Comparison to Ideal

### The Theoretical Optimum

```
Ideal Context Management:
┌─────────────────────────────────────────────────────────────────┐
│ 1. Perfect information preservation                              │
│    - All relevant context retained                              │
│    - All irrelevant context discarded                         │
│    - Zero information loss                                      │
│                                                                 │
│ 2. Optimal compression                                           │
│    - Semantic compression only when beneficial                  │
│    - Dynamic compression ratio                                  │
│    - Task-adaptive strategy selection                           │
│                                                                 │
│ 3. No side effects                                               │
│    - No trajectory elongation                                   │
│    - No quality degradation                                     │
│    - No additional compute overhead                             │
│                                                                 │
│ 4. Perfect timing                                                │
│    - Compress exactly when beneficial                           │
│    - Never compress too early or too late                       │
│    - Semantic boundary detection                                │
└─────────────────────────────────────────────────────────────────┘

Current Reality:
┌─────────────────────────────────────────────────────────────────┐
│ • Fixed thresholds (suboptimal)                                  │
│ • Heuristic triggers (not semantic)                             │
│ • Trajectory elongation (with summarization)                    │
│ • Information loss (with masking)                               │
│ • Side effects (hybrid minimizes but doesn't eliminate)          │
└─────────────────────────────────────────────────────────────────┘
```

## Threats to Validity

### Internal Validity

| Threat | Mitigation |
|--------|------------|
| Implementation bugs | Code released for verification |
| Configuration errors | Validated across multiple runs |
| Measurement error | Bootstrap confidence intervals |
| Selection bias | Random sampling within benchmark |

### External Validity

| Threat | Mitigation |
|--------|------------|
| Domain restriction | Acknowledged in limitations |
| Model diversity | 5 configurations tested |
| Temporal validity | Current models (2024-2025) |
| Scaffold dependence | Two scaffolds tested |

### Construct Validity

| Threat | Mitigation |
|--------|------------|
| Solve rate definition | Industry-standard benchmark |
| Cost measurement | Documented calculation methodology |
| Context quality | Not directly measured (indirect via solve rate) |

## What These Limitations Mean

### For Practitioners

1. **Test in your domain**: SE findings may not transfer exactly
2. **Tune for your scaffold**: M=10 is not universal
3. **Monitor trajectory length**: Elongation may affect your use case
4. **Validate on your data**: Benchmarks are proxies, not guarantees

### For Researchers

1. **Domain extension**: Test on web agents, dialogue, code generation
2. **Adaptive strategies**: Learned thresholds, semantic triggers
3. **Deletion support**: Implement and evaluate record removal
4. **Quality metrics**: Beyond pass/fail, measure patch quality

### For the Field

These limitations define the boundary of current knowledge:
- Inside boundary: Confident conclusions
- Outside boundary: Open questions

## Next Steps

- **[Future Work](02-future-work.md)** - Addressing these limitations
- **[Experimental Setup](../experiments/01-experimental-setup.md)** - Methodology details
- **[Performance Results](../experiments/02-performance-results.md)** - Working within limitations
