# Lost in the Middle: How Language Models Use Long Contexts

## Overview

"Lost in the Middle" is a foundational research paper by Liu et al. (2023) that demonstrates a critical limitation of Large Language Models: they perform significantly worse when relevant information is located in the middle of long input contexts, even for models explicitly designed to handle long contexts.

**Citation**: Liu et al., "Lost in the Middle: How Language Models Use Long Contexts," *Transactions of the Association for Computational Linguistics* (TACL), 2024.

## Core Finding

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   PERFORMANCE BY INFORMATION POSITION                         │
│                                                                             │
│   Performance (% correct)                                                 │
│    100% │                                                                   │
│         │    ┌─────┐                                              ┌─────┐  │
│     90% │    │     │                                              │     │  │
│         │    │     │    ┌────────────────────────────────────┐   │     │  │
│     80% │    │High │    │                                    │   │High │  │
│         │    │     │    │         LOWER PERFORMANCE            │   │     │  │
│     70% │    │     │    │                                    │   │     │  │
│         │    │     │    │       Information "Lost in the      │   │     │  │
│     60% │    │     │    │           Middle"                  │   │     │  │
│         │    │     │    │                                    │   │     │  │
│     50% │    │     │    └────────────────────────────────────┘   │     │  │
│         │    │     │                                              │     │  │
│     40% │    └─────┘                                              └─────┘  │
│         │   Beginning                    Middle                    End      │
│         └────────────────────────────────────────────────────────────────   │
│                                                                             │
│   KEY INSIGHT: LLMs perform best when relevant information is at the        │
│   BEGINNING or END of context. Performance drops significantly when       │
│   information is in the MIDDLE, even with long-context models.              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## The Research

### Authors

- Nelson F. Liu (Stanford)
- Kevin Lin (Stanford)
- John Hewitt (Stanford)
- Ashwin Paranjape (Stanford)
- Michele Bevilacqua (Bocconi University)
- Fabio Petroni (Meta AI)
- Percy Liang (Stanford)

### Tasks Evaluated

| Task | Description | Why It Matters |
|------|-------------|--------------|
| **Multi-Document QA** | Answer questions requiring information from multiple documents | Tests retrieval from diverse sources |
| **Key-Value Retrieval** | Extract value given key from a set of key-value pairs | Tests structured data access |

### Experimental Design

**Controlled manipulation**: Researchers systematically varied the position of relevant information while holding total context length constant.

```
Context Structure:
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Irrelevant documents × N] [Relevant document] [Irrelevant documents × M]    │
│                                                                              │
│  Position variation:                                                         │
│  - Beginning: N=0, M=max                                                    │
│  - Middle: N=M=max/2                                                        │
│  - End: N=max, M=0                                                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Results

### Performance Degradation

| Model | Beginning | Middle | End | Middle Drop |
|-------|-----------|--------|-----|-------------|
| GPT-3.5-Turbo | ~90% | ~60% | ~85% | **-30pp** |
| GPT-3.5-Turbo (16K) | ~85% | ~55% | ~80% | **-30pp** |
| GPT-3.5-Turbo (32K) | ~80% | ~50% | ~75% | **-30pp** |
| Llama-2 (4K) | ~85% | ~60% | ~80% | **-25pp** |
| Llama-2 (8K) | ~80% | ~55% | ~75% | **-25pp** |
| Claude (100K) | ~90% | ~65% | ~85% | **-25pp** |

**Consistent pattern**: 25-30 percentage point drop when information is in the middle.

### Context Length Doesn't Help

```
GPT-3.5-Turbo Family:

Context Window:     4K        16K        32K
                    │          │          │
Beginning perf:     90%        85%        80%
Middle perf:        60%        55%        50%
End perf:           85%        80%        75%
                    │          │          │
Middle gap:        -30pp      -30pp      -30pp

FINDING: Larger context windows don't fix the "lost in the middle" problem!
```

### Why This Matters for Agent Context Management

| Implication | Explanation |
|-------------|-------------|
| **Effective context << Advertised context** | 128K window may have ~32K effective |
| **Context management is essential** | Keeping context short preserves performance |
| **Recency bias is real** | Recent context (end position) gets priority |
| **Old context is "lost"** | Early context may as well not be there |

## Mechanism: Attention Patterns

### The Attention Explanation

```
Multi-Head Attention Pattern in Long Contexts:

