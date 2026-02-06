# Advanced Context Management Strategies (2025 Research)

## Overview

The 2025 research landscape has produced significant advances in context management, moving beyond simple masking and summarization toward hierarchical, learned, and semantic approaches. These strategies offer new ways to balance compression with information preservation.

**Key Development**: Multiple independent research groups have validated that structured, multi-level compression outperforms flat summarization, while adaptive methods based on task state enable more intelligent context management.

---

## 1. H-MEM: Hierarchical Memory Organization

**Paper**: "Hierarchical Memory for High-Efficiency Long-Term Reasoning in LLM Agents" (Sun & Zeng, 2025, arXiv:2507.22925)

### Core Concept

H-MEM introduces a three-tier memory hierarchy based on semantic abstraction, with index-based routing that enables efficient retrieval without exhaustive similarity search.

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         H-MEM HIERARCHY                                     │
│                                                                             │
│  Level 2: High-Level Themes                                                  │
│  ─────────────────────────────────────                                       │
│  • Cross-domain insights and strategic patterns                              │
│  • Abstract problem-solving principles                                       │
│  • Universal patterns across tasks                                           │
│                                                                             │
│       ▲                                                                       │
│       │ Index-based routing                                                   │
│       ▼                                                                       │
│                                                                             │
│  Level 1: Semantic Abstractions                                              │
│  ────────────────────────────────                                            │
│  • Grouped by topic/concept                                                   │
│  • Embedded with positional index encoding                                    │
│  • Points to related sub-memories in Level 0                                 │
│                                                                             │
│       ▲                                                                       │
│       │ Index-based routing                                                   │
│       ▼                                                                       │
│                                                                             │
│  Level 0: Raw Memory Entries                                                 │
│  ───────────────────────────                                                │
│  • Individual interactions, facts, observations                             │
│  • Full detail preserved                                                      │
│  • Direct agent experience                                                    │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Key Innovation: Index-Based Routing                                        │
│                                                                             │
│  Traditional: Exhaustive similarity search across all memories              │
│  H-MEM:       Direct index lookup → O(1) retrieval per level                │
│                                                                             │
│  Each vector contains positional index encoding:                            │
│  { embedding: [...], indices: [L0: 42, L1: 7, L2: 2] }                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Index-Based Routing Algorithm

```python
class HierarchicalMemory:
    """
    H-MEM: Three-tier hierarchical memory with index-based routing.
    
    Each memory entry contains index encoding pointing to related
    sub-memories in the next layer, enabling O(1) layer traversal.
    """
    
    def __init__(self, embedding_dim: int = 768):
        self.level_0 = []  # Raw entries
        self.level_1 = []  # Semantic abstractions
        self.level_2 = []  # High-level themes
        self.embedding_dim = embedding_dim
        
    def store(self, interaction: dict, semantic_topic: str):
        """Store new interaction with hierarchical indexing."""
        
        # Level 0: Store raw entry
        l0_idx = len(self.level_0)
        l0_entry = {
            'content': interaction,
            'embedding': self._embed(interaction),
            'timestamp': time.time()
        }
        self.level_0.append(l0_entry)
        
        # Level 1: Find or create semantic abstraction
        l1_idx = self._find_or_create_l1(semantic_topic)
        
        # Update L1's index to point to this L0 entry
        self.level_1[l1_idx]['indices'].append(l0_idx)
        
        # Level 2: Find or create theme
        theme = self._extract_theme(semantic_topic)
        l2_idx = self._find_or_create_l2(theme)
        
        # Update L2's index to point to this L1 entry
        if l1_idx not in self.level_2[l2_idx]['indices']:
            self.level_2[l2_idx]['indices'].append(l1_idx)
        
        # Store reverse indices in L0 for upward traversal
        l0_entry['l1_parent'] = l1_idx
        l0_entry['l2_parent'] = l2_idx
        
        return l0_idx
    
    def retrieve(self, query: str, strategy: str = "top_down") -> list:
        """
        Retrieve relevant memories using index-based routing.
        
        Args:
            query: Search query
            strategy: "top_down", "bottom_up", or "bidirectional"
        """
        query_emb = self._embed(query)
        results = []
        
        if strategy == "top_down":
            # Start from Level 2, drill down via indices
            l2_matches = self._similarity_search(query_emb, self.level_2, k=2)
            for l2 in l2_matches:
                for l1_idx in l2['indices']:
                    l1 = self.level_1[l1_idx]
                    for l0_idx in l1['indices'][-5:]:  # Recent from each
                        results.append(self.level_0[l0_idx])
                        
        elif strategy == "bottom_up":
            # Start from Level 0, climb via parent pointers
            l0_matches = self._similarity_search(query_emb, self.level_0, k=10)
            for l0 in l0_matches:
                results.append(l0)
                # Add context from parent abstractions
                l1 = self.level_1[l0['l1_parent']]
                l2 = self.level_2[l0['l2_parent']]
                results.extend([l1, l2])
        
        return results
    
    def _find_or_create_l1(self, topic: str) -> int:
        """Find existing L1 abstraction or create new one."""
        topic_emb = self._embed(topic)
        
        for i, entry in enumerate(self.level_1):
            if cosine_similarity(topic_emb, entry['embedding']) > 0.85:
                return i
        
        # Create new L1 abstraction
        self.level_1.append({
            'topic': topic,
            'embedding': topic_emb,
            'indices': [],
            'summary': self._generate_topic_summary(topic)
        })
        return len(self.level_1) - 1
    
    def _find_or_create_l2(self, theme: str) -> int:
        """Find existing L2 theme or create new one."""
        # Similar logic for high-level themes
        pass
```

### Results on LoCoMo Dataset

| Method | Multi-Document QA | Long Conversation | Multi-Turn Search | Avg Score |
|--------|--------------------:|------------------:|------------------:|----------:|
| No Memory | 42.3 | 38.1 | 35.7 | 38.7 |
| MemoryBank | 51.2 | 45.6 | 42.1 | 46.3 |
| A-MEM | 54.8 | 48.3 | 44.9 | 49.3 |
| **H-MEM** | **61.7** | **56.4** | **52.8** | **57.0** |

