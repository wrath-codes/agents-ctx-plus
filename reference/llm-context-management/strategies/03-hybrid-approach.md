# Hybrid Approach: Combining Observation Masking and LLM Summarization

## Overview

The hybrid approach is a novel context management strategy that combines the strengths of both observation masking and LLM summarization. By using observation masking as the default and deferring LLM summarization as a last resort for extremely long trajectories, it achieves superior cost-efficiency while maintaining effectiveness.

**Key Result**: 7% cheaper than masking alone, 11% cheaper than summarization alone, with improved solve rates.

## Core Concept

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         HYBRID APPROACH FLOW                                │
│                                                                             │
│   Phase 1: Observation Masking (Turns 1-43)                                 │
│   ─────────────────────────────────────────                                 │
│                                                                             │
│   [Sys] [User] [T1] [T2] [T3] ... [T35] [T36] [T37] [T38] [T39] [T40]      │
│    ↓     ↓     ↓    ↓    ↓       ↓    ↓    ↓    ↓    ↓    ↓               │
│   Full  Full  Mask Mask Mask     Mask Mask Mask Mask Mask Mask             │
│                        ▲                                                    │
│                        │                                                    │
│              Quick cost reduction, no warm-up penalty                       │
│                                                                             │
│                                    │                                        │
│                                    ▼                                        │
│                                                                             │
│   Phase 2: LLM Summarization (Turn 44+)                                     │
│   ─────────────────────────────────────                                     │
│                                                                             │
│   [Sys] [User]        [Summary]         [T40] [T41] [T42] ... [T53]        │
│    ↓     ↓               ↓               ↓    ↓    ↓         ↓            │
│   Full  Full         Compressed       Full Full Full       Full           │
│                        (from masked            (M=10 tail)                  │
│                         turns 1-33)                                         │
│                                                                             │
│                                    │                                        │
│                                    ▼                                        │
│                                                                             │
│   Continue: Masking on new turns, summarize when needed                     │
│                                                                             │
│   Result: Bounded context + Immediate savings + No elongation               │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Why Combine?

### Problems with Pure Observation Masking

```
Issue: Unbounded growth on very long trajectories

Turn:   100    200    300    400    500
Tokens: ~40K  ~80K  ~120K  ~160K  ~200K

Eventually hits context window limits
```

### Problems with Pure LLM Summarization

```
Issue 1: Warm-up penalty
Turns 1-31: No compression (accumulating)
Turn 32+:   Finally get benefits

Issue 2: Trajectory elongation
Summaries cause agents to run 15-18% longer
Eroding efficiency gains

Issue 3: Summary overhead
5-7% additional API cost from summarization calls
```

### The Hybrid Solution

| Problem | Solution |
|---------|----------|
| Warm-up penalty | Use masking immediately (no warm-up) |
| Unbounded growth | Switch to summarization at threshold |
| Trajectory elongation | Minimize summarization frequency |
| Summary overhead | Defer summarization as long as possible |

## Implementation

### Key Parameters

| Parameter | Symbol | Value | Description |
|-----------|--------|-------|-------------|
| Masking window | **W** | 10 | Recent turns visible (masking phase) |
| Summarize at | **N** | 43 | Turns before first summary |
| Tail window | **M** | 10 | Recent turns visible (summary phase) |

**Why N = 43?**

Research determined this value by matching context accumulation:
```
At N=43 with masking:
  - Context accumulated ≈ 30K tokens
  
This matches:
  - Raw agent at N=21 ≈ 30K tokens
  
Ensures fair comparison and optimal deferral
```

### Algorithm

