# NoLiMa: Long-Context Evaluation Beyond Literal Matching

## Overview

NoLiMa (No Literal Matching) is a rigorous benchmark for evaluating long-context capabilities in Large Language Models. It addresses a critical flaw in traditional Needle-in-a-Haystack (NIAH) tests: models can exploit literal string matching rather than truly understanding and retrieving information from long contexts.

**Citation**: Modarressi et al., "NoLiMa: Long-Context Evaluation Beyond Literal Matching," *Proceedings of ICML*, 2025.

## The Problem with Traditional NIAH

### Literal Matching Loophole

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    TRADITIONAL NEEDLE-IN-HAYSTACK TEST                      │
│                                                                             │
│   Haystack (100K tokens):                                                   │
│   "The quick brown fox jumps over the lazy dog. The weather today is...    │
│    ...[thousands of sentences]...                                         │
│    The special code is: XYZ-123-ABC.                                        │
│    ...[more text]..."                                                       │
│                                                                             │
│   Question: "What is the special code?"                                     │
│                                                                             │
│   How models "cheat":                                                       │
│   1. Look for "special code is:" string match                              │
│   2. Extract "XYZ-123-ABC"                                                 │
│   3. Return answer                                                          │
│                                                                             │
│   This doesn't test:                                                        │
│   - Semantic understanding                                                  │
│   - Information synthesis                                                   │
│   - Reasoning over long contexts                                            │
│   - True comprehension                                                      │
│                                                                             │
│   Result: Inflated performance scores                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Why Literal Matching Matters

| Test Type | What It Measures | Real-World Relevance |
|-----------|-----------------|---------------------|
| Literal NIAH | Pattern matching | Low (can grep) |
| Semantic NIAH | Understanding | High (needs reasoning) |

Real users need models to:
- Understand paraphrased requirements
- Connect information across distant context
- Reason about implicit relationships
- Not rely on keyword matching

## The NoLiMa Solution

### Core Innovation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NOLIMA APPROACH                                     │
│                                                                             │
│   Design Principle: Minimal lexical overlap between question and context   │
│                                                                             │
│   Haystack (100K tokens):                                                   │
│   "The quick brown fox jumps over the lazy dog. The weather today is...   │
│    ...[thousands of sentences]...                                         │
│    Professor Chen expressed deep reservations about the proposed theory,    │
│    noting that the empirical evidence remained insufficient for drawing     │
│    definitive conclusions.                                                │
│    ...[more text]..."                                                     │
│                                                                             │
│   Question: "What was the professor's opinion?"                           │
│                                                                             │
│   Key Features:                                                             │
│   - "Professor Chen" ≠ "the professor" (different reference)              │
│   - "expressed deep reservations" ≠ "opinion" (semantic, not literal)       │
│   - Must infer: "reservations" → negative opinion/skepticism               │
│   - Cannot rely on string matching                                         │
│                                                                             │
│   This tests:                                                               │
│   - Semantic understanding                                                  │
│   - Coreference resolution (Chen → professor)                             │
│   - Inference (reservations → opinion)                                    │
│   - Long-context retention                                                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Needle Design Methodology

NoLiMa needles are carefully constructed with:

| Property | Implementation | Purpose |
|----------|---------------|---------|
| **Minimal lexical overlap** | Question words ≠ Context words | Force semantic reasoning |
| **Diverse linguistic patterns** | Paraphrasing, synonyms, coreference | Test flexibility |
| **Inference required** | Implicit information | Test comprehension |
| **Realistic context** | Natural documents | Ecological validity |

### Question Types

| Type | Example Question | Required Skill |
|------|-----------------|---------------|
| **Coreference** | "What did she decide?" | Link pronoun to entity |
| **Paraphrase** | "What was the main finding?" | Match to "study concluded that..." |
| **Inference** | "How did the character feel?" | Infer from actions/description |
| **Multi-hop** | "Why did X happen?" | Connect distant events |

## The NoLiMa Benchmark

### Dataset Statistics

| Attribute | Value |
|-----------|-------|
| Total questions | 13,000+ |
| Context lengths tested | 1K to 128K tokens |
| Models evaluated | 13 |
| Needle types | 4 categories |
| Lexical overlap | Minimized via design |

### Context Length Points

| Test Point | Tokens | Purpose |
|------------|--------|---------|
| 1K | 1,000 | Baseline (short context) |
| 4K | 4,000 | Early long context |
| 8K | 8,000 | Standard long context |
| 16K | 16,000 | Extended context |
| 32K | 32,000 | Very long context |
| 64K | 64,000 | Extreme context |
| 128K | 128,000 | Maximum test |

