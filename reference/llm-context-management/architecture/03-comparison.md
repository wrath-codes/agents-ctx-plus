# Strategy Comparison: Observation Masking vs. LLM Summarization

## Overview

Two primary approaches have emerged for managing agent context growth. This section provides a detailed comparison of their mechanics, tradeoffs, and use cases.

## Side-by-Side Comparison

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     RAW AGENT (No Management)                               │
│                                                                              │
│   [Sys] [User] [T1] [T2] [T3] [T4] [T5] ... [T250]                          │
│    ↓     ↓     ↓    ↓    ↓    ↓    ↓         ↓                             │
│   Full  Full  Full  Full Full Full Full     Full                           │
│                                                                              │
│   Context grows linearly → Cost explodes, performance degrades                │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                     OBSERVATION MASKING                                      │
│                                                                              │
│   [Sys] [User] [T1] [T2] [T3] [T4] [T5] ... [T246] [T247] [T248] [T249]    │
│    ↓     ↓     ↓    ↓    ↓    ↓    ↓         ↓      ↓      ↓      ↓       │
│   Full  Full  Mask Mask Mask Mask Mask      Full   Full   Full   Full     │
│                        ▲                      ▲                            │
│                        │                      │                            │
│                   Placeholder            Visible window                     │
│                   "[Output omitted]"     (M = 10 most recent)               │
│                                                                              │
│   Observations hidden, reasoning preserved → Reduced cost growth            │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                     LLM SUMMARIZATION                                        │
│                                                                              │
│   [Sys] [User]          [Summary]        [T230] [T231] ... [T249]            │
│    ↓     ↓                 ↓              ↓      ↓          ↓               │
│   Full  Full            Compressed      Full   Full       Full            │
│                            ↓                                                 │
│   [T1-T220] condensed → Summary captures                                  │
│   salient information                                                        │
│                                                                              │
│   Old turns compressed → Bounded context, summary generation cost           │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Detailed Comparison Table

| Aspect | Observation Masking | LLM Summarization |
|--------|----------------------|-------------------|
| **Mechanism** | Replace old observations with placeholders | Use LLM to compress old turns |
| **Context Growth** | Slowed (linear, unbounded) | Bounded (logarithmic) |
| **Preservation** | Recent M turns fully visible | Recent M turns + summary |
| **Key Property** | Reasoning chain intact | Semantic compression |
| **Additional Cost** | None | Summary generation API calls |
| **Warm-up Turns** | M (e.g., 10) | N + M (e.g., 31) |
| **Infinite Scaling** | No (context grows indefinitely) | Yes (theoretically infinite) |
| **Complexity** | Low | Higher |

## Key Differences

### 1. Context Growth Pattern

**Observation Masking**:
```
Tokens
  ▲
  │      ╱
  │     ╱
  │    ╱  (continues growing)
  │   ╱
  │  ╱
  │ ╱
  └─────────────────────────────▶ Turns
     10   50   100   200   250
```

**LLM Summarization**:
```
Tokens
  ▲
  │  ╱╲    ╱╲    ╱╲
  │ ╱  ╲  ╱  ╲  ╱  ╲
  │╱    ╲╱    ╲╱    ╲ (sawtooth pattern, bounded)
  │
  └─────────────────────────────▶ Turns
     10   50   100   200   250
```

### 2. Information Preservation

**Observation Masking**:
- ✅ Full reasoning chain preserved
- ✅ Recent observations available
- ❌ Old observations lost (just placeholder)
- ❌ No semantic compression of old turns

**LLM Summarization**:
- ✅ Old information semantically compressed
- ✅ Recent observations available
- ⚠️ Summary may lose nuanced details
- ⚠️ Summary generation introduces delay

### 3. Cost Structure

**Observation Masking**:
```
Total Cost = Agent LLM calls only
           = Σ (context_tokens_at_turn_t × price_per_token)
           (no additional overhead)
```

**LLM Summarization**:
```
Total Cost = Agent LLM calls + Summarizer LLM calls
           = Σ (context_tokens_at_turn_t × price)
           + Σ (summary_generation_tokens × price)
           (5-7% overhead from summarization)
```

## Advantages and Disadvantages

### Observation Masking