**Key Finding**: H-MEM outperforms all baselines across five task settings, with index-based routing eliminating the O(n) similarity search bottleneck.

### Connection to Complexity Trap

H-MEM validates that hierarchical compression is superior to flat summarization:
- **Multiple granularity levels** → Better information preservation
- **Index-based routing** → Efficient retrieval without exhaustive search
- **Semantic organization** → Natural compression boundaries

---

## 2. HiAgent: Subgoal-Based Working Memory

**Paper**: "HiAgent: Hierarchical Working Memory Management for Solving Long-Horizon Agent Tasks with Large Language Model" (Hu et al., ACL 2025)

### Core Concept

HiAgent uses subgoals as memory chunks, applying hierarchical compression at the working memory level. Current subgoal retains full detail; completed subgoals are compressed to summaries.

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    STANDARD APPROACH (Inefficient)                          │
│                                                                             │
│  Working Memory:                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [Turn 1] Action: search("API docs") → Observation: <1000 tokens>  │   │
│  │ [Turn 2] Action: read_file("config.py") → Observation: <500 tokens>│   │
│  │ [Turn 3] Action: edit_file("config.py") → Observation: <200 tokens>│   │
│  │ [Turn 4] Action: run_test("config") → Observation: <800 tokens>   │   │
│  │ ... (all history retained, growing linearly)                        │   │
│  │ [Turn N] → Context explosion                                          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Context Growth: Linear with trajectory length                              │
│  Memory Usage: O(N) turns                                                   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                      HIAGENT APPROACH (Hierarchical)                        │
│                                                                             │
│  Working Memory:                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ COMPLETED SUBGOALS (Compressed):                                     │   │
│  │ ┌───────────────────────────────────────────────────────────────┐   │   │
│  │ │ [Subgoal 1: "Find API configuration"]                         │   │   │
│  │ │   → Summary: "Found API key in config.py, base URL in env"    │   │   │
│  │ └───────────────────────────────────────────────────────────────┘   │   │
│  │                                                                     │   │
│  │ CURRENT SUBGOAL (Full Detail):                                      │   │
│  │ ┌───────────────────────────────────────────────────────────────┐   │   │
│  │ │ [Subgoal 2: "Implement authentication"]                         │   │   │
│  │ │   → Current: Full action-observation pairs (active)           │   │   │
│  │ │   → Action: edit_file("auth.py") → Observation: <400 tokens>   │   │   │
│  │ │   → Action: run_test("auth") → Observation: <300 tokens>      │   │   │
│  │ └───────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Context Growth: Bounded by active subgoal complexity                       │
│  Memory Usage: O(1) for completed, O(subgoal_size) for active             │
│                                                                             │
│  Result: Only current subgoal retains full detail                          │
│          Previous subgoals compressed to summaries                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Subgoal Detection Algorithm

```python
class HiAgentMemoryManager:
    """
    HiAgent: Hierarchical working memory based on subgoal completion.
    
    Uses subgoals as natural chunking boundaries, applying full-detail
    storage to active subgoals and compression to completed ones.
    """
    
    def __init__(self, llm_client, compression_threshold: int = 5):
        self.llm = llm_client
        self.compression_threshold = compression_threshold
        
        self.subgoals = []  # All subgoals (completed + current)
        self.current_subgoal = None
        self.subgoal_history = []
        
    def execute_turn(self, action: dict, observation: str) -> dict:
        """Execute one turn with hierarchical memory management."""
        
        # Add to current subgoal
        self.subgoal_history.append({
            'action': action,
            'observation': observation,
            'turn': len(self.subgoal_history)
        })
        
        # Check for subgoal completion
        completion_status = self._detect_subgoal_completion(
            self.current_subgoal, 
            self.subgoal_history
        )
        
        if completion_status['is_complete']:
            # Compress completed subgoal
            summary = self._compress_subgoal(self.subgoal_history)
            
            self.subgoals.append({
                'description': self.current_subgoal,
                'summary': summary,
                'turns': len(self.subgoal_history),
                'outcome': completion_status['outcome']
            })
            
            # Generate next subgoal
            self.current_subgoal = self._generate_next_subgoal(
                completion_status['outcome']
            )
            self.subgoal_history = []
            
        return self._build_context()
    
    def _detect_subgoal_completion(self, subgoal: str, history: list) -> dict:
        """
        Detect if current subgoal has been completed.
        
        Uses LLM to analyze trajectory and determine completion status.
        """
        prompt = f"""
        Analyze the following agent trajectory and determine if the subgoal
        has been completed, failed, or is still in progress.
        
        Subgoal: {subgoal}
        
        Recent Actions:
        {self._format_history(history[-5:])}
        
        Determine:
        1. Is the subgoal complete? (yes/no/partial)
        2. What was the outcome?
        3. What should be the next subgoal?
        
        Respond in JSON format.
        """
        
        response = self.llm.generate(prompt)
        return json.loads(response)
    
    def _compress_subgoal(self, history: list) -> str:
        """
        Compress completed subgoal history into summary.
        
        Preserves key outcomes while discarding detailed steps.
        """
        full_trajectory = self._format_full_history(history)
        
        prompt = f"""
        Summarize the following completed subgoal trajectory.
        Preserve:
        - What was accomplished
        - Key findings or outputs
        - Any failures or issues encountered
        
        Trajectory:
        {full_trajectory}
        
        Provide a concise 2-3 sentence summary.
        """
        
        return self.llm.generate(prompt)
    
    def _build_context(self) -> str:
        """Build hierarchical context for LLM."""
        parts = []
        
        # Add completed subgoal summaries
        if self.subgoals:
            parts.append("## Completed Subgoals\n")
            for sg in self.subgoals:
                parts.append(f"- {sg['description']}: {sg['summary']}\n")
        
        # Add current subgoal with full detail
        if self.current_subgoal:
            parts.append(f"\n## Current Subgoal: {self.current_subgoal}\n")
            parts.append("### Recent Actions:\n")
            for entry in self.subgoal_history[-self.compression_threshold:]:
                parts.append(self._format_turn(entry))
        
        return "\n".join(parts)
```

