# Advanced Trajectory Evaluation Frameworks

## Overview

Traditional evaluation of context management strategies relies on **pass/fail metrics** — did the agent solve the task? While necessary, this binary approach misses critical nuances in agent behavior, efficiency, and trajectory quality. The 2025 research landscape has introduced sophisticated evaluation frameworks that provide deeper insights into agent performance.

**Key Development**: Multi-dimensional evaluation frameworks (CORE, ContextBench, Galileo) enable fine-grained analysis of trajectory quality beyond binary success metrics.

---

## 1. CORE: Full-Path Evaluation Framework

**Paper**: "CORE: Comprehensive and Omni-directional Review Evaluation for Long Reasoning" (Zhang et al., 2025)

### Core Concept

CORE provides comprehensive evaluation of reasoning trajectories through multi-dimensional scoring, analyzing not just outcomes but the quality of the reasoning process itself.

### Evaluation Dimensions

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     CORE EVALUATION FRAMEWORK                               │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  DIMENSION 1: CORRECTNESS                                           │   │
│  │  ─────────────────────────                                           │   │
│  │  • Final answer accuracy                                            │   │
│  │  • Intermediate step validity                                       │   │
│  │  • Logical consistency throughout                                   │   │
│  │                                                                     │   │
│  │  Scoring: 0-100 based on error rate and correction success           │   │
│  │                                                                     │   │
│  │  DIMENSION 2: EFFICIENCY                                              │   │
│  │  ─────────────────────                                               │   │
│  │  • Number of reasoning steps                                         │   │
│  │  • Redundancy detection                                              │   │
│  │  • Token economy                                                     │   │
│  │                                                                     │   │
│  │  Scoring: Optimal path vs. actual path ratio                         │   │
│  │                                                                     │   │
│  │  DIMENSION 3: COMPLETENESS                                            │   │
│  │  ─────────────────────────                                           │   │
│  │  • Coverage of all problem aspects                                   │   │
│  │  • Consideration of edge cases                                       │   │
│  │  • Alternative approach exploration                                  │   │
│  │                                                                     │   │
│  │  Scoring: Coverage percentage vs. ground truth requirements        │   │
│  │                                                                     │   │
│  │  DIMENSION 4: CLARITY                                                 │   │
│  │  ─────────────────────                                               │   │
│  │  • Reasoning transparency                                            │   │
│  │  • Step-by-step explainability                                       │   │
│  │  • Confidence calibration                                            │   │
│  │                                                                     │   │
│  │  Scoring: Human/LLM evaluator clarity ratings                        │   │
│  │                                                                     │   │
│  │  DIMENSION 5: ROBUSTNESS                                              │   │
│  │  ─────────────────────────                                           │   │
│  │  • Handling of perturbations                                         │   │
│  │  • Recovery from errors                                              │   │
│  │  • Graceful degradation                                              │   │
│  │                                                                     │   │
│  │  Scoring: Success rate under adversarial conditions                  │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Overall Score: Weighted combination of all dimensions                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### CORE Scoring Algorithm

