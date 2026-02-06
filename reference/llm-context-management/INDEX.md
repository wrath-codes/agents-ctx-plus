# llm-context-management — Sub-Index

> Context management research for LLM-powered SE agents (28 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Introduction and overview|

### [architecture](architecture/)

|file|description|
|---|---|
|[01-research-summary.md](architecture/01-research-summary.md)|Research summary — core contributions, Complexity Trap thesis|
|[02-the-problem.md](architecture/02-the-problem.md)|The problem — context bloat, 84% observation tokens|
|[03-comparison.md](architecture/03-comparison.md)|Strategy comparison — masking vs summarization|

### [strategies](strategies/)

|file|description|
|---|---|
|[01-observation-masking.md](strategies/01-observation-masking.md)|Observation masking — simple omission, placeholder tokens|
|[02-llm-summarization.md](strategies/02-llm-summarization.md)|LLM summarization — compression via LLM calls|
|[03-hybrid-approach.md](strategies/03-hybrid-approach.md)|Hybrid — masking default + summarization fallback (-59% cost)|
|[04-advanced-strategies.md](strategies/04-advanced-strategies.md)|Advanced — H-MEM, HiAgent, Re-TRAC, CASK, ACE, G-Memory|
|[04-semantic-triggers.md](strategies/04-semantic-triggers.md)|Semantic triggers — intent-based, boundary detection|
|[05-acon-training-compression.md](strategies/05-acon-training-compression.md)|ACON — training-time compression guideline optimization|
|[06-ttt-e2e-training.md](strategies/06-ttt-e2e-training.md)|TTT-E2E — test-time training for long-context compression|

### [experiments](experiments/)

|file|description|
|---|---|
|[01-experimental-setup.md](experiments/01-experimental-setup.md)|Setup — SWE-bench Verified, 500 instances, model configs|
|[02-performance-results.md](experiments/02-performance-results.md)|Results — cost reduction (>50%), solve rates, confidence intervals|
|[03-trajectory-elongation.md](experiments/03-trajectory-elongation.md)|Trajectory elongation — summaries cause +15-18% longer runs|
|[05-trajectory-evaluation.md](experiments/05-trajectory-evaluation.md)|Evaluation — CORE, ContextBench, Galileo metrics|

### [cognitive](cognitive/)

|file|description|
|---|---|
|[01-working-memory-hub.md](cognitive/01-working-memory-hub.md)|Working memory — Baddeley's model, episodic buffer for LLMs|

### [related-work](related-work/)

|file|description|
|---|---|
|[01-lost-in-the-middle.md](related-work/01-lost-in-the-middle.md)|Lost in the Middle — attention degradation in long contexts|
|[02-nolima.md](related-work/02-nolima.md)|NoLiMa — long-context evaluation beyond literal matching|
|[03-related-papers.md](related-work/03-related-papers.md)|Related papers — AgentDiet, curriculum learning, hierarchical memory|

### [production](production/)

|file|description|
|---|---|
|[01-google-adk-context.md](production/01-google-adk-context.md)|Google ADK — tiered context, compiled view, prefix caching|
|[02-anthropic-context.md](production/02-anthropic-context.md)|Anthropic — attention budget, compaction, sub-agents|
|[03-vercel-agents-md.md](production/03-vercel-agents-md.md)|Vercel AGENTS.md — passive context beats active retrieval (100% vs 79%)|

### [hardware](hardware/)

|file|description|
|---|---|
|[01-plena-hardware.md](hardware/01-plena-hardware.md)|PLENA — HW-SW co-design, systolic array, FlashAttention|

### [safety](safety/)

|file|description|
|---|---|
|[01-lrm-jailbreaks.md](safety/01-lrm-jailbreaks.md)|LRM jailbreaks — 97.14% ASR, alignment regression|
|[02-dbd-intervention.md](safety/02-dbd-intervention.md)|DBDI — bi-directional safety alignment|

### [challenges](challenges/)

|file|description|
|---|---|
|[01-limitations.md](challenges/01-limitations.md)|Limitations — scope constraints|
|[02-future-work.md](challenges/02-future-work.md)|Future work — open problems|

### Core Finding
```
Simple observation masking ≈ LLM summarization in solve rate
but at >50% lower cost. Hybrid approach: -59% cost, +2.6pp solve rate.
Vercel: passive AGENTS.md (100%) beats active skills (56-79%).
```

---
*28 files*