### Results on Long-Horizon Tasks

| Metric | Standard Agent | HiAgent | Improvement |
|--------|---------------:|--------:|------------:|
| Success Rate | 21% | 42% | **+100%** |
| Progress Rate | 44% | 68% | +54% |
| Avg Steps | 18.2 | 14.4 | -21% |
| Context Length | 100% (baseline) | 65% | **-35%** |
| Runtime | 100% (baseline) | 81% | -19% |

**Key Insight**: "Employing subgoals to compartmentalize action-observation pairs can be conceptualized as a form of chunking methodology" — inspired by cognitive science principles (Miller, 1956; Newell et al., 1972).

### Connection to Complexity Trap

HiAgent implements hierarchical compression at the working memory level:
- **Subgoal-based chunking** → Natural semantic boundaries
- **Full detail on active** → Maintains execution capability
- **Summary on completed** → Efficient long-horizon operation

---

## 3. Re-TRAC: Recursive Trajectory Compression

**Paper**: "RE-TRAC: REcursive TRAjectory Compression for Deep Search Agents" (Zhu et al., 2026, arXiv:2602.02486)

### Core Concept

Re-TRAC enables cross-trajectory exploration through structured state representations that preserve evidence, uncertainties, failures, and plans for globally informed reasoning across multiple attempts.

### Problem Addressed

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 TRADITIONAL REACT (Isolated Trajectories)                   │
│                                                                             │
│  Trajectory 1:                                                                │
│  [Attempt: search → read → edit → test] → Fail (local optimum)            │
│                                                                             │
│  Trajectory 2:                                                                │
│  [Attempt: search → read → edit → test] → Fail (repeats same mistakes)     │
│                                                                             │
│  Trajectory 3:                                                                │
│  [Attempt: search → read → edit → test] → Fail (no learning)               │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════  │
│  Result:                                                                      │
│  - No cross-trajectory knowledge transfer                                   │
│  - Redundant exploration across attempts                                     │
│  - Wasted computation on previously failed paths                             │
│  - Agent cannot "remember" what didn't work                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Re-TRAC Solution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                  RE-TRAC STRUCTURED STATE REPRESENTATION                      │
│                                                                             │
│  Round 1: Execute → Compress → State Representation                         │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ State Representation Structure:                                     │   │
│  │ {                                                                   │   │
│  │   "accumulated_evidence": [                                         │   │
│  │     {"fact": "Bug is in auth.py", "confidence": 0.95, "source": "T3"},│   │
│  │     {"fact": "Test fails with KeyError", "confidence": 1.0, "source": "T5"}│   │
│  │   ],                                                                  │   │
│  │   "unresolved_uncertainties": [                                     │   │
│  │     {"question": "Is the issue in token parsing?", "priority": "high"},│   │
│  │     {"question": "Should we use JWT or OAuth?", "priority": "medium"}│   │
│  │   ],                                                                  │   │
│  │   "identified_failures": [                                          │   │
│  │     {"approach": "Adding null check", "reason": "Didn't fix KeyError"}, │   │
│  │     {"approach": "Changing exception handler", "reason": "Wrong location"}│   │
│  │   ],                                                                  │   │
│  │   "forward_plan": [                                                   │   │
│  │     {"step": 1, "action": "Check token parsing logic"},               │   │
│  │     {"step": 2, "action": "Verify key existence before access"}        │   │
│  │   ],                                                                  │   │
│  │   "incomplete_branches": [                                            │   │
│  │     {"branch": "OAuth implementation", "resumable_from": "T7"}         │   │
│  │   ]                                                                     │   │
│  │ }                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Round 2: State Representation + New Query → Execute                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ - Avoids previously failed paths ("Adding null check" → skip)        │   │
│  │ - Prioritizes unresolved uncertainties (token parsing → high priority) │   │
│  │ - Continues incomplete branches (resume OAuth exploration)            │   │
│  │ - Globally informed planning (not just local context)                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Result: Cross-trajectory learning, resumable exploration                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Structured Compression Specification

| Component | Content | Purpose |
|-----------|---------|---------|
| **Accumulated Evidence** | Verified facts, confirmed data | Build knowledge base |
| **Unresolved Uncertainties** | Open questions, ambiguities | Guide exploration priority |
| **Identified Failures** | Failed approaches with reasons | Avoid repetition |
| **Forward Plan** | Forward-looking strategy | Guide next actions |
| **Incomplete Branches** | Unfinished search paths | Resume exploration |

### Algorithm

