# Efficient Context Management for LLM-Powered Agents

> **A systematic study comparing observation masking and LLM summarization for agent context management**

Research demonstrating that simple observation masking matches or exceeds the performance of complex LLM-based summarization at significantly lower cost, with a novel hybrid approach pushing the efficiency-effectiveness frontier even further.

## Key Findings

- **Context management reduces costs by >50%** without sacrificing performance
- **Simple beats sophisticated** - Observation masking achieves lowest cost while matching LLM-summary solve rates
- **Trajectory elongation** - LLM summaries can cause agents to run longer by hiding failure signals
- **Hybrid approach wins** - 7% cheaper than masking, 11% cheaper than summarization, with better solve rates

## Research Context

| Attribute | Value |
|-----------|-------|
| Authors | Lindenbauer et al. (JetBrains Research, TUM) |
| Venue | NeurIPS 2025 DL4C Workshop |
| Benchmark | SWE-bench Verified (500 instances) |
| Models Tested | Qwen3-32B/480B, Gemini 2.5 Flash |
| Max Turns | 250 |

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│         Software Engineering Agent          │
│         (SWE-agent / OpenHands)             │
├─────────────────────────────────────────────┤
│           Agent Policy (LLM)                │
├─────────────────────────────────────────────┤
│         Context Management                  │
│  ┌───────────────────────────────────────┐  │
│  │ Observation Masking (M=10 turns)     │  │
│  │ - Hide old tool outputs              │  │
│  │ - Preserve reasoning chain           │  │
│  └───────────────────────────────────────┘  │
│  ┌───────────────────────────────────────┐  │
│  │ LLM Summarization (N=21, M=10)       │  │
│  │ - Compress old turns                 │  │
│  │ - Bounded context growth             │  │
│  └───────────────────────────────────────┘  │
├─────────────────────────────────────────────┤
│         Trajectory Storage                  │
│  (Linear Hash Table - 4KB pages)          │
├─────────────────────────────────────────────┤
│           Tool Execution                    │
│  (File read, Test run, Code edit)           │
└─────────────────────────────────────────────┘
```

## Performance Highlights

| Metric | Observation Masking | LLM Summary | Hybrid |
|--------|---------------------|-------------|--------|
| Cost reduction (Qwen3-480B) | -52.7% | -50.4% | **-59%** |
| Solve rate (Qwen3-480B) | 54.8% | 53.8% | **57.4%** |
| Mean turns (Gemini Flash) | 44 | 52 (+18%) | — |
| Summary API overhead | 0% | 5-7% | Minimal |

## Documentation Map

```
reference/llm-context-management/
├── index.md                          # Comprehensive reference
├── architecture/
│   ├── 01-research-summary.md        # Core findings and contributions
│   ├── 02-the-problem.md             # Context bloat and impact
│   └── 03-comparison.md              # Strategy comparison overview
├── strategies/
│   ├── 01-observation-masking.md     # Masking implementation details
│   ├── 02-llm-summarization.md       # Summary generation approach
│   └── 03-hybrid-approach.md         # Combined strategy
├── experiments/
│   ├── 01-experimental-setup.md      # Benchmark configuration
│   ├── 02-performance-results.md     # Results with confidence intervals
│   └── 03-trajectory-elongation.md   # The elongation phenomenon
├── related-work/
│   ├── 01-lost-in-the-middle.md      # Context window limitations
│   ├── 02-nolima.md                  # Long-context benchmark
│   └── 03-related-papers.md          # Concurrent research
└── challenges/
    ├── 01-limitations.md             # Scope and constraints
    └── 02-future-work.md             # Open problems
```

## Quick Links

- **[Complete Reference](index.md)** - Full documentation and navigation
- **[Research Summary](architecture/01-research-summary.md)** - Key contributions
- **[Observation Masking](strategies/01-observation-masking.md)** - The simple winner
- **[LLM Summarization](strategies/02-llm-summarization.md)** - The complex alternative
- **[Performance Results](experiments/02-performance-results.md)** - Benchmark data
- **[Trajectory Elongation](experiments/03-trajectory-elongation.md)** - The hidden cost

## The Core Insight

**Observation masking works because:**

1. **84% of trajectory tokens** are environment observations (file reads, test outputs)
2. Simply hiding these with placeholders removes the bulk of noise
3. The agent's reasoning chain remains intact
4. No additional LLM calls required for summarization

**LLM summarization's hidden cost:**

1. Summaries can smooth over failure signals
2. Agents don't realize they're stuck
3. Continue past the point of sensible termination
4. Trajectory elongation erodes efficiency gains

## Citation

```bibtex
@inproceedings{lindenbauer2025complexity,
  title={The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management},
  author={Lindenbauer, Tobias and Slinko, Igor and Felder, Ludwig and Bogomolov, Egor and Zharov, Yaroslav},
  booktitle={NeurIPS 2025 Workshop: Deep Learning for Code in the Agentic Era},
  year={2025}
}
```

## Resources

- **Paper**: [arXiv:2508.21433](https://arxiv.org/pdf/2508.21433)
- **Code**: [github.com/JetBrains-Research/the-complexity-trap](https://github.com/JetBrains-Research/the-complexity-trap)
- **Data**: [HuggingFace Dataset](https://huggingface.co/datasets/JetBrains-Research/the-complexity-trap)
- **Blog Post**: [JetBrains Research](https://blog.jetbrains.com/research/2025/12/efficient-context-management/)

---

*JetBrains Research & Technical University of Munich, NeurIPS 2025*
