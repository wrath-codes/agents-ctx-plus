# Observation Masking

## Overview

Observation masking is a simple but remarkably effective context management strategy that selectively hides old environment observations while preserving the agent's reasoning chain. It's the dominant approach for cost-efficient context management in software engineering agents.

## Core Concept

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         OBSERVATION MASKING FLOW                           │
│                                                                            │
│   Before Masking:                                                          │
│   [Sys] [User] [T1] [T2] [T3] [T4] [T5] [T6] [T7] [T8] [T9] [T10]...      │
│    ↓     ↓     ↓    ↓    ↓    ↓    ↓    ↓    ↓    ↓    ↓    ↓            │
│   Full  Full  Full Full Full Full Full Full Full Full Full Full          │
│                                                                            │
│                                    │                                       │
│                                    ▼                                       │
│                                                                            │
│   After Masking (M=10, turn 15):                                           │
│   [Sys] [User] [T1] [T2] [T3] [T4] [T5] [T6] [T7] [T8] [T9] [T10] [T11]   │
│    ↓     ↓     ↓    ↓    ↓    ↓    ↓    ↓    ↓    ↓    ↓    ↓     ↓        │
│   Full  Full  Mask Mask Mask Mask Mask Mask Mask Mask Mask Mask Full     │
│                        ▲                                      ↑           │
│                        │                                      │            │
│              Placeholders (hidden)                    Most recent M=10     │
│              "[Observation omitted                   fully visible         │
│               for brevity]"                                                  │
│                                                                            │
│   Result: ~50% token reduction, reasoning chain intact                     │
└────────────────────────────────────────────────────────────────────────────┘
```

## How It Works

### The Masking Function

Given a trajectory at turn t-1:
```
τ_{t-1} = (sys_prompt, user_prompt, (r_1, a_1, o_1), ..., (r_{t-1}, a_{t-1}, o_{t-1}))
```

The masking function produces:
```
τ'_{t-1} = (sys_prompt, user_prompt, (r_1, a_1, o'_1), ..., (r_{t-1}, a_{t-1}, o'_{t-1}))

where o'_i = {
    placeholder_i    if i < t - M    (masked)
    o_i             if i >= t - M    (visible)
}
```

### Example Placeholders

Common placeholder texts:
```
"[Output omitted for brevity]"
"[Previous 50 lines omitted]"
"[Observation hidden - see turn X for full output]"
"[Tool output truncated]"
```

### What Gets Preserved

| Component | Treatment | Rationale |
|-----------|-----------|-----------|
| System prompt | Always visible | Task definition |
| User prompt | Always visible | Initial requirements |
| Reasoning (r) | Always visible | Decision chain |
| Actions (a) | Always visible | What was attempted |
| Observations (o) | Masked if old | Often verbose, less critical |

## Implementation

### Algorithm

```python
def apply_observation_masking(trajectory, current_turn, window_size_m):
    """
    Apply observation masking to trajectory.
    
    Args:
        trajectory: List of turns (reasoning, action, observation)
        current_turn: Current turn number t
        window_size_m: Number of recent turns to keep visible
    
    Returns:
        Masked trajectory
    """
    masked_trajectory = []
    
    for turn_idx, (reasoning, action, observation) in enumerate(trajectory):
        # Keep system and user prompts always visible
        if turn_idx < 2:  # sys_prompt, user_prompt
            masked_trajectory.append((reasoning, action, observation))
            continue
        
        # Calculate if this turn should be masked
        turns_ago = current_turn - turn_idx
        
        if turns_ago > window_size_m:
            # Mask the observation
            masked_observation = f"[Turn {turn_idx} observation omitted for brevity]"
            masked_trajectory.append((reasoning, action, masked_observation))
        else:
            # Keep fully visible
            masked_trajectory.append((reasoning, action, observation))
    
    return masked_trajectory
```

### Pseudocode for Agent Loop

```
FUNCTION AgentLoop(task, max_turns, window_size_m):
    trajectory = [system_prompt, user_prompt]
    
    FOR turn = 1 TO max_turns:
        // Apply masking before sending to LLM
        masked_trajectory = ApplyMasking(trajectory, turn, window_size_m)
        
        // Generate next action
        (reasoning, action) = LLM.Generate(masked_trajectory)
        
        // Execute action in environment
        observation = Environment.Execute(action)
        
        // Store full turn (unmasked) for future masking
        trajectory.Append((reasoning, action, observation))
        
        // Check completion
        IF IsComplete(trajectory, task):
            BREAK
    
    RETURN ExtractSolution(trajectory)