```python
class ReTRACCompressor:
    """
    Re-TRAC: Recursive trajectory compression with structured state.
    
    Preserves exploration state across trajectories, enabling
    cross-attempt learning and resumable search.
    """
    
    def __init__(self, llm_client):
        self.llm = llm_client
        self.state_schema = {
            "accumulated_evidence": [],
            "unresolved_uncertainties": [],
            "identified_failures": [],
            "forward_plan": [],
            "incomplete_branches": []
        }
    
    def compress_trajectory(self, trajectory: list, previous_state: dict = None) -> dict:
        """
        Compress trajectory into structured state representation.
        
        Args:
            trajectory: List of turns from this attempt
            previous_state: State from previous attempts (if any)
            
        Returns:
            Structured state for next attempt
        """
        trajectory_text = self._format_trajectory(trajectory)
        previous_state_text = json.dumps(previous_state, indent=2) if previous_state else "None"
        
        prompt = f"""
        Analyze the following agent trajectory and extract structured state.
        If previous state is provided, merge new learnings with existing knowledge.
        
        Previous State:
        {previous_state_text}
        
        Current Trajectory:
        {trajectory_text}
        
        Extract and return JSON with these fields:
        1. accumulated_evidence: Verified facts with confidence scores
        2. unresolved_uncertainties: Open questions prioritized by importance
        3. identified_failures: What was tried and why it failed
        4. forward_plan: Next steps based on current state
        5. incomplete_branches: Exploration paths to resume
        
        Ensure:
        - Evidence has source citations (turn numbers)
        - Uncertainties are actionable
        - Failures include specific reasons
        - Plan is concrete and executable
        - Branches specify resumption points
        """
        
        response = self.llm.generate(prompt, response_format="json")
        return json.loads(response)
    
    def build_initial_prompt(self, task: str, state: dict) -> str:
        """Build agent prompt incorporating structured state."""
        parts = [
            f"Task: {task}\n",
            "## Previous Attempt Analysis\n"
        ]
        
        if state['accumulated_evidence']:
            parts.append("### Verified Facts:\n")
            for ev in state['accumulated_evidence']:
                parts.append(f"- {ev['fact']} (confidence: {ev['confidence']})\n")
        
        if state['identified_failures']:
            parts.append("\n### Approaches That Didn't Work:\n")
            for fail in state['identified_failures']:
                parts.append(f"- ❌ {fail['approach']}: {fail['reason']}\n")
        
        if state['unresolved_uncertainties']:
            parts.append("\n### Priority Questions:\n")
            for unc in state['unresolved_uncertainties']:
                parts.append(f"- ❓ [{unc['priority']}] {unc['question']}\n")
        
        if state['forward_plan']:
            parts.append("\n### Suggested Next Steps:\n")
            for step in state['forward_plan']:
                parts.append(f"{step['step']}. {step['action']}\n")
        
        if state['incomplete_branches']:
            parts.append("\n### Exploration to Resume:\n")
            for branch in state['incomplete_branches']:
                parts.append(f"- {branch['branch']} (from turn {branch['resumable_from']})\n")
        
        parts.append("\n## Current Attempt\n")
        parts.append("Proceed with the task, avoiding previously failed approaches.\n")
        
        return "".join(parts)
```

### Results on BrowseComp

| Model | Baseline ReAct | Re-TRAC | Improvement |
|-------|---------------:|--------:|------------:|
| GPT-4o | 15.2% | 18.1% | +2.9 pp |
| Claude-3.5-Sonnet | 14.8% | 17.9% | +3.1 pp |
| Llama-3.1-70B | 8.3% | 10.1% | +1.8 pp |
| RE-TRAC-30B-A3B | 16.2% | 19.5% | +3.3 pp |
| RE-TRAC-4B | 9.1% | 11.2% | +2.1 pp |

**Key Achievement**: Monotonic reduction in tool calls and token usage across rounds — progressively targeted exploration.

### Connection to Complexity Trap

Re-TRAC addresses trajectory elongation by enabling early stopping through better state tracking:
- **Structured state representation** → Better than simple summarization
- **Cross-trajectory learning** → Avoids redundant exploration
- **Explicit failure tracking** → Prevents repeated mistakes

---

## 4. CASK: Saliency-Based KV Compression

**Paper**: "Context Adaptive Memory-Efficient LLM Inference for Edge Multi-Agent Systems" (Mohammed et al., AAMAS 2025)

### Core Concept

CASK combines dynamic sparse attention with adaptive KV-cache compression at inference time, tracking recency, frequency, and attention allocation to optimize memory usage.

### Two-Component Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        CASK ARCHITECTURE                                    │
│                                                                             │
│  Component 1: Dynamic Sparse Attention                                     │
│  ─────────────────────────────────────                                     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Mask Generation Module (MGM): Small vision transformer              │   │
│  │                                                                     │   │
│  │ Input: Query + Key tensors                                          │   │
│  │   ↓                                                                  │   │
│  │ MGM: Meta-learned sparse binary mask generation                     │   │
│  │   ↓                                                                  │   │
│  │ Output: Binary mask eliminating low-importance token interactions   │   │
│  │                                                                     │   │
│  │ Result: Reduces attention computation without architecture changes   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Component 2: Adaptive KV-Cache Compression                                │
│  ────────────────────────────────────────────                              │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Saliency Metrics (tracked per KV pair):                             │   │
│  │   • Recency: Time since last access                                 │   │
│  │   • Access frequency: How often used                                │   │
│  │   • Attention allocation: Cumulative attention weight               │   │
│  │                                                                     │   │
│  │ Saliency Score: f(recency, frequency, attention)                     │   │
│  │                                                                     │   │
│  │ Dynamic Actions:                                                    │   │
│  │   • Moderate salience → Lower bit-width quantization                │   │
│  │   • Low salience → Prune from cache                                 │   │
│  │   • High salience → Retain full precision                           │   │
│  │                                                                     │   │
│  │ Thresholds: Validated via runtime reconstruction loss               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Saliency Tracking Algorithm