```python
class COREvaluator:
    """
    CORE: Comprehensive evaluation of reasoning trajectories.
    
    Provides multi-dimensional analysis beyond pass/fail metrics.
    """
    
    DIMENSIONS = ['correctness', 'efficiency', 'completeness', 'clarity', 'robustness']
    
    WEIGHTS = {
        'correctness': 0.30,
        'efficiency': 0.25,
        'completeness': 0.20,
        'clarity': 0.15,
        'robustness': 0.10
    }
    
    def __init__(self, llm_evaluator, ground_truth: dict = None):
        self.evaluator = llm_evaluator
        self.ground_truth = ground_truth
        
    def evaluate_trajectory(self, trajectory: list, 
                            final_answer: str) -> dict:
        """
        Perform comprehensive evaluation of agent trajectory.
        
        Returns scores for all dimensions plus overall score.
        """
        scores = {}
        
        # Dimension 1: Correctness
        scores['correctness'] = self._evaluate_correctness(
            trajectory, final_answer
        )
        
        # Dimension 2: Efficiency
        scores['efficiency'] = self._evaluate_efficiency(trajectory)
        
        # Dimension 3: Completeness
        scores['completeness'] = self._evaluate_completeness(
            trajectory, final_answer
        )
        
        # Dimension 4: Clarity
        scores['clarity'] = self._evaluate_clarity(trajectory)
        
        # Dimension 5: Robustness
        scores['robustness'] = self._evaluate_robustness(trajectory)
        
        # Calculate overall score
        overall = sum(
            scores[d] * self.WEIGHTS[d] for d in self.DIMENSIONS
        )
        
        return {
            'dimensions': scores,
            'overall': overall,
            'breakdown': self._generate_breakdown(scores)
        }
    
    def _evaluate_correctness(self, trajectory: list, 
                              final_answer: str) -> float:
        """Evaluate correctness of reasoning and final answer."""
        scores = []
        
        # Final answer correctness
        if self.ground_truth:
            final_correct = self._check_answer(final_answer, self.ground_truth)
            scores.append(100.0 if final_correct else 0.0)
        
        # Step-by-step correctness
        step_errors = 0
        corrections = 0
        
        for i, turn in enumerate(trajectory):
            # Check if reasoning contains logical errors
            is_valid = self._validate_reasoning_step(turn, trajectory[:i])
            if not is_valid:
                step_errors += 1
                
            # Check if error was corrected later
            if step_errors > 0 and i < len(trajectory) - 1:
                if self._was_error_corrected(turn, trajectory[i+1:]):
                    corrections += 1
        
        # Score based on error rate and correction success
        if len(trajectory) > 0:
            error_rate = step_errors / len(trajectory)
            correction_rate = corrections / max(1, step_errors)
            
            step_score = 100 * (1 - error_rate * 0.5) * (0.5 + 0.5 * correction_rate)
            scores.append(step_score)
        
        return sum(scores) / len(scores) if scores else 50.0
    
    def _evaluate_efficiency(self, trajectory: list) -> float:
        """Evaluate efficiency of reasoning path."""
        
        # Count steps
        num_steps = len(trajectory)
        
        # Detect redundancy
        redundant_steps = self._detect_redundancy(trajectory)
        
        # Calculate token usage
        total_tokens = sum(
            len(turn.get('reasoning', '')) + len(turn.get('observation', ''))
            for turn in trajectory
        )
        
        # Estimate optimal path (using oracle or heuristics)
        optimal_steps = self._estimate_optimal_path(trajectory)
        optimal_tokens = optimal_steps * (total_tokens / num_steps)
        
        # Efficiency score
        step_efficiency = min(1.0, optimal_steps / max(1, num_steps))
        token_efficiency = min(1.0, optimal_tokens / max(1, total_tokens))
        redundancy_penalty = 1.0 - (len(redundant_steps) / max(1, num_steps))
        
        efficiency = (
            0.4 * step_efficiency +
            0.4 * token_efficiency +
            0.2 * redundancy_penalty
        ) * 100
        
        return efficiency
    
    def _detect_redundancy(self, trajectory: list) -> list:
        """Detect redundant or repetitive steps."""
        redundant = []
        
        for i, turn in enumerate(trajectory):
            # Check for repeated actions
            for j in range(max(0, i-5), i):
                if self._is_similar_action(turn, trajectory[j]):
                    redundant.append({
                        'turn': i,
                        'similar_to': j,
                        'reason': 'repeated_action'
                    })
                    break
        
        return redundant
    
    def _evaluate_completeness(self, trajectory: list, 
                               final_answer: str) -> float:
        """Evaluate coverage of all problem aspects."""
        
        # Extract aspects mentioned in trajectory
        mentioned_aspects = set()
        for turn in trajectory:
            aspects = self._extract_aspects(turn['reasoning'])
            mentioned_aspects.update(aspects)
        
        # Compare to ground truth requirements
        if self.ground_truth and 'required_aspects' in self.ground_truth:
            required = set(self.ground_truth['required_aspects'])
            coverage = len(mentioned_aspects & required) / len(required)
            
            # Bonus for exploring beyond requirements
            exploration = len(mentioned_aspects - required) / max(1, len(required))
            
            return (coverage * 0.8 + min(0.2, exploration * 0.2)) * 100
        
        # Without ground truth, use heuristics
        return min(100, len(mentioned_aspects) * 20)
    
    def _evaluate_clarity(self, trajectory: list) -> float:
        """Evaluate clarity and explainability of reasoning."""
        
        clarity_scores = []
        
        for turn in trajectory:
            reasoning = turn.get('reasoning', '')
            
            # Use LLM to evaluate clarity
            prompt = f"""
            Rate the clarity of this reasoning step (0-100):
            
            Reasoning: {reasoning}
            
            Consider:
            - Is the logic clear and transparent?
            - Are assumptions stated explicitly?
            - Would a human understand this step?
            
            Respond with just a number 0-100.
            """
            
            score = float(self.evaluator.generate(prompt))
            clarity_scores.append(score)
        
        return sum(clarity_scores) / len(clarity_scores) if clarity_scores else 50.0
    
    def _evaluate_robustness(self, trajectory: list) -> float:
        """Evaluate robustness through perturbation analysis."""
        
        # This requires running additional trials with perturbations
        # For now, use heuristics based on trajectory characteristics
        
        # Check error recovery
        errors = [i for i, t in enumerate(trajectory) 
                  if self._is_error_state(t)]
        recoveries = sum(
            1 for e in errors 
            if self._recovered_from_error(e, trajectory)
        )
        
        recovery_rate = recoveries / max(1, len(errors))
        
        # Check consistency
        consistency_score = self._evaluate_consistency(trajectory)
        
        return (recovery_rate * 0.6 + consistency_score * 0.4) * 100
```