```python
class HybridContextManager:
    """
    Hybrid context management: Masking by default, 
    summarization as last resort.
    """
    
    def __init__(
        self,
        summarizer_llm,
        masking_window_w: int = 10,
        summarize_at_n: int = 43,
        tail_window_m: int = 10
    ):
        self.summarizer = summarizer_llm
        self.w = masking_window_w
        self.n = summarize_at_n
        self.m = tail_window_m
        
        self.trajectory = []
        self.phase = "masking"  # or "summarizing"
        self.summaries = []
        self.last_summary_idx = 0
    
    def add_turn(self, reasoning: str, action: str, observation: str):
        """Add a new turn."""
        self.trajectory.append({
            'turn': len(self.trajectory),
            'reasoning': reasoning,
            'action': action,
            'observation': observation
        })
        
        # Check if we should switch to summarization
        self._check_phase_transition()
    
    def _check_phase_transition(self):
        """Switch from masking to summarization at threshold."""
        if self.phase == "masking" and len(self.trajectory) >= self.n:
            # Create first summary
            self._create_summary()
            self.phase = "summarizing"
    
    def _create_summary(self):
        """Generate summary of accumulated trajectory."""
        # When summarizing from masking phase, use UNMASKED turns
        # (pass full observations to summarizer)
        turns_to_summarize = self.trajectory[self.last_summary_idx : -self.m]
        
        # Generate summary using full context
        summary = self._generate_summary(turns_to_summarize)
        
        self.summaries.append({
            'end_turn': len(self.trajectory) - self.m,
            'content': summary
        })
        self.last_summary_idx = len(self.trajectory) - self.m
    
    def get_context(self, system_prompt: str, user_prompt: str) -> str:
        """Generate context based on current phase."""
        
        if self.phase == "masking":
            # Use observation masking
            return self._get_masked_context(system_prompt, user_prompt)
        else:
            # Use summarization + masking on recent
            return self._get_summarized_context(system_prompt, user_prompt)
    
    def _get_masked_context(self, system_prompt, user_prompt) -> str:
        """Apply observation masking."""
        parts = [system_prompt, user_prompt]
        
        current_turn = len(self.trajectory)
        
        for turn in self.trajectory:
            turns_ago = current_turn - turn['turn']
            
            # Mask old observations
            if turns_ago > self.w:
                observation = "[Observation omitted]"
            else:
                observation = turn['observation']
            
            parts.append(f"""
Turn {turn['turn']}:
Reasoning: {turn['reasoning']}
Action: {turn['action']}
Observation: {observation}
""")
        
        return "\n".join(parts)
    
    def _get_summarized_context(self, system_prompt, user_prompt) -> str:
        """Apply summarization + masking on tail."""
        parts = [system_prompt, user_prompt]
        
        # Add latest summary
        if self.summaries:
            latest = self.summaries[-1]
            parts.append(f"\n[Summary through turn {latest['end_turn']}:\n{latest['content']}\n]")
        
        # Add tail turns with masking applied
        tail_start = max(0, len(self.trajectory) - max(self.w, self.m))
        current_turn = len(self.trajectory)
        
        for turn in self.trajectory[tail_start:]:
            turns_ago = current_turn - turn['turn']
            
            if turns_ago > self.w:
                observation = "[Observation omitted]"
            else:
                observation = turn['observation']
            
            parts.append(f"""
Turn {turn['turn']}:
Reasoning: {turn['reasoning']}
Action: {turn['action']}
Observation: {observation}
""")
        
        return "\n".join(parts)
    
    def maybe_create_summary(self):
        """Create additional summaries in summarizing phase."""
        if self.phase != "summarizing":
            return
        
        turns_since_summary = len(self.trajectory) - self.last_summary_idx
        
        # Create summary every N turns after the first
        if turns_since_summary >= self.n:
            self._create_summary()
```

### Visual Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    HYBRID ALGORITHM FLOW                        │
│                                                                 │
│  Start                                                          │
│    │                                                            │
│    ▼                                                            │
│  ┌─────────────────────┐                                       │
│  │ Phase: MASKING      │                                       │
│  │ Use M=10 masking    │                                       │
│  │ No summarization    │                                       │
│  └──────────┬──────────┘                                       │
│             │                                                   │
│    ┌────────▼────────┐                                          │
│    │ Turn < N (43)?  │                                          │
│    └────────┬────────┘                                          │
│         │           │                                            │
│        Yes          No                                           │
│         │           │                                            │
│         │           ▼                                            │
│         │    ┌─────────────────────┐                             │
│         │    │ Create summary of   │                             │
│         │    │ accumulated turns   │                             │
│         │    └──────────┬──────────┘                             │
│         │               │                                        │
│         │               ▼                                        │
│         │    ┌─────────────────────┐                             │
│         │    │ Phase: SUMMARIZING  │                             │
│         └───▶│ Mask recent M=W=10  │                             │
│              │ Summarize at N=43   │                             │
│              └──────────┬─────────┘                             │
│                         │                                       │
│              ┌──────────▼──────────┐                           │
│              │ Turns since last    │                           │
│              │ summary >= N?       │                           │
│              └──────────┬──────────┘                           │
│                    │           │                               │
│                   Yes          No                              │
│                    │           │                               │
│                    ▼           │                               │
│         ┌────────────────┐    │                               │
│         │ Create summary │────┘                               │
│         └────────────────┘                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Performance Results