```python
class CASKCompressor:
    """
    CASK: Context Adaptive Sparse Key-value compression.
    
    Combines sparse attention with adaptive KV-cache compression
    based on multi-factor saliency metrics.
    """
    
    def __init__(self, 
                 model,
                 compression_ratio: float = 0.4,
                 reconstruction_threshold: float = 0.95):
        self.model = model
        self.compression_ratio = compression_ratio
        self.reconstruction_threshold = reconstruction_threshold
        
        # KV cache metadata
        self.kv_metadata = {}  # position -> SaliencyMetrics
        
        # MGM (Mask Generation Module)
        self.mask_generator = VisionTransformerMaskGen()
        
    def compute_saliency(self, position: int, 
                         current_layer: int,
                         attention_weights: torch.Tensor) -> dict:
        """
        Compute saliency score for KV pair at given position.
        
        Saliency = weighted combination of:
        - Recency (newer = more salient)
        - Frequency (accessed often = more salient)
        - Attention (high attention weight = more salient)
        """
        metadata = self.kv_metadata.get(position, {
            'last_access': 0,
            'access_count': 0,
            'cumulative_attention': 0.0
        })
        
        current_step = len(self.kv_metadata)
        
        # Recency score (exponential decay)
        recency = exp(-0.1 * (current_step - metadata['last_access']))
        
        # Frequency score (normalized)
        frequency = min(1.0, metadata['access_count'] / 10.0)
        
        # Attention score (from current forward pass)
        attention = attention_weights.mean().item()
        metadata['cumulative_attention'] += attention
        normalized_attention = min(1.0, metadata['cumulative_attention'] / 5.0)
        
        # Combined saliency
        saliency = (
            0.4 * recency +
            0.3 * frequency +
            0.3 * normalized_attention
        )
        
        # Update metadata
        metadata['last_access'] = current_step
        metadata['access_count'] += 1
        self.kv_metadata[position] = metadata
        
        return {
            'saliency': saliency,
            'recency': recency,
            'frequency': frequency,
            'attention': normalized_attention
        }
    
    def compress_kv_cache(self, k_cache: torch.Tensor, 
                          v_cache: torch.Tensor) -> tuple:
        """
        Apply adaptive compression to KV cache.
        
        Strategy:
        - High saliency (>0.7): Retain full precision
        - Moderate saliency (0.3-0.7): Dynamic quantization
        - Low saliency (<0.3): Prune
        """
        batch_size, num_heads, seq_len, head_dim = k_cache.shape
        
        # Compute saliency for each position
        saliencies = []
        for pos in range(seq_len):
            # Get attention weights for this position
            attn = self._get_attention_for_position(pos)
            sal = self.compute_saliency(pos, 0, attn)
            saliencies.append(sal['saliency'])
        
        saliencies = torch.tensor(saliencies)
        
        # Determine compression strategy per position
        high_mask = saliencies > 0.7
        moderate_mask = (saliencies > 0.3) & (saliencies <= 0.7)
        low_mask = saliencies <= 0.3
        
        # Apply compression
        k_compressed = k_cache.clone()
        v_compressed = v_cache.clone()
        
        # High saliency: keep as-is (full precision)
        
        # Moderate saliency: dynamic quantization to 4-bit
        if moderate_mask.any():
            k_compressed[:, :, moderate_mask, :] = self._quantize_4bit(
                k_compressed[:, :, moderate_mask, :]
            )
            v_compressed[:, :, moderate_mask, :] = self._quantize_4bit(
                v_compressed[:, :, moderate_mask, :]
            )
        
        # Low saliency: prune (set to special token or remove)
        if low_mask.any():
            # Mark for pruning or use ultra-low precision
            k_compressed[:, :, low_mask, :] = 0
            v_compressed[:, :, low_mask, :] = 0
        
        # Validate compression quality
        reconstruction_score = self._validate_compression(
            (k_cache, v_cache),
            (k_compressed, v_compressed)
        )
        
        if reconstruction_score < self.reconstruction_threshold:
            # Compression too aggressive, adjust thresholds
            self._adjust_compression_thresholds(reconstruction_score)
        
        return k_compressed, v_compressed
    
    def generate_sparse_mask(self, query: torch.Tensor, 
                             keys: torch.Tensor) -> torch.Tensor:
        """
        Generate sparse attention mask using MGM.
        
        Returns binary mask where 1 = keep attention, 0 = mask out
        """
        # Stack query and keys as "image" for vision transformer
        combined = torch.stack([query, keys], dim=-1)  # [B, H, S, D, 2]
        
        # MGM generates sparse binary mask
        mask_logits = self.mask_generator(combined)
        binary_mask = (mask_logits > 0).float()
        
        return binary_mask
```

### Results on LongBench

| Model | Score | Memory (GB) | Relative Inference Time |
|-------|-------|-------------|------------------------:|
| LLaMA-3.2-90B-128k | 63.2 | 72 | 1.00x |
| H2O | 57.2 | **40** | 0.88x |
| StreamingLLMs | 37.1 | 40.8 | 0.88x |
| **CASK** | **61.5** | **44.5** | **0.77x** |

**Key Achievement**: Maintains 95%+ of baseline accuracy while cutting memory usage by up to 40% and boosting inference speed by 20%.

### Connection to Complexity Trap

CASK provides concrete techniques for adaptive threshold management:
- **Saliency-based compression** → Principled alternative to fixed thresholds
- **Runtime validation** → Prevents over-compression
- **Multi-factor scoring** → More nuanced than simple recency

---

## 5. ACE: Agentic Context Engineering

**Paper**: "Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models" (Zhang et al., ICLR 2026, arXiv:2510.04618)

### Core Concept

ACE treats contexts as evolving playbooks that accumulate, refine, and organize strategies through modular generation, reflection, and curation — preventing "context collapse" and "brevity bias."

### Problems Addressed

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              CONTEXT COLLAPSE (Traditional Methods)                       │
│                                                                             │
│  Iteration 1: Comprehensive context (500 tokens)                              │
│    → Accuracy: 70%                                                            │
│                                                                             │
│  Iteration 2: "Optimized" context (300 tokens) - drops details            │
│    → Accuracy: 68%                                                            │
│                                                                             │
│  Iteration 3: "Streamlined" context (150 tokens) - erodes key info          │
│    → Accuracy: 57% (worse than baseline!)                                     │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════  │
│  Result: Progressive degradation through aggressive compression             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                        ACE SOLUTION                                         │
│                                                                             │
│  ACE Framework:                                                             │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 1. Generator: Creates candidate context additions                   │   │
│  │    • New strategies discovered during execution                     │   │
│  │    • Failure patterns identified                                     │   │
│  │    • Successful tactics recorded                                     │   │
│  │                                                                     │   │
│  │ 2. Reflector: Evaluates and critiques                                │   │
│  │    • Validates factual accuracy                                      │   │
│  │    • Checks for redundancy                                           │   │
│  │    • Identifies gaps                                                 │   │
│  │                                                                     │   │
│  │ 3. Curator: Maintains structured playbook                            │   │
│  │    • Incremental updates (NOT full rewrites)                        │   │
│  │    • Preserves detailed knowledge                                    │   │
│  │    • Organizes by strategy type                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Key Principles:                                                            │
│  • Structured, incremental updates that preserve detailed knowledge         │
│  • NO full-context rewriting — appends to existing strategies                 │
│  • Execution feedback-driven — doesn't require labeled supervision            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Modular Curation Algorithm