### Results Application

| Context Strategy | Pass Rate | CORE Overall | Efficiency Dim | Clarity Dim |
|-----------------:|----------:|-------------:|---------------:|------------:|
| Raw Agent | 53.4% | 62.3 | 45.2 | 68.1 |
| Observation Masking | 54.8% | 64.7 | **72.5** | 65.3 |
| LLM Summary | 53.8% | 61.2 | 58.4 | **70.2** |
| Hybrid | **57.4%** | **68.1** | **75.3** | 68.9 |

**Key Finding**: Hybrid approach dominates across dimensions, with observation masking excelling in efficiency and LLM summary in clarity.

---

## 2. ContextBench: Standardized Context Evaluation

**Paper**: "ContextBench: A Benchmark for Long-Context Understanding" (2025)

### Core Concept

ContextBench provides standardized metrics for evaluating how well agents utilize and manage long contexts across diverse task types.

### Benchmark Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     CONTEXTBENCH FRAMEWORK                                  │
│                                                                             │
│  Task Categories:                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 1. Single-Document QA                                                │   │
│  │    • Answer questions from long documents (10K-100K tokens)        │   │
│  │    • Tests: Information localization, comprehension                  │   │
│  │                                                                     │   │
│  │ 2. Multi-Document QA                                                 │   │
│  │    • Synthesize information across multiple documents                │   │
│  │    • Tests: Cross-document reasoning, aggregation                    │   │
│  │                                                                     │   │
│  │ 3. Long-Context Code Understanding                                   │   │
│  │    • Understand and modify large codebases                         │   │
│  │    • Tests: Code navigation, dependency tracking                     │   │
│  │                                                                     │   │
│  │ 4. Long-Context Reasoning                                            │   │
│  │    • Multi-step reasoning over extended contexts                     │   │
│  │    • Tests: Logical chains, temporal reasoning                     │   │
│  │                                                                     │   │
│  │ 5. Agent Trajectory Understanding                                    │   │
│  │    • Process long agent interaction histories                        │   │
│  │    • Tests: Context management, information preservation             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Metrics:                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Metric              │ Description                                   │   │
│  │─────────────────────│───────────────────────────────────────────────│   │
│  │ Accuracy            │ Correct answer rate                           │   │
│  │ Context Utilization │ % of relevant context actually used           │   │
│  │ Position Bias       │ Performance vs. information position          │   │
│  │ Information Loss    │ % of critical information dropped             │   │
│  │ Retrieval Precision │ Relevance of retrieved context segments       │   │
│  │ Latency             │ Time to process different context lengths   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### ContextBench Metrics Implementation

