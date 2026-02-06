# Google ADK: Tiered Context Architecture for Production Agents

## Overview

Google's Agent Development Kit (ADK) represents one of the first production-grade frameworks to treat context as a **first-class architectural concern** rather than an implementation detail. ADK's context system embodies the shift from "prompt engineering" to "context engineering" — applying systems engineering principles to how information flows through agent systems.

**Framework**: Google Agent Development Kit (ADK)
**Repository**: [github.com/google/adk-python](https://github.com/google/adk-python)
**Documentation**: [google.github.io/adk-docs/context](https://google.github.io/adk-docs/context/)
**Blog Post**: [Architecting Efficient Context-Aware Multi-Agent Framework for Production](https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/)

**Index Terms**: Google ADK, Context Engineering, Tiered Context, Compiled View, Session Management, Memory Service, Artifacts, Context Compaction, Prefix Caching, Multi-Agent Context, Production Patterns

---

## 1. The Context Engineering Philosophy

### From Prompt Engineering to Systems Engineering

Previous-generation agent frameworks treated context as a mutable string buffer — a flat sequence of messages concatenated and shipped to the model. ADK rejects this model entirely, proposing that context management is a **systems engineering problem** that demands the same rigor as storage, compute, and networking.

### The Design Thesis: Context as a Compiled View

ADK is built around a central thesis:

> **Context is a compiled view over a richer stateful system.**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                  CONTEXT AS A COMPILED VIEW                                  │
│                                                                             │
│  SOURCES (Full State)          COMPILER PIPELINE         COMPILED VIEW      │
│  ─────────────────────         ─────────────────         ─────────────      │
│                                                                             │
│  ┌─────────────────┐           ┌─────────────┐           ┌─────────────┐   │
│  │   Sessions      │──────────▶│  Flows &    │──────────▶│  Working    │   │
│  │   (event log)   │           │  Processors │           │  Context    │   │
│  └─────────────────┘           │             │           │             │   │
│  ┌─────────────────┐           │  1. basic   │           │  • Instruct │   │
│  │   Memory        │──────────▶│  2. auth    │           │  • Identity │   │
│  │   (long-term)   │           │  3. confirm │           │  • History  │   │
│  └─────────────────┘           │  4. instruct│           │  • Tools    │   │
│  ┌─────────────────┐           │  5. identity│           │  • Memory   │   │
│  │   Artifacts     │──────────▶│  6. contents│           │  • Artifact │   │
│  │   (binary data) │           │  7. cache   │           │    refs     │   │
│  └─────────────────┘           │  8. plan    │           └─────────────┘   │
│                                │  9. code    │                              │
│                                │ 10. schema  │           Ephemeral:        │
│                                └─────────────┘           Rebuilt per call  │
│                                                          Thrown away after  │
│                                                          Model-agnostic    │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key Insight: You stop hard-coding "the prompt" and start treating it       │
│  as a derived representation you can iterate on.                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Three Design Principles

| Principle | Description | Implication |
|-----------|-------------|-------------|
| **Separate storage from presentation** | Distinguish durable state (Sessions) from per-call views (Working Context) | Evolve storage schemas and prompt formats independently |
| **Explicit transformations** | Context built through named, ordered processors | Compilation step is observable, testable, and extensible |
| **Scope by default** | Every model call sees the minimum context required | Agents must explicitly reach for more information via tools |

### The Compiler Analogy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             TRADITIONAL vs. ADK CONTEXT CONSTRUCTION                         │
│                                                                             │
│  Traditional (String Buffer):                                                │
│  ──────────────────────────────                                              │
│  context = system_prompt + "\n"                                              │
│  context += format_history(messages)                                         │
│  context += "\n" + tool_results                                              │
│  context += "\n" + memory_dump                                               │
│  # → Opaque, fragile, model-specific                                        │
│                                                                             │
│  ADK (Compiler Pipeline):                                                    │
│  ──────────────────────────                                                  │
│  Sources ──▶ IR (Events) ──▶ Processor 1 ──▶ ... ──▶ Processor N ──▶ View  │
│                                                                             │
│  Systems Questions ADK Forces You to Ask:                                   │
│  • What is the intermediate representation?        → Typed Event objects    │
│  • Where do we apply compaction?                   → Session layer          │
│  • How do we make transformations observable?      → Named processors       │
│  • Where do we insert caching?                     → Cache processor        │
│  • How do we scope for multi-agent?                → Per-agent filtering    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. The Tiered Context Model

### Architecture

ADK organizes context into four distinct tiers, each with a specific storage lifetime, access pattern, and role in the system.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ADK TIERED CONTEXT MODEL                                  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  TIER 1: WORKING CONTEXT (Ephemeral)                                 │    │
│  │  ───────────────────────────────────                                  │    │
│  │  • The compiled prompt for THIS model call                           │    │
│  │  • System instructions + agent identity                              │    │
│  │  • Selected history (from Session events)                            │    │
│  │  • Tool outputs from current turn                                     │    │
│  │  • Optional memory results and artifact references                   │    │
│  │                                                                       │    │
│  │  Lifetime: Single model invocation                                     │    │
│  │  Rebuilt from scratch each call                                        │    │
│  │  Thrown away after completion                                          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       ▲                                                                      │
│       │ Compiled from                                                        │
│       │                                                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  TIER 2: SESSION (Durable, Per-Conversation)                          │    │
│  │  ───────────────────────────────────────────                           │    │
│  │  • Chronological event log of the interaction                        │    │
│  │  • Every user message, agent reply, tool call, tool result           │    │
│  │  • Control signals and errors as typed Event objects                  │    │
│  │  • Session state (key-value scratchpad)                               │    │
│  │                                                                       │    │
│  │  Lifetime: Single conversation thread                                  │    │
│  │  Persisted by SessionService                                           │    │
│  │  State prefixes: app: | user: | temp:                                 │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       ▲                                                                      │
│       │ Ingested from                                                        │
│       │                                                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  TIER 3: MEMORY (Persistent, Cross-Session)                           │    │
│  │  ──────────────────────────────────────────                            │    │
│  │  • Long-lived semantic knowledge                                      │    │
│  │  • User preferences, past decisions, domain facts                    │    │
│  │  • Ingested from completed Sessions into vector/keyword corpus       │    │
│  │  • Searchable (not permanently pinned to context)                    │    │
│  │                                                                       │    │
│  │  Lifetime: Outlives individual sessions                               │    │
│  │  Managed by MemoryService                                              │    │
│  │  Access: Agent-directed (reactive or proactive recall)                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       ▲                                                                      │
│       │ Referenced by                                                        │
│       │                                                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  TIER 4: ARTIFACTS (External, Versioned)                              │    │
│  │  ───────────────────────────────────────                               │    │
│  │  • Named, versioned binary or text objects                            │    │
│  │  • Files, logs, images, CSVs, PDFs                                    │    │
│  │  • Addressed by name and version, NOT pasted into prompt             │    │
│  │  • Handle pattern: agent sees reference, loads on demand             │    │
│  │                                                                       │    │
│  │  Lifetime: Per-session or per-user (namespace prefix)                 │    │
│  │  Managed by ArtifactService                                            │    │
│  │  Access: Ephemeral expansion (load → use → offload)                   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Tier Comparison

| Property | Working Context | Session | Memory | Artifacts |
|----------|:--------------:|:-------:|:------:|:---------:|
| **Lifetime** | Single call | Conversation | Cross-session | Per-session/user |
| **Mutability** | Rebuilt each call | Append-only events | Indexed corpus | Versioned objects |
| **In prompt?** | Yes (the prompt) | Partially (selected events) | On demand | On demand |
| **Size** | Bounded by model window | Unbounded (compactable) | Unbounded (searchable) | Unbounded (external) |
| **Access pattern** | Automatic | Automatic + state API | Agent-directed search | Agent-directed load |
| **Managed by** | LLM Flow processors | SessionService | MemoryService | ArtifactService |

---

## 3. Flows and Processors: The Compilation Pipeline

### Pipeline Architecture

ADK compiles context through an ordered sequence of processors, each building on the outputs of previous steps.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                  ADK SINGLE-FLOW PROCESSOR PIPELINE                         │
│                                                                             │
│  Request Processors (before model call):                                    │
│  ───────────────────────────────────────                                     │
│                                                                             │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐                │
│  │ basic    │──▶│ auth     │──▶│ confirm  │──▶│ instruct │                │
│  │          │   │ preproc  │   │ request  │   │          │                │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘                │
│       │                                                                     │
│       ▼                                                                     │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐                │
│  │ identity │──▶│ contents │──▶│ cache    │──▶│ planning │                │
│  │          │   │          │   │ processor│   │          │                │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘                │
│       │                                                                     │
│       ▼                                                                     │
│  ┌──────────┐   ┌──────────┐                                               │
│  │ code     │──▶│ output   │──▶ [MODEL CALL]                               │
│  │ execution│   │ schema   │                                               │
│  └──────────┘   └──────────┘                                               │
│                                                                             │
│  Response Processors (after model call):                                    │
│  ───────────────────────────────────────                                     │
│                                                                             │
│  [MODEL RESPONSE] ──▶ ┌──────────┐ ──▶ ┌──────────┐                       │
│                        │ planning │     │ code     │                       │
│                        │          │     │ execution│                       │
│                        └──────────┘     └──────────┘                       │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key: Order matters. Each processor builds on previous outputs.            │
│  Natural insertion points for custom filtering, compaction, caching.       │
│  You are no longer rewriting "prompt templates" — you're reordering        │
│  processors.                                                                │
└─────────────────────────────────────────────────────────────────────────────┘
```

### The Contents Processor: Session → History

The `contents` processor performs the critical translation from the Session's event stream into the history portion of the Working Context. It executes three steps:

```python
class ContentsProcessor:
    """
    ADK contents processor: transforms Session events into model history.
    
    Three-step process:
    1. Filter events relevant to the current agent
    2. Format events into model-compatible message sequence
    3. Apply any active compaction summaries
    """
    
    def process(self, session: Session, agent: Agent) -> list:
        """Build history from session events."""
        
        # Step 1: Filter events by agent scope
        relevant_events = [
            event for event in session.events
            if self._is_relevant_to_agent(event, agent)
        ]
        
        # Step 2: Check for compaction summaries
        compacted_events = []
        for event in relevant_events:
            if event.actions and event.actions.compaction_summary:
                compacted_events.append(
                    self._format_compaction_event(event)
                )
            elif not self._is_superseded_by_compaction(event, relevant_events):
                compacted_events.append(
                    self._format_event(event)
                )
        
        # Step 3: Format into model-compatible message sequence
        return self._build_message_sequence(compacted_events)
    
    def _is_relevant_to_agent(self, event, agent) -> bool:
        """Filter events by agent hierarchy and scope."""
        return (
            event.author == agent.name
            or event.author == "user"
            or event.author in agent.get_sub_agent_names()
        )
    
    def _is_superseded_by_compaction(self, event, all_events) -> bool:
        """Check if this event has been replaced by a compaction summary."""
        for other in all_events:
            if (other.actions
                and other.actions.compaction_summary
                and event.timestamp < other.actions.compaction_boundary):
                return True
        return False
```

---

## 4. Context Compaction and Filtering

### The Sliding Window Compaction Strategy

ADK's compaction operates at the **Session layer**, summarizing older events before they reach the Working Context. This creates a scalable lifecycle for long-running conversations.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│               ADK CONTEXT COMPACTION (Sliding Window)                       │
│                                                                             │
│  Configuration: compaction_interval=3, overlap_size=1                       │
│                                                                             │
│  Event Timeline:                                                             │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌────┐│
│  │ E1  │ │ E2  │ │ E3  │ │ E4  │ │ E5  │ │ E6  │ │ E7  │ │ E8  │ │ E9 ││
│  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └────┘│
│                                                                             │
│  After Event 3 completes:                                                    │
│  ┌─────────────────────┐ ┌─────┐ ┌─────┐ ┌─────┐                          │
│  │ Summary(E1,E2,E3)   │ │ E4  │ │ E5  │ │ E6  │ ...                      │
│  └─────────────────────┘ └─────┘ └─────┘ └─────┘                          │
│                                                                             │
│  After Event 6 completes (overlap=1 includes E3):                           │
│  ┌─────────────────────┐ ┌─────────────────────────┐ ┌─────┐ ┌─────┐      │
│  │ Summary(E1,E2,E3)   │ │ Summary(E3,E4,E5,E6)    │ │ E7  │ │ E8  │ ... │
│  └─────────────────────┘ └─────────────────────────┘ └─────┘ └─────┘      │
│                                                                             │
│  After Event 9 completes (overlap=1 includes E6):                           │
│  ┌─────────────────────┐ ┌─────────────────────────┐ ┌──────────────────┐  │
│  │ Summary(E1..E3)     │ │ Summary(E3..E6)          │ │ Summary(E6..E9)  │  │
│  └─────────────────────┘ └─────────────────────────┘ └──────────────────┘  │
│                                                                             │
│  Benefits:                                                                   │
│  • Operates on Event stream (not prompt) → changes cascade downstream      │
│  • Contents processor works over already-compacted history                  │
│  • Compaction strategy is decoupled from agent code                         │
│  • Overlap ensures continuity across compaction boundaries                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Compaction Configuration

```python
from google.adk.apps.app import App, EventsCompactionConfig
from google.adk.apps.llm_event_summarizer import LlmEventSummarizer
from google.adk.models import Gemini


class ADKCompactionManager:
    """
    ADK context compaction configuration and lifecycle.
    
    Compaction operates at the Session layer using a sliding window
    approach with configurable interval and overlap.
    """
    
    def __init__(
        self,
        compaction_interval: int = 3,
        overlap_size: int = 1,
        summarizer_model: str = "gemini-2.5-flash"
    ):
        self.compaction_interval = compaction_interval
        self.overlap_size = overlap_size
        self.summarizer_model = summarizer_model
    
    def configure_app(self, root_agent) -> App:
        """Configure an ADK App with context compaction."""
        
        summarization_llm = Gemini(model=self.summarizer_model)
        summarizer = LlmEventSummarizer(llm=summarization_llm)
        
        return App(
            name='compacted-agent',
            root_agent=root_agent,
            events_compaction_config=EventsCompactionConfig(
                compaction_interval=self.compaction_interval,
                overlap_size=self.overlap_size,
                summarizer=summarizer,
            ),
        )
    
    def estimate_compaction_schedule(self, total_events: int) -> list:
        """Estimate when compaction will trigger."""
        triggers = []
        for i in range(self.compaction_interval, total_events + 1,
                       self.compaction_interval):
            window_start = max(
                1, i - self.compaction_interval - self.overlap_size + 1
            )
            window_end = i
            triggers.append({
                'trigger_at_event': i,
                'window': (window_start, window_end),
                'overlap_from': max(1, window_start)
            })
        return triggers


# Example compaction schedule for interval=3, overlap=1, 12 events:
# Event 3:  Summarize E1..E3
# Event 6:  Summarize E3..E6  (E3 overlaps)
# Event 9:  Summarize E6..E9  (E6 overlaps)
# Event 12: Summarize E9..E12 (E9 overlaps)
```

### Deterministic Filtering

For rule-based reduction, ADK offers **Filtering** as a sibling to compaction. Prebuilt plugins can globally drop or trim context based on deterministic rules before it reaches the model.

| Approach | Mechanism | When to Use |
|----------|-----------|-------------|
| **Compaction** | LLM-based summarization of older events | Semantic compression of long conversations |
| **Filtering** | Rule-based dropping/trimming of events | Deterministic removal of noise (debug logs, verbose tool output) |

---

## 5. Context Caching: Prefix Reuse

### Cache-Friendly Context Architecture

ADK's separation of storage (Session) and presentation (Working Context) provides a natural substrate for **prefix caching**, where the inference engine reuses attention computation across calls.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              ADK CONTEXT CACHING (Prefix Reuse)                              │
│                                                                             │
│  Context Window Layout (Optimized for Caching):                             │
│                                                                             │
│  ┌─────────────────────────────────────────┬────────────────────────────┐   │
│  │          STABLE PREFIX (Cached)          │   VARIABLE SUFFIX          │   │
│  │                                         │                            │   │
│  │  • Static instructions (immutable)      │  • Latest user turn        │   │
│  │  • Agent identity                       │  • New tool outputs        │   │
│  │  • Long-lived compaction summaries      │  • Incremental updates     │   │
│  │  • Preloaded memory snippets            │  • Current reasoning       │   │
│  │                                         │                            │   │
│  │  KV-cache reused across calls           │  Recomputed each call      │   │
│  │  Attention computation saved            │  Small incremental cost    │   │
│  └─────────────────────────────────────────┴────────────────────────────┘   │
│                                                                             │
│  static_instruction Primitive:                                              │
│  ─────────────────────────────                                              │
│  • Guarantees immutability for system prompts                              │
│  • Ensures cache prefix remains valid across invocations                    │
│  • Prevents accidental invalidation of cached computation                  │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Pipeline ordering is a HARD DESIGN CONSTRAINT:                            │
│  Frequently reused segments → front (stable)                               │
│  Highly dynamic content → end (variable)                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Caching Configuration

```python
from google.adk import Agent
from google.adk.apps.app import App
from google.adk.agents.context_cache_config import ContextCacheConfig


class ADKCacheManager:
    """
    ADK context caching configuration.
    
    Leverages Gemini's prefix caching to reuse attention computation
    across calls. Divides context into stable prefix and variable suffix.
    """
    
    def __init__(
        self,
        min_tokens: int = 2048,
        ttl_seconds: int = 600,
        cache_intervals: int = 5
    ):
        self.min_tokens = min_tokens
        self.ttl_seconds = ttl_seconds
        self.cache_intervals = cache_intervals
    
    def configure_app(self, root_agent) -> App:
        """Configure an ADK App with context caching."""
        
        return App(
            name='cached-agent',
            root_agent=root_agent,
            context_cache_config=ContextCacheConfig(
                min_tokens=self.min_tokens,
                ttl_seconds=self.ttl_seconds,
                cache_intervals=self.cache_intervals,
            ),
        )
    
    def estimate_cache_savings(
        self,
        prefix_tokens: int,
        suffix_tokens: int,
        num_calls: int,
        cache_hit_rate: float = 0.85
    ) -> dict:
        """Estimate token savings from prefix caching."""
        
        total_without_cache = (prefix_tokens + suffix_tokens) * num_calls
        
        cached_calls = int(num_calls * cache_hit_rate)
        uncached_calls = num_calls - cached_calls
        
        total_with_cache = (
            (prefix_tokens + suffix_tokens) * uncached_calls
            + suffix_tokens * cached_calls
        )
        
        return {
            'total_tokens_without_cache': total_without_cache,
            'total_tokens_with_cache': total_with_cache,
            'tokens_saved': total_without_cache - total_with_cache,
            'reduction_pct': round(
                (1 - total_with_cache / total_without_cache) * 100, 1
            ),
            'cache_hit_rate': cache_hit_rate
        }


# Example: 4096-token prefix, 512-token suffix, 50 calls
# Without cache: (4096 + 512) * 50 = 230,400 tokens
# With cache (85% hit): 6,912 * 8 + 512 * 42 = 76,800 tokens
# Savings: ~67% token reduction
```

---

## 6. Relevance: Agentic Management of Context

### The Human-Agent Collaboration Model

ADK answers the question "What belongs in the model's window right now?" through a collaboration between human domain knowledge and agentic decision-making.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│           RELEVANCE: HUMAN + AGENT COLLABORATION                            │
│                                                                             │
│  ┌──────────────────────────────┐  ┌──────────────────────────────┐        │
│  │  HUMAN ENGINEER DEFINES:     │  │  AGENT PROVIDES:              │        │
│  │  ─────────────────────────   │  │  ─────────────────────────    │        │
│  │  • Where data lives          │  │  • Dynamic retrieval          │        │
│  │  • How it is summarized      │  │  • When to "reach" for memory │        │
│  │  • What filters apply        │  │  • Which artifacts to load    │        │
│  │  • Pipeline ordering         │  │  • What knowledge gaps exist  │        │
│  │  • Compaction strategies     │  │  • Real-time relevance judg.  │        │
│  │                              │  │                                │        │
│  │  Cost-effective but rigid    │  │  Flexible but expensive       │        │
│  └──────────────────────────────┘  └──────────────────────────────┘        │
│                              │        │                                     │
│                              ▼        ▼                                     │
│                     ┌───────────────────────┐                               │
│                     │  OPTIMAL WORKING      │                               │
│                     │  CONTEXT              │                               │
│                     │  (Negotiated balance) │                               │
│                     └───────────────────────┘                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Artifacts: The Handle Pattern

ADK treats large data as externalized objects, applying a **handle pattern** to prevent the "context dumping" anti-pattern.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              ARTIFACT HANDLE PATTERN                                         │
│                                                                             │
│  Anti-Pattern (Context Dumping):                                            │
│  ───────────────────────────────                                            │
│  Turn 1: User uploads 5MB CSV                                               │
│  Turn 2: [5MB CSV in context] + "What's the average?"                       │
│  Turn 3: [5MB CSV in context] + "Now filter by date"                        │
│  Turn 4: [5MB CSV in context] + "Show top 10"                               │
│  Cost: 5MB × 4 = 20MB of context tokens                                    │
│                                                                             │
│  ADK Handle Pattern (Ephemeral Expansion):                                  │
│  ──────────────────────────────────────────                                 │
│  Turn 1: User uploads CSV → stored as artifact "sales_data.csv"            │
│  Turn 2: Agent sees reference: {name: "sales_data.csv", summary: "..."}    │
│           Agent calls LoadArtifactsTool → CSV loaded into Working Context   │
│           Model processes → response generated                              │
│           CSV offloaded from Working Context                                │
│  Turn 3: Agent sees reference only → loads if needed                        │
│  Turn 4: Agent sees reference only → loads if needed                        │
│  Cost: Reference × 4 + Full load × N (on demand)                           │
│                                                                             │
│  Result: "5MB of noise in every prompt" → precise, on-demand resource       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Memory: Reactive and Proactive Recall

ADK's MemoryService supports two distinct retrieval patterns, replacing the "context stuffing" anti-pattern with agent-directed recall.

```python
class ADKMemoryPatterns:
    """
    ADK Memory access patterns: reactive and proactive recall.
    
    Memory is searchable (not permanently pinned) and retrieval
    is agent-directed.
    """
    
    def __init__(self, memory_service):
        self.memory_service = memory_service
    
    def reactive_recall(self, agent_query: str) -> list:
        """
        Reactive: Agent recognizes a knowledge gap and explicitly
        calls load_memory_tool to search the corpus.
        
        Example: "What is the user's dietary restriction?"
        → Agent calls load_memory_tool(query="dietary restriction")
        → MemoryService returns relevant past interactions
        """
        return self.memory_service.search(query=agent_query, top_k=5)
    
    def proactive_recall(self, user_input: str) -> list:
        """
        Proactive: System pre-processor runs similarity search based
        on latest user input, injecting relevant snippets BEFORE
        the model is invoked via preload_memory_tool.
        
        Example: User says "I want to order dinner"
        → Pre-processor searches: "dinner preferences"
        → Injects: "User prefers vegetarian, allergic to nuts"
        → Model sees this context without explicit tool call
        """
        return self.memory_service.search(query=user_input, top_k=3)
    
    def ingest_session(self, session) -> None:
        """
        Ingest completed session into long-term memory corpus.
        
        Finished sessions are processed into vector/keyword index
        for future recall.
        """
        self.memory_service.ingest(session)
```

---

## 7. Multi-Agent Context Sharing

### Two Architectural Patterns

ADK maps multi-agent interactions into two distinct patterns, each with different context-sharing semantics.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│            ADK MULTI-AGENT CONTEXT PATTERNS                                  │
│                                                                             │
│  PATTERN 1: AGENTS AS TOOLS                                                 │
│  ─────────────────────────────                                              │
│                                                                             │
│  ┌────────────────┐         ┌────────────────┐                              │
│  │  Root Agent     │ ──────▶│  Specialist    │                              │
│  │                │ call()  │  Agent         │                              │
│  │  Full session  │         │                │                              │
│  │  history       │ ◀──────│  Sees ONLY:    │                              │
│  │                │ result  │  • Focused     │                              │
│  └────────────────┘         │    prompt      │                              │
│                             │  • Necessary   │                              │
│                             │    artifacts   │                              │
│                             │  • NO history  │                              │
│                             └────────────────┘                              │
│                                                                             │
│  Context Isolation: Complete                                                 │
│  Use When: Specialized subtasks, token-heavy processing                     │
│  Benefit: Parent context stays clean; specialist processes in isolation     │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│                                                                             │
│  PATTERN 2: AGENT TRANSFER (Hierarchy)                                      │
│  ────────────────────────────────────────                                    │
│                                                                             │
│  ┌────────────────┐  transfer  ┌────────────────┐                           │
│  │  Root Agent     │ ─────────▶│  Sub-Agent     │                           │
│  │                │            │                │                           │
│  │  Releases      │            │  Inherits:     │                           │
│  │  control       │            │  • Session view│                           │
│  │                │            │  • State access│                           │
│  └────────────────┘            │  • Can call    │                           │
│                                │    own tools   │                           │
│                                │  • Can transfer│                           │
│                                │    further     │                           │
│                                └────────────────┘                           │
│                                                                             │
│  Context Sharing: Inherited (scoped view over Session)                      │
│  Use When: Full conversation handoff, workflow continuation                 │
│  Benefit: Sub-agent can drive the workflow with full conversational state   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### State Sharing and Scoping

```python
class ADKMultiAgentContext:
    """
    ADK multi-agent context sharing patterns.
    
    Demonstrates state scoping, agent isolation, and
    context flow between parent and child agents.
    """
    
    # State prefix conventions
    STATE_PREFIXES = {
        'app:': 'Shared across all sessions for the application',
        'user:': 'Shared across sessions for a specific user',
        'temp:': 'Temporary, only for current invocation',
        '': 'Default: scoped to current session'
    }
    
    @staticmethod
    def sequential_pipeline_context():
        """
        Sequential agents share the SAME InvocationContext.
        State written by Step1 is readable by Step2.
        """
        # Step1 writes: context.state['data'] = processed_data
        # Step2 reads:  data = context.state.get('data')
        # output_key shortcut: agent saves final response to state key
        pass
    
    @staticmethod
    def parallel_agent_context():
        """
        Parallel agents get distinct branches but share state.
        
        context.branch is modified per child: "ParentBranch.ChildName"
        All children access the SAME session.state
        Use distinct keys to avoid race conditions.
        """
        # Agent A writes: context.state['result_a'] = ...
        # Agent B writes: context.state['result_b'] = ...
        # Parent reads both after parallel completion
        pass
    
    @staticmethod
    def agent_tool_context():
        """
        AgentTool: parent invokes child as a function.
        
        Child runs in isolation, parent gets:
        - Final response text as tool result
        - State changes forwarded back to parent context
        - Artifact changes forwarded back
        """
        # Parent LLM generates function call → AgentTool
        # AgentTool.run_async() executes child agent
        # Child's response → tool result for parent
        # Child's state/artifact changes → merged into parent context
        pass
```

---

## 8. Connection to Complexity Trap

### Validating the Core Finding

ADK's production architecture provides strong validation for the Complexity Trap research findings, while also revealing where additional sophistication is warranted.

| Complexity Trap Finding | ADK Implementation | Validation |
|-------------------------|-------------------|------------|
| **Simple masking is effective** | Compaction uses sliding window with overlap — structurally similar to observation masking with bounded history | ADK's default compaction is conservative, not aggressive summarization |
| **Trajectory elongation from summarization** | Compaction operates at Session layer, not in-prompt — summaries don't directly mask failure signals in recent turns | Architectural separation mitigates elongation risk |
| **Hybrid approach wins** | Compaction (LLM summary) + Filtering (deterministic rules) — dual strategy mirrors hybrid approach | ADK independently arrived at the same hybrid insight |
| **Context management is essential** | Context is a "first-class system with its own architecture, lifecycle, and constraints" | Production deployment confirms necessity |
| **Cost reduction through efficiency** | Prefix caching, ephemeral artifact expansion, agent-directed memory — multiple cost reduction vectors | Goes beyond research to production-scale optimization |

### Where ADK Extends the Research

```
┌─────────────────────────────────────────────────────────────────────────────┐
│        COMPLEXITY TRAP RESEARCH vs. ADK PRODUCTION PATTERNS                  │
│                                                                             │
│  Research Scope:                     ADK Extensions:                        │
│  ───────────────                     ──────────────                          │
│  • Observation masking               • Tiered storage (4 layers)            │
│  • LLM summarization                 • Compilation pipeline (processors)    │
│  • Hybrid combination                • Prefix caching (hardware-aware)      │
│  • Cost measurement                  • Artifact handle pattern              │
│                                      • Multi-agent context scoping          │
│  Single-agent focus                  • Agent-directed memory recall         │
│  SWE-bench evaluation                • Production deployment patterns       │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key Alignment:                                                              │
│  Both reject the "just expand the context window" approach.                 │
│  Both demonstrate that structured management beats raw accumulation.        │
│  Both show that simple baselines should not be ignored.                     │
│                                                                             │
│  Key Divergence:                                                             │
│  Research focuses on WHICH strategy; ADK focuses on HOW to architect.       │
│  Research measures cost per instance; ADK optimizes cost per system.        │
│  Research evaluates single agents; ADK addresses multi-agent teams.         │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Implications for Agent Builders

| Decision | Research Guidance | ADK Pattern | Combined Recommendation |
|----------|------------------|-------------|------------------------|
| **Default strategy** | Observation masking | Scoped Working Context + filtering | Start with deterministic filtering, add compaction as needed |
| **When to summarize** | Only as last resort (hybrid) | Configurable compaction interval | Set high compaction intervals to defer summarization |
| **Large data handling** | Not addressed | Artifact handle pattern | Externalize large data; load on demand |
| **Cross-session knowledge** | Not addressed | MemoryService with reactive/proactive recall | Separate long-term memory from per-turn context |
| **Multi-agent systems** | Not addressed | Agents-as-Tools vs. Agent Transfer | Isolate specialists; share state through scoped keys |
| **Cost optimization** | Token reduction via masking | Prefix caching + ephemeral expansion | Combine structural caching with content reduction |

---

## 9. Production Deployment Patterns

### Hot/Cold Context Separation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              HOT/COLD CONTEXT PATTERN                                        │
│                                                                             │
│  HOT CONTEXT (Session State):                                               │
│  ─────────────────────────────                                              │
│  • Frequently accessed                                                       │
│  • Current conversation state                                                │
│  • Workflow progress                                                         │
│  • Cached intermediate results                                               │
│  • Fast access (in-memory)                                                   │
│  • Always in Working Context                                                 │
│                                                                             │
│  COLD CONTEXT (Memory + Artifacts):                                         │
│  ──────────────────────────────────                                         │
│  • Rarely accessed                                                           │
│  • Historical knowledge                                                      │
│  • Large data objects                                                        │
│  • User preferences                                                          │
│  • Loaded on demand (agent-directed)                                        │
│  • Only in Working Context when needed                                      │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Optimization: Agents pay the cost of loading cold context only when        │
│  needed. Hot context remains immediately accessible without external calls. │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Context-First Agent Design

ADK's production experience reveals a design methodology:

| Step | Action | Purpose |
|------|--------|---------|
| 1 | Map information flows | Identify what each agent needs and where it comes from |
| 2 | Define state boundaries | Separate hot (session) from cold (memory/artifacts) |
| 3 | Configure compaction | Set interval and overlap based on expected conversation length |
| 4 | Enable caching | Identify stable prefixes, ensure static instructions are immutable |
| 5 | Scope multi-agent | Choose isolation (tools) vs. sharing (transfer) per sub-agent |
| 6 | Monitor and tune | Track context size, cache hit rate, compaction frequency |

### Multi-Tier Compaction Strategy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│           MULTI-TIER COMPACTION (Production Best Practice)                   │
│                                                                             │
│  Tier 1: Conservative (Initial)                                             │
│  ──────────────────────────────                                             │
│  • High compaction_interval (e.g., 10)                                      │
│  • Large overlap_size (e.g., 3)                                             │
│  • Preserves most detail                                                    │
│  • Minimal information loss                                                  │
│                                                                             │
│  Tier 2: Moderate (As context grows)                                        │
│  ──────────────────────────────────                                         │
│  • Medium compaction_interval (e.g., 5)                                     │
│  • Medium overlap_size (e.g., 2)                                            │
│  • Balanced compression                                                      │
│  • Important information tagged and protected                               │
│                                                                             │
│  Tier 3: Aggressive (Last resort)                                           │
│  ────────────────────────────────                                           │
│  • Low compaction_interval (e.g., 3)                                        │
│  • Small overlap_size (e.g., 1)                                             │
│  • Maximum compression                                                       │
│  • Only critical information preserved                                      │
│  • Failed operations trigger context reconstruction from memory             │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  This mirrors the Complexity Trap hybrid approach: defer aggressive          │
│  summarization as long as possible, use it only when needed.                │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 10. Comparison with Research Strategies

| Dimension | Observation Masking | LLM Summarization | ADK Tiered Model |
|-----------|:-------------------:|:-----------------:|:----------------:|
| **Scope** | Single agent | Single agent | Multi-agent systems |
| **Storage** | In-prompt manipulation | In-prompt replacement | Tiered (Session/Memory/Artifacts) |
| **Compilation** | Direct masking | LLM-generated summary | Pipeline of named processors |
| **Large data** | Masked observations | Summarized observations | Externalized artifacts |
| **Cross-session** | Not supported | Not supported | MemoryService |
| **Caching** | Not addressed | Not addressed | Prefix caching with static instructions |
| **Multi-agent** | Not addressed | Not addressed | Two patterns (tools vs. transfer) |
| **Elongation risk** | Low | High (+15-18%) | Mitigated (Session-layer compaction) |
| **Complexity** | Minimal | Moderate | High (framework overhead) |
| **Deployment readiness** | Research prototype | Research prototype | Production framework |

---

## References

1. Google Developers Blog, "Architecting Efficient Context-Aware Multi-Agent Framework for Production," 2025 ([developers.googleblog.com](https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/))
2. Google ADK Documentation, "Context" ([google.github.io/adk-docs/context](https://google.github.io/adk-docs/context/))
3. Google ADK Documentation, "Context Compression" ([google.github.io/adk-docs/context/compaction](https://google.github.io/adk-docs/context/compaction/))
4. Google ADK Documentation, "Context Caching" ([google.github.io/adk-docs/context/caching](https://google.github.io/adk-docs/context/caching/))
5. Google ADK Documentation, "Sessions & Memory" ([google.github.io/adk-docs/sessions](https://google.github.io/adk-docs/sessions/))
6. Google ADK Documentation, "Artifacts" ([google.github.io/adk-docs/artifacts](https://google.github.io/adk-docs/artifacts/))
7. Google ADK Documentation, "Multi-Agent Systems" ([google.github.io/adk-docs/agents/multi-agents](https://google.github.io/adk-docs/agents/multi-agents/))
8. Lindenbauer et al., "The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management," NeurIPS 2025 DL4C Workshop ([arXiv:2508.21433](https://arxiv.org/pdf/2508.21433))

---

## Next Steps

- **[Observation Masking](../strategies/01-observation-masking.md)** - The simple baseline ADK's filtering resembles
- **[Hybrid Approach](../strategies/03-hybrid-approach.md)** - Research hybrid mirrored by ADK's compaction + filtering
- **[Trajectory Elongation](../experiments/03-trajectory-elongation.md)** - The hidden cost ADK's architecture mitigates
- **[Advanced Strategies](../strategies/04-advanced-strategies.md)** - H-MEM, HiAgent, and other approaches
- **[Future Directions](../challenges/02-future-work.md)** - Open problems in production context management

---

*Based on Google ADK documentation and the Google Developers Blog, 2025*