```

## Key Parameter: Window Size (M)

The window size **M** determines how many recent turns remain fully visible.

### Optimal Value

Research found **M = 10** is the sweet spot:

| Window Size | Solve Rate | Cost | Notes |
|-------------|------------|------|-------|
| M = 5 | Lower | Lower | Too aggressive, loses context |
| **M = 10** | **Optimal** | **Low** | **Best balance** |
| M = 20 | Similar | Higher | Diminishing returns |

### Why M = 10 Works

1. **Recent context matters most** - SE tasks often require only recent file contents
2. **Reasoning chain intact** - Can trace agent's decision path
3. **Actions visible** - Know what was attempted
4. **Not too aggressive** - Doesn't prematurely hide useful observations

### Tuning Considerations

| Factor | Recommendation |
|--------|----------------|
| Task complexity | Complex tasks may need larger M |
| Tool verbosity | More verbose tools → smaller M acceptable |
| Agent scaffold | OpenHands needs larger M than SWE-agent |
| Turn limit | Longer trajectories → consider hybrid |

## Why It's So Effective

### The 84% Rule

In SE agent trajectories:
```
┌────────────────────────────────────────────┐
│           Token Distribution               │
│                                            │
│   Observations  ████████████████████  84%  │
│   Reasoning     ██                    8%   │
│   Actions       ██                    8%   │
│                                            │
└────────────────────────────────────────────┘
```

By masking only observations, we remove the bulk of context with minimal information loss.

### Preserved vs. Lost

**Preserved (always visible)**:
- What the agent was thinking (reasoning)
- What the agent tried to do (actions)
- Task requirements (system/user prompts)

**Masked (hidden)**:
- Old file contents (can re-read if needed)
- Old test outputs (usually not relevant)
- Old search results (agent already processed)

### Cost Impact

```
Raw trajectory at turn 100:     ~180,000 tokens
Masked trajectory (M=10):       ~45,000 tokens
                                 ─────────────
Reduction:                      ~75% fewer tokens
```

## Integration with Linear Hashing

The masked trajectory is stored in a linear hash table for efficient retrieval:

```
Storage Layout:
┌─────────────────────────────────────────────────────┐
│              Linear Hash Table                     │
│                                                     │
│   Bucket 0: [Page 0] → [Page 1] → ...              │
│   Bucket 1: [Page 0] → ...                         │
│   ...                                               │
│                                                     │
│   Each page (4KB) stores:                          │
│   ┌─────────────────────────────────────────────┐  │
│   │ Header: record count, next page pointer     │  │
│   ├─────────────────────────────────────────────┤  │
│   │ Records: (turn_id, reasoning, action, obs)   │  │
│   │  - Recent: full observation                 │  │
│   │  - Old: placeholder reference               │  │
│   └─────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

See [Linear Hashing](../../document-store/architecture/02-linear-hashing.md) for storage details.

## Advantages

### 1. Simplicity

```python
# Core masking logic is just a few lines
def mask_observations(turns, current_turn, m=10):
    return [
        (r, a, "[omitted]" if i < current_turn - m else o)
        for i, (r, a, o) in enumerate(turns)
    ]
```

No additional LLM calls, no complex summarization prompts.

### 2. No Warm-up Period

Unlike LLM summarization which needs N+M turns before first compression, masking starts working at turn M+1:

```
Turn 1-10:  Building up window
Turn 11+:   Masking active, cost reduction begins immediately
```

### 3. Deterministic

Same trajectory always produces same masked result. No variability from LLM-generated summaries.

### 4. Fastest Cost Reduction

No summary generation overhead. Every token saved is a direct cost reduction.

### 5. Reasoning Chain Intact

The agent's thought process is never hidden. Easy to debug and understand agent behavior.

## Limitations

### 1. Unbounded Growth

```
Context size with masking (M=10):

Turns:    10    50    100   200   500   1000
Tokens:  ~5K  ~15K  ~30K  ~60K  ~150K  ~300K

Still grows linearly, just slower.
```

For extremely long trajectories, context window limits may still be reached.

### 2. No Semantic Compression

Old turns are either fully visible or fully masked. No middle ground of compressed meaning.