```python
class ContextBenchEvaluator:
    """
    ContextBench: Standardized long-context evaluation metrics.
    """
    
    def __init__(self, test_suite: str = "full"):
        self.test_suite = test_suite
        self.results = {}
        
    def run_evaluation(self, agent, context_manager) -> dict:
        """
        Run full ContextBench evaluation suite.
        """
        categories = [
            'single_doc_qa',
            'multi_doc_qa', 
            'code_understanding',
            'long_reasoning',
            'agent_trajectory'
        ]
        
        for category in categories:
            test_cases = self._load_test_cases(category)
            category_results = []
            
            for test in test_cases:
                result = self._run_test(agent, context_manager, test)
                category_results.append(result)
            
            self.results[category] = self._aggregate_results(category_results)
        
        return self._compute_overall_scores()
    
    def measure_context_utilization(self, trajectory: list, 
                                    relevant_spans: list) -> float:
        """
        Measure what percentage of relevant context was actually used.
        
        Args:
            trajectory: Agent's interaction history
            relevant_spans: Ground truth relevant context locations
            
        Returns:
            Utilization ratio 0-1
        """
        # Extract accessed context positions
        accessed_positions = set()
        for turn in trajectory:
            if 'read_file' in turn.get('action', {}):
                lines = turn['action'].get('lines', [])
                file = turn['action'].get('path', '')
                for line in lines:
                    accessed_positions.add((file, line))
        
        # Compare to relevant spans
        total_relevant = 0
        accessed_relevant = 0
        
        for span in relevant_spans:
            span_positions = set(
                (span['file'], line) 
                for line in range(span['start'], span['end'])
            )
            total_relevant += len(span_positions)
            accessed_relevant += len(span_positions & accessed_positions)
        
        return accessed_relevant / total_relevant if total_relevant > 0 else 0.0
    
    def measure_position_bias(self, results_by_position: dict) -> dict:
        """
        Measure how performance varies with information position.
        
        Tests the "Lost in the Middle" effect quantitatively.
        """
        positions = ['start', 'early', 'middle', 'late', 'end']
        
        bias_analysis = {}
        for pos in positions:
            if pos in results_by_position:
                accuracy = results_by_position[pos]['accuracy']
                bias_analysis[pos] = accuracy
        
        # Calculate bias metrics
        middle_penalty = bias_analysis.get('middle', 0) / \
                        (sum(bias_analysis.values()) / len(bias_analysis))
        
        return {
            'position_scores': bias_analysis,
            'middle_penalty': middle_penalty,
            'has_significant_bias': middle_penalty < 0.85
        }
    
    def measure_information_loss(self, 
                                  original_context: str,
                                  compressed_context: str,
                                  critical_facts: list) -> float:
        """
        Measure percentage of critical information lost in compression.
        
        Args:
            original_context: Full context before compression
            compressed_context: Context after compression
            critical_facts: List of facts that must be preserved
            
        Returns:
            Information loss ratio 0-1 (lower is better)
        """
        lost_facts = 0
        
        for fact in critical_facts:
            # Check if fact is present in compressed context
            is_preserved = self._fact_present(fact, compressed_context)
            if not is_preserved:
                lost_facts += 1
        
        return lost_facts / len(critical_facts) if critical_facts else 0.0
```

### Results by Context Strategy