### Cost Reduction (Qwen3-Coder 480B on SWE-bench Verified-50)

| Strategy | Instance Cost | vs Raw | vs Masking | vs Summary |
|----------|---------------|--------|------------|------------|
| Raw Agent | $1.29 | 100% | — | — |
| Observation Masking | $0.61 | 47% | — | — |
| LLM Summarization | $0.64 | 50% | +5% | — |
| **Hybrid** | **$0.57** | **44%** | **-7%** | **-11%** |

### Solve Rate Improvement

| Strategy | Solve Rate | vs Raw | vs Masking | vs Summary |
|----------|------------|--------|------------|------------|
| Raw Agent | 53.4% | — | — | — |
| Observation Masking | 54.8% | +1.4pp | — | — |
| LLM Summarization | 53.8% | +0.4pp | -1.0pp | — |
| **Hybrid** | **57.4%** | **+4.0pp** | **+2.6pp** | **+3.6pp** |

### Savings at Scale

| Benchmark Size | vs Masking Savings | vs Summary Savings |
|--------------|-------------------|-------------------|
| 50 instances | $2.00 | $3.50 |
| 500 instances | $20.00 | $35.00 |
| 1000 instances | $40.00 | $70.00 |

## Why It Works

### 1. Immediate Cost Reduction

Unlike pure summarization which waits N+M=31 turns:
```
Hybrid starts masking at turn 11 (M+1)
→ Savings begin 20 turns earlier
→ Realizes cost benefits on shorter trajectories
```

### 2. Bounded Context for Long Trajectories

Unlike pure masking which grows indefinitely:
```
Hybrid switches to summarization at turn 43
→ Context becomes bounded
→ Can handle infinite trajectories
```

### 3. Minimized Trajectory Elongation

By deferring summarization:
```
Pure summary: Every 21 turns → More summaries → More elongation
Hybrid: First at 43, then every 43 → Fewer summaries → Less elongation
```

### 4. Reduced Summary Overhead

Fewer summary calls = lower API costs:
```
Pure summary: Summary every 21 turns
Hybrid: First summary at 43, then every 43
→ ~50% fewer summarization calls
→ Significant cost savings
```

### 5. Better Information Flow

```
Pure masking: Old observations completely lost
Pure summary: Old observations compressed
Hybrid: Recent observations visible + Old compressed
→ Best of both worlds
```

## Design Considerations

### Hyperparameter Selection

The choice of N=43 is critical. Research tested alternatives:

| Configuration | Cost | Solve Rate | Notes |
|---------------|------|------------|-------|
| N=21, M=W=10 (naive) | Higher | Similar | KV cache inefficiency |
| **N=43, M=W=10 (designed)** | **Lowest** | **Best** | **Optimal** |

**Why N=21 failed:**
- Compounding KV cache inefficiencies
- Cost overhead from too-frequent summarization
- Didn't properly defer summarization

**Why N=43 succeeds:**
- Matches context accumulation with raw agent at N=21
- Properly defers summarization
- Avoids cache inefficiencies

### Scaffold Adaptation

Different agent frameworks may need tuning:

| Scaffold | Recommended W/M | N | Rationale |
|----------|-----------------|---|-----------|
| SWE-agent | 10 | 43 | Optimal from experiments |
| OpenHands | 58 | ~80 | Retains retry turns |

## Advantages Over Pure Strategies

| Advantage | vs Masking | vs Summary |
|-----------|-----------|------------|
| **Cost** | ✅ 7% cheaper | ✅ 11% cheaper |
| **Solve Rate** | ✅ +2.6pp | ✅ +3.6pp |
| **Bounded Context** | ✅ Yes | ✅ Yes |
| **No Warm-up** | ✅ Yes | ✅ Yes |
| **Less Elongation** | Same | ✅ Better |
| **Lower Overhead** | Same | ✅ Better |