Position:  1     50    100   150   200   250   300   350   400
           │      │      │     │     │     │     │     │     │
Attention: ████████████████████░░░░░░░░░░░░░░░░░░████████████████
            ↑                                      ↑
         Beginning                              End
         (strong attn)                         (strong attn)
                                              
         Middle (weak attention)
```

The attention mechanism naturally focuses on:
1. **Beginning** - Initial context, system prompts
2. **End** - Most recent information
3. **Middle** - Attenuated attention, harder to retrieve

### U-Shaped Attention Curve

```
Attention Weight by Position:

Weight
  ▲
  │    ╭─────╮
  │   ╱       ╲
  │  ╱         ╲_________________________╱
  │ ╱           (middle gets less attention)
  │╱
  └────────────────────────────────────────▶ Position
     Beginning              Middle          End
```

## Connection to Agent Context Management

### Implication 1: Context Compression is Necessary

```
Unmanaged agent context (250 turns):
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Old turns ... 200 turns ago] [Recent turns ... now]                        │
│      ↓                                                             ↓        │
│   "Lost in the middle"                                         "End position"│
│   (weakly attended)                                            (strong attn)│
│                                                                              │
│ Result: 200 turns of "noise" that the model can't effectively use         │
└─────────────────────────────────────────────────────────────────────────────┘

With context management:
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Summary/placeholder] [Recent turns ... now]                               │
│        ↓                      ↓                                             │
│   Compressed              End position (strong attn)                        │
│                                                                              │
│ Result: Context fits within effective window size                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Implication 2: Recency is Critical

| Observation | Implication for Agents |
|-------------|------------------------|
| End position gets strong attention | Recent context is most important |
| Beginning is well-attended | System/user prompts preserved |
| Middle is weakly attended | Old observations are "lost" anyway |

**This justifies observation masking**: If old observations are "lost in the middle" anyway, hiding them with placeholders has minimal impact on performance while reducing cost.

### Implication 3: The "Effective Context" Concept

| Model | Advertised | Effective | Ratio |
|-------|-----------|-----------|-------|
| GPT-4 | 128K | ~32K | 25% |
| Claude 3 | 200K | ~50K | 25% |
| Gemini 2.5 Flash | 1M | ~100K | 10% |

**Effective context**: The length at which performance degradation becomes significant.

## Supporting Evidence from Agent Research

### The 84% Rule

In SE agent trajectories (from current research):
```
Token Distribution:
Observations:  ████████████████████████████████████████████████████  84%
Reasoning:     ████                                              8%
Actions:       ████                                              8%

Middle of long trajectory:
- Old observations: Weakly attended ("lost in the middle")
- Recent reasoning: Well attended ("end position")

Conclusion: Most of the context is in poorly-attended positions anyway!
```

### Why Masking Works

| Factor | Explanation |
|--------|-------------|
| Old observations are "lost" anyway | Due to attention patterns |
| Recent context preserved | End position, well attended |
| Reasoning chain intact | Critical for decision-making |
| Cost dramatically reduced | 84% of tokens masked |

## Beyond Literal Matching

### NoLiMa Extension

Modarressi et al. (2025) extended this work with the **NoLiMa** benchmark:

**Key innovation**: Minimal lexical overlap between questions and relevant context

```
Traditional NIAH (Needle-in-Haystack):
- "The needle is: [fact]"
- Model can match literal strings

NoLiMa:
- Question: "What was the professor's opinion?"
- Context: "The lecturer expressed skepticism about..."
- Requires semantic inference, not literal matching
```

### NoLiMa Results

| Model | Short (<1K) | Long (32K) | Performance Drop |
|-------|-------------|------------|------------------|
| GPT-4o | 99.3% | 69.7% | **-30pp** |
| Claude 3.5 Sonnet | 98% | 60% | **-38pp** |
| 11 of 13 models | >90% | <50% | **>40pp** |

**Finding**: Even with reasoning/CoT prompting, models struggle with long-context retrieval.

## Implications for Agent Design

### Design Principle: Effective Context Budget

