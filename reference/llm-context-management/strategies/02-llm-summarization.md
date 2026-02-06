# LLM Summarization

## Overview

LLM summarization is a more sophisticated context management strategy that uses a separate LLM (or the same model in a different role) to compress old trajectory turns into semantic summaries. This approach aims to preserve the meaning of past interactions while dramatically reducing token count.

Used by prominent agents including **OpenHands** and **Cursor**.

## Core Concept

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       LLM SUMMARIZATION FLOW                                │
│                                                                             │
│   Trajectory Before Summarization (Turn 50):                                │
│   [Sys] [User] [T1] [T2] [T3] ... [T40] [T41] [T42] [T43] [T44] [T45]      │
│    ↓     ↓     ↓    ↓    ↓       ↓    ↓    ↓    ↓    ↓    ↓               │
│   Full  Full  Full Full Full     Full Full Full Full Full Full             │
│                                                                             │
│                                    │                                        │
│                                    ▼                                        │
│                                                                             │
│   When turns_since_last_summary = N (e.g., 21):                            │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │ Summarizer LLM:                                                   │  │
│   │ Input: [T1] through [T35] (N=21 turns to summarize)               │  │
│   │ Output: "Summary: Agent identified bug in utils.py, attempted    │  │
│   │          fix by adding null check, tests still failing.          │  │
│   │          Need to investigate edge case in line 42."               │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│                                    ▼                                        │
│                                                                             │
│   Trajectory After Summarization:                                           │
│   [Sys] [User]        [Summary]          [T41] [T42] [T43] [T44] [T45]    │
│    ↓     ↓               ↓                 ↓    ↓    ↓    ↓    ↓            │
│   Full  Full         Compressed        Full Full Full Full Full            │
│                        Summary                                              │
│                                                                             │
│   ┌─────────────┐                                                           │
│   │ T1-T35      │  Replaced by summary (massive token reduction)           │
│   │ condensed   │                                                           │
│   │ to ~100     │  Recent M=10 turns (T36-T45) remain fully visible        │
│   │ tokens      │                                                           │
│   └─────────────┘                                                           │
│                                                                             │
│   Result: Bounded context growth (sawtooth pattern)                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

## How It Works

### Key Parameters

| Parameter | Symbol | Typical Value | Description |
|-----------|--------|---------------|-------------|
| Summarize window | **N** | 21 | Turns to accumulate before summarizing |
| Tail window | **M** | 10 | Recent turns kept fully visible |
| Warm-up | N + M | 31 | Turns before first summary |

### The Summarization Process

```
Step 1: Accumulate turns
  - Wait until we have N + M turns since last summary
  - N turns will be summarized
  - M turns remain visible as "tail"

Step 2: Prepare input for summarizer
  - Previous summary (or problem statement if first)
  - N turns to summarize: (reasoning, action, observation)

Step 3: Generate summary
  - Call summarizer LLM with special prompt
  - Output: Condensed representation of N turns

Step 4: Reconstruct trajectory
  - System prompt
  - User prompt
  - NEW summary
  - M recent turns (full)

Step 5: Continue agent loop
  - Context is now bounded
  - Repeat when next N turns accumulate
```

### Formal Algorithm

```python
def llm_summarize(trajectory, last_summary_idx, n, m, summarizer_llm):
    """
    Apply LLM summarization to trajectory.
    
    Args:
        trajectory: Full trajectory of turns
        last_summary_idx: Index of last summarized turn
        n: Number of turns to summarize
        m: Number of tail turns to keep visible
        summarizer_llm: LLM for generating summaries
    
    Returns:
        New summary, updated trajectory
    """
    current_turn = len(trajectory)
    
    # Check if we have enough turns to summarize
    turns_since_summary = current_turn - last_summary_idx
    
    if turns_since_summary < n + m:
        # Not enough turns yet, return as-is
        return None, trajectory
    
    # Identify turns to summarize (excluding tail)
    turns_to_summarize = trajectory[last_summary_idx : current_turn - m]
    tail_turns = trajectory[current_turn - m : current_turn]
    
    # Get previous summary context
    previous_summary = trajectory[last_summary_idx].get('summary', user_prompt)
    
    # Generate new summary
    new_summary = generate_summary(
        previous_summary=previous_summary,
        turns_to_summarize=turns_to_summarize,
        llm=summarizer_llm
    )
    
    # Reconstruct condensed trajectory
    condensed = [
        {'type': 'system', 'content': system_prompt},
        {'type': 'user', 'content': user_prompt},
        {'type': 'summary', 'content': new_summary, 'covers_turns': (last_summary_idx, current_turn - m)},
    ]
    
    # Add tail turns in full
    for turn in tail_turns:
        condensed.append({
            'type': 'turn',
            'reasoning': turn['reasoning'],
            'action': turn['action'],
            'observation': turn['observation']
        })
    
    return new_summary, condensed
```

