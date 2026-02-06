# ACON: Training-Time Context Compression for Long-Horizon Agents

## Overview

ACON (Agent Context Optimization) is a unified framework for optimally compressing both environment observations and interaction histories for LLM agents. Unlike inference-time strategies (observation masking, LLM summarization), ACON optimizes compression guidelines through training-time failure analysis, producing concise yet informative context that reduces peak tokens by 26–54% while preserving or improving task performance.

**Paper**: "ACON: Optimizing Context Compression for Long-horizon LLM Agents" (Kang et al., 2025)  
**Authors**: Minki Kang, Wei-Ning Chen, Dongge Han, Huseyin A. Inan, Lukas Wutschitz, Yanzhi Chen, Robert Sim, Saravan Rajmohan  
**Institution**: Microsoft Research  
**Published**: October 2025, [arXiv:2510.00615](https://arxiv.org/abs/2510.00615)  
**Code**: [github.com/microsoft/acon](https://github.com/microsoft/acon)

---

## Core Concept

ACON frames context compression as a guideline optimization problem in natural language space. Rather than hand-crafting compression rules or relying on generic summarization prompts, ACON learns what information to preserve by analyzing cases where compression causes agent failures.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      ACON vs. EXISTING APPROACHES                           │
│                                                                             │
│  Observation Masking (Complexity Trap):                                      │
│  ─────────────────────────────────────                                       │
│  • Hides old observations with placeholders                                  │
│  • No learning — fixed rule (window size M)                                 │
│  • No additional LLM calls                                                   │
│  • Effective but blind to information importance                            │
│                                                                             │
│  LLM Summarization (Complexity Trap):                                        │
│  ─────────────────────────────────────                                       │
│  • Compresses via generic summary prompt                                    │
│  • No learning — fixed summarization prompt                                 │
│  • Causes trajectory elongation (+15-18%)                                   │
│  • Risk of smoothing over failure signals                                   │
│                                                                             │
│  ACON (Training-Time Optimization):                                          │
│  ──────────────────────────────────                                          │
│  • Learns WHAT to preserve through contrastive failure analysis             │
│  • Optimized guideline in natural language space                             │
│  • Distillable into small models (95%+ accuracy preserved)                  │
│  • Task-aware compression — preserves critical information                  │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════   │
│  Key Insight: Compression quality depends on WHAT you preserve,             │
│               not HOW MUCH you compress                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Architecture

### Compression Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        ACON COMPRESSION PIPELINE                            │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                     TRAINING PHASE                                    │  │
│  │                                                                       │  │
│  │  Step 1: Baseline Collection                                          │  │
│  │  ─────────────────────────────                                        │  │
│  │  Run agent on training tasks WITHOUT compression                      │  │
│  │  → Record successes (full context trajectories)                       │  │
│  │                                                                       │  │
│  │  Step 2: Contrastive Failure Collection                               │  │
│  │  ─────────────────────────────────────                                │  │
│  │  Run agent on SAME tasks WITH current compression guideline           │  │
│  │  → Identify tasks: baseline succeeds, compressed fails                │  │
│  │                                                                       │  │
│  │  Step 3: Failure Analysis (Contrastive Feedback)                      │  │
│  │  ───────────────────────────────────────────────                      │  │
│  │  Capable LLM (o3) compares paired trajectories:                       │  │
│  │    "What critical information did compression lose?"                   │  │
│  │    "Why did the compressed agent fail where full context succeeded?"   │  │
│  │  → Generates natural language feedback                                │  │
│  │                                                                       │  │
│  │  Step 4: Guideline Update                                             │  │
│  │  ────────────────────────                                              │  │
│  │  LLM optimizer refines compression guideline using feedback           │  │
│  │  → Multiple candidates generated, best selected on held-out set       │  │
│  │                                                                       │  │
│  │  Repeat Steps 2-4 for R rounds                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│                              │                                               │
│                              ▼                                               │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                     INFERENCE PHASE                                    │  │
│  │                                                                       │  │
│  │  Agent Loop:                                                          │  │
│  │  ┌─────────┐    ┌──────────────┐    ┌───────────────────────────┐    │  │
│  │  │  Agent   │───▶│  Environment │───▶│  Compressor               │    │  │
│  │  │  (LLM)  │    │  (tool exec) │    │  (optimized guideline)    │    │  │
│  │  └────▲────┘    └──────────────┘    │                           │    │  │
│  │       │                              │  if tokens > threshold:   │    │  │
│  │       │                              │    compress(history)       │    │  │
│  │       │                              │  if obs > threshold:       │    │  │
│  │       │                              │    compress(observation)   │    │  │
│  │       │         ┌──────────────┐    └───────────┬───────────────┘    │  │
│  │       └─────────┤  Compressed  │◄───────────────┘                    │  │
│  │                 │  Context     │                                      │  │
│  │                 └──────────────┘                                      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Two-Stage Optimization

ACON uses an alternating optimization that separates task performance from compression efficiency:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                  ALTERNATING GUIDELINE OPTIMIZATION                         │
│                                                                             │
│  Stage A: Utility Maximization (Reward-First)                               │
│  ──────────────────────────────────────────────                              │
│  Goal: Maximize task success under compression                              │
│                                                                             │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐              │
│  │ Full Context │      │ Compressed   │      │ Contrastive  │              │
│  │ (succeeds)   │─────▶│ (fails)      │─────▶│ Feedback     │              │
│  └──────────────┘      └──────────────┘      └──────┬───────┘              │
│                                                      │                      │
│                                                      ▼                      │
│                                              ┌──────────────┐              │
│                                              │ Update g     │              │
│                                              │ to preserve  │              │
│                                              │ critical info│              │
│                                              └──────────────┘              │
│                                                                             │
│  Stage B: Compression Maximization (Cost-Second)                            │
│  ────────────────────────────────────────────────                            │
│  Goal: Maximize compression ratio while preserving accuracy                 │
│                                                                             │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐              │
│  │ Current g_U  │      │ Cost-aware   │      │ Compression  │              │
│  │ (from A)     │─────▶│ Feedback     │─────▶│ Feedback     │              │
│  └──────────────┘      └──────────────┘      └──────┬───────┘              │
│                                                      │                      │
│                                                      ▼                      │
│                                              ┌──────────────┐              │
│                                              │ Update g_UC  │              │
│                                              │ for shorter  │              │
│                                              │ compression  │              │
│                                              └──────────────┘              │
│                                                                             │
│  Result:                                                                    │
│  g_U  → Optimized for accuracy (ACON_U)                                     │
│  g_UC → Optimized for accuracy + compression (ACON_UC)                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## The Alternating Guideline Optimization Algorithm

### Algorithm 1: Full Specification

```python
class ACONGuidelineOptimizer:
    """
    Alternating Guideline Optimization for ACON.
    
    Two-stage optimization:
      Stage A (Utility Maximization): Learn what to preserve
      Stage B (Compression Maximization): Learn to compress further
    
    Uses contrastive failure analysis: comparing trajectories where
    full context succeeds but compressed context fails.
    """
    
    def __init__(
        self,
        agent_model: str,
        compressor_model: str,
        optimizer_model: str = "o3",
        initial_guideline: str = "",
        num_rounds: int = 3,
        num_candidates: int = 4,
    ):
        self.agent = agent_model
        self.compressor = compressor_model
        self.optimizer = optimizer_model
        self.guideline = initial_guideline
        self.num_rounds = num_rounds
        self.num_candidates = num_candidates
    
    def optimize(self, training_tasks: list, held_out_tasks: list) -> str:
        """
        Main optimization loop.
        
        Args:
            training_tasks: Tasks for guideline optimization
            held_out_tasks: Tasks for candidate selection
            
        Returns:
            Optimized compression guideline (natural language)
        """
        # Step 0: Collect baseline trajectories (no compression)
        baselines = {}
        for task in training_tasks:
            context_seq, success = self.run_agent(task, compression=False)
            baselines[task.id] = {
                'context': context_seq,
                'success': success
            }
        
        # Identify tasks where baseline succeeds
        successful_tasks = [
            t for t in training_tasks 
            if baselines[t.id]['success']
        ]
        
        # Stage A: Utility Maximization
        for round_idx in range(self.num_rounds):
            self.guideline = self._utility_maximization_step(
                successful_tasks, baselines
            )
            
            # Early stopping if convergence
            if self._check_convergence():
                break
        
        guideline_u = self.guideline  # Save utility-optimized guideline
        
        # Stage B: Compression Maximization (optional)
        for round_idx in range(self.num_rounds):
            self.guideline = self._compression_maximization_step(
                successful_tasks, baselines
            )
            
            if self._check_convergence():
                break
        
        guideline_uc = self.guideline  # Save fully-optimized guideline
        
        return guideline_u, guideline_uc
    
    def _utility_maximization_step(
        self, 
        tasks: list, 
        baselines: dict
    ) -> str:
        """
        Stage A: Focus on preserving task-critical information.
        
        Compares full-context success vs. compressed-context failure
        to identify what information the compressor must preserve.
        """
        feedback_batch = []
        
        for task in tasks:
            # Run agent WITH current compression guideline
            compressed_context, success, cost = self.run_agent(
                task, compression=True, guideline=self.guideline
            )
            
            if not success:
                # Contrastive feedback: analyze WHY compression caused failure
                feedback = self._generate_contrastive_feedback(
                    baseline_trajectory=baselines[task.id]['context'],
                    compressed_trajectory=compressed_context,
                    task=task
                )
                feedback_batch.append(feedback)
        
        if not feedback_batch:
            return self.guideline  # No failures — guideline is good
        
        # Generate candidate updated guidelines
        candidates = []
        combined_feedback = "\n".join(feedback_batch)
        
        for _ in range(self.num_candidates):
            candidate = self._update_guideline(
                current_guideline=self.guideline,
                feedback=combined_feedback,
                objective="utility"
            )
            candidates.append(candidate)
        
        # Select best candidate on held-out set
        best_guideline = self._select_best_candidate(candidates)
        
        return best_guideline
    
    def _compression_maximization_step(
        self, 
        tasks: list, 
        baselines: dict
    ) -> str:
        """
        Stage B: Focus on maximizing compression ratio.
        
        Given a utility-optimized guideline, further refine to
        produce shorter compressions while preserving accuracy.
        """
        feedback_batch = []
        
        for task in tasks:
            compressed_context, success, cost = self.run_agent(
                task, compression=True, guideline=self.guideline
            )
            
            if success:
                # Cost feedback: how can we compress more?
                feedback = self._generate_compression_feedback(
                    compressed_trajectory=compressed_context,
                    cost=cost,
                    task=task
                )
                feedback_batch.append(feedback)
        
        combined_feedback = "\n".join(feedback_batch)
        
        candidates = []
        for _ in range(self.num_candidates):
            candidate = self._update_guideline(
                current_guideline=self.guideline,
                feedback=combined_feedback,
                objective="compression"
            )
            candidates.append(candidate)
        
        best_guideline = self._select_best_candidate(candidates)
        
        return best_guideline
    
    def _generate_contrastive_feedback(
        self,
        baseline_trajectory: list,
        compressed_trajectory: list,
        task: dict
    ) -> str:
        """
        Generate contrastive feedback by comparing paired trajectories.
        
        Uses a capable LLM (e.g., o3) to analyze:
        - What critical information was lost during compression?
        - Why did the compressed agent fail?
        - What should the guideline preserve?
        """
        prompt = f"""
        Compare these two agent trajectories for the same task:
        
        TASK: {task.description}
        
        TRAJECTORY A (full context — SUCCEEDED):
        {baseline_trajectory}
        
        TRAJECTORY B (compressed context — FAILED):
        {compressed_trajectory}
        
        Analyze:
        1. What critical information was present in A but missing in B?
        2. At which step did B diverge from A's successful path?
        3. What specific types of information must the compression preserve?
        4. Suggest concrete rules for the compression guideline.
        """
        
        return self.llm_call(self.optimizer, prompt)
    
    def _update_guideline(
        self, 
        current_guideline: str, 
        feedback: str, 
        objective: str
    ) -> str:
        """
        Update compression guideline using optimizer LLM.
        
        Analogous to a textual gradient descent step:
        feedback serves as the "gradient" and the optimizer
        applies an "update" in natural language space.
        """
        if objective == "utility":
            instruction = (
                "Update the compression guideline to better preserve "
                "information that the agent needs for task success. "
                "Focus on preventing the identified failure modes."
            )
        else:
            instruction = (
                "Update the compression guideline to produce shorter "
                "compressions while maintaining the information that "
                "enables task success."
            )
        
        prompt = f"""
        Current compression guideline:
        {current_guideline}
        
        Feedback from agent failures/successes:
        {feedback}
        
        Instruction: {instruction}
        
        Generate an updated compression guideline.
        """
        
        return self.llm_call(self.optimizer, prompt)
```

### Compression Thresholds

ACON triggers compression conditionally based on token thresholds:

```python
class ACONCompressor:
    """
    ACON compression with threshold-based triggering.
    
    History compression: triggered when total context exceeds threshold.
    Observation compression: triggered when latest observation exceeds threshold.
    """
    
    def __init__(
        self,
        guideline: str,
        history_threshold: int = 4096,
        observation_threshold: int = 1024,
        compressor_model: str = "gpt-4.1"
    ):
        self.guideline = guideline
        self.history_threshold = history_threshold
        self.observation_threshold = observation_threshold
        self.compressor = compressor_model
    
    def maybe_compress_history(self, trajectory: list) -> list:
        """
        Compress history if total tokens exceed threshold.
        
        Keeps the last action-observation pair intact to preserve
        the agent's most recent state.
        """
        total_tokens = sum(count_tokens(turn) for turn in trajectory)
        
        if total_tokens <= self.history_threshold:
            return trajectory  # No compression needed
        
        # Separate last turn (always preserved)
        history = trajectory[:-1]
        last_turn = trajectory[-1]
        
        # Compress history using optimized guideline
        compressed = self.llm_compress(
            content=history,
            guideline=self.guideline,
            mode="history"
        )
        
        return [compressed, last_turn]
    
    def maybe_compress_observation(self, observation: str) -> str:
        """
        Compress latest observation if it exceeds threshold.
        """
        obs_tokens = count_tokens(observation)
        
        if obs_tokens <= self.observation_threshold:
            return observation  # No compression needed
        
        return self.llm_compress(
            content=observation,
            guideline=self.guideline,
            mode="observation"
        )
    
    def llm_compress(self, content, guideline: str, mode: str) -> str:
        """Apply guideline-directed compression."""
        prompt = f"""
        Compress the following {mode} according to these guidelines:
        
        COMPRESSION GUIDELINES:
        {guideline}
        
        CONTENT TO COMPRESS:
        {content}
        
        Produce a concise yet informative compression that preserves
        all information specified in the guidelines.
        """
        return self.compressor_call(prompt)
```

### Optimal Threshold Values

| Benchmark | History Threshold | Observation Threshold |
|-----------|------------------:|----------------------:|
| AppWorld | 4,096 tokens | 1,024 tokens |
| OfficeBench | 4,096 tokens | 2,048 tokens |
| 8-Objective QA | 2,048 tokens | 1,024 tokens |

Moderate thresholds provide the best accuracy-efficiency trade-off. Smaller thresholds incur more frequent compression calls and degrade accuracy; larger thresholds preserve accuracy but reduce savings.

---

## Compressor Distillation

A key contribution of ACON is distilling the optimized compression logic from a large LLM (e.g., GPT-4.1) into smaller models, reducing the overhead of the additional compression module.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      COMPRESSOR DISTILLATION PIPELINE                       │
│                                                                             │
│  Step 1: Collect Teacher Compressions                                       │
│  ─────────────────────────────────────                                      │
│  Run GPT-4.1 compressor with optimized guideline on training tasks          │
│  → Collect (input, compressed_output) pairs                                 │
│                                                                             │
│  Step 2: Fine-Tune Student Model                                            │
│  ─────────────────────────────────                                          │
│  Train smaller model (Qwen3-14B, Qwen3-8B, Phi-4)                          │
│  on teacher's compression examples                                          │
│  → Student learns to apply optimized compression logic                      │
│                                                                             │
│  Step 3: Deploy Distilled Compressor                                        │
│  ──────────────────────────────────                                          │
│  Replace GPT-4.1 compressor with student model                              │
│  → 95%+ accuracy preserved at fraction of cost                              │
│                                                                             │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐              │
│  │  Teacher      │      │  Student      │      │  Deployed     │              │
│  │  GPT-4.1     │─────▶│  Qwen3-14B   │─────▶│  Qwen3-14B   │              │
│  │  (expensive) │      │  (training)   │      │  (inference)  │              │
│  │  $$$         │      │  $$           │      │  $            │              │
│  └──────────────┘      └──────────────┘      └──────────────┘              │
│                                                                             │
│  Key Finding: Optimized guidelines distill better than unoptimized ones     │
│               → The guideline quality transfers to the student model        │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Experimental Results

### Benchmarks

| Benchmark | Domain | Task Type | Avg Steps |
|-----------|--------|-----------|----------:|
| AppWorld | API-driven tasks | Multi-API coordination | 15-40+ |
| OfficeBench | Office automation | Document/spreadsheet tasks | 15-30+ |
| 8-Objective QA | Knowledge QA | Multi-hop research retrieval | 15-25+ |

### AppWorld Results (Test-Normal, GPT-4.1 Agent)

#### History Compression

| Method | Avg Acc. | Easy Acc. | Medium Acc. | Hard Acc. | Peak Tokens (K) | Dependency (K) |
|--------|:--------:|:---------:|:-----------:|:---------:|:----------------:|:--------------:|
| No Compression | 47.6 | 75.4 | 50.0 | 20.6 | 9.63 | 6.17 |
| Prompting | 37.5 | 66.7 | 29.2 | 17.5 | 7.08 | 4.37 |
| AutoCompressAgent | 42.9 | 73.7 | 39.6 | 17.5 | 7.56 | 4.90 |
| MemoryBank | 39.9 | 66.7 | 31.3 | 22.2 | 7.40 | 5.06 |
| **ACON_U** | **47.0** | 68.4 | **58.3** | 19.1 | **7.22** | **4.66** |
| **ACON_UC** | **48.2** | **71.9** | 52.1 | **23.8** | 7.08 | 4.59 |

#### Observation Compression

| Method | Avg Acc. | Peak Tokens (K) | Dependency (K) |
|--------|:--------:|:----------------:|:--------------:|
| No Compression | 47.6 | 9.63 | 6.17 |
| Prompting | 46.4 | 7.55 | 5.71 |
| **ACON_U** | **48.2** | **7.41** | **5.38** |
| **ACON_UC** | 47.6 | 7.14 | 5.15 |

### OfficeBench Results (GPT-4.1 Agent)

| Method | Avg Acc. | Peak Tokens (K) | Dependency (K) |
|--------|:--------:|:----------------:|:--------------:|
| No Compression | 76.0 | 7.89 | 4.35 |
| Prompting | 73.3 | 5.84 | 3.08 |
| AutoCompressAgent | 72.0 | 5.96 | 3.20 |
| MemoryBank | 72.7 | 5.98 | 3.20 |
| **ACON_U** | **74.7** | **5.54** | **2.94** |
| **ACON_UC** | 74.0 | 5.42 | 2.83 |

### 8-Objective QA Results (GPT-4.1 Agent)

| Method | EM | F1 | Peak Tokens (K) | Dependency (K) |
|--------|:---:|:---:|:----------------:|:--------------:|
| No Compression | 0.251 | 0.344 | 17.6 | 12.8 |
| Prompting | 0.213 | 0.308 | 8.9 | 5.6 |
| AutoCompressAgent | 0.221 | 0.314 | 10.2 | 6.2 |
| **ACON_U** | **0.268** | **0.363** | 8.0 | 4.9 |
| **ACON_UC** | 0.253 | 0.348 | **8.0** | **4.9** |

### Distillation Results (GPT-4.1 Agent, History Compression)

| Student Compressor | AppWorld Acc. | OfficeBench Acc. | 8-Obj QA EM | Accuracy Retention |
|-------------------|:-------------:|:----------------:|:-----------:|:------------------:|
| GPT-4.1 (teacher) | 47.0 | 74.7 | 0.268 | 100% |
| GPT-4.1-mini | 45.2 | 73.3 | 0.253 | ~96% |
| Qwen3-14B | 44.6 | 72.7 | 0.258 | **~95%** |
| Qwen3-8B | 43.5 | 72.0 | 0.246 | ~93% |
| Phi-4 | 44.0 | 71.3 | 0.251 | ~94% |

### Small Agent Improvement (Qwen3-14B as Agent + Distilled Compressor)

| Benchmark | Without ACON | With ACON | Improvement |
|-----------|:-----------:|:---------:|:-----------:|
| AppWorld | 26.8% | 33.9% | **+26.5%** |
| OfficeBench | — | — | **+20%** |
| 8-Objective QA | 0.158 EM | 0.197 EM | **+24.7%** |

---

## Ablation Studies

### Prompt Optimizer Selection

| Optimizer | Contrastive Feedback | AppWorld Acc. |
|-----------|:--------------------:|:-------------:|
| **o3** | **Yes** | **47.0** |
| o3 | No (failures only) | 44.6 |
| GPT-4.1 | Yes | 45.2 |
| GPT-4.1-mini | Yes | 43.5 |

Contrastive feedback (comparing success vs. failure trajectories) is critical — using only failed trajectories produces weaker guidelines.

### History + Observation Compression (Combined)

| Mode | AppWorld Acc. | Peak Tokens (K) |
|------|:------------:|:----------------:|
| History only | 47.0 | 7.22 |
| Observation only | 48.2 | 7.41 |
| **Both** | 45.8 | **5.85** |

Combining both compression types achieves the largest token reduction but leads to accuracy degradation, indicating tension between the two compression modes.

### Additional Optimization Rounds

Running an extra utility maximization step after the standard U→UC sequence results in performance drops, confirming that a single round of alternating optimization is sufficient.

---

## Connection to Complexity Trap Research

### Validating the Core Thesis

ACON provides strong independent evidence for the Complexity Trap's central finding that simpler approaches can match sophisticated ones:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 ACON ↔ COMPLEXITY TRAP CONNECTIONS                          │
│                                                                             │
│  1. "More Context" Is Often Detrimental                                     │
│  ───────────────────────────────────────                                     │
│  Complexity Trap: Observations are 84% of trajectory, mostly noise          │
│  ACON:           Small LMs improve by 20-46% WITH compression              │
│                  → Long context actively hurts smaller models               │
│                  → Confirms "information overload" problem                  │
│                                                                             │
│  2. Compression Quality > Compression Quantity                              │
│  ─────────────────────────────────────────────                               │
│  Complexity Trap: Simple masking ≈ LLM summarization in performance         │
│  ACON:           Optimized guidelines outperform naive prompting            │
│                  → WHAT you preserve matters more than HOW MUCH             │
│                  → Naive compression (prompting) degrades on hard tasks     │
│                                                                             │
│  3. Trajectory Elongation Mitigation                                        │
│  ────────────────────────────────────                                        │
│  Complexity Trap: LLM summaries smooth over failures → +15-18% turns       │
│  ACON:           Task-aware compression preserves failure signals           │
│                  → Guided by what CAUSED failures, not generic summaries    │
│                  → Avoids the "progress framing" that causes elongation     │
│                                                                             │
│  4. Training vs. Inference Trade-Off                                        │
│  ────────────────────────────────────                                        │
│  Complexity Trap: Masking needs zero training, works well at inference      │
│  ACON:           Requires training phase but produces better guidelines     │
│                  → Investment in training pays off for repeated tasks       │
│                  → Distillation amortizes cost across deployments           │
│                                                                             │
│  5. The Hybrid Opportunity                                                  │
│  ─────────────────────────                                                  │
│  Complexity Trap: Hybrid (masking + summary) achieves best results          │
│  ACON:           ACON guidelines could replace generic summary prompts     │
│                  → Hybrid + ACON = masking by default, ACON when needed     │
│                  → Potential for even better cost-effectiveness frontier    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Where ACON Extends Beyond the Complexity Trap

| Aspect | Complexity Trap | ACON |
|--------|----------------|------|
| Compression rules | Fixed (window size M) | Learned through failure analysis |
| Training required | None | Yes (paired trajectories) |
| Model-specific | No | Yes (adapts to agent model) |
| Task-specific | No | Yes (adapts to task domain) |
| Distillable | N/A | Yes (95%+ accuracy preserved) |
| Small model benefit | Not studied | Significant (+20-46%) |

### Positioning in the Strategy Landscape

```
Effectiveness
     ▲
     │                           ┌──────────┐
  60 │                           │  ACON +   │ ← Potential frontier
     │                    ┌──────┤  Hybrid   │
  55 │            ┌───────┤ACON  └──────────┘
     │            │       │ (trained)
  50 │    ┌───────┤Hybrid │
     │    │ MASK  │       │
  45 │    │       │    ┌──┴──────┐
     │    │       │    │SUMMARY  │
  40 │────┼───────┼────┼─────────┼───────
     │    │       │    │         │
     │    │       │    │         │
     │    │       │    │         │
     └────┴───────┴────┴─────────┴──────────▶ Cost ($)
       Low                              High
```

---

## Limitations

### Training Overhead

- Requires paired trajectory collection (baseline + compressed)
- Needs a capable optimizer LLM (o3) for failure analysis
- Multiple rounds of training add upfront cost
- Not suitable for one-off or novel task domains

### Combined Compression Tension

- History + observation compression together degrades accuracy
- Suggests information loss compounds across compression modes
- Single-mode compression (history OR observation) is safer

### Domain Specificity

- Optimized guidelines are benchmark-specific
- Transfer across domains not yet validated
- May require re-optimization for new task types

### Compressor Overhead

- Even distilled compressors add inference cost per turn
- Threshold tuning required per benchmark
- Additional complexity vs. simple observation masking

---

## Key Takeaways

### For Practitioners

1. **ACON is best for repeated task domains** — training cost amortizes over many deployments
2. **Distillation makes it production-viable** — 95%+ accuracy with small compressor models
3. **Small LMs benefit most** — compression removes distraction, enabling +20-46% improvement
4. **Use moderate thresholds** — 4096 tokens for history, 1024 for observations

### For Researchers

1. **Contrastive failure analysis works** — comparing success/failure pairs produces stronger guidelines than using failures alone
2. **Compression quality > quantity** — optimized guidelines preserve critical information better than generic prompts
3. **Distillation preserves guideline quality** — the learned compression logic transfers to smaller models
4. **Combined compression is fragile** — history + observation compression together compounds information loss

### Relationship to Observation Masking

ACON and observation masking occupy different points on the complexity-effectiveness spectrum:

| Criterion | Observation Masking | ACON |
|-----------|:-------------------:|:----:|
| Setup cost | None | Moderate (training) |
| Per-turn cost | None | Low (distilled) |
| Token reduction | ~50-75% | 26-54% |
| Small model benefit | Unknown | Significant |
| Task adaptability | None | High |
| Implementation complexity | Trivial | Moderate |

---

## Next Steps

- **[Observation Masking](01-observation-masking.md)** — The simple baseline ACON improves upon
- **[LLM Summarization](02-llm-summarization.md)** — Generic compression ACON replaces
- **[Hybrid Approach](03-hybrid-approach.md)** — Combining masking with learned compression
- **[Advanced Strategies](04-advanced-strategies.md)** — Complementary 2025 research
- **[Trajectory Elongation](../experiments/03-trajectory-elongation.md)** — The failure signal problem ACON addresses
- **[Future Work](../challenges/02-future-work.md)** — Open problems and improvements
