# Trajectory Elongation: The Hidden Cost of LLM Summarization

## Overview

Trajectory elongation is a critical phenomenon discovered in this research where LLM summarization causes agents to run for more turns than they would with raw or masked contexts. This unexpected side effect significantly erodes the theoretical efficiency gains from bounded context.

## The Phenomenon

### What is Trajectory Elongation?

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     TRAJECTORY ELONGATION ILLUSTRATED                       │
│                                                                             │
│   Without Summarization (Raw/Masked):                                       │
│   ─────────────────────────────────────                                       │
│                                                                             │
│   Turn:    35   36   37   38   39   40   41   42   43   44   45            │
│   Status:  ✓    ✓    ✗    ✗    ✗    ✗    ─    ─    ─    ─    ─             │
│                                                                             │
│   Agent sees: "Test failed" "Test failed" "Test failed" "Test failed"      │
│   Interpretation: "I'm stuck, should stop"                                   │
│   Result: Stops at turn 40 (sees repeated failure)                          │
│                                                                             │
│   Total turns: 40                                                           │
│                                                                             │
│                                                                             │
│   With Summarization:                                                         │
│   ───────────────────                                                         │
│                                                                             │
│   Turn:    35   36   37   38   39   40   41   42   43   44   45            │
│   Status:  ✓    ✓    ✗    ✗    ✗    ✗    ✗    ✗    ✓    ✓    ✓             │
│                                                                             │
│   Summary at turn 38: "Agent has been debugging the test failure,            │
│                         attempted several fixes, investigating edge case"     │
│   Interpretation: "Progress is being made, should continue"                 │
│   Result: Continues through failures, eventually succeeds                 │
│                                                                             │
│   Total turns: 45 (+12.5%)                                                  │
│                                                                             │
│   ═══════════════════════════════════════════════════════════════════       │
│   KEY INSIGHT: Summary smooths over failure signals → Agent doesn't          │
│                realize it's stuck → Continues past sensible stop point      │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Why It Happens

### The Signal Smoothing Mechanism

```
Raw Context:
┌────────────────────────────────────────────────────────────────────────┐
│ Turn 35: Reasoning: Let me run the test                               │
│          Action: run_test("test_foo")                                 │
│          Observation: FAILED - AssertionError at line 45              │
│                                                                        │
│ Turn 36: Reasoning: Let me try a fix                                  │
│          Action: edit_file("foo.py", line=45, content="...")          │
│          Observation: FAILED - AssertionError at line 45              │
│                                                                        │
│ Turn 37: Reasoning: Let me check the imports                          │
│          Action: read_file("foo.py", lines=[1,20])                     │
│          Observation: import os, sys, json... (file content)          │
│                                                                        │
│ Turn 38: Reasoning: Let me try another fix                            │
│          Action: edit_file("foo.py", line=45, content="...")          │
│          Observation: FAILED - AssertionError at line 45              │
│                                                                        │
│ Pattern Recognition: FAILED → FAILED → FAILED (stuck in loop)          │
│ Agent Decision: "I should stop and report failure or ask for help"       │
└────────────────────────────────────────────────────────────────────────┘

Summarized Context (at turn 38):
┌────────────────────────────────────────────────────────────────────────┐
│ [Summary of turns 1-28: Agent has analyzed the codebase, identified      │
│  the bug location in foo.py, attempted fixes, still investigating       │
│  the root cause]                                                        │
│                                                                        │
│ Turn 36: Reasoning: Let me try a fix                                   │
│          Action: edit_file("foo.py", line=45, content="...")          │
│          Observation: FAILED - AssertionError at line 45              │
│                                                                        │
│ Turn 37: Reasoning: Let me check the imports                           │
│          Action: read_file("foo.py", lines=[1,20])                     │
│          Observation: [Observation omitted for brevity]               │
│                                                                        │
│ Turn 38: Reasoning: Let me try another fix                            │
│          Action: edit_file("foo.py", line=45, content="...")          │
│          Observation: FAILED - AssertionError at line 45              │
│                                                                        │
│ Pattern Recognition: Summary shows "investigating" + recent failure     │
│ Agent Decision: "I need to keep trying different approaches"          │
└────────────────────────────────────────────────────────────────────────┘
```

### Mechanism Summary

| Aspect | Raw/Masked | Summarized |
|--------|-----------|------------|
| **Failure visibility** | Clear repetition | Obscured by summary |
| **Stuck detection** | Easy (same error × 3) | Hard (summarized as "investigating") |
| **Agent perception** | "Not making progress" | "Making progress" |
| **Termination signal** | Strong | Weakened |
| **Result** | Earlier termination | Later termination |

## Measured Impact

### Trajectory Length Comparison