### 3. Potential Context Loss

If the agent needs to reference old file contents, they're gone (unless agent re-reads the file).

### 4. Scaffold-Specific Tuning

Different agent frameworks need different M values:
- SWE-agent: M=10 optimal
- OpenHands: M=58 needed (retains retry turns)

## When to Use

### ✅ Ideal For

- Cost-sensitive production deployments
- Short to medium trajectories (< 100 turns)
- When simplicity is valued
- When recent context is most relevant
- Initial agent deployments

### ⚠️ Not Ideal For

- Extremely long-running agents (> 500 turns)
- When old context semantic meaning is critical
- When bounded context is strictly required

## Open Source Implementations

| Framework | Implementation | Window Size |
|-----------|----------------|-------------|
| SWE-agent | `sweagent/context.py` | M=10 (configurable) |
| SWE-Search | Built-in masking | Configurable |
| Custom | Easy to implement | User-defined |

## Code Example: Full Implementation

```python
class ObservationMaskingContextManager:
    """Simple observation masking for LLM agents."""
    
    def __init__(self, window_size_m: int = 10, placeholder: str = None):
        self.m = window_size_m
        self.placeholder = placeholder or "[Observation omitted for brevity]"
        self.trajectory = []
    
    def add_turn(self, reasoning: str, action: str, observation: str):
        """Add a new turn to the trajectory (stored unmasked)."""
        self.trajectory.append({
            'turn': len(self.trajectory) + 1,
            'reasoning': reasoning,
            'action': action,
            'observation': observation,
            'timestamp': time.time()
        })
    
    def get_masked_context(self, system_prompt: str, user_prompt: str) -> str:
        """Generate masked context for LLM consumption."""
        context_parts = [system_prompt, user_prompt]
        
        current_turn = len(self.trajectory)
        
        for turn_data in self.trajectory:
            turn_num = turn_data['turn']
            turns_ago = current_turn - turn_num
            
            # Always show reasoning and action
            reasoning = turn_data['reasoning']
            action = turn_data['action']
            
            # Mask observation if older than window
            if turns_ago > self.m:
                observation = self.placeholder
            else:
                observation = turn_data['observation']
            
            context_parts.append(f"""
Turn {turn_num}:
Reasoning: {reasoning}
Action: {action}
Observation: {observation}
""")
        
        return "\n".join(context_parts)
    
    def estimate_token_savings(self) -> dict:
        """Estimate tokens saved by masking."""
        total_obs_tokens = sum(
            len(t['observation'].split()) 
            for t in self.trajectory
        )
        
        visible_turns = min(len(self.trajectory), self.m)
        masked_turns = len(self.trajectory) - visible_turns
        
        # Assume masked placeholder is ~5 tokens vs ~500 for full observation
        placeholder_tokens = 5
        avg_obs_tokens = total_obs_tokens / len(self.trajectory) if self.trajectory else 0
        
        saved_tokens = masked_turns * (avg_obs_tokens - placeholder_tokens)
        
        return {
            'total_obs_tokens': total_obs_tokens,
            'masked_turns': masked_turns,
            'visible_turns': visible_turns,
            'estimated_saved_tokens': saved_tokens,
            'reduction_percent': (saved_tokens / total_obs_tokens * 100) if total_obs_tokens else 0
        }


# Usage Example
manager = ObservationMaskingContextManager(window_size_m=10)

# Agent loop
for turn in range(1, max_turns + 1):
    # Get masked context for LLM
    context = manager.get_masked_context(system_prompt, user_prompt)
    
    # Generate next action
    reasoning, action = llm.generate(context)
    
    # Execute and observe
    observation = environment.execute(action)
    
    # Store full turn
    manager.add_turn(reasoning, action, observation)
    
    # Check if done
    if is_complete(reasoning):
        break

# Analyze savings
savings = manager.estimate_token_savings()
print(f"Saved {savings['estimated_saved_tokens']} tokens ({savings['reduction_percent']:.1f}%)")
```

## Next Steps

- **[LLM Summarization](02-llm-summarization.md)** - The complex alternative
- **[Hybrid Approach](03-hybrid-approach.md)** - Combining strategies
- **[Performance Results](../experiments/02-performance-results.md)** - Empirical validation
- **[Trajectory Elongation](../experiments/03-trajectory-elongation.md)** - Why masking wins