## The Summarization Prompt

### Standard Prompt Structure

Based on OpenHands implementation (adapted for SWE-agent):

```
You are maintaining a context-aware state summary for an interactive agent.

You will be given a list of events corresponding to actions taken by the 
agent, and the most recent previous summary if one exists. Track:

USER_CONTEXT: (Preserve essential user requirements, goals, and 
                clarifications in concise form)
COMPLETED: (Tasks completed so far, with brief results)
PENDING: (Tasks that still need to be done)
CURRENT_STATE: (Current variables, data structures, or relevant state)

For code-specific tasks, also include:
CODE_STATE: (File paths, function signatures, data structures)
TESTS: (Failing cases, error messages, outputs)
CHANGES: (Code edits, variable updates)
DEPS: (Dependencies, imports, external calls)
VERSION_CONTROL_STATUS: (Repository state, current branch, PR status, 
                         commit history)

PRIORITIZE:
1. Adapt tracking format to match the actual task type
2. Capture key user requirements and goals
3. Distinguish between completed and pending tasks
4. Keep all sections concise and relevant

SKIP: Tracking irrelevant details for the current task type

Example formats:

For code tasks:
USER_CONTEXT: Fix FITS card float representation issue
COMPLETED: Modified mod_float() in card.py, all tests passing
PENDING: Create PR, update documentation
CODE_STATE: mod_float() in card.py updated
TESTS: test_format() passed
CHANGES: str(val) replaces f"{val:.16G}"
DEPS: None modified
VERSION_CONTROL_STATUS: Branch: fix-float-precision, 
                        Latest commit: a1b2c3d

<PREVIOUS_SUMMARY>
[Previous summary or problem statement]
</PREVIOUS_SUMMARY>

<TURN-0>
[Reasoning, Action, Observation]
</TURN-0>
...
<TURN-20>
[Reasoning, Action, Observation]
</TURN-20>
```

### Prompt Engineering Considerations

| Aspect | Approach | Rationale |
|--------|----------|-----------|
| Output format | Structured sections | Easier for agent to parse |
| Length guidance | "Keep concise" | Prevents verbose summaries |
| Domain adaptation | Code-specific sections | SE task relevance |
| Few-shot examples | 1-2 examples in prompt | Guide output style |
| Temperature | 0.0 (deterministic) | Consistent summaries |

## Bounded Context Growth

### The Sawtooth Pattern

```
Context Size Over Time:

Tokens
  ▲
  │
  │     ╱╲        ╱╲        ╱╲
  │    ╱  ╲      ╱  ╲      ╱  ╲
  │   ╱    ╲    ╱    ╲    ╱    ╲
  │  ╱      ╲  ╱      ╲  ╱      ╲
  │ ╱        ╲╱        ╲╱        ╲
  │╱                              ╲
  └────────────────────────────────────────▶ Turns
     10  20  30  40  50  60  70  80
     
     ↑   ↑   ↑   ↑   ↑   ↑   ↑   ↑
     │   │   │   │   │   │   │   │
     └───┴───┴───┴───┴───┴───┴───┘
         Summarization events (every N=21 turns)
```

### Mathematical Bounds

Maximum context size with summarization:
```
max_tokens ≈ system_prompt + user_prompt + summary_size + (M × avg_turn_size)

Where:
- summary_size ≈ 100-200 tokens (compressed N turns)
- avg_turn_size ≈ 500-1000 tokens (reasoning + action + observation)
- M = 10 (tail window)

Example:
max_tokens ≈ 1000 + 500 + 150 + (10 × 800) = 9,650 tokens
           (vs. potentially 100K+ tokens unbounded)
```