```python
class ACEContextEngine:
    """
    ACE: Agentic Context Engineering with modular curation.
    
    Evolves contexts through generation, reflection, and curation
    without full-context rewriting.
    """
    
    def __init__(self, llm_client):
        self.llm = llm_client
        
        # Structured playbook organized by strategy type
        self.playbook = {
            'task_strategies': [],  # High-level approaches
            'failure_patterns': [],  # Known pitfalls
            'successful_tactics': [],  # Verified solutions
            'domain_knowledge': []  # Persistent facts
        }
        
        self.pending_additions = []
        
    def process_trajectory(self, trajectory: list, outcome: dict):
        """
        Process completed trajectory to update context playbook.
        
        Three-stage pipeline: Generate → Reflect → Curate
        """
        # Stage 1: Generate candidate additions
        candidates = self._generate_candidates(trajectory, outcome)
        
        # Stage 2: Reflect on candidates
        validated_candidates = self._reflect_on_candidates(
            candidates, self.playbook
        )
        
        # Stage 3: Curate playbook
        self._curate_playbook(validated_candidates)
        
        return self._build_context()
    
    def _generate_candidates(self, trajectory: list, outcome: dict) -> list:
        """
        Generator: Create candidate additions from trajectory.
        """
        trajectory_text = self._format_trajectory(trajectory)
        
        prompt = f"""
        Analyze this completed agent trajectory and extract learnings.
        
        Outcome: {'Success' if outcome['success'] else 'Failure'}
        Final State: {outcome['description']}
        
        Trajectory:
        {trajectory_text}
        
        Generate candidate additions for the playbook:
        1. TASK_STRATEGIES: What high-level approach was taken?
        2. FAILURE_PATTERNS: What went wrong or nearly went wrong?
        3. SUCCESSFUL_TACTICS: What specific actions worked well?
        4. DOMAIN_KNOWLEDGE: What facts were learned about the codebase?
        
        For each candidate, provide:
        - content: The specific learning
        - confidence: 0-1 score based on evidence quality
        - source: Which turns support this
        """
        
        response = self.llm.generate(prompt, response_format="json")
        return json.loads(response)['candidates']
    
    def _reflect_on_candidates(self, candidates: list, 
                                playbook: dict) -> list:
        """
        Reflector: Evaluate candidates against existing playbook.
        """
        playbook_text = json.dumps(playbook, indent=2)
        candidates_text = json.dumps(candidates, indent=2)
        
        prompt = f"""
        Evaluate these candidate playbook additions against the existing playbook.
        
        Existing Playbook:
        {playbook_text}
        
        Candidates:
        {candidates_text}
        
        For each candidate, determine:
        1. is_novel: Does this add new information? (avoid redundancy)
        2. is_accurate: Is this factually correct?
        3. is_actionable: Is this useful for future tasks?
        4. replaces_existing: Should this update an existing entry?
        5. final_decision: ACCEPT / REJECT / MERGE / UPDATE
        
        Be critical - prefer rejecting borderline candidates over
        accepting potentially incorrect information.
        """
        
        response = self.llm.generate(prompt, response_format="json")
        evaluations = json.loads(response)['evaluations']
        
        # Filter to accepted candidates
        validated = []
        for candidate, eval in zip(candidates, evaluations):
            if eval['final_decision'] == 'ACCEPT':
                candidate['evaluation'] = eval
                validated.append(candidate)
            elif eval['final_decision'] == 'UPDATE':
                self._update_existing_entry(eval['existing_id'], candidate)
        
        return validated
    
    def _curate_playbook(self, validated_candidates: list):
        """
        Curator: Incrementally update playbook (NO full rewrites).
        """
        for candidate in validated_candidates:
            category = candidate['category']
            
            # Append to appropriate section (never rewrite)
            self.playbook[category].append({
                'content': candidate['content'],
                'confidence': candidate['confidence'],
                'timestamp': time.time(),
                'usage_count': 0
            })
        
        # Optional: Prune low-confidence, unused entries
        self._prune_playbook()
    
    def _build_context(self) -> str:
        """Build context from curated playbook."""
        parts = ["## Context Playbook\n"]
        
        # Include high-confidence entries from each category
        for category, entries in self.playbook.items():
            if entries:
                parts.append(f"\n### {category.replace('_', ' ').title()}\n")
                
                # Sort by confidence × recency
                sorted_entries = sorted(
                    entries,
                    key=lambda e: e['confidence'] * (1 / (1 + time.time() - e['timestamp'])),
                    reverse=True
                )
                
                for entry in sorted_entries[:5]:  # Top 5 per category
                    parts.append(f"- {entry['content']}\n")
                    entry['usage_count'] += 1
        
        return "".join(parts)
```

### Results on AppWorld

| Setting | Baseline | ACE | Improvement |
|---------|---------:|----:|------------:|
| Agent Tasks (AppWorld) | 51.9% | 59.5% | **+10.6%** |
| Domain Tasks (Finance) | 69.5% | 76.5% | **+8.6%** |
| Adaptation Latency | High | Low | Faster convergence |
| Rollout Cost | High | Low | More efficient |

**Notable Achievement**: Matches top-1 ranked production-level agent (IBM-CUGA with GPT-4.1) using a smaller open-source model (DeepSeek-V3.1).

### Connection to Complexity Trap

ACE validates that adaptive thresholds and semantic triggers are essential:
- **Modular curation** → Principled implementation of adaptive strategy
- **Incremental updates** → Avoids information loss from compression
- **Execution feedback** → Real-time adaptation without supervision

---

## 6. G-Memory: Three-Tier Graph for Multi-Agent Systems