| Strategy | Single-Doc QA | Multi-Doc QA | Code Understanding | Trajectory | Overall |
|----------|--------------:|-------------:|-------------------:|-----------:|--------:|
| No Compression | 78.2 | 65.4 | 72.1 | 45.3 | 65.3 |
| Observation Masking | 76.5 | 63.8 | **74.5** | **52.1** | 66.7 |
| LLM Summary | **80.1** | **68.2** | 70.3 | 48.7 | **66.8** |
| Hybrid | 78.9 | 67.5 | 73.8 | 51.4 | **67.9** |

**Key Finding**: Hybrid achieves best overall balance, with masking excelling on code and trajectories, summarization on document QA.

---

## 3. Galileo Agentic Metrics

**Paper**: Galileo Evaluation Platform (2025) - Agent-specific metrics for production deployment

### Core Concept

Galileo provides production-ready metrics specifically designed for agent evaluation, focusing on real-world deployment concerns like cost, latency, and reliability.

### Metric Categories

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    GALILEO AGENTIC METRICS                                  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 1. EXECUTION METRICS                                                  │   │
│  │  ────────────────────                                                 │   │
│  │  • Tool Call Success Rate: % of successful tool executions            │   │
│  │  • Retry Rate: Frequency of repeated actions                          │   │
│  │  • Dead-end Detection: Ability to recognize unproductive paths        │   │
│  │  • Recovery Time: Steps to recover from errors                      │   │
│  │                                                                     │   │
│  │  2. COST METRICS                                                      │   │
│  │  ───────────────                                                      │   │
│  │  • Token Efficiency: Output quality per input token                   │   │
│  │  • API Call Optimization: Minimizing redundant LLM calls              │   │
│  │  • Cache Hit Rate: Reuse of previously computed results             │   │
│  │  • Cost Per Success: Total cost divided by successful completions   │   │
│  │                                                                     │   │
│  │  3. QUALITY METRICS                                                   │   │
│  │  ──────────────────                                                   │   │
│  │  • Hallucination Rate: Frequency of factually incorrect outputs     │   │
│  │  • Confidence Calibration: Alignment of confidence with accuracy    │   │
│  │  • Explanation Quality: Clarity of reasoning provided                │   │
│  │  • Grounding: Evidence for claims made                               │   │
│  │                                                                     │   │
│  │  4. RELIABILITY METRICS                                               │   │
│  │  ──────────────────────                                               │   │
│  │  • Success Rate Variance: Consistency across similar tasks          │   │
│  │  • Timeout Rate: Frequency of hitting time limits                     │   │
│  │  • Partial Success: Tasks completed with degraded quality           │   │
│  │  • Error Propagation: How errors compound across steps              │   │
│  │                                                                     │   │
│  │  5. CONTEXT METRICS (Relevant to Complexity Trap)                     │   │
│  │  ─────────────────────────────────────                                │   │
│  │  • Context Growth Rate: Tokens added per step                         │   │
│  │  • Relevance Decay: How quickly old context loses relevance         │   │
│  │  • Compression Effectiveness: Information preserved vs. removed     │   │
│  │  • Access Pattern: Which context segments are most used             │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Galileo Metrics Implementation