| Advantages | Disadvantages |
|------------|---------------|
| Simple to implement | Context still grows (unbounded) |
| No additional API calls | Old observations completely lost |
| No warm-up period | No semantic compression |
| Preserves reasoning chain | May miss relevant old context |
| Fastest cost reduction | Infinite trajectories problematic |

### LLM Summarization

| Advantages | Disadvantages |
|------------|---------------|
| Bounded context growth | Additional API costs (5-7%) |
| Semantic compression | Longer warm-up period (N+M) |
| Theoretically infinite | "Trajectory elongation" effect |
| Captures salient info | Summary quality dependent |
| Complex implementation | Cache inefficiency (unique sequences) |

## The Trajectory Elongation Effect

A critical finding: **LLM summaries can cause agents to run longer**.

### Why It Happens

```
Scenario: Agent is stuck on a bug

Raw/Masked Context:
  "Test failed with AssertionError at line 45"
  "Test failed with AssertionError at line 45"
  "Test failed with AssertionError at line 45"
  → Agent clearly sees repeated failure → Stops

Summarized Context:
  "Summary: Agent has been debugging line 45,
   made several attempts, need to try different approach"
  → Agent thinks progress is being made → Continues
```

Summaries can **smooth over failure signals** that would otherwise prompt termination.

### Measured Impact

| Model | Masking (turns) | Summary (turns) | Increase |
|-------|-----------------|-----------------|----------|
| Gemini 2.5 Flash | 44 | 52 | **+18%** |
| Qwen3-Coder 480B | ~ | ~ | **+15%** |

More turns = more cost, eroding the efficiency gains from bounded context.

## Use Case Suitability

### When to Use Observation Masking

✅ **Best for**:
- Short to medium trajectories (< 100 turns)
- Cost-sensitive deployments
- When recent context is most important
- When reasoning chain preservation is critical
- Initial production deployment (simplicity)

⚠️ **Avoid when**:
- Trajectories are extremely long (> 500 turns)
- Old context is frequently needed
- Bounded context is strictly required

### When to Use LLM Summarization

✅ **Best for**:
- Very long-running agents
- When old context semantic meaning matters
- When bounded context is required
- Sufficient budget for summary overhead

⚠️ **Avoid when**:
- Cost is the primary constraint
- Short trajectories (warm-up overhead dominates)
- Failure detection is critical

## The Surprising Result

Despite theoretical advantages of bounded growth:

| Metric | Winner |
|--------|--------|
| **Cost** | Observation Masking (4/5 configurations) |
| **Solve Rate** | Tie (no consistent winner) |
| **Simplicity** | Observation Masking |
| **Effectiveness** | Observation Masking |

**Why?**
1. 84% of tokens are observations - masking removes most bulk
2. No summary generation overhead (5-7% savings)
3. No trajectory elongation (saves 15-18% turns)
4. Recent context is often sufficient for SE tasks

## Hybrid: Best of Both Worlds

The research introduces a hybrid combining strengths:

```
Phase 1 (Turns 1-43): Observation Masking
  → Quick cost reduction, no warm-up penalty

Phase 2 (Turn 44+): LLM Summarization when needed
  → Bounded context for extremely long trajectories

Result: 7% cheaper than masking, 11% cheaper than summary
```

See [Hybrid Approach](../strategies/03-hybrid-approach.md) for details.

## Decision Framework

```
                    Start Here
                        │
                        ▼
            ┌───────────────────────┐
            │ What's the trajectory │
            │ length distribution?  │
            └───────────┬───────────┘
                        │
            ┌───────────┼───────────┐
            ▼           ▼           ▼
        Mostly       Mixed      Mostly
        Short      (<50/       Long
       (<50)      >50 split)   (>100)
            │           │           │
            ▼           ▼           ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │ Observation │ │   HYBRID    │ │    LLM      │
    │   Masking   │ │  (Masking   │ │  Summary    │
    │             │ │  + Summary) │ │             │
    └─────────────┘ └─────────────┘ └─────────────┘
```

## Next Steps

- **[Observation Masking](../strategies/01-observation-masking.md)** - Deep dive into the simple winner
- **[LLM Summarization](../strategies/02-llm-summarization.md)** - Understanding the complex alternative
- **[Hybrid Approach](../strategies/03-hybrid-approach.md)** - Combining strengths
- **[Performance Results](../experiments/02-performance-results.md)** - Empirical comparison data