```
Don't design for advertised context window.
Design for effective context window.

Example:
- Advertised: 128K tokens
- Effective: ~32K tokens  
- Design target: Keep context < 30K tokens
- Management strategy: Compress beyond 30K
```

### Position Strategy

| Information Type | Recommended Position |
|-----------------|---------------------|
| Task definition | Beginning (system prompt) |
| User requirements | Beginning (user prompt) |
| Critical constraints | Recent context or repeated |
| Exploration history | Summarized/placeholder |
| Current state | Recent (end position) |

## Connection to Other Research

### Context Rot (Hong et al., 2025)

> "Context is a finite attention budget. Just because a model *accepts* 100K tokens doesn't mean it *pays equal attention* to all of them."

**The analogy**: Like human memory, LLM "attention budget" gets diluted with more information.

### The ReAct Pattern

Yao et al.'s ReAct (2023) framework:
```
Reasoning → Action → Observation → ... → Answer

The observations (often verbose) accumulate, but:
- Recent observations: End position, well attended
- Old observations: Middle position, poorly attended
- Only recent observations truly matter
```

### Needle-in-Haystack (NIAH) Tests

Standard evaluation for long-context models:
```
Hide a specific fact ("needle") in long text ("haystack")
Test if model can retrieve it

Limitation: Models can exploit literal matches
NoLiMa fixes this with semantic matching
```

## Practical Guidelines

### For Agent Developers

1. **Assume smaller effective context**:
   - Advertised 128K → Plan for 32K
   - Advertised 1M → Plan for 100K

2. **Put critical info at beginning or end**:
   - System prompts: Beginning
   - Current state: Recent (end)
   - Constraints: Repeat periodically

3. **Compress aggressively**:
   - Old observations: Mask/summarize
   - Trajectory: Keep under effective limit
   - Redundancy: Eliminate

4. **Test with position variations**:
   - Critical info at different positions
   - Verify model can access it

### For Model Users

| Scenario | Recommendation |
|----------|----------------|
| 32K context window | Use up to ~20K for reliable performance |
| 128K context window | Use up to ~40K for reliable performance |
| 1M context window | Use up to ~200K for reliable performance |
| Retrieval tasks | Put target at beginning or end |
| Multi-hop reasoning | Keep intermediate results visible |

## Citations and References

### Primary Paper

```bibtex
@article{liu2024lost,
  title={Lost in the Middle: How Language Models Use Long Contexts},
  author={Liu, Nelson F and Lin, Kevin and Hewitt, John and Paranjape, Ashwin and Bevilacqua, Michele and Petroni, Fabio and Liang, Percy},
  journal={Transactions of the Association for Computational Linguistics},
  volume={12},
  pages={157--173},
  year={2024},
  publisher={MIT Press}
}
```

### Related Papers

```bibtex
@article{modarressi2025nolima,
  title={NoLiMa: Long-Context Evaluation Beyond Literal Matching},
  author={Modarressi, Ali and Deilamsalehy, Hanieh and Dernoncourt, Franck and Bui, Trung and Rossi, Ryan A and Yoon, Seunghyun and Sch{\"u}tze, Hinrich},
  journal={Proceedings of ICML},
  year={2025}
}

@article{hong2025context,
  title={Context Rot: How Increasing Input Tokens Impacts LLM Performance},
  author={Hong, Kelly and Troynikov, Anton and Huber, Jeff},
  journal={Chroma Research Technical Report},
  year={2025}
}

@inproceedings{yao2023react,
  title={ReAct: Synergizing Reasoning and Acting in Language Models},
  author={Yao, Shunyu and Zhao, Jeffrey and Yu, Dian and Du, Nan and Shafran, Izhak and Narasimhan, Karthik and Cao, Yuan},
  booktitle={International Conference on Learning Representations},
  year={2023}
}
```

## Next Steps

- **[NoLiMa Benchmark](02-nolima.md)** - Extended long-context evaluation
- **[Related Papers](03-related-papers.md)** - Concurrent research
- **[The Problem](../architecture/02-the-problem.md)** - Context bloat in agents
- **[Observation Masking](../strategies/01-observation-masking.md)** - Response to "lost in the middle"
- **[Performance Results](../experiments/02-performance-results.md)** - Empirical validation