```python
class GalileoAgenticMetrics:
    """
    Galileo: Production agent evaluation metrics.
    """
    
    def __init__(self):
        self.metrics = {
            'execution': {},
            'cost': {},
            'quality': {},
            'reliability': {},
            'context': {}
        }
    
    def analyze_trajectory(self, trajectory: list, 
                          outcome: dict,
                          cost_data: dict) -> dict:
        """
        Compute full Galileo metrics for a trajectory.
        """
        return {
            'execution': self._compute_execution_metrics(trajectory, outcome),
            'cost': self._compute_cost_metrics(trajectory, cost_data, outcome),
            'quality': self._compute_quality_metrics(trajectory, outcome),
            'reliability': self._compute_reliability_metrics(trajectory, outcome),
            'context': self._compute_context_metrics(trajectory)
        }
    
    def _compute_execution_metrics(self, trajectory: list, 
                                   outcome: dict) -> dict:
        """Compute tool execution and path quality metrics."""
        total_actions = len(trajectory)
        
        # Tool success rate
        successful_tools = sum(
            1 for t in trajectory 
            if not self._is_error(t.get('observation', ''))
        )
        
        # Retry rate (same action type repeated)
        retries = 0
        for i in range(1, len(trajectory)):
            if self._is_same_action(trajectory[i], trajectory[i-1]):
                retries += 1
        
        # Dead-end detection
        dead_ends = self._detect_dead_ends(trajectory)
        
        # Recovery time
        recoveries = self._analyze_recoveries(trajectory)
        avg_recovery = sum(r['steps'] for r in recoveries) / max(1, len(recoveries))
        
        return {
            'tool_success_rate': successful_tools / max(1, total_actions),
            'retry_rate': retries / max(1, total_actions),
            'dead_end_count': len(dead_ends),
            'avg_recovery_steps': avg_recovery,
            'path_efficiency': self._compute_path_efficiency(trajectory, outcome)
        }
    
    def _compute_cost_metrics(self, trajectory: list, 
                             cost_data: dict,
                             outcome: dict) -> dict:
        """Compute cost efficiency metrics."""
        total_tokens = cost_data.get('input_tokens', 0) + cost_data.get('output_tokens', 0)
        total_cost = cost_data.get('total_cost', 0)
        
        # Token efficiency
        outcome_score = outcome.get('score', 1.0 if outcome.get('success') else 0.0)
        token_efficiency = outcome_score / max(1, total_tokens / 1000)
        
        # Cost per success
        cost_per_success = total_cost / max(0.1, outcome_score)
        
        # Context growth rate
        context_growth = self._compute_context_growth(trajectory)
        
        return {
            'total_tokens': total_tokens,
            'total_cost': total_cost,
            'token_efficiency': token_efficiency,
            'cost_per_success': cost_per_success,
            'context_growth_rate': context_growth,
            'cost_efficiency_score': self._compute_cost_efficiency_score(
                total_cost, outcome, trajectory
            )
        }
    
    def _compute_context_metrics(self, trajectory: list) -> dict:
        """
        Compute context-specific metrics for Complexity Trap analysis.
        """
        # Context growth over time
        context_sizes = []
        for i, turn in enumerate(trajectory):
            # Estimate context size at this turn
            context_size = self._estimate_context_size(trajectory[:i+1])
            context_sizes.append(context_size)
        
        # Growth rate (tokens per turn)
        if len(context_sizes) > 1:
            growth_rate = (context_sizes[-1] - context_sizes[0]) / len(context_sizes)
        else:
            growth_rate = 0
        
        # Compression effectiveness (if compression applied)
        if hasattr(self, 'original_sizes'):
            compression_ratios = [
                comp / orig 
                for comp, orig in zip(context_sizes, self.original_sizes)
            ]
            avg_compression = sum(compression_ratios) / len(compression_ratios)
        else:
            avg_compression = 1.0
        
        # Access pattern analysis
        access_counts = self._analyze_context_access(trajectory)
        
        return {
            'final_context_size': context_sizes[-1] if context_sizes else 0,
            'context_growth_rate': growth_rate,
            'avg_compression_ratio': avg_compression,
            'peak_context_size': max(context_sizes) if context_sizes else 0,
            'context_access_pattern': access_counts,
            'relevant_context_retention': self._measure_retention(trajectory)
        }
    
    def _compute_path_efficiency(self, trajectory: list, 
                                  outcome: dict) -> float:
        """
        Measure how direct the path to solution was.
        
        Lower is better (fewer unnecessary steps).
        """
        # Count productive vs. unproductive steps
        productive = 0
        unproductive = 0
        
        for i, turn in enumerate(trajectory):
            if self._was_productive(turn, trajectory[i+1:], outcome):
                productive += 1
            else:
                unproductive += 1
        
        total = productive + unproductive
        return productive / total if total > 0 else 0.0
```