## Key Findings

### Performance Degradation

| Model | Short (<1K) | Long (32K) | Drop |
|-------|-------------|------------|------|
| **GPT-4o** | **99.3%** | **69.7%** | **-30pp** |
| Claude 3.5 Sonnet | 98.0% | 60.0% | -38pp |
| Gemini 1.5 Pro | 95.0% | 52.0% | -43pp |
| 11 of 13 models | >90% | <50% | >40pp |

**Critical finding**: Even the best models (GPT-4o) drop 30 percentage points at 32K context.

### Model Rankings at 32K

```
Performance at 32K Context:

GPT-4o              ████████████████████████████████████  69.7%
GPT-4o-mini         ██████████████████████████████         58.2%
Claude 3.5 Sonnet   ████████████████████████████           60.0%
Claude 3 Haiku      ████████████████████████                48.5%
Gemini 1.5 Pro      ██████████████████████                  52.0%
Gemini 1.5 Flash    ████████████████████                    45.1%
Command R+          ████████████████                        38.2%
DBRX                ███████████████                         35.8%
Mixtral 8x22B       ██████████████                          33.4%
Llama-3-70B         ████████████                            28.9%
Qwen-2-72B          ██████████                              25.1%
Yi-34B              ████████                                19.7%
Mistral Large       ███████                                 17.3%
                    │←───────────────────────────────────────→│
                    0%                                     100%
```

### The Deterioration Pattern

```
Performance vs. Context Length (Typical Model):

Accuracy
  100%│
      │    ╭────╮
   90%│   ╱      ╲
      │  ╱        ╲
   80%│ ╱          ╲_________________________
      │╱            (plateau or slow decline)
   70%│
      │
   60%│
      └────────────────────────────────────────▶ Context Length
          1K   4K   8K   16K   32K   64K   128K

Phases:
1. Short (<4K): Strong performance
2. Medium (4K-16K): Rapid degradation begins
3. Long (16K+): Performance stabilizes at lower level
```

### Even Reasoning Models Struggle

| Model | CoT/Reasoning | 32K Performance |
|-------|--------------|-----------------|
| GPT-4o + CoT | Enabled | 71.2% (+1.5pp) |
| Claude 3.5 + CoT | Enabled | 62.1% (+2.1pp) |

**Finding**: Chain-of-thought and reasoning capabilities provide minimal help for long-context retrieval.

### Position Effects

Consistent with "Lost in the Middle":

| Position | 32K Performance |
|----------|-----------------|
| Beginning | ~75% |
| Middle | ~45% |
| End | ~70% |

## Implications for Agent Context Management

### Why Context Management is Critical

```
Agent Context Reality Check:

Traditional view:
"Our model supports 128K context, so agents can use 128K"

NoLiMa reality:
"At 32K, performance drops to ~50-70%, even for best models"
"At 128K, effective comprehension is severely degraded"

Implication:
Agents should keep context well below degradation threshold
Use context management to stay within effective window
```

### The Effective Context Threshold

| Advertised Context | NoLiMa 50% Point | Recommended Agent Limit |
|---------------------|------------------|----------------------|
| 128K | ~32K | ~20K |
| 200K | ~50K | ~30K |
| 1M | ~100K | ~60K |

**Recommendation**: Keep agent context at 20-30% of advertised maximum.

### Validating Observation Masking

| Claim | NoLiMa Support |
|-------|----------------|
| Old context is poorly retained | ✅ Confirmed at 32K+ |
| Recent context is well retained | ✅ End position strong |
| Most tokens are "wasted" | ✅ Middle positions fail |
| Masking is low-risk | ✅ Context mostly ignored anyway |

### Comparison with "Lost in the Middle"

| Aspect | Liu et al. (2023) | Modarressi et al. (2025) |
|--------|------------------|---------------------------|
| Test type | Multi-doc QA, KV retrieval | Semantic NIAH |
| Lexical overlap | Some | Minimized |
| Models tested | GPT-3.5, Llama-2, Claude | 13 diverse models |
| Finding | 25-30pp drop | 30-40pp drop |
| Key innovation | Position effects | Semantic understanding |
| Combined message | LLMs struggle with long context | Even more than we thought |

## Design Implications

### For Agent Developers