| Model | Raw Agent | Observation Masking | LLM Summary | Increase |
|-------|-----------|---------------------|-------------|----------|
| **Qwen3-Coder 480B** | ~avg | ~avg | ~avg | **+15%** |
| **Gemini 2.5 Flash** | 50 | 44 | **52** | **+18%** |
| **Gemini 2.5 Flash (thinking)** | ~ | ~ | ~ | Similar |
| **Qwen3-32B** | ~ | ~ | ~ | Similar |
| **Qwen3-32B (thinking)** | ~15 | ~17 | ~ | Masking +13% |

**Note**: ~ indicates data not explicitly reported; trends from paper analysis.

### Qwen3-Coder 480B Distribution

```
Trajectory Length Distribution (Turns):

LLM Summary:
  0    10    20    30    40    50    60    70    80    90   100   110   120   130   140   150   160   170   180   190   200   210   220   230   240   250
  │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │
  ██████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████████░░░░░░░░░░░
  ▲                                                                                                                                    ▲
  Min                                                                                                                                 Mean
  (few turns)                                                                                                                       (longer avg)

Observation Masking:
  0    10    20    30    40    50    60    70    80    90   100   110   120   130   140   150   160   170   180   190   200   210   220   230   240   250
  │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │     │
  ████████████████████████████████████████████████████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
                                                                                    ▲
                                                                                  (shorter avg)

Mean Difference: ~15% more turns with LLM Summary
```

### Gemini 2.5 Flash Detailed Data

| Statistic | Observation Masking | LLM Summary | Difference |
|-----------|---------------------|-------------|------------|
| Mean turns | 44 | 52 | **+8 turns (+18%)** |
| Median | ~42 | ~50 | +8 turns |
| Distribution | Compressed | Extended | Longer tail |

### Cost Impact of Elongation

Each additional turn costs:
```
Cost per additional turn ≈ (context_tokens + output_tokens) × price

Example (Gemini 2.5 Flash):
- +8 turns × $0.001 per turn ≈ +$0.008 per instance
- × 500 instances = +$4.00 per benchmark
- × 1M tasks = +$8,000 total
```

The 18% elongation partially offsets the theoretical savings from bounded context.

## Why Elongation Reduces Efficiency

### The Theoretical vs. Actual Tradeoff

**Theoretical LLM Summary advantage**:
```
Context size:
- Raw: Linear growth to 250K+ tokens
- Summary: Sawtooth pattern, max ~30K tokens
- Expected savings: ~80% token reduction
```

**Actual with elongation**:
```
Turns:
- Raw: 44 turns average
- Summary: 52 turns average (+18%)

Tokens:
- Raw: 44 × avg_context_size
- Summary: 52 × avg_context_size (smaller per-turn, but more turns)

Net savings reduced from ~80% to ~40-50%
```

### The Efficiency Erosion Calculation

```
Raw Agent:
- Turns: 44
- Avg context at turn t: varies
- Total tokens: Σ context_size(t)
- Cost: $0.41

LLM Summary (theoretical, no elongation):
- Turns: 44
- Bounded context: max ~30K tokens
- Expected cost: ~$0.15
- Expected savings: 63%

LLM Summary (actual, with elongation):
- Turns: 52 (+18%)
- Bounded context: max ~30K tokens
- Actual cost: $0.24
- Actual savings: 41%

Erosion: 63% → 41% = 22 percentage points lost to elongation
```

## Why Observation Masking Avoids Elongation

### Preservation of Failure Signals

| Aspect | Observation Masking | LLM Summary |
|--------|----------------------|-------------|
| Old observations | Hidden (placeholder) | Compressed in summary |
| Recent observations | **Fully visible** | Fully visible |
| Failure repetition | **Agent sees raw failures** | Agent sees summary |
| Pattern recognition | **Intact** | Obscured |
| Stuck detection | **Works normally** | Weakened |

### The Critical Difference

```
Turn 38 with Masking (M=10):
┌────────────────────────────────────────────────────────────────────────┐
│ Recent turns visible: T28, T29, T30, T31, T32, T33, T34, T35, T36, T37│
│                                                                        │
│ T35: Observation: FAILED - AssertionError                             │
│ T36: Observation: [Observation omitted]                               │
│ T37: Observation: FAILED - AssertionError                             │
│                                                                        │
│ Agent sees: "FAILED, (hidden), FAILED"                                  │
│ Still recognizes: Repeated failures → stuck                             │
└────────────────────────────────────────────────────────────────────────┘

Turn 38 with Summary:
┌────────────────────────────────────────────────────────────────────────┐
│ Summary: "Agent has been debugging, attempted fixes, investigating"       │
│                                                                        │
│ Recent turns: T28-T37 (visible)                                        │
│                                                                        │
│ Agent sees: "Investigating (from summary), + recent context"            │
│ Interpretation: "Progress being made"                                   │
│                                                                        │
│ The summary casts the investigation in a positive light                │
└────────────────────────────────────────────────────────────────────────┘
```

