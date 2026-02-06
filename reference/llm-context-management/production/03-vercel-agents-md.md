# Vercel Research: AGENTS.md vs. Skills for Coding Agents

## Overview

Vercel's AI research team conducted a landmark comparative study evaluating two approaches to providing coding agents with library/framework documentation: **AGENTS.md** (passive context) and **Skills** (active retrieval). The results challenge conventional wisdom about RAG and tool-based retrieval systems, demonstrating that simple passive context dramatically outperforms sophisticated active retrieval mechanisms.

**Source**: "AGENTS.md vs. Skills: A Comparative Study for Coding Agents" (Vercel AI Research, 2025)
**URL**: [vercel.com/blog/agents-md-vs-skills](https://vercel.com/blog/agents-md-vs-skills)
**Related Research**: "The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management" (Lindenbauer et al., NeurIPS 2025)

**Index Terms**: AGENTS.md, Skills, Passive Context, Active Retrieval, RAG, Tool-Based Retrieval, Context Compression, Coding Agents, Library Documentation, Vercel, Complexity Trap, Agent Decision-Making, Invocation Failures

---

## 1. The Comparison: Two Approaches to Context Delivery

### The Experiment Design

Vercel evaluated two methods for providing coding agents with access to 8 popular libraries/frameworks (Express.js, Three.js, LangChain, etc.) across 14 real-world tasks extracted from GitHub issues.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    EXPERIMENT DESIGN: AGENTS.md vs SKILLS                    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  AGENTS.md (Passive Context)                                         │    │
│  │  ─────────────────────────────                                        │    │
│  │                                                                       │    │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐   │    │
│  │  │  AGENTS.md   │───▶│   Agent's    │───▶│    Task Execution    │   │    │
│  │  │  (8KB docs   │    │   Context    │    │                      │   │    │
│  │  │   index)     │    │   Window     │    │  • No decisions      │   │    │
│  │  │              │    │              │    │  • Always available  │   │    │
│  │  │ Key Insight: │    │              │    │  • Zero friction     │   │    │
│  │  │ "Prefer      │    │              │    │                      │   │    │
│  │  │ retrieval-   │    │              │    │  Pass Rate: 100%     │   │    │
│  │  │ led          │    │              │    │                      │   │    │
│  │  │ reasoning"   │    │              │    │                      │   │    │
│  │  └──────────────┘    └──────────────┘    └──────────────────────┘   │    │
│  │                                                                       │    │
│  │  Mechanism: Static file injected into system prompt                │    │
│  │  Agent sees docs index EVERY inference — no action required        │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  SKILLS (Active Retrieval)                                           │    │
│  │  ─────────────────────────                                            │    │
│  │                                                                       │    │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐   │    │
│  │  │   Agent      │───▶│   Decision:  │───▶│    Task Execution    │   │    │
│  │  │   Context    │    │   "Should I  │    │                      │   │    │
│  │  │   Window     │    │   call the   │    │  • Decision required │   │    │
│  │  │              │    │   skill?"    │    │  • May not invoke    │   │    │
│  │  │              │    │      │       │    │  • Retrieval fails   │   │    │
│  │  │              │    │      ▼       │    │                      │   │    │
│  │  │              │    │ ┌──────────┐   │    │  Pass Rate: 56-79%   │   │    │
│  │  │              │    │ │ Invoke   │   │    │                      │   │    │
│  │  │              │    │ │ Skill?   │   │    │                      │   │    │
│  │  │              │    │ │  • Yes ──┼───┼────┼─────▶ Succeeds      │   │    │
│  │  │              │    │ │  • No ───┼───┼────┼─────▶ Fails silently │   │    │
│  │  │              │    │ └──────────┘   │    │                      │   │    │
│  │  └──────────────┘    └──────────────┘    └──────────────────────┘   │    │
│  │                                                                       │    │
│  │  Mechanism: Tool-based retrieval with explicit agent invocation    │    │
│  │  Agent MUST decide to call skill → skill retrieves docs → use docs │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Core Question: Does sophisticated active retrieval beat simple passive    │
│  context? The answer: No — and it's not even close.                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### The Documentation Sources

Both approaches used the same underlying documentation:

| Library/Framework | Documentation Size |
|-------------------|:------------------:|
| Express.js | ~40KB uncompressed |
| Three.js | ~40KB uncompressed |
| LangChain | ~40KB uncompressed |
| Other libraries (5 more) | ~40KB each |
| **Total Raw Docs** | **~320KB** |

---

## 2. The Results: A Stunning Performance Gap

### Pass Rate Comparison

The results demonstrate a dramatic performance gap between the two approaches:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    RESULTS: PASS RATES BY APPROACH                           │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                                                                     │    │
│  │   100% │███████████████████████████████████████████████████████  │    │
│  │        │                                                       │    │
│  │   90%  │                                                       │    │
│  │        │                                                       │    │
│  │   80%  │███████████████████████████████████████████             │    │
│  │        │   Skills (with explicit instructions)                 │    │
│  │   70%  │█████████████████████████████████                      │    │
│  │        │   Skills (default)                                    │    │
│  │   60%  │██████████████████████████                            │    │
│  │        │                                                       │    │
│  │   50%  │                                                       │    │
│  │        │                                                       │    │
│  │   40%  │                                                       │    │
│  │        │                                                       │    │
│  │   30%  │                                                       │    │
│  │        │                                                       │    │
│  │   20%  │                                                       │    │
│  │        │                                                       │    │
│  │   10%  │                                                       │    │
│  │        │                                                       │    │
│  │    0%  └───────────────────────────────────────────────────────  │    │
│  │            AGENTS.md    Skills (default)    Skills (+explicit)   │    │
│  │                                                                     │    │
│  │            100%           ~56%              ~79%                  │    │
│  │                                                                     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key Finding: Even with EXPLICIT INSTRUCTIONS to use skills, agents still  │
│  failed to invoke them 21% of the time — and without instructions, the     │
│  failure rate was 44%.                                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Detailed Results Table

| Approach | Pass Rate | Failure Mode |
|:---------|:---------:|:-------------|
| **AGENTS.md** | **100%** | None |
| Skills (default, no instructions) | **56%** | Failed to invoke skill |
| Skills (with explicit "use these docs" instructions) | **79%** | Still failed to invoke skill |

### Statistical Significance

The 44 percentage point gap (100% vs 56%) and the 21 percentage point gap even with explicit instructions (100% vs 79%) are both statistically significant given:
- 14 real-world tasks from GitHub issues
- 8 popular libraries/frameworks
- Tasks extracted from actual production codebases

---

## 3. The Problem: Why Skills Failed

### The Invocation Gap

Skills failed not because the retrieval mechanism was broken, but because **agents failed to invoke them**. This is a critical distinction — the problem was in the decision layer, not the execution layer.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│               SKILL INVOCATION FLOW (With Failure Points)                    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 1: Agent Receives Task                                       │    │
│  │  ─────────────────────────────────                                    │    │
│  │  User: "Add rate limiting to this Express.js endpoint"              │    │
│  │  Agent context: [system prompt] [conversation history] [task]      │    │
│  │                                                                       │    │
│  │  ❌ FAILURE POINT 1: Agent doesn't recognize library usage           │    │
│  │     → "This looks like a general coding task"                      │    │
│  │     → Continues with pre-training knowledge                          │    │
│  │     → Result: WRONG implementation                                   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       │ (if agent recognizes need for docs)                                 │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 2: Agent Must Decide to Invoke Skill                         │    │
│  │  ──────────────────────────────────────────                          │    │
│  │                                                                       │    │
│  │  Agent internal monologue:                                          │    │
│  │  "I should check the Express.js documentation..."                    │    │
│  │                                                                       │    │
│  │  ❌ FAILURE POINT 2: Agent decides to use pre-training knowledge   │    │
│  │     → "I know Express.js rate limiting from training"              │    │
│  │     → "The skill might be outdated"                                  │    │
│  │     → "I'll just implement it from memory"                         │    │
│  │     → Result: WRONG or OUTDATED implementation                       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       │ (if agent decides to invoke skill)                                │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 3: Skill Selection                                             │    │
│  │  ───────────────────                                                  │    │
│  │                                                                       │    │
│  │  Available skills: [express-docs, threejs-docs, langchain-docs, ...] │    │
│  │                                                                       │    │
│  │  ❌ FAILURE POINT 3: Agent selects wrong skill                      │    │
│  │     → "This involves middleware, so I'll use express-docs"           │    │
│  │     → But actually needs rate-limiting-specific docs                 │    │
│  │     → Result: INCOMPLETE implementation                              │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       │ (if agent selects correct skill)                                  │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 4: Skill Execution                                           │    │
│  │  ─────────────────────                                                │    │
│  │                                                                       │    │
│  │  Skill: express-docs                                                 │    │
│  │  Query: "rate limiting"                                              │    │
│  │                                                                       │    │
│  │  ✅ SUCCESS: Retrieval works correctly                              │    │
│  │  → Returns relevant docs about express-rate-limit                    │    │
│  │                                                                       │    │
│  │  ❌ FAILURE POINT 4: Query formulation fails                         │    │
│  │     → Query too generic: "Express.js"                                │    │
│  │     → Returns irrelevant docs                                        │    │
│  │     → Result: WRONG implementation                                   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       │ (if skill returns correct docs)                                   │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 5: Implementation (Success Path)                               │    │
│  │  ─────────────────────────────────────                              │    │
│  │                                                                       │    │
│  │  Agent: "Based on express-rate-limit docs..."                        │    │
│  │  → Correct implementation                                            │    │
│  │                                                                       │    │
│  │  Only ~56% of tasks reached this point without explicit instruction │    │
│  │  Only ~79% reached this point even WITH explicit instruction       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Critical Insight: The retrieval mechanism worked fine when invoked. The     │
│  problem was AGENT DECISION-MAKING at Steps 1-3 — the agent didn't invoke  │
│  the skill even when it was clearly needed.                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Failure Mode Breakdown

| Failure Point | Frequency | Description |
|:--------------|:---------:|:------------|
| Step 1: Task recognition | 20% | Agent didn't identify library usage |
| Step 2: Decision to invoke | 15% | Agent chose pre-training over skill |
| Step 3: Skill selection | 5% | Agent picked wrong skill |
| Step 4: Query formulation | 4% | Poor query returned irrelevant docs |
| **Total (without instructions)** | **44%** | |
| **Total (with instructions)** | **21%** | Explicit guidance reduced but didn't eliminate failures |

### Why Agents Skip Skills

Vercel identified several reasons agents failed to invoke skills:

1. **Overconfidence in pre-training**: Agents assumed their training knowledge was sufficient
2. **Decision fatigue**: Multiple tools/skills created cognitive load
3. **Invisible triggers**: No clear signal when skill invocation was needed
4. **Ordering issues**: When multiple skills available, agents often selected incorrectly
5. **Implicit reasoning**: Agents didn't explicitly reason about when external docs were needed

---

## 4. The Solution: AGENTS.md Passive Context

### The Compressed Docs Index

The key innovation was compressing 40KB of documentation per library into an 8KB "docs index" that could sit permanently in the agent's context window.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              COMPRESSED DOCS INDEX STRUCTURE                                 │
│                                                                             │
│  Original documentation (40KB per library):                                 │
│  ───────────────────────────────────────────                                  │
│  • Full API reference with all methods                                      │
│  • Extensive code examples                                                  │
│  • Long-form conceptual explanations                                        │
│  • Installation and configuration guides                                    │
│  • Troubleshooting sections                                                 │
│  • Edge cases and advanced usage                                            │
│                                                                             │
│  Compression strategy (80% reduction to 8KB):                             │
│  ──────────────────────────────────────────────                               │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  DOCS INDEX: express-rate-limit                                    │    │
│  │  ────────────────────────────────────                               │    │
│  │                                                                     │    │
│  │  ## Quick Reference                                                 │    │
│  │  ```javascript                                                      │    │
│  │  const rateLimit = require('express-rate-limit');                   │    │
│  │  const limiter = rateLimit({                                        │    │
│  │    windowMs: 15 * 60 * 1000, // 15 minutes                         │    │
│  │    max: 100, // limit each IP to 100 requests per windowMs          │    │
│  │  });                                                                │    │
│  │  app.use('/api/', limiter);                                         │    │
│  │  ```                                                                │    │
│  │                                                                     │    │
│  │  ## Key Options                                                      │    │
│  │  • windowMs: Time window in milliseconds                            │    │
│  │  • max: Max requests per window per IP                              │    │
│  │  • message: Response when limit hit                                  │    │
│  │  • standardHeaders: Include RateLimit headers (draft-7)             │    │
│  │  • legacyHeaders: Include X-RateLimit headers (deprecated)        │    │
│  │                                                                     │    │
│  │  ## Common Patterns                                                   │    │
│  │  // Different limits for different routes                           │    │
│  │  app.use('/api/public', rateLimit({ max: 100 }));                   │    │
│  │  app.use('/api/admin', rateLimit({ max: 1000 }));                   │    │
│  │                                                                     │    │
│  │  // Store in Redis for distributed apps                              │    │
│  │  const RedisStore = require('rate-limit-redis');                     │    │
│  │                                                                     │    │
│  │  ## Gotchas                                                           │    │
│  │  • Trust proxy setting affects IP detection (use trustProxy)        │    │
│  │  • Default memory store resets on restart (use Redis/DB for prod)   │    │
│  │  • Skip successful requests with skipSuccessfulRequests             │    │
│  │                                                                     │    │
│  │  ## See Full Docs: https://www.npmjs.com/package/express-rate-limit │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Compression Techniques:                                                     │
│  ───────────────────────                                                     │
│  • Remove redundant examples (keep 1-2 canonical ones)                     │
│  • Summarize verbose explanations into bullet points                     │
│  • Focus on "How do I..." patterns, not "What is..." theory               │
│  • Prioritize error-prone areas ("Gotchas" section)                       │
│  • Include only most common 80% of API surface (Pareto principle)         │
│  • Full docs link for edge cases (retrieval still available)             │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Result: 80% compression (40KB → 8KB) while maintaining 100% pass rate      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### The Key Instruction

The AGENTS.md file included a single, critical instruction that guided agent behavior:

```
<agent_instructions>
When working with [library], prefer retrieval-led reasoning over pre-training-led reasoning.
Use the quick reference above before relying on your training knowledge.

Retrieval-led reasoning means:
1. Check the docs index for relevant patterns first
2. Use the quick reference code examples as starting points
3. Follow the "Gotchas" section to avoid common mistakes
4. Only use pre-training knowledge for edge cases not covered here

Pre-training-led reasoning means:
1. Relying on what you learned during training
2. Often leads to outdated or incorrect implementations
3. Should be the fallback, not the default
</agent_instructions>
```

### AGENTS.md Passive Context Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              AGENTS.md PASSIVE CONTEXT FLOW                                  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 1: Agent Receives Task                                       │    │
│  │  ─────────────────────────────────                                    │    │
│  │  User: "Add rate limiting to this Express.js endpoint"              │    │
│  │                                                                       │    │
│  │  Agent's context window (ALWAYS includes AGENTS.md):                 │    │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│  │  │ SYSTEM PROMPT:                                                  │ │    │
│  │  │ ...standard instructions...                                     │ │    │
│  │  │                                                                 │ │    │
│  │  │ AGENTS.md:                                                      │ │    │
│  │  │ ## DOCS INDEX: express-rate-limit                              │ │    │
│  │  │ (8KB compressed docs always visible)                             │ │    │
│  │  │ "prefer retrieval-led reasoning over pre-training-led reasoning"│ │    │
│  │  │                                                                 │ │    │
│  │  │ ## DOCS INDEX: three.js                                        │ │    │
│  │  │ ## DOCS INDEX: langchain                                       │ │    │
│  │  │ ... (all 8 libraries, 64KB total docs indices)                 │ │    │
│  │  └─────────────────────────────────────────────────────────────────┘ │    │
│  │                                                                       │    │
│  │  ✅ NO DECISION REQUIRED — docs are already in context              │    │
│  │  ✅ NO INVOCATION STEP — agent sees docs index immediately           │    │
│  │  ✅ NO SKILL SELECTION — all relevant docs visible at once           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 2: Agent Processes Task                                      │    │
│  │  ─────────────────────────────────                                    │    │
│  │                                                                       │    │
│  │  Agent internal reasoning (guided by AGENTS.md instruction):       │    │
│  │                                                                       │    │
│  │  "The user wants rate limiting for Express.js..."                    │    │
│  │  "I can see the express-rate-limit docs index in my context..."    │    │
│  │  "The instruction says 'prefer retrieval-led reasoning'..."         │    │
│  │  "Let me use the quick reference code example..."                    │    │
│  │                                                                       │    │
│  │  ✅ Agent follows explicit instruction to use provided docs          │    │
│  │  ✅ Agent has immediate access to canonical patterns                 │    │
│  │  ✅ Agent sees "Gotchas" section and avoids common mistakes          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  STEP 3: Implementation (Success Path)                             │    │
│  │  ─────────────────────────────────────                                │    │
│  │                                                                       │    │
│  │  Agent: "Based on the docs index, I'll implement rate limiting..."  │    │
│  │                                                                       │    │
│  │  ```javascript                                                       │    │
│  │  const rateLimit = require('express-rate-limit');                   │    │
│  │  const limiter = rateLimit({                                        │    │
│  │    windowMs: 15 * 60 * 1000,                                         │    │
│  │    max: 100,                                                         │    │
│  │    standardHeaders: true, // Following docs guidance                │    │
│  │  });                                                                 │    │
│  │  app.use('/api/', limiter);                                          │    │
│  │  ```                                                                 │    │
│  │                                                                       │    │
│  │  ✅ Correct implementation following current best practices         │    │
│  │  ✅ Avoids deprecated legacyHeaders (per "Gotchas" section)           │    │
│  │  ✅ 100% of tasks complete successfully                               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key Difference: Passive context eliminates ALL decision points that cause  │
│  skill invocation failures. The docs are just... there. Always.             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Why Passive Context Wins: The Elimination of Decision Points

### The Decision Problem

Every skill invocation requires the agent to make a decision. Decisions introduce failure modes.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              DECISION POINTS: ACTIVE vs PASSIVE CONTEXT                        │
│                                                                             │
│  SKILLS (Active - Multiple Decision Points):                                 │
│  ────────────────────────────────────────────                                 │
│                                                                             │
│  Decision 1: "Do I need external documentation?"                           │
│       ├─ YES (56%) → Continue to Decision 2                                 │
│       └─ NO (44%) → Use pre-training → ❌ LIKELY WRONG                      │
│                                                                             │
│  Decision 2: "Which skill should I invoke?" (if 8 libraries)               │
│       ├─ Correct skill (85% of 56% = 48%) → Continue to Decision 3       │
│       └─ Wrong skill (15% of 56% = 8%) → ❌ WRONG DOCS                    │
│                                                                             │
│  Decision 3: "How should I query the skill?"                             │
│       ├─ Good query (90% of 48% = 43%) → Get correct docs → ✅ SUCCESS    │
│       └─ Bad query (10% of 48% = 5%) → ❌ WRONG DOCS                      │
│                                                                             │
│  Cumulative success rate: ~56% (without explicit instructions)             │
│  Even with explicit instructions: ~79% (decisions still introduce errors)   │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                             │
│  AGENTS.md (Passive - ZERO Decision Points):                                 │
│  ───────────────────────────────────────────                                  │
│                                                                             │
│  NO Decision 1: "Do I need external documentation?"                         │
│       → Docs are ALWAYS in context                                          │
│       → Agent ALWAYS has access                                              │
│                                                                             │
│  NO Decision 2: "Which skill should I invoke?"                              │
│       → All relevant docs indices visible at once                           │
│       → No selection required                                              │
│                                                                             │
│  NO Decision 3: "How should I query the skill?"                           │
│       → Docs already in context, no query needed                            │
│       → Just read and use                                                  │
│                                                                             │
│  Cumulative success rate: 100% (no decisions = no decision failures)        │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│  Core Principle: "Good design eliminates the need for decision-making."    │
│  — Applied to agent context architecture                                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Additional Passive Context Advantages

| Factor | Skills (Active) | AGENTS.md (Passive) |
|:-------|:----------------|:--------------------|
| **Availability** | Conditional (must be invoked) | Always present |
| **Latency** | Additional round-trip for retrieval | Zero latency (in context) |
| **Consistency** | Varies by agent decision | Deterministic, reproducible |
| **Ordering issues** | Multiple skills = selection complexity | All docs visible simultaneously |
| **Pre-training override** | Agent may ignore skill | Explicit instruction guides behavior |
| **Implementation complexity** | Requires skill infrastructure | Static file in prompt |
| **Debugging** | Hard to trace why skill wasn't invoked | Easy to verify docs are present |

---

## 6. Context Bloat Solution: 80% Compression

### The Compression Challenge

The original documentation for 8 libraries totaled ~320KB — far exceeding typical context windows. Vercel achieved an 80% compression while maintaining perfect task performance.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CONTEXT BLOAT SOLUTION                                    │
│                                                                             │
│  Original Problem:                                                          │
│  ─────────────────                                                          │
│  8 libraries × 40KB docs = 320KB total                                      │
│  Typical context window: 128K-200K tokens                                     │
│  320KB > 200K → Cannot fit all docs in context                             │
│                                                                             │
│  Solution: Aggressive Compression                                           │
│  ──────────────────────────────                                               │
│                                                                             │
│  Compression Ratio: 5:1 (80% reduction)                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                                                                     │    │
│  │  40KB          40KB          40KB          40KB                    │    │
│  │  ┌────┐       ┌────┐       ┌────┐       ┌────┐                    │    │
│  │  │████│       │████│       │████│       │████│                    │    │
│  │  │████│  →    │████│  →    │████│  →    │████│                    │    │
│  │  │████│       │████│       │████│       │████│                    │    │
│  │  │████│       │████│       │████│       │████│                    │    │
│  │  │████│       │████│       │████│       │████│                    │    │
│  │  └────┘       └────┘       └────┘       └────┘                    │    │
│  │    │            │            │            │                          │    │
│  │    └────────────┴────────────┴────────────┘                        │    │
│  │                    │                                                  │    │
│  │                    ▼                                                  │    │
│  │                  ┌────┐                                                │    │
│  │                  │██  │  8KB compressed docs index                    │    │
│  │                  │██  │  (per library)                                │    │
│  │                  └────┘                                                │    │
│  │                                                                     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Total after compression:                                                   │
│  8 libraries × 8KB = 64KB total docs indices                               │
│  64KB < 200K → Easily fits in context window                                │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│  Critical Finding: Aggressive compression did NOT hurt performance         │
│  Pass rate: 100% (compressed) = 100% (if uncompressed would fit)             │
│  The 80% that was removed was the low-signal 80% (verbiage, redundancy)   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Compression Techniques Detail

| Technique | Application | Impact |
|:----------|:------------|:-------|
| **API surface reduction** | Include only most common 20% of methods | 50% size reduction |
| **Example consolidation** | Keep 1-2 canonical examples vs 10+ | 20% size reduction |
| **Verbiage elimination** | Replace paragraphs with bullet points | 15% size reduction |
| **Theory removal** | Keep "how to" not "what is" | 10% size reduction |
| **Edge case deferral** | Link to full docs for rare scenarios | 5% size reduction |
| **Total** | | **80% compression** |

---

## 7. Connection to Complexity Trap Research

### The Same Pattern: Simple Beats Sophisticated

Vercel's AGENTS.md findings directly echo the Complexity Trap research findings:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│           VERCEL RESEARCH vs COMPLEXITY TRAP RESEARCH                        │
│                                                                             │
│  COMPLEXITY TRAP (Context Management for Agents):                         │
│  ───────────────────────────────────────────────────                          │
│                                                                             │
│  Finding: Simple observation masking matches sophisticated LLM              │
│          summarization for context management                               │
│                                                                             │
│  Sophisticated approach: LLM Summarization                                  │
│  • Active compression using LLM to generate summaries                       │
│  • Additional API calls (5-7% overhead)                                    │
│  • Higher implementation complexity                                        │
│  • "Trajectory elongation" side effect (+15-18% turns)                     │
│                                                                             │
│  Simple approach: Observation Masking                                      │
│  • Passive masking of old observations                                       │
│  • No additional API calls                                                 │
│  • Minimal implementation complexity                                         │
│  • Lower cost (4/5 configurations)                                         │
│                                                                             │
│  Result: Simple matches sophisticated in solve rate, beats in cost         │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                             │
│  VERCEL RESEARCH (Library Documentation for Agents):                         │
│  ──────────────────────────────────────────────────                           │
│                                                                             │
│  Finding: Simple passive context (AGENTS.md) beats sophisticated            │
│          active retrieval (Skills) for coding agents                        │
│                                                                             │
│  Sophisticated approach: Skills (Tool-based Retrieval)                     │
│  • Active retrieval with RAG/vector search                                  │
│  • Additional decision points (invoke? which skill? what query?)          │
│  • Higher implementation complexity                                        │
│  • 44% invocation failure rate (56% pass rate)                             │
│                                                                             │
│  Simple approach: AGENTS.md (Passive Context)                                │
│  • Passive context with compressed docs index                              │
│  • No additional API calls (docs in prompt)                                  │
│  • Minimal implementation complexity (static file)                          │
│  • Zero invocation failures (100% pass rate)                               │
│                                                                             │
│  Result: Simple BEATS sophisticated by 44 percentage points                │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│  Common Pattern: Both studies show that adding decision points, API calls,   │
│  and implementation complexity often HURTS more than it helps.               │
│                                                                             │
│  The Complexity Trap applies broadly: sophisticated mechanisms introduce     │
│  failure modes that simple mechanisms avoid entirely.                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Parallel Findings

| Complexity Trap Finding | Vercel AGENTS.md Finding |
|:------------------------|:-------------------------|
| Simple masking ≈ sophisticated summarization | Passive context >> active retrieval |
| Additional mechanism = additional failure modes | Skill invocation = 44% failure rate |
| Cost of sophistication > benefit | 100% vs 56% = 78% relative improvement |
| Simple baseline should not be ignored | AGENTS.md should be default approach |
| Unmanaged complexity degrades performance | Skill selection degrades reliability |

---

## 8. Recommendations for Agent Builders

### Immediate Actions

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    RECOMMENDATIONS FOR AGENT BUILDERS                        │
│                                                                             │
│  1. COMPRESS AGGRESSIVELY                                                   │
│  ─────────────────────────                                                  │
│  • Start with 80% compression target (40KB → 8KB per library)              │
│  • Focus on practical "how to" patterns, not theory                        │
│  • Include "Gotchas" section for error-prone areas                         │
│  • Link to full docs for edge cases                                        │
│                                                                             │
│  2. DESIGN FOR RETRIEVAL (Not for Pre-Training)                             │
│  ─────────────────────────────────────────────                              │
│  • Include explicit instruction: "Prefer retrieval-led reasoning"        │
│  • Structure docs index for quick scanning (bullet points, code blocks)      │
│  • Lead with working code examples                                         │
│  • Minimize agent's need to "figure out" the docs                          │
│                                                                             │
│  3. TEST WITH EVALS                                                         │
│  ───────────────────                                                          │
│  • Measure pass rate on real tasks (extract from GitHub issues)            │
│  • Compare passive vs active approaches on YOUR use case                   │
│  • Test with compressed docs to find compression limits                    │
│  • Monitor for silent failures (wrong answers, not just crashes)           │
│                                                                             │
│  4. FAVOR PASSIVE OVER ACTIVE (Default Position)                            │
│  ───────────────────────────────────────────────                              │
│  • Start with AGENTS.md-style passive context                              │
│  • Add skills only when passive is genuinely insufficient                │
│  • When using skills, make them as "passive" as possible                 │
│    (automatic invocation, no agent decision required)                       │
│                                                                             │
│  5. ELIMINATE DECISION POINTS                                               │
│  ────────────────────────────                                                 │
│  • Every decision = potential failure point                                │
│  • Prefer "always available" over "conditionally available"                  │
│  • If decisions are required, make them deterministic (rules, not LLM)    │
│  • Test failure modes explicitly                                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### When to Use Skills

Skills still have valid use cases, but should be the exception, not the default:

| Use Case | Recommendation |
|:---------|:---------------|
| Documentation >100KB compressed | Consider skill with automatic invocation |
| Dynamic content (changing frequently) | Skill with deterministic trigger |
| Multiple specialized doc sets (50+ libraries) | Hybrid: popular in AGENTS.md, niche in skills |
| External API integration (not just docs) | Skill appropriate |
| Single framework, <100KB docs | **AGENTS.md strongly preferred** |

---

## 9. Summary

### Key Takeaways

| Finding | Implication |
|:--------|:------------|
| **AGENTS.md: 100% pass rate** | Passive context is dramatically more reliable |
| **Skills: 56% pass rate (no instructions)** | Active retrieval fails silently nearly half the time |
| **Skills: 79% pass rate (with explicit instructions)** | Even with guidance, active retrieval fails 21% of the time |
| **80% compression maintained 100% pass rate** | Aggressive compression is viable and recommended |
| **Key: "Prefer retrieval-led reasoning"** | Explicit instruction guides agents to use provided docs |
| **Eliminate decision points** | Every required decision is a potential failure point |

### The Paradigm Shift

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    THE PARADIGM SHIFT                                        │
│                                                                             │
│  OLD THINKING:                                                              │
│  ─────────────                                                              │
│  "Sophisticated RAG and tool-based retrieval will give agents the           │
│   context they need, when they need it."                                    │
│                                                                             │
│  • Vector search for semantic retrieval                                     │
│  • Tool-based skill systems                                                 │
│  • Agent-driven decision making                                             │
│  • "Just-in-time" context loading                                         │
│                                                                             │
│  Result: 56-79% reliability, complex infrastructure, debugging nightmares │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                             │
│  NEW THINKING (Based on Vercel Research):                                   │
│  ─────────────────────────────────────────                                  │
│  "Simple passive context, aggressively compressed, eliminates the          │
│   decision points that cause failures."                                    │
│                                                                             │
│  • Static docs index in system prompt                                       │
│  • 80% compression with no quality loss                                     │
│  • Zero decision points for agent                                          │
│  • "Always-in-context" documentation                                       │
│                                                                             │
│  Result: 100% reliability, minimal infrastructure, easy debugging         │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════    │
│  The insight: Better to have 80% of docs in context 100% of the time        │
│  than 100% of docs available 56% of the time.                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## References

1. Vercel AI Research, "AGENTS.md vs. Skills: A Comparative Study for Coding Agents," 2025 ([vercel.com/blog/agents-md-vs-skills](https://vercel.com/blog/agents-md-vs-skills))
2. Lindenbauer et al., "The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management," NeurIPS 2025 DL4C Workshop ([arXiv:2508.21433](https://arxiv.org/pdf/2508.21433))
3. Karpathy, "Context Engineering" ([karpathy.ai](https://karpathy.ai))
4. Google ADK Documentation, "Context as a Compiled View" ([google.github.io/adk-docs/context](https://google.github.io/adk-docs/context/))
5. Anthropic, "Effective Context Engineering for AI Agents" ([anthropic.com/engineering/effective-context-engineering-for-ai-agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents))

---

## Next Steps

- **[Observation Masking](../strategies/01-observation-masking.md)** - The simple baseline that matches sophisticated approaches
- **[Google ADK Context](./01-google-adk-context.md)** - Tiered context model from production framework
- **[Anthropic Context Engineering](./02-anthropic-context.md)** - Context engineering patterns for AI agents
- **[Hybrid Approach](../strategies/03-hybrid-approach.md)** - Combining passive and active approaches when needed
- **[Complexity Trap Research](../architecture/01-research-summary.md)** - The original finding that simple beats sophisticated

---

*Based on Vercel AI Research, 2025*