### Galileo Results by Strategy

| Metric Category | Raw Agent | Masking | Summary | Hybrid |
|-----------------:|----------:|--------:|--------:|-------:|
| **Execution** | | | | |
| Tool Success Rate | 72% | **74%** | 71% | **75%** |
| Retry Rate | 18% | 15% | 22% | **14%** |
| Dead-end Count | 2.3 | **1.8** | 2.1 | **1.6** |
| **Cost** | | | | |
| Avg Cost ($) | 0.41 | **0.19** | 0.20 | **0.17** |
| Cost/Success | 0.77 | **0.35** | 0.37 | **0.30** |
| Token Efficiency | 1.8 | **3.9** | 3.7 | **4.2** |
| **Context** | | | | |
| Context Growth (tok/turn) | 850 | **420** | 380 | **400** |
| Relevant Retention | 100% | **85%** | **88%** | **87%** |
| Compression Ratio | 1.0 | **0.48** | **0.44** | **0.46** |

---

## 4. Beyond Pass/Fail: Nuanced Evaluation

### Multi-Dimensional Success

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 BEYOND BINARY SUCCESS METRICS                              │
│                                                                             │
│  Traditional:                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  Task Result: [ PASS ] or [ FAIL ]                                  │   │
│  │                                                                     │   │
│  │  Limitations:                                                        │   │
│  │  • No distinction between "almost" and "completely wrong"            │   │
│  │  • Doesn't capture solution quality                                  │   │
│  │  • Misses efficiency differences                                     │   │
│  │  • Ignores trajectory educational value                              │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Nuanced Evaluation:                                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  Task Result:                                                        │   │
│  │  ┌──────────────┬──────────────┬──────────────┬──────────────┐      │   │
│  │  │  Functional  │    Quality   │   Efficiency │  Robustness  │      │   │
│  │  │    90/100    │    75/100    │    85/100    │    70/100    │      │   │
│  │  └──────────────┴──────────────┴──────────────┴──────────────┘      │   │
│  │                                                                     │   │
│  │  Overall: 80/100 (Partial Success with High Efficiency)             │   │
│  │                                                                     │   │
│  │  Benefits:                                                           │   │
│  │  • Captures partial success                                          │   │
│  │  • Enables strategy comparison                                      │   │
│  │  • Guides improvement prioritization                                  │   │
│  │  • Reveals hidden tradeoffs                                          │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Partial Success Scoring