## Advantages

### 1. Bounded Context

Unlike observation masking, summarization ensures context never grows beyond a limit:

```
Maximum context size:
- Observation Masking: Unbounded (linear growth)
- LLM Summarization: Bounded (logarithmic growth)
```

This enables theoretically infinite trajectories.

### 2. Semantic Compression

Preserves meaning of old interactions, not just presence:

```
Raw turns (T1-T20): ~10,000 tokens
Summary: ~150 tokens
Compression ratio: ~98.5%

But meaning is preserved:
- What was tried
- What worked/failed
- Current state
- Next steps
```

### 3. Hierarchical Organization

Creates a natural hierarchy:
- Summary: High-level overview
- Tail turns: Recent detailed context
- System prompt: Task definition

### 4. Human-Readable

Summaries provide human-readable checkpoint of agent progress.

## Disadvantages and Challenges

### 1. Additional API Costs

Summary generation requires additional LLM calls:

| Model | Summary Cost per Instance | % of Total |
|-------|--------------------------|------------|
| Qwen3-32B | $0.0143 | 2.86% |
| Qwen3-Coder 480B | $0.0439 | 7.20% |
| Gemini 2.5 Flash | $0.0161 | 6.71% |

### 2. Cache Inefficiency

Summary calls process unique sequences, limiting cache reuse:

```
Normal agent calls:
- Similar system prompts → cache hits
- Reusable context chunks → cache hits

Summary calls:
- Each trajectory slice is unique
- Only system prompt caches
- Cache hit rate: Low
```

For APIs with cache pricing (e.g., Gemini: cache miss $0.15/M, cache hit $0.015/M), this matters significantly.

### 3. Warm-up Period

Must accumulate N + M turns before first summary:

```
N = 21, M = 10 → 31 turns before first compression

Problem:
- Short trajectories (< 31 turns): No benefit from summarization
- Medium trajectories: Brief period of compression
- Cost savings only realized on long trajectories
```

### 4. Trajectory Elongation

**The most significant disadvantage**: Summaries can cause agents to run longer.

See [Trajectory Elongation](../experiments/03-trajectory-elongation.md) for full analysis.

Brief explanation:
```
Raw/Masked signal: "Test failed, test failed, test failed"
→ Agent sees: "I'm stuck, should stop"

Summary signal: "Agent has been debugging the test failure"
→ Agent sees: "Progress being made, should continue"
```

### 5. Summary Quality Dependency

Effectiveness depends on summarizer LLM quality:
- May miss critical details
- May misinterpret agent actions
- Compression quality varies

## Open Source Implementations

| Framework | Implementation | Parameters |
|-----------|----------------|------------|
| OpenHands | `openhands/memory/` | N=21, M=10 (default) |
| Cursor | Proprietary | Unknown |
| Custom | Easy to implement | User-defined |

## Complete Implementation Example