| Decision | Recommendation | Rationale |
|----------|-----------------|-----------|
| Context limit | 20-30K effective | Beyond this, retrieval fails |
| Compression | Aggressive | Most context won't be accessed |
| Recency | Prioritize | End position works best |
| Repetition | Periodic | Re-insert critical info |
| Summary quality | Critical | Semantic accuracy matters |

### For Model Users

| Scenario | Strategy |
|----------|----------|
| 128K context available | Use up to 32K reliably |
| Need 64K+ context | Expect retrieval failures |
| Critical info at 50K+ | Repeat periodically |
| Complex reasoning + long context | Consider RAG over full context |

## Benchmark Construction

### Needle Types Detail

#### 1. Coreference Resolution

```
Context: "Dr. Sarah Johnson arrived at the conference. She presented 
          her research on neural networks. The professor answered 
          questions for two hours."

Question: "Who answered questions?"

Mapping: "Dr. Sarah Johnson" → "She" → "The professor"
         Must resolve coreference chain
```

#### 2. Paraphrase Recognition

```
Context: "The experimental results conclusively demonstrated that 
          the hypothesis was incorrect."

Question: "What did the study find?"

Mapping: "conclusively demonstrated" = "found"
         "hypothesis was incorrect" = "negative result"
```

#### 3. Inference

```
Context: "After reviewing the data, John slammed his fist on the 
          table and stormed out of the room."

Question: "How did John react to the data?"

Inference: Physical actions indicate anger/frustration
           No explicit emotion stated
```

#### 4. Multi-hop Connection

```
Context: [Paragraph about Alice starting a company at position 10K]
         ...
         [Paragraph about Bob investing in Alice's company at position 80K]

Question: "Who invested in Alice's company?"

Requires: Connecting information across 70K tokens
```

### Quality Control

| Check | Method | Purpose |
|-------|--------|---------|
| Lexical overlap | Token overlap ratio | Ensure minimization |
| Answerability | Human verification | Questions are answerable |
| Uniqueness | Deduplication | No duplicate needles |
| Naturalness | Human review | Context reads naturally |

## Comparison to Other Benchmarks

| Benchmark | Focus | Lexical Overlap | Difficulty |
|-----------|-------|-----------------|------------|
| Traditional NIAH | Retrieval | High | Low |
| NoLiMa | Semantic retrieval | Low | High |
| LongBench | Multi-task | Medium | Medium |
| L-Eval | Long-document QA | Medium | Medium |
| RULER | Synthetic tasks | Variable | Variable |

**NoLiMa advantage**: Isolates semantic understanding from pattern matching.

## Future Directions

### Potential Extensions

| Extension | Description |
|-----------|-------------|
| Multi-modal NoLiMa | Images, audio in long context |
| Code NoLiMa | Long code context comprehension |
| Structured NoLiMa | Tables, JSON in long context |
| Dynamic NoLiMa | Changing context over time |

### Open Questions

1. Can training improve long-context retrieval?
2. Do different architectures (Mamba, RWKV) perform better?
3. What is the fundamental limit of attention-based retrieval?
4. Can explicit memory mechanisms help?

## Citations

### Primary Paper

```bibtex
@inproceedings{modarressi2025nolima,
  title={NoLiMa: Long-Context Evaluation Beyond Literal Matching},
  author={Modarressi, Ali and Deilamsalehy, Hanieh and Dernoncourt, Franck and Bui, Trung and Rossi, Ryan A and Yoon, Seunghyun and Sch{\"u}tze, Hinrich},
  booktitle={Proceedings of the International Conference on Machine Learning},
  year={2025},
  publisher={PMLR}
}
```

### Dataset and Code

- **Dataset**: [github.com/adobe-research/NoLiMa](https://github.com/adobe-research/NoLiMa)
- **Paper**: [arXiv:2502.05167](https://arxiv.org/abs/2502.05167)

## Related Research

| Paper | Contribution |
|-------|-------------|
| Liu et al. (2024) | Position effects in long context |
| Hong et al. (2025) | "Context Rot" degradation |
| Xiao et al. (2025) | Trajectory reduction for agents |
| Tang et al. (2025) | Deep search with dynamic context |

## Next Steps

- **[Lost in the Middle](01-lost-in-the-middle.md)** - Foundational position effects research
- **[Related Papers](03-related-papers.md)** - Concurrent agent context research
- **[The Problem](../architecture/02-the-problem.md)** - Context bloat in practice
- **[Performance Results](../experiments/02-performance-results.md)** - Empirical agent results