## Limitations

### 1. Complexity

More complex than either pure strategy:
- Two-phase logic
- Phase transition handling
- More parameters to tune

### 2. Parameter Sensitivity

Naive parameter choices (N=21, M=W=10) actually degrade performance:
- Requires careful hyperparameter selection
- Scaffold-specific tuning needed

### 3. Diminishing Returns

On short trajectories (< 50 turns), behaves like masking:
- Benefits only realized on longer tasks
- Overhead of summary logic not justified

## When to Use

### ✅ Ideal For

- **Production deployments** requiring maximum efficiency
- **Long-horizon tasks** (50+ turns typical)
- **Cost-sensitive applications** where every % matters
- **Mixed trajectory lengths** (some short, some long)

### ⚠️ Not Ideal For

- **Simple implementations** (use masking alone)
- **Very short tasks** (< 30 turns, no benefit)
- **Research experiments** (complexity not justified)

## Production Implementation

```python
class ProductionHybridContextManager:
    """
    Production-ready hybrid context management.
    Includes optimizations for real-world deployment.
    """
    
    def __init__(
        self,
        agent_llm,
        summarizer_llm,
        config: dict = None
    ):
        self.agent_llm = agent_llm
        self.summarizer = summarizer_llm
        
        # Scaffold-specific defaults
        self.config = config or {
            'masking_window': 10,
            'summarize_at': 43,
            'tail_window': 10,
            'max_trajectory_length': 250
        }
        
        # State
        self.trajectory = []
        self.summaries = []
        self.phase = 'masking'
        self.last_summary_idx = 0
        
        # Metrics
        self.metrics = {
            'masking_turns': 0,
            'summarizing_turns': 0,
            'summaries_created': 0,
            'tokens_saved': 0
        }
    
    def run_agent(self, task: str, max_turns: int = None) -> dict:
        """Run agent with hybrid context management."""
        max_turns = max_turns or self.config['max_trajectory_length']
        
        system_prompt = self._get_system_prompt()
        
        for turn in range(max_turns):
            # Get context based on phase
            context = self._get_context(system_prompt, task)
            
            # Generate next action
            reasoning, action = self.agent_llm.generate(context)
            
            # Execute
            observation = self._execute(action)
            
            # Store
            self._add_turn(reasoning, action, observation)
            
            # Check completion
            if self._is_complete(reasoning):
                return self._build_result(completed=True)
        
        return self._build_result(completed=False, hit_limit=True)
    
    def _get_context(self, system_prompt: str, user_prompt: str) -> str:
        """Get appropriately masked/summarized context."""
        
        if self.phase == 'masking':
            return self._apply_masking(system_prompt, user_prompt)
        else:
            return self._apply_hybrid(system_prompt, user_prompt)
    
    def _apply_masking(self, system_prompt: str, user_prompt: str) -> str:
        """Apply observation masking."""
        # ... implementation ...
        pass
    
    def _apply_hybrid(self, system_prompt: str, user_prompt: str) -> str:
        """Apply masking + summarization."""
        # ... implementation ...
        pass
    
    def get_efficiency_report(self) -> dict:
        """Generate efficiency analysis."""
        return {
            'phases': {
                'masking': self.metrics['masking_turns'],
                'summarizing': self.metrics['summarizing_turns']
            },
            'summaries': {
                'count': self.metrics['summaries_created'],
                'frequency': len(self.trajectory) / max(1, self.metrics['summaries_created'])
            },
            'savings': {
                'tokens': self.metrics['tokens_saved'],
                'estimated_cost': self.metrics['tokens_saved'] * 0.00001
            },
            'efficiency_score': self._calculate_efficiency_score()
        }
```

## Next Steps

- **[Observation Masking](01-observation-masking.md)** - Phase 1 implementation
- **[LLM Summarization](02-llm-summarization.md)** - Phase 2 implementation
- **[Performance Results](../experiments/02-performance-results.md)** - Full benchmark data
- **[Future Work](../challenges/02-future-work.md)** - Potential improvements