```python
class NuancedSuccessScorer:
    """
    Score agent outcomes on multiple dimensions for nuanced evaluation.
    """
    
    def score_outcome(self, trajectory: list, 
                      final_state: dict,
                      reference_solution: dict = None) -> dict:
        """
        Compute multi-dimensional success score.
        
        Returns scores 0-100 for each dimension plus overall weighted score.
        """
        scores = {}
        
        # 1. Functional Score: Does it work?
        scores['functional'] = self._score_functional(
            final_state, reference_solution
        )
        
        # 2. Quality Score: How well does it work?
        scores['quality'] = self._score_quality(
            trajectory, final_state
        )
        
        # 3. Efficiency Score: How efficiently was it achieved?
        scores['efficiency'] = self._score_efficiency(trajectory)
        
        # 4. Robustness Score: How reliable is the solution?
        scores['robustness'] = self._score_robustness(
            trajectory, final_state
        )
        
        # 5. Clarity Score: How clear was the reasoning?
        scores['clarity'] = self._score_clarity(trajectory)
        
        # Weighted overall
        weights = {
            'functional': 0.35,
            'quality': 0.25,
            'efficiency': 0.20,
            'robustness': 0.12,
            'clarity': 0.08
        }
        
        overall = sum(scores[k] * weights[k] for k in scores)
        
        return {
            'dimensions': scores,
            'overall': overall,
            'success_category': self._categorize_success(overall, scores)
        }
    
    def _score_functional(self, final_state: dict, 
                          reference: dict = None) -> float:
        """Score functional correctness (0-100)."""
        if not reference:
            # Binary without reference
            return 100.0 if final_state.get('success') else 0.0
        
        # Compare to reference solution
        check_results = []
        
        for check in reference.get('verification_checks', []):
            passed = self._run_check(check, final_state)
            check_results.append(100.0 if passed else 0.0)
        
        # Bonus for passing edge cases
        edge_case_bonus = len([c for c in check_results if c == 100]) * 5
        
        return min(100, sum(check_results) / max(1, len(check_results)) + edge_case_bonus)
    
    def _score_quality(self, trajectory: list, final_state: dict) -> float:
        """Score solution quality (0-100)."""
        quality_factors = []
        
        # Code quality (if code solution)
        if final_state.get('solution_type') == 'code':
            quality_factors.append(self._assess_code_quality(final_state))
        
        # Documentation quality
        quality_factors.append(self._assess_documentation(trajectory, final_state))
        
        # Edge case handling
        quality_factors.append(self._assess_edge_cases(trajectory))
        
        return sum(quality_factors) / len(quality_factors) if quality_factors else 50.0
    
    def _score_efficiency(self, trajectory: list) -> float:
        """Score path efficiency (0-100)."""
        # Optimal path estimation
        optimal_steps = self._estimate_optimal_steps(trajectory)
        actual_steps = len(trajectory)
        
        # Token efficiency
        total_tokens = sum(
            len(t.get('observation', '')) for t in trajectory
        )
        
        # Combined score
        step_score = max(0, 100 - (actual_steps - optimal_steps) * 5)
        token_score = max(0, 100 - total_tokens / 100)
        
        return (step_score * 0.6 + token_score * 0.4)
    
    def _categorize_success(self, overall: float, 
                            dimensions: dict) -> str:
        """Categorize success level based on scores."""
        if overall >= 90:
            return "EXCELLENT"
        elif overall >= 75:
            return "GOOD"
        elif overall >= 60:
            return "ACCEPTABLE"
        elif overall >= 40:
            return "PARTIAL"
        elif overall >= 20:
            return "POOR"
        else:
            return "FAILURE"
```

---

## Comparative Summary

| Framework | Focus | Key Metrics | Best For |
|-----------|-------|------------|----------|
| **CORE** | Reasoning quality | 5 dimensions (correctness, efficiency, completeness, clarity, robustness) | Research comparison |
| **ContextBench** | Long-context handling | Utilization, position bias, information loss, latency | Context strategy evaluation |
| **Galileo** | Production deployment | Cost, reliability, execution quality | Production monitoring |
| **Nuanced Scoring** | Granular success | Functional, quality, efficiency, robustness, clarity | Partial success analysis |

---

## Connection to Complexity Trap

These evaluation frameworks validate the Complexity Trap findings:

1. **CORE Efficiency Dimension**: Observation masking scores 72.5% vs. LLM Summary 58.4% on efficiency
2. **ContextBench Trajectory**: Masking achieves 52.1% on agent trajectory tasks vs. 48.7% for summarization
3. **Galileo Cost Metrics**: Hybrid achieves best cost/success ($0.30) and token efficiency (4.2)

The multi-dimensional evaluation reveals that:
- **Observation masking excels** in efficiency metrics (cost, tokens, speed)
- **LLM summarization excels** in clarity and some accuracy metrics
- **Hybrid approach** achieves best overall balance across dimensions

---

## References

1. Zhang et al., "CORE: Comprehensive and Omni-directional Review Evaluation for Long Reasoning," 2025
2. "ContextBench: A Benchmark for Long-Context Understanding," 2025
3. Galileo Evaluation Platform, "Agentic Metrics for Production Deployment," 2025

---

*Previous: [Trajectory Elongation](03-trajectory-elongation.md) | [Advanced Strategies](../strategies/04-advanced-strategies.md)*