## The Critic-Enhanced Summary Experiment

Researchers tested whether making summaries more critical would help:

### Enhanced Summary Prompt

```
You are maintaining a context-aware state checkpoint AND assessing
whether the agent is on track.

Generate CHECKPOINT with:
- USER_CONTEXT, COMPLETED, PENDING, CODE_STATE

Then generate REFLECTIONS:
- Is the agent making progress?
- Is it stuck in a loop?
- What critical information might help?
- Provide up to 2 specific problems and fixes
```

### Results

| Metric | Standard Summary | Critic-Enhanced |
|--------|-----------------|-----------------|
| Solve rate | Baseline | **No improvement** |
| Trajectory length | Baseline | **Even longer** |
| Cost | Baseline | **Higher** |

**Why critic-enhanced made it worse**:
- Critic provides "helpful suggestions" to try
- Agent interprets as "new avenues to explore"
- Encourages continued exploration
- Further elongates trajectory

**Key insight**: Execution-free feedback can paradoxically increase cost by extending exploration.

## Implications for Agent Design

### The Paradox of Helpful Summaries

```
Intuition: Better summary → Better agent performance
Reality: Better summary → More exploration → Higher cost → Same/worse outcome
```

### Design Recommendations

| Goal | Recommendation |
|------|----------------|
| Minimize cost | Use observation masking (no elongation) |
| Bounded context | Use hybrid (minimal summarization) |
| Maximize solve rate | Use hybrid (better than either alone) |
| Simple implementation | Use observation masking |

### When Summarization Makes Sense

Despite elongation, LLM summarization may still be preferred when:
1. **Bounded context is mandatory** (extremely long trajectories)
2. **Old semantic context matters** (need to know what was tried)
3. **Cost is secondary** to capability
4. **Combined with hybrid approach** (defer summarization)

## The Hybrid Solution

### How Hybrid Reduces Elongation

```
Pure LLM Summary:
- Summarize every 21 turns
- Frequent summaries → Frequent "progress framing"
- More elongation

Hybrid (N=43, M=W=10):
- First summary at turn 43
- Then every 43 turns
- ~50% fewer summaries → Less elongation
```

### Hybrid Elongation Analysis

| Strategy | Mean Turns | Elongation vs Raw |
|----------|-----------|-------------------|
| Raw Agent | ~ | Baseline |
| Observation Masking | ~ | Similar |
| LLM Summary | +15-18% | Significant |
| **Hybrid** | +smaller | **Reduced** |

**Result**: Hybrid achieves bounded context with less elongation penalty.

## Future Research Directions

### Open Questions

1. **Can we detect when summarization causes elongation?**
   - Monitor turn count vs. progress metrics
   - Switch strategies mid-trajectory

2. **Can we make summaries that don't cause elongation?**
   - Include explicit "stuck" detection in summary
   - Preserve failure repetition signals
   - Balance compression with fidelity

3. **What about adaptive summarization?**
   - Only summarize when trajectory exceeds threshold
   - Don't summarize during apparent progress
   - Dynamic N based on context growth

### Potential Solutions

| Approach | Mechanism | Expected Impact |
|----------|-----------|-----------------|
| Explicit stuck detection | Summary includes "stuck confidence" | Could reduce elongation |
| Semantic triggering | Summarize only on semantic boundaries | More natural compression |
| Multi-level summaries | Hierarchical compression | Better preservation |
| Agent self-monitoring | Agent detects its own stagnation | Earlier termination |

## Key Takeaways

### For Practitioners

1. **Observation masking is safer** - No risk of elongation
2. **LLM summary has hidden cost** - 15-18% more turns
3. **Hybrid balances tradeoffs** - Bounded context, less elongation
4. **Test your specific use case** - Elongation varies by task type

### For Researchers

1. **Efficiency must include trajectory length** - Not just token count
2. **Simple baselines are essential** - Don't assume complexity helps
3. **Side effects matter** - Consider all consequences of design choices
4. **Elongation is underexplored** - More research needed

## Next Steps

- **[Observation Masking](../strategies/01-observation-masking.md)** - No elongation approach
- **[LLM Summarization](../strategies/02-llm-summarization.md)** - When to use despite elongation
- **[Hybrid Approach](../strategies/03-hybrid-approach.md)** - Minimizing elongation
- **[Performance Results](./02-performance-results.md)** - Full benchmark data
- **[Future Work](../challenges/02-future-work.md)** - Addressing elongation