**Paper**: "G-Memory: Tracing Hierarchical Memory for Multi-Agent Systems" (Zhang et al., NeurIPS 2025, arXiv:2506.07398)

### Core Concept

G-Memory introduces a three-tier graph hierarchy (insight, query, interaction) for efficient context sharing in multi-agent systems, enabling cross-trial learning and agent-specific customization.

### Problem with Current MAS Memory

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              CURRENT MAS MEMORY LIMITATIONS                                 │
│                                                                             │
│  1. Overly simplistic — disregards nuanced inter-agent                       │
│     collaboration trajectories                                               │
│                                                                             │
│  2. Lacks cross-trial and agent-specific customization                      │
│     (unlike expressive single-agent memory)                                  │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════  │
│  Result: Poor self-evolution capability in multi-agent teams                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### G-Memory Three-Tier Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              G-MEMORY ARCHITECTURE                                          │
│        (Inspired by Organizational Memory Theory)                         │
│                                                                             │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  TIER 1: INSIGHT GRAPH (High-Level)                                  │   │
│  │  ─────────────────────────────────                                   │   │
│  │  • Generalizable insights across trials                              │   │
│  │  • Strategic patterns and principles                                 │   │
│  │  • Cross-domain knowledge                                            │   │
│  │                                                                     │   │
│  │  Nodes: Abstract concepts                                            │   │
│  │  Edges: Causal/associative relationships                             │   │
│  │                                                                     │   │
│  │       ▲                                                                │   │
│  │       │ Top-Down (General → Specific)                                │   │
│  │       │ "What strategies work for this type of problem?"               │   │
│  │       ▼                                                                │   │
│  │                                                                     │   │
│  │  TIER 2: QUERY GRAPH (Mid-Level)                                     │   │
│  │  ────────────────────────────────                                    │   │
│  │  • Task-specific query patterns                                      │   │
│  │  • Problem decomposition strategies                                  │   │
│  │  • Solution approaches                                               │   │
│  │                                                                     │   │
│  │  Nodes: Query types                                                  │   │
│  │  Edges: Dependency relationships                                     │   │
│  │                                                                     │   │
│  │       ▲                                                                │   │
│  │       │ "How was this type of problem handled before?"               │   │
│  │       ▼                                                                │   │
│  │                                                                     │   │
│  │  TIER 3: INTERACTION GRAPH (Fine-Grained)                            │   │
│  │  ────────────────────────────────────                                │   │
│  │  • Condensed interaction trajectories                                │   │
│  │  • Agent-to-agent communication patterns                               │   │
│  │  • Execution details                                                 │   │
│  │                                                                     │   │
│  │  Nodes: Individual interactions                                      │   │
│  │  Edges: Temporal/sequential relationships                              │   │
│  │                                                                     │   │
│  │       │ Bottom-Up (Specific → General)                                │   │
│  │       │ "What exactly was done last time?"                           │   │
│  │       ▼                                                                │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Bi-Directional Traversal: Top-Down + Bottom-Up retrieval                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Bi-Directional Traversal Algorithm