```python
class LLMSummarizationContextManager:
    """LLM-based summarization for agent context management."""
    
    def __init__(
        self,
        summarizer_llm,
        summarize_window_n: int = 21,
        tail_window_m: int = 10,
        summary_temperature: float = 0.0
    ):
        self.summarizer = summarizer_llm
        self.n = summarize_window_n
        self.m = tail_window_m
        self.temp = summary_temperature
        
        self.trajectory = []
        self.summaries = []  # List of (turn_idx, summary)
        self.last_summary_idx = 0
        
        # Prompt template
        self.summary_prompt = """You are maintaining a context-aware state summary 
for an interactive agent working on software engineering tasks.

Previous context:
{previous_context}

Recent turns to summarize:
{turns_text}

Generate a concise summary tracking:
- USER_CONTEXT: Key requirements
- COMPLETED: What's been done
- PENDING: What remains
- CODE_STATE: Current code state
- TESTS: Test status
- CHANGES: Code modifications

Keep it under 200 tokens."""
    
    def add_turn(self, reasoning: str, action: str, observation: str):
        """Add a new turn to the trajectory."""
        self.trajectory.append({
            'turn': len(self.trajectory),
            'reasoning': reasoning,
            'action': action,
            'observation': observation
        })
    
    def maybe_summarize(self, force: bool = False) -> Optional[str]:
        """
        Check if summarization is needed and perform if so.
        
        Returns:
            New summary if created, None otherwise
        """
        current_turn = len(self.trajectory)
        turns_since_summary = current_turn - self.last_summary_idx
        
        # Check if we have enough turns
        if not force and turns_since_summary < self.n + self.m:
            return None
        
        # Prepare turns to summarize
        turns_to_summarize = self.trajectory[self.last_summary_idx : current_turn - self.m]
        
        # Get previous context
        if self.summaries:
            previous_context = self.summaries[-1][1]
        else:
            previous_context = self.user_prompt  # or system prompt
        
        # Format turns
        turns_text = "\n\n".join([
            f"Turn {t['turn']}:\nReasoning: {t['reasoning'][:200]}...\n"
            f"Action: {t['action']}\n"
            f"Observation: {t['observation'][:300]}..."
            for t in turns_to_summarize
        ])
        
        # Generate summary
        prompt = self.summary_prompt.format(
            previous_context=previous_context,
            turns_text=turns_text
        )
        
        summary = self.summarizer.generate(
            prompt,
            temperature=self.temp,
            max_tokens=250
        )
        
        # Store summary
        self.summaries.append((current_turn - self.m, summary))
        self.last_summary_idx = current_turn - self.m
        
        return summary
    
    def get_context(self, system_prompt: str, user_prompt: str) -> str:
        """Generate context for agent LLM."""
        self.user_prompt = user_prompt
        
        parts = [system_prompt, user_prompt]
        
        # Add latest summary
        if self.summaries:
            _, latest_summary = self.summaries[-1]
            parts.append(f"\n[Summary of turns {self.last_summary_idx}:\n{latest_summary}\n]")
        
        # Add tail turns (most recent M)
        tail_start = max(0, len(self.trajectory) - self.m)
        for turn in self.trajectory[tail_start:]:
            parts.append(f"""
Turn {turn['turn']}:
Reasoning: {turn['reasoning']}
Action: {turn['action']}
Observation: {turn['observation']}
""")
        
        return "\n".join(parts)
    
    def get_stats(self) -> dict:
        """Get summarization statistics."""
        total_turns = len(self.trajectory)
        num_summaries = len(self.summaries)
        
        # Estimate compression
        if self.summaries:
            avg_summary_length = sum(len(s[1].split()) for s in self.summaries) / num_summaries
            turns_per_summary = self.n
            raw_tokens_per_summary = turns_per_summary * 500  # estimate
            compression = 1 - (avg_summary_length / raw_tokens_per_summary)
        else:
            compression = 0
        
        return {
            'total_turns': total_turns,
            'num_summaries': num_summaries,
            'summaries': self.summaries,
            'estimated_compression': compression,
            'summary_cost': num_summaries * 0.02  # rough estimate
        }


# Usage Example
manager = LLMSummarizationContextManager(
    summarizer_llm=llm_client,
    summarize_window_n=21,
    tail_window_m=10
)

for turn in range(1, max_turns + 1):
    # Check if we should summarize
    manager.maybe_summarize()
    
    # Get context for agent
    context = manager.get_context(system_prompt, user_prompt)
    
    # Generate next action
    reasoning, action = agent_llm.generate(context)
    
    # Execute
    observation = environment.execute(action)
    
    # Store turn
    manager.add_turn(reasoning, action, observation)

# Analyze
stats = manager.get_stats()
print(f"Created {stats['num_summaries']} summaries")
print(f"Estimated compression: {stats['estimated_compression']:.1%}")
```

## Next Steps

- **[Observation Masking](01-observation-masking.md)** - The simple alternative
- **[Hybrid Approach](03-hybrid-approach.md)** - Combining strategies
- **[Trajectory Elongation](../experiments/03-trajectory-elongation.md)** - Critical analysis
- **[Performance Results](../experiments/02-performance-results.md)** - Empirical comparison
