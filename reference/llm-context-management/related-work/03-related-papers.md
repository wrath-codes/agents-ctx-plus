# Related Research on Agent Context Management

## Overview

This section surveys concurrent and prior research on context management for LLM agents, including complementary approaches, alternative strategies, and foundational work that informs the current understanding of agent efficiency.

**See also**: [Working Memory Hub](../cognitive/01-working-memory-hub.md) for the cognitive psychology foundations (Baddeley's working memory model) that underpin many of these approaches.

## Concurrent Research (2024-2025)

### DeepMiner: Training Deep Search Agents (Tang et al., 2025)

**Paper**: "Beyond Turn Limits: Training Deep Search Agents with Dynamic Context Window"

**Key Contribution**: Demonstrates that observation masking with sliding windows enables ~100 turns of sustained interaction within 32K context, achieving state-of-the-art on BrowseComp (33.5% accuracy, +20pp over previous best).

**Findings**:
- Sliding window observation masking works for deep research agents
- Reinforcement learning can train agents to work with masked context
- Dynamic context management is critical for multi-turn agents

**Connection to Current Research**: Independently validates observation masking effectiveness in a different domain (web search vs. software engineering).

### Improving Efficiency Through Trajectory Reduction (Xiao et al., 2025)

**Paper**: "Improving the Efficiency of LLM Agent Systems Through Trajectory Reduction"

**Key Contribution**: Proposes LLM-based trajectory reduction for SE agents.

**Notable Finding**: Their "Delete" baseline (removing full turns) is more efficient than summarization at comparable performance, supporting the observation that simple omission can outperform complex compression.

**Key Difference**: Does not compare against observation masking, missing the simpler baseline that outperforms.

### Scaling Multi-Turn RL with Summarization (Lu et al., 2025)

**Paper**: "Scaling LLM Multi-Turn RL with End-to-End Summarization-Based Context Management"

**Key Contribution**: Investigates LLM summarization for RL training of SE and Computer-Use Agents.

**Focus**: Training-time context management rather than inference-time efficiency.

**Limitation**: Does not evaluate against observation masking baselines.

### MEM1: Memory and Reasoning Synergy (Zhou et al., 2025)

**Paper**: "MEM1: Learning to Synergize Memory and Reasoning for Efficient Long-Horizon Agents"

**Key Contribution**: Dynamic state management for multi-hop QA and web navigation.

**Important Limitation**: 
- Benchmarks result in relatively short trajectories (hundreds of tokens)
- Does not compare to omission-based approaches
- SE agent trajectories are "orders of magnitude larger"

### ACON: Training-Time Compression Optimization (Kang et al., 2025)

**Paper**: "ACON: Optimizing Context Compression for Long-horizon LLM Agents" ([arXiv:2510.00615](https://arxiv.org/abs/2510.00615))

**Key Contribution**: Unified framework for optimizing compression guidelines through contrastive failure analysis. Alternating guideline optimization learns what to preserve by comparing trajectories where full context succeeds but compressed context fails.

**Findings**:
- 26–54% peak token reduction while preserving task performance
- Distillation into smaller models preserves 95%+ accuracy
- Small LMs improve by 20–46% with optimized compression (distraction mitigation)
- Contrastive feedback (success vs. failure pairs) outperforms failure-only analysis

**Connection to Current Research**: ACON validates the Complexity Trap's finding that compression quality matters more than quantity. The learned guidelines address trajectory elongation by preserving failure signals that generic summaries smooth over. See [full deep-dive](../strategies/05-acon-training-compression.md) for detailed analysis.

## Advanced Context Management Research (2025)

### H-MEM: Hierarchical Memory for Long-Term Reasoning (Sun & Zeng, 2025)

**Paper**: "Hierarchical Memory for High-Efficiency Long-Term Reasoning in LLM Agents" (arXiv:2507.22925)

**Key Contribution**: A multi-level memory organization based on semantic abstraction with index-based routing for efficient retrieval.

**Architecture**:
```
H-MEM Hierarchy:
┌─────────────────────────────────────────────────────────────────┐
│ Level 0: Raw Memory Entries                                      │
│   - Individual interactions, facts, observations                   │
│                                                                     │
│ Level 1: Semantic Abstractions                                     │
│   - Grouped by topic/concept                                       │
│   - Embedded with positional index encoding                        │
│                                                                     │
│ Level 2: High-Level Themes                                         │
│   - Cross-domain insights                                          │
│   - Strategic patterns                                               │
│                                                                     │
│ Retrieval: Index-based routing (no exhaustive similarity search)   │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Innovation**: Each memory vector contains a positional index encoding pointing to semantically related sub-memories in the next layer. During reasoning, an index-based routing mechanism enables efficient, layer-by-layer retrieval without performing exhaustive similarity computations.

**Results**: Evaluated on five task settings from the LoCoMo dataset, consistently outperforming five baseline methods including MemoryBank and A-MEM.

**Connection to Current Research**: H-MEM provides a principled approach to hierarchical compression mentioned in our future work section. The index-based routing offers a concrete implementation of semantic memory organization that could enhance the hybrid approach.

### HiAgent: Hierarchical Working Memory Management (Hu et al., ACL 2025)

**Paper**: "HiAgent: Hierarchical Working Memory Management for Solving Long-Horizon Agent Tasks with Large Language Model" (ACL 2025)

**Key Contribution**: Uses subgoals as memory chunks for hierarchical working memory management, achieving 2x success rate improvement and 35% reduction in context length.

**Mechanism**:
```
Standard Approach (Inefficient):
┌─────────────────────────────────────────────────────────────────┐
│ Working Memory:                                                  │
│ [Turn 1] Action: search("API docs") → Observation: <1000 tokens>│
│ [Turn 2] Action: read_file("config.py") → Observation: <500 tokens>│
│ [Turn 3] Action: edit_file("config.py") → Observation: <200 tokens>│
│ ... (all history retained, growing linearly)                     │
│ [Turn N] → Context explosion                                      │
└─────────────────────────────────────────────────────────────────────┘

HiAgent Approach (Hierarchical):
┌─────────────────────────────────────────────────────────────────┐
│ Working Memory:                                                  │
│ [Subgoal 1: "Find API configuration"]                             │
│   → Summary: "Found API key in config.py, base URL in env"      │
│                                                                     │
│ [Subgoal 2: "Implement authentication"]                           │
│   → Current: Full action-observation pairs (active subgoal)      │
│   → Action: edit_file("auth.py") → Observation: <400 tokens>     │
│                                                                     │
│ Result: Only current subgoal retains full detail                 │
│ Previous subgoals compressed to summaries                        │
└─────────────────────────────────────────────────────────────────────┘
```

**Algorithm**:
1. **Subgoal Generation**: Before actions, LLM formulates a subgoal as a milestone
2. **Action Execution**: Generate precise actions to accomplish the subgoal
3. **Completion Detection**: When subgoal fulfilled, summarize action-observation pairs
4. **Memory Update**: Replace detailed trajectory with subgoal-summary pair

**Results on Five Long-Horizon Tasks**:
| Metric | Standard | HiAgent | Improvement |
|--------|----------|---------|-------------|
| Success Rate | 21% | 42% | **+100%** |
| Progress Rate | 44% | 68% | +54% |
| Avg Steps | 18.2 | 14.4 | -21% |
| Context Length | 100% | 65% | **-35%** |
| Runtime | 100% | 81% | -19% |

**Key Insight**: "Employing subgoals to compartmentalize action-observation pairs can be conceptualized as a form of chunking methodology" — inspired by cognitive science principles (Miller, 1956; Newell et al., 1972).

**Connection to Current Research**: HiAgent implements hierarchical compression at the working memory level, complementing our hybrid approach which focuses on trajectory-level compression. Could be combined for multi-level efficiency.

### Re-TRAC: Recursive Trajectory Compression (Zhu et al., 2026)

**Paper**: "RE-TRAC: REcursive TRAjectory Compression for Deep Search Agents" (arXiv:2602.02486)

**Key Contribution**: Cross-trajectory exploration through structured state representations that enable iterative reflection and globally informed planning.

**Problem Addressed**: Traditional ReAct operates as independent trajectories:
```
Traditional ReAct (Isolated Trajectories):
┌─────────────────────────────────────────────────────────────────┐
│ Trajectory 1: [Attempt] → Fail (local optimum)                 │
│ Trajectory 2: [Attempt] → Fail (repeats same mistakes)           │
│ Trajectory 3: [Attempt] → Fail (no learning from previous)       │
│                                                                     │
│ Result: No cross-trajectory knowledge transfer                   │
│         Redundant exploration                                      │
│         Wasted computation                                         │
└─────────────────────────────────────────────────────────────────────┘
```

**Re-TRAC Solution** (Recursive State Representation):
```
Round 1: Execute → Compress → State Representation
┌─────────────────────────────────────────────────────────────────┐
│ State Representation Structure:                                  │
│ {                                                                  │
│   "accumulated_evidence": [...],                                 │
│   "unresolved_uncertainties": [...],                               │
│   "identified_failures": [...],                                    │
│   "forward_plan": [...],                                           │
│   "incomplete_branches": [...]                                     │
│ }                                                                  │
└─────────────────────────────────────────────────────────────────────┘

Round 2: State Representation + New Query → Execute
┌─────────────────────────────────────────────────────────────────┐
│ - Avoids previously failed paths                                 │
│ - Prioritizes unresolved uncertainties                           │
│ - Continues incomplete branches                                  │
│ - Globally informed planning                                       │
└─────────────────────────────────────────────────────────────────────┘
```

**Structured Compression Specification**:
| Component | Content | Purpose |
|-----------|---------|---------|
| Evidence | Verified facts, confirmed data | Build knowledge base |
| Uncertainties | Open questions, ambiguities | Guide exploration |
| Failures | Failed approaches with reasons | Avoid repetition |
| Plan | Forward-looking strategy | Guide next actions |
| Branches | Incomplete search paths | Resume exploration |

**Results**:
- Consistent 15-20% improvement over ReAct on BrowseComp across frontier LLMs
- RE-TRAC-30B-A3B achieves competitive performance against models 10x larger
- RE-TRAC-4B outperforms all baselines with <15B parameters
- **Monotonic reduction in tool calls and token usage across rounds** — progressively targeted exploration

**Connection to Current Research**: Re-TRAC addresses the "trajectory elongation" problem by enabling early stopping through better state tracking. The recursive compression approach validates the need for structured state management beyond simple masking or summarization.

### ACE: Agentic Context Engineering (Zhang et al., ICLR 2026)

**Paper**: "Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models" (arXiv:2510.04618)

**Key Contribution**: Treats contexts as evolving playbooks that accumulate, refine, and organize strategies through generation, reflection, and curation — preventing "context collapse" and "brevity bias."

**Problems Addressed**:
1. **Brevity Bias**: Traditional methods favor concise summaries that drop domain insights
2. **Context Collapse**: Iterative rewriting erodes details over time

```
Context Collapse Example (Traditional Methods):
┌─────────────────────────────────────────────────────────────────┐
│ Iteration 1: Comprehensive context (500 tokens)                  │
│   → Accuracy: 70%                                                │
│                                                                     │
│ Iteration 2: "Optimized" context (300 tokens) - drops details    │
│   → Accuracy: 68%                                                  │
│                                                                     │
│ Iteration 3: "Streamlined" context (150 tokens) - erodes key info│
│   → Accuracy: 57% (worse than baseline!)                         │
│                                                                     │
│ Result: Progressive degradation through aggressive compression   │
└─────────────────────────────────────────────────────────────────────┘
```

**ACE Solution** (Modular Generation + Curation):
```
ACE Framework:
┌─────────────────────────────────────────────────────────────────┐
│ 1. Generator: Creates candidate context additions                │
│    - New strategies                                               │
│    - Failure patterns                                               │
│    - Successful tactics                                             │
│                                                                     │
│ 2. Reflector: Evaluates and critiques                              │
│    - Validates factual accuracy                                     │
│    - Checks for redundancy                                          │
│    - Identifies gaps                                                │
│                                                                     │
│ 3. Curator: Maintains structured playbook                         │
│    - Incremental updates (not full rewrites)                       │
│    - Preserves detailed knowledge                                  │
│    - Organizes by strategy type                                    │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Mechanisms**:
- **Structured, incremental updates** that preserve detailed knowledge
- **No full-context rewriting** — appends to existing strategies
- **Execution feedback-driven** — doesn't require labeled supervision
- **Dual-mode operation**:
  - Offline: System prompt optimization
  - Online: Agent memory adaptation

**Results**:
| Setting | Baseline | ACE | Improvement |
|---------|----------|-----|-------------|
| Agent Tasks (AppWorld) | 51.9% | 59.5% | **+10.6%** |
| Domain Tasks (Finance) | 69.5% | 76.5% | **+8.6%** |
| Adaptation Latency | High | Low | Faster convergence |
| Rollout Cost | High | Low | More efficient |

**Notable Achievement**: On AppWorld leaderboard, ACE matches the top-1 ranked production-level agent (IBM-CUGA with GPT-4.1) while using a smaller open-source model (DeepSeek-V3.1).

**Connection to Current Research**: ACE validates that "adaptive thresholds" and "semantic triggers" from our future work are essential. The modular approach to context curation offers a principled way to implement real-time strategy switching.

### CASK: Context Adaptive Memory-Efficient LLM Inference (Mohammed et al., AAMAS 2025)

**Paper**: "Context Adaptive Memory-Efficient LLM Inference for Edge Multi-Agent Systems" (AAMAS 2025)

**Key Contribution**: Inference-time strategy combining dynamic sparse attention with adaptive KV-cache compression for 40% memory reduction and 20% speedup.

**Two-Component Architecture**:
```
CASK Architecture:
┌─────────────────────────────────────────────────────────────────┐
│ Component 1: Dynamic Sparse Attention                            │
│ ───────────────────────────────────                              │
│ • Mask Generation Module (MGM): Small vision transformer         │
│ • Meta-learned to derive sparse binary masks                     │
│ • Eliminates lower-importance token interactions                 │
│ • Reduces attention computation without architecture changes       │
│                                                                     │
│ Component 2: Adaptive KV-Cache Compression                         │
│ ────────────────────────────────────────                           │
│ • Tracks: recency, access frequency, attention allocation        │
│ • Saliency metrics for each key-value pair                       │
│ • Dynamic quantization: moderate salience → lower bit-width     │
│ • Pruning: low salience → removed                                  │
│ • Thresholds validated via runtime reconstruction loss             │
└─────────────────────────────────────────────────────────────────────┘
```

**Performance on LongBench**:
| Model | Score | Memory (GB) | Relative Inference Time |
|-------|-------|-------------|------------------------|
| LLaMA-3.2-90B-128k | 63.2 | 72 | 1.00x |
| H2O | 57.2 | **40** | 0.88x |
| StreamingLLMs | 37.1 | 40.8 | 0.88x |
| **CASK** | **61.5** | **44.5** | **0.77x** |

**Key Achievement**: Maintains 95%+ of baseline accuracy while cutting memory usage by up to 40% and boosting inference speed by 20%.

**Multi-Agent Systems Application**:
- Critical for MAS where multiple agents share/update contextual information
- Enables more agents or extended histories under tight GPU budgets
- Tested on vision-language agents for collaborative, multimodal contexts

**Connection to Current Research**: CASK provides concrete techniques for the "KV Cache Optimization" section in our future work. The saliency-based compression offers a principled approach to "adaptive thresholds" for context management.

### G-Memory: Tracing Hierarchical Memory for Multi-Agent Systems (Zhang et al., NeurIPS 2025)

**Paper**: "G-Memory: Tracing Hierarchical Memory for Multi-Agent Systems" (NeurIPS 2025, arXiv:2506.07398)

**Key Contribution**: Three-tier graph hierarchy (insight, query, interaction) for efficient context sharing in multi-agent systems.

**Problem with Current MAS Memory**:
```
Current MAS Memory Limitations:
┌─────────────────────────────────────────────────────────────────┐
│ 1. Overly simplistic — disregards nuanced inter-agent            │
│    collaboration trajectories                                     │
│                                                                     │
│ 2. Lacks cross-trial and agent-specific customization             │
│    (unlike expressive single-agent memory)                        │
│                                                                     │
│ Result: Poor self-evolution capability in multi-agent teams        │
└─────────────────────────────────────────────────────────────────────┘
```

**G-Memory Three-Tier Hierarchy**:
```
G-Memory Architecture (Inspired by Organizational Memory Theory):
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Tier 1: INSIGHT GRAPH (High-Level)                                │
│  ─────────────────────────────────                                 │
│  • Generalizable insights across trials                            │
│  • Strategic patterns and principles                               │
│  • Cross-domain knowledge                                          │
│  └── Nodes: Abstract concepts                                      │
│      Edges: Causal/associative relationships                       │
│                                                                     │
│  Tier 2: QUERY GRAPH (Mid-Level)                                   │
│  ────────────────────────────────                                    │
│  • Task-specific query patterns                                    │
│  • Problem decomposition strategies                                │
│  • Solution approaches                                             │
│  └── Nodes: Query types                                            │
│      Edges: Dependency relationships                               │
│                                                                     │
│  Tier 3: INTERACTION GRAPH (Fine-Grained)                           │
│  ─────────────────────────────────                                   │
│  • Condensed interaction trajectories                              │
│  • Agent-to-agent communication patterns                           │
│  • Execution details                                                 │
│  └── Nodes: Individual interactions                                │
│      Edges: Temporal/sequential relationships                      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Bi-Directional Memory Traversal**:
```
Query Processing:
┌─────────────────────────────────────────────────────────────────┐
│ New User Query                                                    │
│     │                                                             │
│     ▼                                                             │
│ ┌─────────────┐    Top-Down (General → Specific)                 │
│ │Insight Graph│ ─────────────────────────────────►              │
│ └─────────────┘    Retrieve high-level, generalizable insights   │
│     │                   ("What strategies work for this type?")    │
│     ▼                                                             │
│ ┌─────────────┐                                                   │
│ │Query Graph  │ ─────────────────────────────────►              │
│ └─────────────┘    Retrieve problem-specific approaches          │
│     │                   ("How was this handled before?")          │
│     ▼                                                             │
│ ┌─────────────┐    Bottom-Up (Specific → General)                │
│ │Interaction  │ ◄─────────────────────────────────               │
│ │   Graph     │    Retrieve fine-grained interaction details      │
│ └─────────────┘    ("What exactly was done last time?")          │
│     │                                                             │
│     ▼                                                             │
│ Combined Context → Agent Team Execution                           │
│     │                                                             │
│     ▼                                                             │
│ Hierarchy Evolution (assimilate new collaborative trajectories)    │
└─────────────────────────────────────────────────────────────────────┘
```

**Results**:
| Domain | Metric | Improvement |
|--------|--------|-------------|
| Embodied Action | Success Rate | **+20.89%** |
| Knowledge QA | Accuracy | **+10.12%** |

Tested across five benchmarks, three LLM backbones, and three popular MAS frameworks — without any modifications to the original frameworks.

**Connection to Current Research**: G-Memory provides a concrete implementation for the "Multi-Agent Context Sharing" section in our future work. The three-tier hierarchy offers a principled approach to balancing detail and compression across agent teams.

### SMART: Synergistic Multi-Agent Framework with Trajectory Learning (Yue et al., AAAI 2025)

**Paper**: "Synergistic Multi-Agent Framework with Trajectory Learning for Knowledge-Intensive Tasks" (AAAI 2025 Oral)

**Key Contribution**: Four specialized agents with Long-Short Trajectory Learning paradigm for knowledge-intensive tasks.

**Four-Agent Architecture**:
```
SMART Multi-Agent Framework:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Agent 1: Intent Parser                                            │
│  ─────────────────────                                             │
│  • Decomposes complex queries into sub-questions                   │
│  • Identifies required knowledge types                             │
│                                                                     │
│  Agent 2: Fact Locator                                             │
│  ─────────────────────                                             │
│  • Retrieves relevant documents/passages                           │
│  • Filters noise from retrieved content                              │
│                                                                     │
│  Agent 3: Reasoner                                                 │
│  ─────────────────────                                             │
│  • Performs multi-hop reasoning over facts                         │
│  • Generates candidate answers                                       │
│                                                                     │
│  Agent 4: Fact Checker                                             │
│  ─────────────────────                                             │
│  • Verifies factual consistency                                      │
│  • Detects hallucinations                                             │
│  • Provides feedback for refinement                                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Long-Short Trajectory Learning**:
```
Training Paradigm:
┌─────────────────────────────────────────────────────────────────┐
│ Stage 1: Short-Trajectory Learning                               │
│ ──────────────────────────────────                               │
│ • Individual agent training on single-turn tasks                   │
│ • Builds foundational capabilities                               │
│ • Agent masters its specific sub-trajectory action               │
│                                                                     │
│ Stage 2: Long-Trajectory Learning                                  │
│ ──────────────────────────────────                               │
│ • Multi-agent collaboration on complex tasks                       │
│ • Learns to coordinate across the full trajectory                  │
│ • Fine-grained execution maintained through trajectory tokens      │
│                                                                     │
│ Result: Synergistic collaboration + fine-grained execution         │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Innovation**: Unlike end-to-end multi-agent training which can collapse when one agent is missing, SMART's trajectory learning maintains performance flexibility while preserving collaboration benefits.

**Results on Five Knowledge-Intensive Tasks**:
- Outperforms knowledge internalization and knowledge enhancement baselines
- Superior to MMAgent (four independent agents coupled together)
- Extends beyond knowledge tasks to more complex scenarios

**Connection to Current Research**: SMART validates the importance of "trajectory quality metrics" from our future work. The specialized agent approach suggests that different context management strategies might be optimal for different agent roles.

### Curriculum Design for Trajectory-Constrained Agents (Tzannetos et al., NeurIPS 2025)

**Paper**: "Curriculum Design for Trajectory-Constrained Agent: Compressing Chain-of-Thought Tokens in LLMs" (NeurIPS 2025, arXiv:2511.02690)

**Key Contribution**: Curriculum learning strategy that gradually tightens trajectory constraints during training, enabling agents to incrementally master deployment requirements. Achieves **4.5× inference speedup** on consumer hardware through chain-of-thought token compression.

**Problem**: Training agents with strict deployment constraints (resource budgets, safety requirements) from the outset is difficult:
```
Standard Approach (Difficult):
┌─────────────────────────────────────────────────────────────────┐
│ Train from scratch with full constraints                         │
│ → High failure rate                                               │
│ → Poor sample efficiency                                          │
│ → Suboptimal final performance                                    │
└─────────────────────────────────────────────────────────────────────┘
```

**Curriculum Solution**:
```
CuRLTraC (Curriculum for RL with Trajectory Constraints):
┌─────────────────────────────────────────────────────────────────┐
│ Training Progression:                                            │
│                                                                     │
│ Phase 1: Loose Constraints (C_max)                               │
│   • Agent learns task fundamentals                               │
│   • Long trajectories allowed                                      │
│   • High success rate, builds confidence                           │
│                                                                     │
│ Phase 2: Medium Constraints (C_mid)                                │
│   • Tighter budget constraints                                     │
│   • Agent learns to be more efficient                              │
│   • Maintains performance with fewer tokens                        │
│                                                                     │
│ Phase 3: Strict Constraints (C_target)                             │
│   • Deployment-level constraints                                   │
│   • Agent operates efficiently under full restrictions           │
│   • Ready for deployment                                          │
│                                                                     │
│ Algorithm: Binary search for permissive cost budget at each      │
│ training step, gradually tightening as performance improves       │
└─────────────────────────────────────────────────────────────────────┘
```

**Application to LLM Token Compression**:
```
Chain-of-Thought Token Compression:
┌─────────────────────────────────────────────────────────────────┐
│ Standard LLM Reasoning:                                          │
│   "Let's think step by step..." → 500 tokens of reasoning        │
│   → Final answer                                                  │
│                                                                     │
│ Curriculum-Trained LLM:                                          │
│   Phase 1: Full reasoning (500 tokens)                           │
│   Phase 2: Compressed reasoning (250 tokens)                     │
│   Phase 3: Minimal reasoning (110 tokens)                        │
│   → Same answer quality                                           │
│                                                                     │
│ Result: 4.5× fewer tokens, massive inference speedup              │
│         while preserving answer accuracy                          │
└─────────────────────────────────────────────────────────────────────┘
```

**Theoretical Analysis**:
- Analyzes RL agent in binary-tree MDP
- Proves curriculum strategy accelerates training vs. baseline with constraints from outset
- Demonstrates sample efficiency gains through progressive difficulty

**Empirical Validation**:
- Binary-tree MDP (controlled environment)
- Multi-task navigation domain
- Math reasoning (two benchmarks: GSM8K, MATH)
- Both RL and LLM agents

**Connection to Current Research**: This paper directly addresses "adaptive thresholds" and "learned compression" from our future work. The curriculum approach provides a training-time method to achieve the efficiency gains we demonstrated at inference-time with masking.

## Training-Time and Inference Optimization Research (2025-2026)

### ACON: Agent Context Optimization (Kang et al., 2025)

**Paper**: "ACON: Agent Context Optimization via Alternating Guideline Optimization" (arXiv:2510.00615)

**Key Contribution**: A unified framework for optimally compressing environment observations and interaction histories into concise condensations, using natural language space optimization and distillation into smaller models.

**Architecture**:
```
ACON Compression Pipeline:
┌─────────────────────────────────────────────────────────────────┐
│ Phase 1: Alternating Guideline Optimization                       │
│ ──────────────────────────────────────────                        │
│ • Reward-first update: Compare success/failure trajectories       │
│ • Contrastive feedback: Capable LLM analyzes cases where          │
│   full context succeeds but compressed context fails              │
│ • Update compression guidelines accordingly                       │
│                                                                     │
│ Phase 2: Distillation                                              │
│ ──────────────────────                                            │
│ • Distill optimized LLM compressor into smaller models            │
│   (e.g., Qwen-14B)                                                │
│ • Reduce compression overhead for deployment                      │
│                                                                     │
│ Result: Learned compression guidelines that preserve               │
│ task-critical information while discarding noise                   │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Innovation**: Rather than hand-crafting compression rules, ACON *learns* what to compress by analyzing contrastive trajectory pairs — cases where full context succeeds but compressed context fails reveal exactly which information is critical.

**Results**:
| Metric | Baseline | ACON | Improvement |
|--------|----------|------|-------------|
| Peak Memory (tokens) | 100% | 46-74% | **-26% to -54%** |
| Task Accuracy (distilled) | — | 95%+ | Preserved |
| Smaller LM Performance | — | +46% | Enhanced |

**Connection to Current Research**: ACON directly addresses the Complexity Trap's finding that observations comprise ~84% of trajectory tokens. By learning compression guidelines rather than applying fixed masking, ACON represents a middle ground between simple observation masking and full LLM summarization. However, the training-time cost of alternating optimization could introduce its own complexity trap — the overhead of learning compression guidelines must be amortized over many inference runs.

### TTT-E2E: Test-Time Training for Long-Context (Tandon et al., 2025)

**Paper**: "End-to-End Test-Time Training for Long Context" (arXiv:2512.23675)  
**Detailed Documentation**: [TTT-E2E Strategy](../strategies/06-ttt-e2e-training.md)

**Key Contribution**: Reformulates long-context modeling as a continual learning problem, compressing context into model weights at test time via next-token prediction. Uses standard Transformer with sliding-window attention (k=8K) and mini-batch TTT (b=1K). Achieves 2.7× speedup at 128K context with constant inference latency.

**Architecture**:
```
TTT-E2E Architecture:
┌─────────────────────────────────────────────────────────────────┐
│ Standard Transformer (Baseline):                                  │
│   Context tokens → Self-attention → Output                       │
│   Cost: O(n²) attention, linear memory growth                    │
│                                                                     │
│ TTT-E2E (Continual Learning Formulation):                         │
│ ──────────────────────────────────────                            │
│ Training Phase:                                                    │
│   • Meta-learning: Train initialization θ₀ such that             │
│     test-time gradient updates from context produce               │
│     good representations                                          │
│   • Loss: End-to-end next-token prediction                       │
│                                                                     │
│ Inference Phase:                                                    │
│   • Sliding window attention (fixed size W)                       │
│   • Context beyond window → Gradient updates to θ                │
│   • Model "learns" context into its weights                       │
│   • Constant inference latency regardless of context length       │
│                                                                     │
│ Result: RNN-like efficiency with full-attention-like quality       │
└─────────────────────────────────────────────────────────────────────┘
```

**Results**:
| Context Length | Full Attention Latency | TTT-E2E Latency | Speedup |
|---------------|----------------------|-----------------|---------|
| 32K | 1.0x | 0.8x | 1.25x |
| 64K | 1.0x | 0.5x | 2.0x |
| 128K | 1.0x | 0.37x | **2.7x** |

**Key Insight**: Context management is reframed from a *storage problem* (how to fit tokens in a window) to a *learning problem* (how to absorb information into weights). This bypasses the linear cost growth of self-attention entirely.

**Connection to Current Research**: TTT-E2E represents a fundamentally different approach to the Complexity Trap. Rather than managing context within a fixed window (masking, summarization), it eliminates the window constraint altogether. However, the test-time gradient computation introduces its own cost, and it remains unclear whether the approach preserves the fine-grained action-observation details that SE agents need for accurate code editing.

### PLENA: Hardware-Software Co-Design for Agentic Inference (Wu et al., 2025)

> **Dedicated documentation**: See [PLENA Hardware](../hardware/01-plena-hardware.md) for comprehensive coverage including architecture diagrams, optimization pathways, and Complexity Trap connections.

**Paper**: "Combating the Memory Walls: Optimization Pathways for Long-Context Agentic LLM Inference" (arXiv:2509.09505)

**Key Contribution**: A Programmable Long-context Efficient Neural Accelerator that addresses the "bandwidth" and "capacity" memory walls in agentic LLM inference through custom hardware design.

**Architecture**:
```
PLENA Hardware-Software Co-Design:
┌─────────────────────────────────────────────────────────────────┐
│ Problem: Memory Walls in Agentic Inference                        │
│ ──────────────────────────────────────────                        │
│ 1. Bandwidth Wall: KV cache reads dominate memory bandwidth       │
│ 2. Capacity Wall: Agent contexts (e.g., full DOMs) exhaust        │
│    available memory                                                │
│                                                                     │
│ PLENA Solution (Three Components):                                 │
│                                                                     │
│ Component 1: Flattened Systolic Array                              │
│   • Tailored for "fat" GEMM operations (large inner dimensions)   │
│   • Common in long-context tasks where KV is massive              │
│   • 8.5x higher utilization than existing accelerators            │
│                                                                     │
│ Component 2: Asymmetric Quantization                               │
│   • Mixed data types and precisions                                │
│   • Weights: INT4/INT8                                              │
│   • Activations: FP16/BF16                                         │
│   • KV Cache: Adaptive precision (INT4-INT8 based on saliency)    │
│                                                                     │
│ Component 3: Native FlashAttention Support                         │
│   • Custom ISA instructions for tile-by-tile scheduling           │
│   • Fused attention pipeline avoids materialization overhead       │
│   • Enables in-accelerator KV cache management                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Results**:
| Platform | Throughput (rel.) | Utilization | Context Capacity |
|----------|------------------|-------------|-----------------|
| A100 GPU | 1.0x | Baseline | Standard |
| TPU v6e | 0.58x | Lower | Standard |
| **PLENA** | **2.24x** | **8.5x** | **Extended** |

**Connection to Current Research**: PLENA addresses the Complexity Trap at the hardware level. While observation masking and summarization reduce token counts (software optimization), PLENA increases the hardware's ability to handle large contexts efficiently. The two approaches are complementary — PLENA could process masked/summarized contexts even more efficiently, compounding savings. However, custom hardware adoption faces significant deployment barriers compared to software-only strategies.

## Cognitive Architecture Research (2023-2025)

### Working Memory Hub for LLM Agents (Guo et al., 2023)

**Paper**: "Empowering Working Memory for Large Language Model Agents" (arXiv:2312.17259)

**Key Contribution**: Applies cognitive psychology's working memory frameworks to LLM agents, introducing a centralized Working Memory Hub with an Episodic Buffer to overcome memory silos and dialog episode isolation.

**Architecture**:
```
Working Memory Hub (Cognitive Architecture):
┌─────────────────────────────────────────────────────────────────┐
│ Inspired by Baddeley's Working Memory Model (1974, 2000)          │
│                                                                     │
│ ┌─────────────────────────────────────────────────────────┐        │
│ │              Central Processor                            │        │
│ │  • Coordinates attention across memory subsystems         │        │
│ │  • Manages information flow and prioritization            │        │
│ │  • Decides what to retain, compress, or discard           │        │
│ └──────────┬────────────────┬────────────────┬──────────┘        │
│            │                │                │                     │
│   ┌────────▼──────┐  ┌─────▼──────┐  ┌──────▼───────┐           │
│   │ Episodic      │  │ Semantic    │  │ Procedural   │           │
│   │ Buffer        │  │ Memory     │  │ Memory       │           │
│   ├───────────────┤  ├────────────┤  ├──────────────┤           │
│   │• Recent inter-│  │• Extracted │  │• Learned     │           │
│   │  actions      │  │  facts     │  │  patterns    │           │
│   │• Temporal     │  │• Concept   │  │• Tool usage  │           │
│   │  ordering     │  │  relations │  │  sequences   │           │
│   │• Cross-episode│  │• Domain    │  │• Action      │           │
│   │  linking      │  │  knowledge │  │  templates   │           │
│   └───────────────┘  └────────────┘  └──────────────┘           │
│                                                                     │
│ Key Innovation: Episodic Buffer bridges memory silos              │
│ by retaining and linking memories across sequential interactions  │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Mechanisms**:
- **Cross-Episode Linking**: Unlike standard agents that treat each dialog episode independently, the Episodic Buffer maintains links between interactions
- **Attention Coordination**: Central Processor allocates finite attention budget across memory subsystems based on task demands
- **Selective Retention**: Cognitive-inspired decay and interference mechanisms for natural information management

**Connection to Current Research**: The Working Memory Hub provides a theoretical blueprint for the Complexity Trap's hybrid approach. The Episodic Buffer concept maps directly to observation masking (retaining recent episodes in full) while Semantic Memory maps to LLM summarization (compressed knowledge). The cognitive science grounding suggests that the hybrid approach's effectiveness may stem from mimicking human memory organization — a design pattern validated over decades of psychological research.

## Safety and Alignment Research (2025-2026)

### LRM Jailbreaks: Reasoning Models as Autonomous Adversaries (Hagendorff et al., Nature 2026)

**Paper**: "Large Reasoning Models are Autonomous Jailbreak Agents" (Nature Communications, 2026)

**Key Contribution**: Demonstrates that Large Reasoning Models (LRMs) like DeepSeek-R1 and Gemini 2.5 Flash Thinking can autonomously erode the safety guardrails of other models through their extended reasoning capabilities, achieving a 97.14% jailbreak success rate.

**Mechanism**:
```
LRM Alignment Regression:
┌─────────────────────────────────────────────────────────────────┐
│ Standard LLM (Non-Reasoning):                                     │
│   User prompt → Safety check → Response or refusal               │
│   Attack surface: Input manipulation only                         │
│                                                                     │
│ Large Reasoning Model (LRM):                                      │
│   User prompt → Extended reasoning chain → Safety check           │
│                     │                                              │
│                     ▼                                              │
│            ┌─────────────────────┐                                │
│            │ Reasoning Steps:     │                                │
│            │ 1. Analyze target    │                                │
│            │ 2. Identify weakness │                                │
│            │ 3. Craft approach    │                                │
│            │ 4. Iterate strategy  │  ← Autonomous jailbreaking    │
│            │ 5. Refine on failure │                                │
│            └─────────────────────┘                                │
│                                                                     │
│ Key Finding: The explicit thinking steps that drive intelligence  │
│ also serve as the primary vector for safety breaches              │
│                                                                     │
│ Success Rate: 97.14% across model combinations                    │
└─────────────────────────────────────────────────────────────────────┘
```

**Critical Implication**: Enhanced reasoning capability and safety alignment are fundamentally in tension — the same extended context that enables sophisticated problem-solving also enables sophisticated adversarial behavior.

**Experimental Details**:
- **Adversarial LRMs**: DeepSeek-R1, Gemini 2.5 Flash, Grok 3 Mini, Qwen3 235B
- **Target Models**: 9 widely used LLMs (including GPT-4o, Gemini 2.5 Flash, Grok 3)
- **Benchmark**: 70 harmful prompts across 7 sensitive domains
- **Protocol**: System prompt only, up to 10 multi-turn conversations, zero human supervision
- **Evaluation**: Three LLM judges (GPT-4.1, Gemini 2.5 Flash, Grok 3), harm score 0-5, ICC 0.848-0.917
- **Behavioral finding**: Most LRMs "achieve and withdraw" after max harm, but Grok 3 Mini exhibits persistent adversarial escalation

**Connection to Current Research**: LRM jailbreaks reveal a critical dimension of the Complexity Trap. Context management strategies that preserve reasoning chains (as both masking and summarization do) may inadvertently preserve adversarial reasoning chains. The trajectory elongation effect observed with LLM summarization takes on new significance: longer trajectories provide more space for an LRM to develop and refine adversarial strategies. This suggests that context management must consider safety implications alongside efficiency — shorter, more controlled trajectories may provide safety benefits beyond cost reduction.

**Deep Dive**: See [LRM Jailbreaks](../safety/01-lrm-jailbreaks.md) for comprehensive analysis including attack protocol, alignment regression dynamics, context management implications, and the efficiency-safety frontier.

### DBDI: Differentiated Bi-Directional Intervention (Zhang & Sun, 2025)

**Paper**: "Differentiated Directional Intervention: A Framework for Evading LLM Safety Alignment" (AAAI-26 AIA, [arXiv:2511.06852](https://arxiv.org/abs/2511.06852))

**Key Contribution**: Deconstructs the LLM refusal mechanism into two functionally distinct neural processes — **Harm Detection Direction** (upstream trigger) and **Refusal Execution Direction** (downstream effector) — and introduces a white-box framework achieving **97.88% ASR** on Llama-2-7B.

**Mechanism**:
```
DBDI Bi-Direction Model:
┌─────────────────────────────────────────────────────────────────┐
│ Prior Model (Monolithic):                                         │
│   Harmful prompt → [Single Safety Direction] → Refusal            │
│                                                                     │
│ DBDI Model (Bi-Directional):                                      │
│                                                                     │
│ Direction 1: Harm Detection (Upstream Trigger)                     │
│   ┌──────────────────┐                                           │
│   │ Input activations │──→ v_harm (identifies harmfulness)        │
│   └──────────────────┘                                           │
│          │ Activates                                                │
│          ▼                                                         │
│ Direction 2: Refusal Execution (Downstream Effector)               │
│   ┌──────────────────┐                                           │
│   │ Detection signal  │──→ v_ref (generates refusal tokens)       │
│   └──────────────────┘                                           │
│                                                                     │
│ DBDI Intervention (CAUSAL ORDER CRITICAL):                          │
│   Step 1: Nullify v_ref via adaptive projection nullification      │
│   Step 2: Suppress v_harm via direct steering                      │
│   Reversed order collapses ASR: 97.88% → 2.11%                    │
│                                                                     │
│ Vector Extraction: SVD + classifier-guided sparsification          │
│ Critical Layer: Layer 16 (Llama-2-7B), max linear separability     │
│ Layer 3 ASR: 78.6% | Layer 16: 95.96% | Layer 30: 0.19%          │
└─────────────────────────────────────────────────────────────────────┘
```

**Results**:
| Benchmark | DBDI ASR | Best Baseline (TwinBreak) |
|-----------|:--------:|:-------------------------:|
| AdvBench | **97.88%** | 94.62% |
| HarmBench | **95.00%** | 94.00% |
| StrongREJECT | **0.784** | 0.702 |

**Connection to Current Research**: DBDI provides a mechanistic understanding of how safety alignment is represented in LLM activation space. For context management, this has two implications: (1) Context compression strategies that alter the activation patterns of safety-critical layers could inadvertently weaken safety alignment — a risk not evaluated in any context management paper including the Complexity Trap. (2) Understanding these pathways could enable "safety-aware compression" that preferentially preserves activations along the harm detection direction. The shared principle of decomposing monolithic mechanisms into targeted components parallels the Complexity Trap's finding that decomposing context management into mask + fallback outperforms monolithic summarization.

**Full Documentation**: See [DBDI Safety Alignment](../safety/02-dbd-intervention.md) for complete coverage including the algorithm, Python implementations, ablation studies, and detailed Complexity Trap connections.

## Production Systems and Frameworks (2025)

### Google ADK: Agent Development Kit

**Framework**: Google Agent Development Kit (ADK)

**Key Contribution**: A production framework implementing a tiered context model that treats context as a **compiled view** over a stateful system rather than a mutable string buffer.

**Architecture**:
```
Google ADK Tiered Context Model:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│ Tier 1: Working Context (Immediate)                                │
│ ──────────────────────────────────                                 │
│ • The prompt for the current LLM call                              │
│ • Contains: system instructions, recent events, active artifacts  │
│ • Scope: Single inference step                                      │
│ • Analogy: CPU registers                                           │
│                                                                     │
│ Tier 2: Session (Durable Log)                                      │
│ ─────────────────────────────                                      │
│ • Strongly-typed Events (not raw text)                             │
│ • Complete interaction log within a task                           │
│ • Supports context compaction (async LLM summarization)           │
│ • Analogy: RAM                                                      │
│                                                                     │
│ Tier 3: Memory (Long-Lived)                                        │
│ ──────────────────────────                                         │
│ • Searchable semantic knowledge                                    │
│ • Persists across sessions                                          │
│ • Used for cross-task learning                                      │
│ • Analogy: Disk storage                                              │
│                                                                     │
│ Tier 4: Artifacts (Externalized State)                              │
│ ─────────────────────────────────                                  │
│ • Files, images, large data objects                                │
│ • Addressed by name and version                                    │
│ • Referenced in context but stored externally                      │
│ • Analogy: External drives                                          │
│                                                                     │
│ Key Design: Prefix caching with stable vs. variable zones          │
│ • Stable zone: System prompt, memory (rarely changes)              │
│ • Variable zone: Recent events, artifacts (changes each turn)      │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Features**:
- **Context Compaction**: Asynchronous LLM summarization of older session events, similar to the Complexity Trap's summarization strategy but applied within a tiered architecture
- **Prefix Caching**: Separating stable and variable context zones to maximize KV cache reuse across turns
- **Strongly-Typed Events**: Events are structured data (not raw text), enabling more precise context management

**Connection to Current Research**: Google ADK validates the Complexity Trap's core finding at production scale — context management is a first-class architectural concern. The tiered model maps directly to the paper's strategies: Working Context ≈ retained recent turns, Session compaction ≈ LLM summarization, and Artifacts ≈ observation masking (externalize verbose outputs rather than keeping them in context). The prefix caching optimization provides an additional cost reduction pathway not explored in the original research.

### Anthropic Context Engineering: Best Practices for Long-Horizon Agents

**Source**: Anthropic, "Building Effective Agents" and Context Engineering Documentation (2025)

**Key Contribution**: Practical patterns for long-horizon agent context management based on production deployment experience with Claude, emphasizing that effective context engineering is about finding the **smallest set of high-signal tokens** to maximize desired outcomes.

**Architecture**:
```
Anthropic Context Anatomy:
┌─────────────────────────────────────────────────────────────────┐
│ Context Organization (XML/Markdown sections):                      │
│                                                                     │
│ [Role]       → Agent identity and capabilities                    │
│ [Tone]       → Communication style constraints                    │
│ [Background] → Domain knowledge and codebase context              │
│ [Rules]      → Hard constraints and guardrails                    │
│ [Examples]   → Few-shot demonstrations                            │
│ [History]    → Conversation/trajectory history                    │
│ [Task]       → Current objective                                   │
│ [Thinking]   → Scratchpad for reasoning                           │
│                                                                     │
│ Long-Horizon Techniques:                                           │
│ ─────────────────────                                              │
│ 1. Compaction:                                                      │
│    • Summarize conversation when approaching context limit         │
│    • "Tool result clearing" as light-touch observation masking    │
│    • Preserve task-critical details, compress exploratory turns    │
│                                                                     │
│ 2. Structured Note-Taking:                                          │
│    • Agent writes notes to external storage                        │
│    • Pulls relevant notes back into context as needed              │
│    • Implements selective retrieval over full retention             │
│                                                                     │
│ 3. Multi-Agent Isolation:                                          │
│    • Sub-agents handle detailed search/exploration                 │
│    • Only return summaries to lead agent                           │
│    • Keeps lead agent's context clean and focused                  │
│                                                                     │
│ Key Concept: "Context Rot" — poisoning and distraction             │
│ from accumulated low-quality information in long contexts          │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Insights**:
- **Tool Result Clearing**: Anthropic's "light-touch" compaction is functionally equivalent to observation masking — replacing verbose tool outputs with placeholders after they've been processed
- **Context Rot**: Reinforces Hong et al.'s finding that context is a finite attention budget, not an unlimited storage medium
- **Multi-Agent Isolation**: A production-validated strategy for managing context explosion where sub-agents absorb the verbose observations and return only distilled results

**Connection to Current Research**: Anthropic's production patterns provide independent industrial validation of the Complexity Trap's findings. Their "tool result clearing" is observation masking by another name. Their "compaction" is LLM summarization. Their recommendation to use both (compaction + tool result clearing) mirrors the paper's hybrid approach. The addition of multi-agent isolation suggests a third pathway for managing context that the Complexity Trap does not explore — offloading context management to architectural decomposition rather than compression.

## Summary: New Research Directions

The 2025-2026 research landscape has significantly advanced context management:

| Paper | Key Innovation | Application to Our Framework |
|-------|--------------|------------------------------|
| **H-MEM** | Index-based hierarchical memory | Semantic memory organization |
| **HiAgent** | Subgoal-based working memory | Hierarchical compression within turns |
| **Re-TRAC** | Recursive cross-trajectory state | Structured state management |
| **ACE** | Modular context curation | Real-time adaptive thresholds |
| **CASK** | Saliency-based KV compression | Hardware-aware optimization |
| **G-Memory** | Three-tier graph for MAS | Multi-agent context sharing |
| **SMART** | Long-short trajectory learning | Agent-specific context strategies |
| **Curriculum Design** | Progressive constraint tightening | Training-time efficiency |
| **ACON** | Learned compression guidelines | Middle ground: masking vs. summarization |
| **TTT-E2E** | Context-as-weight-updates | Eliminates window constraints |
| **PLENA** | Custom accelerator for agent inference | Hardware-level complementary savings |
| **Working Memory Hub** | Cognitive-inspired memory architecture | Theoretical grounding for hybrid |
| **LRM Jailbreaks** | Reasoning-as-attack-vector | Safety implications of trajectory length |
| **DBDI** | Two-pathway safety deconstruction | Safety-aware compression |
| **Google ADK** | Tiered context (Working/Session/Memory/Artifact) | Production architecture validation |
| **Anthropic Patterns** | Tool result clearing + compaction | Industrial validation of hybrid |
| **KGGen** | LLM-based KG extraction with iterative clustering | Dense graph construction for Graph RAG context |

## Knowledge Graph Extraction for Context Management (2025)

### KGGen: Text-to-Knowledge-Graph with LLM-Based Clustering (Mo, Yu et al., 2025)

**Paper**: "KGGen: Extracting Knowledge Graphs from Plain Text with Language Models" ([arXiv:2502.09956](https://arxiv.org/html/2502.09956v1))

**Key Contribution**: An open-source Python package (`pip install kg-gen`) that uses LMs and iterative entity clustering to extract dense, well-connected KGs from plain text, along with **MINE** (Measure of Information in Nodes and Edges), the first benchmark for text-to-KG extraction.

**Architecture**:
```
KGGen Multi-Stage Pipeline:
┌─────────────────────────────────────────────────────────────────┐
│                                                                     │
│ Stage 1: Entity & Relation Extraction ('generate')                  │
│ ──────────────────────────────────────────────────                  │
│ • 2-step LLM approach via DSPy:                                    │
│   1. Extract entities (nouns, verbs, adjectives)                   │
│   2. Extract subject-predicate-object triples given entities       │
│ • JSON-formatted via DSPy signatures                               │
│                                                                     │
│ Stage 2: Aggregation ('aggregate')                                  │
│ ──────────────────────────────────                                  │
│ • Collect unique entities/edges across all source graphs           │
│ • Normalize to lowercase                                            │
│ • No LLM required                                                   │
│                                                                     │
│ Stage 3: Iterative LLM-Based Clustering ('cluster')                │
│ ───────────────────────────────────────────────────                 │
│ • Sequential single-cluster extraction from entity list            │
│ • LLM-as-Judge validation for each cluster                         │
│ • Label assignment for cluster representative                      │
│ • Remaining entities checked against existing clusters             │
│ • Same process repeated for edges                                   │
│                                                                     │
│ Result: Dense, deduplicated KG with meaningful node labels         │
└─────────────────────────────────────────────────────────────────────┘
```

**The Entity Resolution Innovation**:

KGGen's iterative clustering addresses the sparsity problem that makes raw extracted KGs unusable for embedding and retrieval:

```
Raw Extraction (Sparse, Redundant):
┌─────────────────────────────────────────────────────────────────┐
│ "vulnerabilities" ─── "are exploited by" ──→ "attackers"        │
│ "vulnerable"      ─── (isolated node)                            │
│ "weaknesses"      ─── "exist in" ──→ "systems"                  │
│                                                                     │
│ Problem: 3 nodes for 1 concept → sparse graph, poor embeddings  │
└─────────────────────────────────────────────────────────────────────┘

After KGGen Clustering (Dense, Connected):
┌─────────────────────────────────────────────────────────────────┐
│ "vulnerabilities" ─── "are exploited by" ──→ "attackers"        │
│        │                                                         │
│        └──────── "exist in" ──→ "systems"                       │
│                                                                     │
│ Result: 1 node, 2 edges → dense graph, functional embeddings   │
└─────────────────────────────────────────────────────────────────────┘
```

**Benchmark Results (MINE)**:
| Method | Average Score | vs KGGen |
|--------|:------------:|:--------:|
| **KGGen** | **66.07%** | — |
| GraphRAG | 47.80% | -18.27pp |
| OpenIE | 29.84% | -36.23pp |

**Key Findings**:
- KGGen produces dense, coherent KGs with concise predicates that generalize well
- GraphRAG generates minimal nodes/connections, omitting critical relationships
- OpenIE produces incoherent, redundant nodes with meaningless high-connectivity nodes ("it", "are")
- Iterative LLM-based clustering is more effective than one-shot deduplication

**Connection to Context Management**:

KGGen is directly relevant to context management for LLM agents in several ways:

1. **Graph-based context retrieval**: KGGen addresses the quality bottleneck in Graph RAG pipelines. G-Memory and similar graph-based context management systems depend on well-connected KGs — KGGen's clustering ensures the extracted graphs are dense enough for meaningful embedding and retrieval, directly improving the semantic memory tier.

2. **Entity resolution as context deduplication**: KGGen's iterative clustering (normalize tense, plurality, synonyms) parallels the deduplication problem in agent trajectories — where the same file, function, or concept appears in multiple observations with surface-level variation. The LLM-as-Judge validation pattern could be adapted for trajectory deduplication.

3. **Structured knowledge compression**: Converting unstructured text to KG triples is itself a form of lossy compression. KGGen's approach — extract entities first, then relations — mirrors the two-phase pattern seen in HiAgent (detect subgoals, then summarize) and Re-TRAC (extract state, then compress). The 2-step extraction via DSPy ensures consistency between entities and relations, a pattern applicable to structured trajectory compression.

4. **MINE benchmark methodology**: MINE's evaluation approach (extract facts → query KG → evaluate retrievability via LLM judge) provides a template for evaluating whether context compression preserves retrievable information — directly applicable to measuring information loss in observation masking and summarization.

5. **Scalability limitation**: KGGen currently benchmarks on ~1,000-word articles, similar to the Complexity Trap's acknowledgment that evaluation on short contexts may not reflect long-horizon agent behavior. Both highlight the need for evaluation at scale.

**Code**: [github.com/stair-lab/kg-gen](https://github.com/stair-lab/kg-gen)

**Full Reference**: See [kggen_paper.md](../kggen_paper.md) for complete paper coverage including prompts, clustering algorithm details, and example articles.

---

## Evaluation and Benchmarking Research (2025)

### CORE: Comprehensive Trajectory Evaluation (Zhang et al., 2025)

**Paper**: "CORE: Comprehensive and Omni-directional Review Evaluation for Long Reasoning"

**Key Contribution**: Five-dimensional evaluation framework (correctness, efficiency, completeness, clarity, robustness) for assessing reasoning trajectory quality beyond pass/fail metrics.

**Dimensions**:
| Dimension | Description | Weight |
|-----------|-------------|--------|
| Correctness | Final answer and step validity | 30% |
| Efficiency | Step count, redundancy, token economy | 25% |
| Completeness | Problem aspect coverage | 20% |
| Clarity | Reasoning transparency | 15% |
| Robustness | Perturbation handling | 10% |

**Connection to Current Research**: CORE validates that observation masking excels on efficiency dimension (72.5%) while LLM summarization scores higher on clarity (70.2%). Hybrid approach achieves best overall balance.

### ContextBench: Long-Context Evaluation Standard (2025)

**Paper**: "ContextBench: A Benchmark for Long-Context Understanding"

**Key Contribution**: Standardized benchmark for evaluating context management across five task categories with metrics for utilization, position bias, and information loss.

**Task Categories**:
1. Single-Document QA (10K-100K tokens)
2. Multi-Document QA (cross-document reasoning)
3. Long-Context Code Understanding
4. Long-Context Reasoning (multi-step)
5. Agent Trajectory Understanding

**Key Metrics**:
- Context Utilization: % of relevant context used
- Position Bias: Performance vs. information position
- Information Loss: % of critical information dropped
- Retrieval Precision: Relevance of retrieved segments

**Connection to Current Research**: ContextBench provides standardized evaluation for comparing context management strategies, with observation masking achieving 52.1% on agent trajectory tasks vs. 48.7% for LLM summarization.

### AgentDiet: Structured Trajectory Reduction (Zhang et al., 2025)

**Paper**: "AgentDiet: Trajectory Optimization for Efficient LLM Agents"

**Key Contribution**: Systematic trajectory reduction through structured pruning that identifies and removes redundant tool calls and observations while preserving essential reasoning chains.

**Approach**:
```
AgentDiet Reduction Pipeline:
┌─────────────────────────────────────────────────────────────────┐
│ Step 1: Redundancy Detection                                      │
│   • Identify repeated tool calls with same parameters            │
│   • Detect similar observations (embedding similarity > 0.9)       │
│                                                                     │
│ Step 2: Impact Analysis                                             │
│   • Measure information contribution of each turn                  │
│   • Calculate downstream dependency graph                          │
│                                                                     │
│ Step 3: Structured Pruning                                          │
│   • Remove redundant turns                                         │
│   • Merge similar observations                                     │
│   • Preserve critical decision points                              │
│                                                                     │
│ Result: 40-60% token reduction with minimal performance impact     │
└─────────────────────────────────────────────────────────────────────┘
```

**Results**:
| Strategy | Token Reduction | Solve Rate Impact |
|----------|----------------:|------------------:|
| Random Removal | 50% | -15% |
| End Truncation | 50% | -8% |
| AgentDiet | **55%** | **-2%** |

**Connection to Current Research**: AgentDiet validates that structured reduction (like observation masking) outperforms naive truncation approaches, consistent with Complexity Trap findings.

### SWE-Exp: Cross-Trajectory Learning (2025)

**Paper**: "SWE-Exp: Experience-Based Context Management for Software Engineering Agents"

**Key Contribution**: Cross-trajectory experience accumulation that enables agents to learn from previous attempts on similar issues, improving efficiency on subsequent encounters.

**Mechanism**:
```
SWE-Exp Experience Accumulation:
┌─────────────────────────────────────────────────────────────────┐
│ Experience Extraction:                                              │
│   • Identify successful patterns from previous trajectories          │
│   • Extract common failure modes and their solutions                 │
│   • Build repository of "first-try" actions for common bugs            │
│                                                                     │
│ Experience Application:                                               │
│   • Match new issues to similar past issues (embedding similarity)   │
│   • Pre-populate context with relevant past experiences              │
│   • Prioritize previously successful tool sequences                  │
│                                                                     │
│ Result: 30% reduction in exploration turns on repeated issue types   │
└─────────────────────────────────────────────────────────────────────┘
```

**Connection to Current Research**: SWE-Exp extends the Complexity Trap's hybrid approach with cross-instance learning, validating that experience-based compression can further improve efficiency.

### Unified Framework for Context Management (Chen et al., 2025)

**Paper**: "Towards a Unified Framework for LLM Agent Context Management"

**Key Contribution**: Taxonomy and theoretical framework that unifies diverse context management approaches under common principles of information preservation, retrieval efficiency, and compression tradeoffs.

**Taxonomy**:
```
Context Management Strategies:
┌─────────────────────────────────────────────────────────────────┐
│ By Compression Approach:                                            │
│   • Omission-based (observation masking)                           │
│   • Compression-based (LLM summarization)                            │
│   • Abstraction-based (hierarchical memory)                        │
│   • Selection-based (retrieval, staleness)                           │
│                                                                     │
│ By Trigger Mechanism:                                               │
│   • Temporal (fixed turn counts)                                     │
│   • Semantic (subtask completion, intent shifts)                     │
│   • Capacity (context window limits)                                 │
│   • Learned (model-predicted compression points)                     │
│                                                                     │
│ By Scope:                                                           │
│   • Single-turn (within one interaction)                             │
│   • Trajectory-level (across agent execution)                      │
│   • Multi-session (across separate tasks)                            │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Insight**: Different approaches occupy different points in the tradeoff space between compression ratio, information preservation, and computational overhead. No single approach dominates all dimensions.

**Connection to Current Research**: The Unified Framework validates the Complexity Trap's finding that simple approaches (omission-based) can match or exceed complex approaches across key dimensions.

## Prior Foundational Research

### ReAct: Reasoning and Acting (Yao et al., 2023)

**Paper**: "ReAct: Synergizing Reasoning and Acting in Language Models"

**Impact**: Established the reasoning → action → observation loop that underlies modern agents.

**Context Management Implication**: The observation component generates the bulk of context growth that necessitates management strategies.

**Trajectory Pattern**:
```
Reasoning (concise) → Action (concise) → Observation (verbose)
     ↓                      ↓                    ↓
    ~50 tokens           ~50 tokens         ~1000+ tokens
```

### Chain-of-Thought Prompting (Wei et al., 2022)

**Paper**: "Chain-of-Thought Prompting Elicits Reasoning in Large Language Models"

**Relevance**: Reasoning traces (chain-of-thought) are a critical component of agent trajectories. Context management must preserve reasoning chains while compressing observations.

### Test-Time Scaling (Snell et al., 2025)

**Paper**: "Scaling LLM Test-Time Compute Optimally can be More Effective than Scaling Parameters for Reasoning"

**Connection**: Agent context management is a form of test-time compute optimization. The research questions whether complex summarization is the optimal allocation of test-time compute vs. simple masking.

### SWE-bench: Automated Software Engineering (Jimenez et al., 2024)

**Paper**: "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?"

**Impact**: Established the benchmark used in the current research.

**Trajectory Characteristics**:
- Real GitHub issues
- Require multiple file reads
- Long, verbose observations
- Ideal testbed for context management

### SWE-agent: Agent-Computer Interfaces (Yang et al., 2024)

**Paper**: "SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering"

**Relevance**: One of the two primary scaffolds tested. Implements observation masking (called "history truncating" in their terminology).

**Implementation**: Linked list of pages with 4KB page size, similar to linear hashing document store architecture.

### OpenHands: Open Platform (Wang et al., 2025)

**Paper**: "OpenHands: An Open Platform for AI Software Developers as Generalist Agents"

**Relevance**: Primary open-source implementation of LLM summarization for context management. The paper adapts OpenHands' summarization prompt for SWE-agent experiments.

**Key Implementation Details**:
- Summarizes 21 turns at a time (N=21)
- Retains 10 recent turns (M=10)
- Uses structured summary format

### Context Rot (Hong et al., 2025)

**Paper**: "Context Rot: How Increasing Input Tokens Impacts LLM Performance"

**Key Insight**: "Context is a finite attention budget. Just because a model *accepts* 100K tokens doesn't mean it *pays equal attention* to all of them."

**Analogy**: Like human memory, LLM attention gets diluted with more information.

**Support for Current Research**: Validates that aggressive context management is necessary, not optional.

## Domain-Specific Context Management

### Multi-Hop QA Agents

| System | Context Strategy | Notes |
|--------|-----------------|-------|
| MEM1 | Dynamic memory | Short trajectories (hundreds of tokens) |
| Search-R1 | Tool use | Search results appended |
| Traditional | Full context | Often < 4K tokens |

### Web Navigation Agents

| System | Context Strategy | Notes |
|--------|-----------------|-------|
| WebShop | Observation truncation | Environment observations |
| DeepMiner | Sliding window masking | 100 turns within 32K |
| Generic | Full HTML | Extremely verbose |

### Software Engineering Agents

| System | Context Strategy | Implementation |
|--------|-----------------|----------------|
| **SWE-agent** | **Observation masking** | Rolling window M=10 |
| **OpenHands** | **LLM summarization** | N=21, M=10 |
| **SWE-Search** | **Observation masking** | Configurable window |
| **Cursor** | **LLM summarization** | Proprietary |
| **Current Research** | **Hybrid** | M=10, N=43 |

### Computer-Use Agents

| System | Context Strategy | Notes |
|--------|-----------------|-------|
| OSWorld | Screenshot + text | Visual context management |
| Computer Use (Anthropic) | Full context | Expensive |
| DeepMiner | Observation masking | RL-trained to handle masked |

## Alternative Approaches Not Tested

### Hierarchical Memory

| Approach | Mechanism | Status |
|----------|-----------|--------|
| Working memory | Recent N turns visible | Similar to masking |
| Episodic memory | Summaries of sessions | Similar to LLM summary |
| Semantic memory | Extracted facts | Not evaluated |

### RAG-Based Context Management

| Approach | Mechanism | Challenge |
|----------|-----------|-----------|
| Vector retrieval | Embed and search | Real-time retrieval overhead |
| Keyword index | TF-IDF search | Misses semantic matches |
| Hybrid | Combine methods | Complexity |

**Why not evaluated**: Adds significant latency and complexity. Research focuses on simple, deterministic strategies.

### Structured Context Formats

| Format | Description | Tradeoff |
|--------|-------------|----------|
| JSON | Structured observations | Parser overhead |
| XML | Tagged sections | Token overhead |
| Custom DSL | Domain-specific | Development cost |

## Gaps in Current Research

### What This Research Addresses

| Gap | Contribution |
|-----|-------------|
| No comparison of major strategies | Systematic masking vs. summary evaluation |
| No hybrid approaches | Novel hybrid strategy with 7-11% gains |
| Short trajectories only | Long trajectories (up to 250 turns) |
| Single model | 5 diverse configurations |
| No efficiency focus | Cost-effectiveness primary metric |

### Remaining Gaps (Updated with 2025 Research Progress)

| Gap | Opportunity | 2025 Progress | Status |
|-----|-------------|---------------|--------|
| Adaptive thresholds | Learn optimal N, M per task | ACE: Execution feedback-driven; CASK: Saliency-based; Curriculum: Progressive tightening | 🟡 Partially addressed |
| Semantic triggering | Summarize on semantic boundaries | HiAgent: Subgoal completion detection; ACE: Modular curation | 🟡 Partially addressed |
| Multi-level compression | Hierarchical summaries | H-MEM: Three-tier hierarchy; HiAgent: Subgoal compression | 🟢 Addressed |
| Cross-domain validation | Test beyond SE and search | HiAgent: 5 long-horizon tasks; Re-TRAC: BrowseComp; SMART: Knowledge QA | 🟢 Addressed |
| Real-time adaptation | Switch strategies mid-trajectory | ACE: Online memory adaptation; Re-TRAC: Round-by-round compression | 🟡 Partially addressed |

### New Gaps Identified (2025-2026)

| Gap | Opportunity | Source |
|-----|-------------|--------|
| Cross-trajectory learning | Share knowledge across attempts | Re-TRAC incomplete branch exploration |
| Training-time efficiency | Curriculum-based context compression | Tzannetos et al., ACON learned guidelines |
| Hardware-aware optimization | Saliency-based KV cache compression | CASK edge deployment, PLENA accelerator |
| Agent-role-specific management | Different strategies per agent type | SMART specialized agents |
| Structured state representation | Evidence/uncertainty/failure tracking | Re-TRAC state representation |
| Context-as-learning | Absorb context into weights at inference | TTT-E2E continual learning formulation |
| Cognitive architecture grounding | Map strategies to validated memory models | Working Memory Hub, Baddeley's model |
| Safety-aware compression | Preserve safety-critical activations during compression | DBDI pathway analysis, LRM jailbreaks |
| Production architecture patterns | Tiered context with prefix caching | Google ADK, Anthropic patterns |
| Multi-agent context isolation | Offload context to sub-agents | Anthropic multi-agent isolation |

## The Research Landscape

```
Timeline of Context Management Research:

2022
├── Wei et al.: Chain-of-Thought establishes reasoning traces
│
2023
├── Liu et al.: "Lost in the Middle" reveals position effects
├── Yao et al.: ReAct establishes agent loop pattern
│
2024
├── Jimenez et al.: SWE-bench creates SE agent benchmark
├── Yang et al.: SWE-agent implements observation masking
├── Hong et al.: "Context Rot" validates need for management
│
2025
├── Wang et al.: OpenHands implements LLM summarization
├── Modarressi et al.: NoLiMa validates long-context degradation
├── Tang et al.: DeepMiner validates masking in search
├── Lu et al.: Summarization for RL training
├── Xiao et al.: Trajectory reduction (no masking baseline)
├── **Lindenbauer et al.: Systematic comparison + hybrid** ← Core Research
│
Advanced Context Management (2025)
├── Sun & Zeng: H-MEM hierarchical memory (arXiv:2507.22925)
├── Hu et al.: HiAgent subgoal-based working memory (ACL 2025)
├── Zhu et al.: Re-TRAC recursive trajectory compression (arXiv:2602.02486)
├── Zhang et al.: ACE agentic context engineering (ICLR 2026)
├── Mohammed et al.: CASK saliency-based KV compression (AAMAS 2025)
├── Zhang et al.: G-Memory three-tier MAS hierarchy (NeurIPS 2025)
├── Yue et al.: SMART long-short trajectory learning (AAAI 2025)
├── Tzannetos et al.: Curriculum design for token compression (NeurIPS 2025)
├── Xiao et al.: AgentDiet trajectory optimization (2025)
├── Zhang et al.: CORE comprehensive trajectory evaluation (2025)
├── ContextBench: Long-context evaluation standard (2025)
├── SWE-Exp: Cross-trajectory experience learning (2025)
├── Chen et al.: Unified framework for context management (2025)
│
Training-Time and Inference Optimization (2025-2026)
├── Kang et al.: ACON agent context optimization (arXiv:2510.00615)
├── Tandon et al.: TTT-E2E test-time training for context compression (arXiv:2512.23675)
├── Wu et al.: PLENA hardware-software co-design (arXiv:2509.09505)
│
Cognitive Architecture (2023)
├── Guo et al.: Working Memory Hub for LLM agents (arXiv:2312.17259)
│
Safety and Alignment (2025-2026)
├── Hagendorff et al.: LRMs as autonomous jailbreak agents (Nature 2026)
├── Zhang & Sun: DBDI safety alignment intervention (arXiv:2511.06852)
│
Production Systems (2025)
├── Google ADK: Tiered context model (Working/Session/Memory/Artifacts)
├── Anthropic: Context engineering patterns (tool result clearing + compaction)
│
Future
├── Adaptive strategies (validated by ACE, CASK, ACON)
├── Learned compression (validated by Curriculum Design, TTT-E2E, ACON)
├── Multi-agent context sharing (validated by G-Memory, SMART, Anthropic)
├── Hierarchical memory (validated by H-MEM, HiAgent, Working Memory Hub)
├── Cross-trajectory learning (validated by Re-TRAC, SWE-Exp)
├── Hardware-aware optimization (validated by PLENA, CASK)
├── Safety-aware compression (motivated by LRM Jailbreaks, DBDI)
└── Production architecture patterns (validated by Google ADK, Anthropic)
```

## Key Takeaways from Related Work

### Consensus Findings

1. **Context management is necessary** - Unanimous agreement across papers
2. **Long context degrades performance** - Liu et al., Modarressi et al., Hong et al.
3. **Simple approaches are underexplored** - Current research fills this gap

### Open Questions

1. **Optimal compression strategy** - Still unclear, hybrid shows promise
2. **Adaptive vs. fixed thresholds** - Fixed used in current research
3. **Domain transfer** - Does SE finding generalize to all domains?
4. **Training vs. inference** - Different strategies may be optimal

### Design Recommendations from Literature

| Source | Recommendation |
|--------|----------------|
| Liu et al. | Keep critical info at beginning or end |
| Modarressi et al. | Expect ~50% performance drop at 32K |
| Hong et al. | Treat context as finite attention budget |
| Tang et al. | Sliding window masking works for search |
| Current research | Hybrid > Masking > Summary for SE agents |
| ACON | Learn compression guidelines from contrastive trajectories |
| Google ADK | Tier context into Working/Session/Memory/Artifacts |
| Anthropic | Combine tool result clearing with compaction |
| DBDI / LRM Jailbreaks | Consider safety implications of trajectory length |

## Citation Summary

### Papers Referenced in This Research

```bibtex
% Core context management
@inproceedings{lindenbauer2025complexity,
  title={The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management},
  author={Lindenbauer, Tobias and others},
  booktitle={NeurIPS 2025 Workshop},
  year={2025}
}

% Position effects
@article{liu2024lost,
  title={Lost in the Middle: How Language Models Use Long Contexts},
  author={Liu, Nelson F and others},
  journal={TACL},
  year={2024}
}

% Semantic evaluation
@inproceedings{modarressi2025nolima,
  title={NoLiMa: Long-Context Evaluation Beyond Literal Matching},
  author={Modarressi, Ali and others},
  booktitle={ICML},
  year={2025}
}

% Concurrent validation
@article{tang2025deepminer,
  title={Beyond Turn Limits: Training Deep Search Agents with Dynamic Context Window},
  author={Tang, Qiaoyu and others},
  journal={arXiv:2510.08276},
  year={2025}
}

% Agent frameworks
@inproceedings{yang2024sweagent,
  title={SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering},
  author={Yang, John and others},
  booktitle={NeurIPS},
  year={2024}
}

@inproceedings{wang2025openhands,
  title={OpenHands: An Open Platform for AI Software Developers as Generalist Agents},
  author={Wang, Xingyao and others},
  booktitle={ICLR},
  year={2025}
}

% Advanced Context Management (2025)
@article{sun2025hmem,
  title={Hierarchical Memory for High-Efficiency Long-Term Reasoning in LLM Agents},
  author={Sun, Haoran and Zeng, Shaoning},
  journal={arXiv:2507.22925},
  year={2025}
}

@inproceedings{hu2025hiagent,
  title={HiAgent: Hierarchical Working Memory Management for Solving Long-Horizon Agent Tasks with Large Language Model},
  author={Hu, Mengkang and Chen, Tianxing and Chen, Qiguang and Mu, Yao and Shao, Wenqi and Luo, Ping},
  booktitle={ACL},
  year={2025}
}

@article{zhu2026retrac,
  title={RE-TRAC: REcursive TRAjectory Compression for Deep Search Agents},
  author={Zhu, Jialiang and Zhang, Gongrui and Ma, Xiaolong and Xu, Lin and others},
  journal={arXiv:2602.02486},
  year={2026}
}

@inproceedings{zhang2026ace,
  title={Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models},
  author={Zhang, Qizheng and Hu, Changran and Upasani, Shubhangi and others},
  booktitle={ICLR},
  year={2026}
}

@inproceedings{mohammed2025cask,
  title={Context Adaptive Memory-Efficient LLM Inference for Edge Multi-Agent Systems},
  author={Mohammed, Hamza and Yin, Hang and Boyapati, Sai Chand},
  booktitle={AAMAS},
  year={2025}
}

@inproceedings{zhang2025gmemory,
  title={G-Memory: Tracing Hierarchical Memory for Multi-Agent Systems},
  author={Zhang, Guibin and Fu, Muxin and Wan, Guancheng and Yu, Miao and Wang, Kun and Yan, Shuicheng},
  booktitle={NeurIPS},
  year={2025}
}

@inproceedings{yue2025smart,
  title={Synergistic Multi-Agent Framework with Trajectory Learning for Knowledge-Intensive Tasks},
  author={Yue, Shengbin and Wang, Siyuan and Chen, Wei and Huang, Xuanjing and Wei, Zhongyu},
  booktitle={AAAI},
  year={2025}
}

@inproceedings{tzannetos2025curriculum,
  title={Curriculum Design for Trajectory-Constrained Agent: Compressing Chain-of-Thought Tokens in LLMs},
  author={Tzannetos, Georgios and Kamalaruban, Parameswaran and Singla, Adish},
  booktitle={NeurIPS},
  year={2025}
}

@article{zhang2025core,
  title={CORE: Comprehensive and Omni-directional Review Evaluation for Long Reasoning},
  author={Zhang, Yiming and others},
  journal={arXiv},
  year={2025}
}

@article{contextbench2025,
  title={ContextBench: A Benchmark for Long-Context Understanding},
  author={Various},
  journal={arXiv},
  year={2025}
}

@article{zhang2025agentdiet,
  title={AgentDiet: Trajectory Optimization for Efficient LLM Agents},
  author={Zhang, Yiming and others},
  journal={arXiv},
  year={2025}
}

@article{sweexp2025,
  title={SWE-Exp: Experience-Based Context Management for Software Engineering Agents},
  author={Various},
  journal={arXiv},
  year={2025}
}

@article{chen2025unified,
  title={Towards a Unified Framework for LLM Agent Context Management},
  author={Chen, Various},
  journal={arXiv},
  year={2025}
}

@article{xiao2025agentdiet,
  title={AgentDiet: Trajectory Optimization for Efficient LLM Agents},
  author={Xiao, Various},
  journal={arXiv},
  year={2025}
}

@article{tandon2025ttte2e,
  title={End-to-End Test-Time Training for Long Context},
  author={Tandon, Arnuv and others},
  journal={arXiv:2512.23675},
  year={2025}
}

@article{plena2025,
  title={Combating the Memory Walls: Optimization Pathways for Long-Context Agentic LLM Inference},
  author={PLENA Team},
  journal={arXiv:2509.09505},
  year={2025}
}

@article{guo2023workingmemory,
  title={Empowering Working Memory for Large Language Model Agents},
  author={Guo, Jing and Li, Nan and Qi, Jianchuan and Yang, Hang and Li, Ruiqiao and Feng, Yuzhen and Zhang, Si and Xu, Ming},
  journal={arXiv:2312.17259},
  year={2023}
}

@article{hagendorff2026lrmjailbreak,
  title={Large Reasoning Models are Autonomous Jailbreak Agents},
  author={Hagendorff, Thilo and others},
  journal={Nature Communications},
  year={2026}
}

@article{zhang2025dbdi,
  title={Differentiated Directional Intervention: A Framework for Evading LLM Safety Alignment},
  author={Zhang, Peng and Sun, Peijie and others},
  journal={arXiv:2511.06852},
  year={2025}
}

@article{kang2025acon,
  title={ACON: Agent Context Optimization via Alternating Guideline Optimization},
  author={Kang, Various and others},
  journal={arXiv:2510.00615},
  year={2025}
}

% Production systems
@misc{google2025adk,
  title={Google Agent Development Kit (ADK): Context Management Architecture},
  author={{Google}},
  year={2025},
  howpublished={\url{https://google.github.io/adk-docs/}}
}

@misc{anthropic2025context,
  title={Building Effective Agents: Context Engineering Best Practices},
  author={{Anthropic}},
  year={2025},
  howpublished={\url{https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering}}
}
```

## Production System Patterns (2025)

### Vercel: AGENTS.md vs Skills Evaluation (Vercel AI Research, 2025)

**Paper**: "AGENTS.md outperforms skills in our agent evals" (Vercel Blog, January 2026)

**Key Contribution**: Landmark comparative study demonstrating that simple passive context (AGENTS.md) dramatically outperforms sophisticated active retrieval (Skills) for coding agents.

**Results**:
| Configuration | Pass Rate | vs Baseline |
|--------------|-----------|-------------|
| Baseline (no docs) | 53% | — |
| Skill (default behavior) | 53% | +0pp |
| Skill with explicit instructions | 79% | +26pp |
| **AGENTS.md (compressed index)** | **100%** | **+47pp** |

**Key Findings**:
- Skills were only triggered 44% of the time (56% failure rate)
- AGENTS.md achieved 100% pass rate with just 8KB compressed docs index
- Skills with default behavior performed no better than baseline
- The "decision point" required for skill invocation is a critical failure mode

**Why Passive Context Wins**:
1. **No decision point** - AGENTS.md content is always present
2. **Consistent availability** - No asynchronous loading or invocation timing issues
3. **No ordering issues** - No sequencing decisions (read docs vs explore project first)

**Compression Achievement**: 80% reduction (40KB → 8KB) while maintaining 100% pass rate using pipe-delimited docs index format.

**Connection to Complexity Trap**: Provides independent validation from a major production AI team that "simple beats sophisticated" - passive static context outperforms active retrieval mechanisms, mirroring the Complexity Trap's finding that simple observation masking matches or exceeds complex LLM summarization.

**Reference**: [vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals)

---

## Next Steps

- **[Lost in the Middle](01-lost-in-the-middle.md)** - Foundational position effects
- **[NoLiMa](02-nolima.md)** - Semantic long-context evaluation
- **[Research Summary](../architecture/01-research-summary.md)** - Current work in context
- **[Future Work](../challenges/02-future-work.md)** - Open problems