```python
class GMemoryMAS:
    """
    G-Memory: Three-tier graph hierarchy for multi-agent systems.
    
    Enables efficient context sharing and cross-trial learning
    through bi-directional graph traversal.
    """
    
    def __init__(self, num_agents: int):
        self.num_agents = num_agents
        
        # Three-tier graph structure
        self.insight_graph = nx.DiGraph()  # Tier 1: High-level
        self.query_graph = nx.DiGraph()     # Tier 2: Mid-level
        self.interaction_graph = nx.DiGraph()  # Tier 3: Fine-grained
        
        # Cross-tier mappings
        self.insight_to_query = defaultdict(list)
        self.query_to_interaction = defaultdict(list)
        
    def store_collaboration(self, trajectory: list, agent_assignments: dict,
                           outcome: dict):
        """
        Store multi-agent collaboration into three-tier hierarchy.
        """
        # Tier 3: Store interaction graph
        interaction_nodes = self._build_interaction_graph(
            trajectory, agent_assignments
        )
        
        # Tier 2: Extract query patterns
        query_nodes = self._extract_query_patterns(
            interaction_nodes, outcome
        )
        
        # Tier 1: Extract insights
        insight_nodes = self._extract_insights(query_nodes, outcome)
        
        # Create cross-tier mappings
        self._link_tiers(insight_nodes, query_nodes, interaction_nodes)
        
    def retrieve_for_query(self, query: str, agent_id: int = None,
                          strategy: str = "balanced") -> dict:
        """
        Retrieve relevant context using bi-directional traversal.
        
        Args:
            query: The new query to process
            agent_id: Specific agent requesting context (for customization)
            strategy: "top_down", "bottom_up", or "balanced"
        """
        results = {
            'insights': [],
            'query_patterns': [],
            'interactions': []
        }
        
        if strategy == "top_down":
            # Start from insights, drill down
            insights = self._search_insights(query, top_k=3)
            for insight in insights:
                results['insights'].append(insight)
                # Follow edges to query patterns
                for query_id in self.insight_to_query[insight['id']]:
                    query_node = self.query_graph.nodes[query_id]
                    results['query_patterns'].append(query_node)
                    # Follow to interactions
                    for int_id in self.query_to_interaction[query_id]:
                        results['interactions'].append(
                            self.interaction_graph.nodes[int_id]
                        )
                        
        elif strategy == "bottom_up":
            # Start from interactions, climb up
            interactions = self._search_interactions(query, top_k=5)
            for interaction in interactions:
                results['interactions'].append(interaction)
                # Climb to parent queries
                for query_id in interaction.get('parent_queries', []):
                    query_node = self.query_graph.nodes[query_id]
                    results['query_patterns'].append(query_node)
                    # Climb to insights
                    for insight_id in query_node.get('parent_insights', []):
                        results['insights'].append(
                            self.insight_graph.nodes[insight_id]
                        )
                        
        else:  # balanced
            # Combine both approaches
            td_results = self.retrieve_for_query(query, agent_id, "top_down")
            bu_results = self.retrieve_for_query(query, agent_id, "bottom_up")
            # Merge and deduplicate
            results = self._merge_results(td_results, bu_results)
        
        # Agent-specific filtering
        if agent_id is not None:
            results = self._filter_for_agent(results, agent_id)
        
        return results
    
    def _build_interaction_graph(self, trajectory: list, 
                                  assignments: dict) -> list:
        """Build Tier 3: Fine-grained interaction graph."""
        nodes = []
        
        for i, turn in enumerate(trajectory):
            agent = assignments.get(i, 'unknown')
            node_id = f"int_{int(time.time())}_{i}"
            
            self.interaction_graph.add_node(
                node_id,
                turn=i,
                agent=agent,
                action=turn['action'],
                observation=turn['observation'],
                timestamp=turn.get('timestamp')
            )
            
            # Add temporal edges
            if i > 0:
                prev_id = nodes[-1]['id']
                self.interaction_graph.add_edge(prev_id, node_id, type='next')
            
            nodes.append({'id': node_id, 'agent': agent})
        
        return nodes
    
    def _extract_query_patterns(self, interaction_nodes: list, 
                                outcome: dict) -> list:
        """Extract Tier 2: Query patterns from interactions."""
        # Use LLM to identify query patterns
        trajectory_summary = self._summarize_trajectory(interaction_nodes)
        
        prompt = f"""
        Identify the high-level query/task patterns in this trajectory.
        
        Trajectory Summary: {trajectory_summary}
        Outcome: {outcome}
        
        Extract query patterns like:
        - Problem decomposition approach
        - Information retrieval strategy
        - Coordination protocol used
        
        Return 2-3 patterns with confidence scores.
        """
        
        patterns = self.llm.generate(prompt, response_format="json")
        
        nodes = []
        for pattern in patterns['patterns']:
            node_id = f"query_{int(time.time())}_{len(self.query_graph.nodes)}"
            self.query_graph.add_node(node_id, **pattern)
            nodes.append({'id': node_id, 'pattern': pattern['type']})
        
        return nodes
    
    def _extract_insights(self, query_nodes: list, outcome: dict) -> list:
        """Extract Tier 1: Insights from query patterns."""
        # Identify generalizable insights
        query_patterns = [self.query_graph.nodes[q['id']] for q in query_nodes]
        
        prompt = f"""
        Identify generalizable insights from these query patterns.
        
        Query Patterns: {query_patterns}
        Outcome: {outcome}
        
        Extract insights like:
        - Strategic principles that generalize
        - Cross-domain patterns
        - Meta-learnings about problem-solving
        
        Return 1-2 high-level insights.
        """
        
        insights = self.llm.generate(prompt, response_format="json")
        
        nodes = []
        for insight in insights['insights']:
            node_id = f"insight_{int(time.time())}_{len(self.insight_graph.nodes)}"
            self.insight_graph.add_node(node_id, **insight)
            nodes.append({'id': node_id, 'insight': insight['content']})
        
        return nodes
```

### Results

| Domain | Metric | Improvement |
|--------|--------|------------:|
| Embodied Action | Success Rate | **+20.89%** |
| Knowledge QA | Accuracy | **+10.12%** |

Tested across five benchmarks, three LLM backbones, and three MAS frameworks — without any modifications to the original frameworks.

### Connection to Complexity Trap

G-Memory provides a concrete implementation for multi-agent context sharing:
- **Three-tier hierarchy** → Balances detail and compression
- **Bi-directional traversal** → Efficient retrieval
- **Cross-trial learning** → Agents improve with experience

---

## Comparative Summary

| Strategy | Core Innovation | Compression Level | Best Use Case |
|----------|--------------|------------------:|---------------|
| **H-MEM** | Index-based hierarchical routing | 3 levels | Long-horizon reasoning |
| **HiAgent** | Subgoal-based working memory | Subgoal-level | Structured tasks |
| **Re-TRAC** | Structured state representation | Evidence/uncertainty/failure | Multi-attempt problems |
| **CASK** | Saliency-based KV compression | Per-token variable | Inference optimization |
| **ACE** | Modular curation (no rewriting) | Semantic categories | Self-improving agents |
| **G-Memory** | Three-tier graph hierarchy | 3 tiers | Multi-agent systems |

---

## Connection to Core Research

These 2025 advances validate and extend the Complexity Trap findings:

1. **Hierarchical > Flat**: H-MEM and HiAgent demonstrate that multi-level compression outperforms flat summarization
2. **Structured > Unstructured**: Re-TRAC's state representation preserves more actionable information than text summaries
3. **Adaptive > Fixed**: CASK and ACE validate that adaptive thresholds (vs. fixed M/N) are essential
4. **Specialized > Generic**: SMART and G-Memory show that context management should vary by agent role

The hybrid approach from the Complexity Trap can integrate these advances:
- Use HiAgent-style subgoal detection for semantic boundaries
- Apply H-MEM's index-based routing for efficient retrieval
- Incorporate CASK's saliency metrics for adaptive thresholds
- Use ACE's modular curation for strategy evolution

---

## References

1. Sun & Zeng, "Hierarchical Memory for High-Efficiency Long-Term Reasoning in LLM Agents," arXiv:2507.22925
2. Hu et al., "HiAgent: Hierarchical Working Memory Management for Solving Long-Horizon Agent Tasks with Large Language Model," ACL 2025
3. Zhu et al., "RE-TRAC: REcursive TRAjectory Compression for Deep Search Agents," arXiv:2602.02486
4. Mohammed et al., "Context Adaptive Memory-Efficient LLM Inference for Edge Multi-Agent Systems," AAMAS 2025
5. Zhang et al., "Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models," ICLR 2026, arXiv:2510.04618
6. Zhang et al., "G-Memory: Tracing Hierarchical Memory for Multi-Agent Systems," NeurIPS 2025, arXiv:2506.07398

---

*Next: [Semantic Triggers](04-semantic-triggers.md)*
