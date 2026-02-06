# Anthropic: Context Engineering for AI Agents

## Overview

Anthropic's context engineering framework represents a production-proven approach to managing the finite attention budget of LLM-powered agents. Developed through building Claude Code, Claude's Research feature, and Claude playing Pokémon, these patterns address the same fundamental problem identified by the Complexity Trap research: **unmanaged context degrades both cost and performance**.

**Source**: "Effective Context Engineering for AI Agents" (Anthropic Engineering Blog, September 29, 2025)
**URL**: [anthropic.com/engineering/effective-context-engineering-for-ai-agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
**Additional Sources**:
- "Effective Harnesses for Long-Running Agents" (November 26, 2025) ([anthropic.com/engineering/effective-harnesses-for-long-running-agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents))
- "How We Built Our Multi-Agent Research System" (June 13, 2025) ([anthropic.com/engineering/multi-agent-research-system](https://www.anthropic.com/engineering/multi-agent-research-system))
- "Managing Context on the Claude Developer Platform" (September 29, 2025) ([anthropic.com/news/context-management](https://www.anthropic.com/news/context-management))

**Index Terms**: Context Engineering, Anthropic, Claude Code, Compaction, Structured Note-Taking, Multi-Agent Architecture, Agentic Search, Tool Result Clearing, Memory Tool, Long-Horizon Agents, Attention Budget, Context Rot, Progressive Disclosure

---

## 1. The Context Engineering Philosophy

### From Prompt Engineering to Context Engineering

Anthropic draws a clear distinction between prompt engineering and context engineering, viewing the latter as the natural evolution of the former:

> **Prompt engineering** refers to methods for writing and organizing LLM instructions for optimal outcomes.
>
> **Context engineering** refers to the set of strategies for curating and maintaining the optimal set of tokens (information) during LLM inference, including all the other information that may land there outside of the prompts.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│          PROMPT ENGINEERING vs. CONTEXT ENGINEERING                          │
│                                                                             │
│  Prompt Engineering (Discrete):                                              │
│  ──────────────────────────────                                              │
│  • Write a system prompt                                                     │
│  • Optimize word choices and formatting                                      │
│  • One-shot classification/generation tasks                                  │
│  • Primary focus: HOW to write instructions                                  │
│  • Static: prompt is fixed per deployment                                    │
│                                                                             │
│  Context Engineering (Iterative):                                            │
│  ────────────────────────────────                                            │
│  • Curate the entire context state                                           │
│  • Manage system prompts + tools + MCP + data + history                     │
│  • Multi-turn agentic interactions                                           │
│  • Primary focus: WHAT configuration of tokens to pass                      │
│  • Dynamic: curation happens every inference step                           │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key Shift: "Context engineering is the art and science of curating          │
│  what will go into the limited context window from that constantly            │
│  evolving universe of possible information." — Anthropic                     │
│                                                                             │
│  Echoes Karpathy: "Context engineering is the delicate art and science       │
│  of filling the context window with just the right information for           │
│  the next step."                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### The Attention Budget Model

Anthropic frames context as a **finite resource with diminishing marginal returns**, analogous to human working memory capacity:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    THE ATTENTION BUDGET                                       │
│                                                                             │
│  Architectural Constraint:                                                    │
│  ─────────────────────────                                                   │
│  • Transformers compute n² pairwise relationships for n tokens              │
│  • As context grows, pairwise attention gets "stretched thin"               │
│  • Models trained on shorter sequences → less experience with               │
│    context-wide dependencies at longer lengths                               │
│  • Position encoding interpolation enables longer contexts but               │
│    with degradation in token position understanding                          │
│                                                                             │
│  Result: Performance Gradient (Not Hard Cliff)                              │
│  ─────────────────────────────────────────────                               │
│                                                                             │
│  Attention Quality                                                           │
│       ▲                                                                      │
│  100% │████████████████                                                      │
│       │                █████████                                              │
│   80% │                         █████████                                    │
│       │                                  ██████████                           │
│   60% │                                            ████████████              │
│       │                                                        ████████     │
│   40% │                                                                ████ │
│       └──────────────────────────────────────────────────────────────────▶   │
│       0     32K     64K    128K    256K    512K    1M    Tokens              │
│                                                                             │
│  "Context Rot" (Hong et al., Chroma Research):                              │
│  As token count increases, the model's ability to accurately recall          │
│  information decreases — similar to human memory degradation                │
│  under information overload.                                                 │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Guiding Principle: Find the SMALLEST POSSIBLE set of HIGH-SIGNAL            │
│  tokens that maximize the likelihood of the desired outcome.                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Connection to Complexity Trap

The attention budget model provides the theoretical foundation for the Complexity Trap's empirical findings:

| Complexity Trap Finding | Anthropic's Framing |
|-------------------------|---------------------|
| Observations are ~84% of trajectory | Every token depletes the attention budget |
| Unmanaged costs double without benefit | Diminishing marginal returns from added context |
| Simple masking matches summarization | The smallest high-signal set is often sufficient |
| "Lost in the Middle" effects | Performance gradient, not hard cliff |

---

## 2. The Anatomy of Effective Context

### Context Components

Anthropic identifies four primary components that make up an agent's context, each requiring distinct engineering strategies:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 ANATOMY OF EFFECTIVE CONTEXT                                 │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  SYSTEM PROMPTS                                                      │    │
│  │  ──────────────                                                       │    │
│  │  • Clear, simple, direct language                                    │    │
│  │  • "Right altitude" — between brittle logic and vague guidance      │    │
│  │  • Organized with XML tags / Markdown headers                       │    │
│  │  • Minimal set that fully outlines expected behavior                 │    │
│  │  • Start minimal with best model, iterate on failure modes          │    │
│  │                                                                       │    │
│  │  Failure Modes:                                                       │    │
│  │  ┌────────────────────────────────────────────────────────────┐     │    │
│  │  │ TOO LOW (Brittle)        GOLDILOCKS        TOO HIGH (Vague)│     │    │
│  │  │ ─────────────────        ─────────         ────────────────│     │    │
│  │  │ if x then do y           Heuristics +      "Be helpful"    │     │    │
│  │  │ else if z then w         examples that     "Do the right   │     │    │
│  │  │ Complex hardcoded        guide behavior     thing"         │     │    │
│  │  │ logic chains             flexibly           Assumes shared │     │    │
│  │  │ High maintenance                            context        │     │    │
│  │  └────────────────────────────────────────────────────────────┘     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  TOOLS (including MCP)                                                │    │
│  │  ─────────────────────                                                │    │
│  │  • Self-contained and robust to error                                │    │
│  │  • Clear, unambiguous descriptions and parameters                   │    │
│  │  • Minimal overlap in functionality                                  │    │
│  │  • Token-efficient return values                                     │    │
│  │  • Return only what the agent needs, not everything available       │    │
│  │                                                                       │    │
│  │  Critical Rule: "If a human engineer can't definitively say which   │    │
│  │  tool should be used in a given situation, an AI agent can't be     │    │
│  │  expected to do better."                                              │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  EXAMPLES (Few-Shot)                                                  │    │
│  │  ───────────────────                                                  │    │
│  │  • Curate diverse, canonical examples                               │    │
│  │  • Avoid stuffing laundry list of edge cases                        │    │
│  │  • "Examples are the pictures worth a thousand words"               │    │
│  │  • Show expected behavior across key scenarios                      │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  MESSAGE HISTORY                                                      │    │
│  │  ───────────────                                                      │    │
│  │  • Most dynamic and problematic component                           │    │
│  │  • Grows linearly with agent turns                                  │    │
│  │  • Requires active management (compaction, clearing, note-taking)   │    │
│  │  • Dominated by tool results (observations)                         │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Overall Guidance: Be thoughtful and keep context informative, yet tight.  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### The "Right Altitude" for System Prompts

```python
# Anthropic's System Prompt Altitude Model

class PromptAltitude:
    """
    Anthropic recommends finding the "Goldilocks zone" for system prompts:
    specific enough to guide behavior, flexible enough to provide heuristics.
    """
    
    # TOO LOW: Brittle hardcoded logic
    BAD_LOW_ALTITUDE = """
    If the user asks about authentication, check if they mention OAuth.
    If OAuth, respond with steps 1-5 from the OAuth guide.
    If not OAuth, check if they mention API keys.
    If API keys, respond with the API key documentation link.
    Otherwise, ask which auth method they want.
    """
    
    # TOO HIGH: Vague, assumes shared context
    BAD_HIGH_ALTITUDE = """
    Help users with authentication. Be thorough and accurate.
    """
    
    # GOLDILOCKS: Heuristic-driven with clear structure
    GOOD_ALTITUDE = """
    <instructions>
    You help users implement authentication in their applications.
    
    ## Approach
    - Identify the user's auth requirements (OAuth, API keys, JWT, etc.)
    - Recommend the simplest approach that meets their needs
    - Provide implementation guidance with code examples
    
    ## Tool guidance
    - Use `search_docs` to find relevant authentication documentation
    - Use `read_file` to examine existing auth configuration
    - Use `run_tests` to verify authentication works end-to-end
    
    ## Output description
    - Start with a brief assessment of their current setup
    - Provide step-by-step implementation guidance
    - Include security best practices relevant to their approach
    </instructions>
    """
```

---

## 3. Context Retrieval and Agentic Search

### The "Just-in-Time" Context Paradigm

Anthropic identifies a fundamental shift from pre-computed retrieval (traditional RAG) to **agent-directed, just-in-time context loading**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│           CONTEXT RETRIEVAL PARADIGMS                                        │
│                                                                             │
│  Traditional RAG (Pre-Inference):                                            │
│  ────────────────────────────────                                            │
│  User Query ──▶ Embedding ──▶ Vector Search ──▶ Top-K Chunks ──▶ LLM      │
│                                                                             │
│  Problems:                                                                   │
│  • Static retrieval — fixed at query time                                   │
│  • Stale indexing — corpus changes not reflected                            │
│  • Complex syntax trees — struggle with code understanding                  │
│  • All-or-nothing — retrieves everything upfront                            │
│                                                                             │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─     │
│                                                                             │
│  Agentic Search (Just-in-Time):                                              │
│  ──────────────────────────────                                              │
│  Agent maintains LIGHTWEIGHT IDENTIFIERS:                                    │
│  • File paths                                                                │
│  • Stored queries                                                            │
│  • Web links                                                                 │
│  • Naming conventions and folder hierarchies                                │
│                                                                             │
│  Agent DYNAMICALLY LOADS data into context at runtime using tools:          │
│  • glob, grep → navigate file systems                                       │
│  • head, tail → sample large files                                          │
│  • Targeted SQL queries → extract specific data                             │
│  • Web search → find current information                                    │
│                                                                             │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─     │
│                                                                             │
│  Hybrid Strategy (Claude Code):                                              │
│  ──────────────────────────────                                              │
│  1. CLAUDE.md files → naively loaded upfront (static, always relevant)     │
│  2. glob, grep tools → navigate and retrieve just-in-time (dynamic)        │
│                                                                             │
│  "Do the simplest thing that works" — Anthropic's best advice              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Progressive Disclosure

Anthropic describes how agents can incrementally discover context through exploration, mirroring human cognition:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              PROGRESSIVE DISCLOSURE                                          │
│                                                                             │
│  Each interaction yields context that informs the next decision:            │
│                                                                             │
│  Step 1: ls src/                                                             │
│          → File sizes suggest complexity                                    │
│          → Naming conventions hint at purpose                               │
│          → Timestamps proxy for relevance                                   │
│                                                                             │
│  Step 2: head -20 src/auth.py                                               │
│          → Import structure reveals dependencies                            │
│          → Class signatures indicate architecture                           │
│                                                                             │
│  Step 3: grep -r "def authenticate" src/                                    │
│          → Finds specific implementation location                           │
│          → Discovers test files and usage patterns                          │
│                                                                             │
│  Step 4: cat src/auth.py:45-80                                              │
│          → Full implementation of target function                           │
│          → Only loads the precise lines needed                              │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Insight: Agents assemble understanding LAYER BY LAYER, maintaining          │
│  only what's necessary in working memory — never drowning in exhaustive     │
│  but potentially irrelevant information.                                     │
│                                                                             │
│  Human Analogy: "We don't memorize entire corpuses but rather introduce     │
│  external organization and indexing systems like file systems, inboxes,     │
│  and bookmarks to retrieve relevant information on demand."                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Agentic Search Algorithm

```python
class AgenticContextRetrieval:
    """
    Anthropic's agentic search pattern: maintain lightweight references,
    load data on demand, and build understanding incrementally.
    
    Used in Claude Code for complex data analysis over large databases.
    """
    
    def __init__(self, tools: dict):
        self.tools = tools
        self.references = {}  # Lightweight identifiers
        self.working_memory = []  # Currently loaded context
        
    def discover(self, workspace: str) -> dict:
        """
        Phase 1: Build a map of available information
        without loading full content into context.
        """
        # Collect lightweight references
        directory_tree = self.tools['bash']('find . -type f -name "*.py"')
        
        references = {}
        for path in directory_tree:
            references[path] = {
                'size': self._get_size(path),
                'modified': self._get_mtime(path),
                'purpose': self._infer_purpose_from_path(path),
                # Never load full content at this stage
            }
        
        self.references = references
        return references
    
    def explore(self, query: str) -> list:
        """
        Phase 2: Targeted exploration based on task requirements.
        
        Uses progressive disclosure — each step informs the next.
        """
        results = []
        
        # Step 1: Narrow candidates using metadata
        candidates = self._rank_by_relevance(query, self.references)
        
        # Step 2: Sample headers of top candidates
        for path in candidates[:5]:
            header = self.tools['bash'](f'head -30 {path}')
            relevance = self._assess_relevance(header, query)
            
            if relevance > 0.7:
                results.append({
                    'path': path,
                    'header': header,
                    'relevance': relevance
                })
        
        # Step 3: Load full content only for confirmed matches
        for result in results:
            if result['relevance'] > 0.9:
                content = self.tools['read_file'](result['path'])
                result['content'] = content
                self.working_memory.append(result)
        
        return results
    
    def analyze_large_data(self, database: str, query: str) -> str:
        """
        Claude Code pattern: analyze large databases without loading
        full data objects into context.
        
        The model writes targeted queries, stores results, and uses
        head/tail to analyze volumes of data efficiently.
        """
        # Write targeted query (not SELECT *)
        sql = self._generate_targeted_query(query)
        
        # Execute and store results
        result_path = '/tmp/query_results.csv'
        self.tools['bash'](f'psql -c "{sql}" > {result_path}')
        
        # Sample results without loading everything
        row_count = self.tools['bash'](f'wc -l {result_path}')
        header = self.tools['bash'](f'head -5 {result_path}')
        tail = self.tools['bash'](f'tail -5 {result_path}')
        
        # Analyze structure and sample — never full load
        return self._synthesize_findings(header, tail, row_count)
    
    def _infer_purpose_from_path(self, path: str) -> str:
        """
        Metadata as context signal.
        
        'test_utils.py' in tests/ implies different purpose
        than the same name in src/core_logic/
        """
        parts = path.split('/')
        if 'tests' in parts:
            return 'test_utility'
        elif 'src' in parts and 'core' in parts:
            return 'core_logic'
        elif 'config' in parts:
            return 'configuration'
        return 'unknown'
```

---

## 4. Long-Horizon Techniques

### The Long-Horizon Challenge

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              THE LONG-HORIZON PROBLEM                                        │
│                                                                             │
│  Tasks spanning tens of minutes to MULTIPLE HOURS of continuous work:       │
│  • Large codebase migrations                                                 │
│  • Comprehensive research projects                                           │
│  • Multi-feature application development                                     │
│  • Extended game playing (Claude Plays Pokémon: thousands of steps)         │
│                                                                             │
│  "Waiting for larger context windows might seem like an obvious tactic.     │
│  But it's likely that for the foreseeable future, context windows of         │
│  ALL sizes will be subject to context pollution and information              │
│  relevance concerns."                                                        │
│                                                                             │
│  THREE TECHNIQUES:                                                           │
│  ─────────────────                                                           │
│                                                                             │
│  1. COMPACTION                                                               │
│     └── Summarize + reinitiate (within-session continuity)                  │
│                                                                             │
│  2. STRUCTURED NOTE-TAKING                                                   │
│     └── Persist state outside context window (cross-session memory)         │
│                                                                             │
│  3. MULTI-AGENT ARCHITECTURES                                                │
│     └── Divide work across clean context windows (parallel exploration)     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Technique 1: Compaction

Compaction is the practice of summarizing a conversation nearing the context limit and reinitiating with the summary.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         COMPACTION                                            │
│                                                                             │
│  BEFORE COMPACTION:                                                          │
│  ──────────────────                                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ [System Prompt]                                          200 tokens │    │
│  │ [Turn 1: Reasoning + Action + Observation]             2,500 tokens │    │
│  │ [Turn 2: Reasoning + Action + Observation]             3,200 tokens │    │
│  │ ...                                                                  │    │
│  │ [Turn 45: Reasoning + Action + Observation]            1,800 tokens │    │
│  │ ────────────────────────────────────────────────────────            │    │
│  │ Total: ~185,000 tokens    ← Approaching context limit              │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│                          │ Compaction                                        │
│                          ▼                                                    │
│                                                                             │
│  AFTER COMPACTION:                                                           │
│  ─────────────────                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ [System Prompt]                                          200 tokens │    │
│  │ [Compressed Summary of Turns 1-40]                     3,000 tokens │    │
│  │   • Architectural decisions preserved                               │    │
│  │   • Unresolved bugs preserved                                       │    │
│  │   • Implementation details preserved                                │    │
│  │   • Redundant tool outputs DISCARDED                                │    │
│  │ [5 Most Recently Accessed Files]                       5,000 tokens │    │
│  │ [Turn 41-45: Full Detail]                              9,000 tokens │    │
│  │ ────────────────────────────────────────────────────────            │    │
│  │ Total: ~17,200 tokens     ← Ready for continued work               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  WHAT TO KEEP:                        WHAT TO DISCARD:                      │
│  ──────────────                        ──────────────────                    │
│  • Architectural decisions             • Raw tool outputs from old turns    │
│  • Unresolved bugs                     • Redundant file reads              │
│  • Implementation state                • Intermediate search results       │
│  • Critical error messages             • Superseded observations           │
│  • Current objectives                  • Verbose test output               │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  The Art: "Overly aggressive compaction can result in the loss of subtle    │
│  but critical context whose importance only becomes apparent later."        │
│                                                                             │
│  Recommendation: Start by maximizing RECALL (capture everything relevant), │
│  then iterate to improve PRECISION (eliminate superfluous content).         │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Tool Result Clearing: The Lightest Touch

Anthropic identifies tool result clearing as the **safest, lightest-touch form of compaction**:

```python
class ToolResultClearing:
    """
    Anthropic's Context Editing feature: automatically clear stale
    tool calls and results from within the context window.
    
    "Once a tool has been called deep in the message history, why
    would the agent need to see the raw result again?"
    
    Production Results:
    - 100-turn web search: enabled completion of otherwise-failing workflows
    - Token consumption reduced by 84%
    - Combined with memory tool: 39% performance improvement over baseline
    - Context editing alone: 29% improvement
    """
    
    def clear_stale_results(self, messages: list, keep_recent: int = 5) -> list:
        """
        Clear tool results from older turns while preserving
        the conversation flow (action names remain visible).
        """
        managed = []
        total = len(messages)
        
        for i, msg in enumerate(messages):
            if msg['role'] == 'tool_result' and i < total - keep_recent:
                # Replace verbose result with minimal marker
                managed.append({
                    'role': 'tool_result',
                    'tool_use_id': msg['tool_use_id'],
                    'content': '[Result cleared — tool was executed successfully]'
                })
            else:
                managed.append(msg)
        
        return managed
```

#### Compaction Algorithm

```python
class AnthropicCompaction:
    """
    Compaction as implemented in Claude Code.
    
    Summarize conversation nearing context limit, reinitiate
    with summary + recent files + recent turns.
    """
    
    def __init__(self, model: str, context_limit: int = 200_000):
        self.model = model
        self.context_limit = context_limit
        self.compaction_threshold = int(context_limit * 0.8)
        
    def should_compact(self, messages: list) -> bool:
        """Trigger compaction when approaching context limit."""
        total_tokens = sum(self._count_tokens(m) for m in messages)
        return total_tokens >= self.compaction_threshold
    
    def compact(self, messages: list, recent_files: list = None) -> list:
        """
        Perform compaction: summarize old turns, keep recent detail.
        
        Strategy (from Anthropic's guidance):
        1. Maximize recall first (capture all relevant info)
        2. Then improve precision (eliminate superfluous content)
        """
        # Separate recent turns from compaction candidates
        recent_turns = messages[-10:]  # Keep last ~5 exchanges
        old_turns = messages[:-10]
        
        # Generate high-fidelity summary of old turns
        summary = self._generate_summary(old_turns)
        
        # Reconstruct context
        compacted = [
            messages[0],  # System prompt (always keep)
            {
                'role': 'user',
                'content': f'<compaction_summary>\n{summary}\n</compaction_summary>'
            }
        ]
        
        # Add 5 most recently accessed files
        if recent_files:
            for f in recent_files[:5]:
                compacted.append({
                    'role': 'user',
                    'content': f'<recent_file path="{f["path"]}">\n{f["content"]}\n</recent_file>'
                })
        
        # Add recent turns with full detail
        compacted.extend(recent_turns)
        
        return compacted
    
    def _generate_summary(self, messages: list) -> str:
        """
        Summarize old turns with high fidelity.
        
        Preserve: architectural decisions, unresolved bugs,
                  implementation details, critical constraints.
        Discard:  redundant tool outputs, superseded observations,
                  intermediate search results.
        """
        prompt = """Summarize this agent conversation history.
        
PRESERVE (critical for continued work):
- Architectural decisions and their rationale
- Unresolved bugs and error messages
- Implementation state and progress
- User requirements and constraints
- Key file paths and structures discovered

DISCARD (superfluous):
- Raw tool outputs that have been acted upon
- Redundant file reads
- Intermediate search results
- Verbose test output
- Superseded observations

Format as a structured summary the agent can use to continue work."""
        
        return self._call_model(prompt, messages)
```

### Technique 2: Structured Note-Taking

Structured note-taking is where the agent writes notes persisted **outside the context window**, pulling them back in at later times.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    STRUCTURED NOTE-TAKING                                     │
│                                                                             │
│  Pattern: Agent maintains external state file(s) that persist               │
│  across context resets, compaction events, and even sessions.               │
│                                                                             │
│  CLAUDE CODE Example:                                                        │
│  ────────────────────                                                        │
│  Agent creates and maintains a to-do list / progress tracker               │
│  in the file system, reading it at session start and updating              │
│  it at milestones.                                                           │
│                                                                             │
│  LONG-RUNNING AGENT HARNESS Example:                                        │
│  ───────────────────────────────────                                         │
│  claude-progress.txt + git history + feature_list.json                      │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────┐       │
│  │  claude-progress.txt (Updated by each agent session):            │       │
│  │                                                                    │       │
│  │  ## Session 1 — 2025-11-26 14:30 UTC                             │       │
│  │  - Set up initial environment (init.sh, feature_list.json)       │       │
│  │  - Implemented basic chat UI (React + Tailwind)                  │       │
│  │  - Git commit: a3f2b1c "Initial chat UI scaffold"               │       │
│  │                                                                    │       │
│  │  ## Session 2 — 2025-11-26 15:45 UTC                             │       │
│  │  - Implemented message streaming (SSE endpoint)                  │       │
│  │  - Fixed: message ordering bug in concurrent requests            │       │
│  │  - Git commit: 7d4e9a2 "Add SSE streaming for messages"         │       │
│  │  - KNOWN BUG: sidebar doesn't update on new conversation        │       │
│  │                                                                    │       │
│  │  ## Session 3 — 2025-11-26 17:00 UTC                             │       │
│  │  - Fixed sidebar update bug (WebSocket subscription)             │       │
│  │  - Implemented conversation list with search                     │       │
│  │  - 14/200 features passing                                         │       │
│  │  - Git commit: b9c1f3e "Sidebar updates + conversation list"    │       │
│  └──────────────────────────────────────────────────────────────────┘       │
│                                                                             │
│  CLAUDE PLAYS POKÉMON Example:                                               │
│  ─────────────────────────────                                               │
│  Without ANY prompting about memory structure, the agent:                   │
│  • Maintains precise tallies: "for the last 1,234 steps I've been          │
│    training my Pokémon in Route 1, Pikachu has gained 8 levels             │
│    toward the target of 10"                                                  │
│  • Develops maps of explored regions                                        │
│  • Tracks key achievements unlocked                                         │
│  • Records combat strategies (which attacks work against which foes)       │
│  • After context resets, reads own notes and continues seamlessly          │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  This pattern enables long-horizon strategies IMPOSSIBLE when keeping       │
│  all information in the LLM's context window alone.                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### The Initializer + Coding Agent Pattern

Anthropic's long-running agent harness uses a **two-agent pattern** for structured note-taking across many context windows:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│          INITIALIZER + CODING AGENT HARNESS                                  │
│   (From "Effective Harnesses for Long-Running Agents", Nov 2025)            │
│                                                                             │
│  PROBLEM: Even with compaction, agents fail at multi-session tasks:         │
│  1. "One-shotting" — trying to do everything at once, running out           │
│     of context mid-implementation                                            │
│  2. "Declaring victory" — seeing progress and declaring the job done       │
│                                                                             │
│  SOLUTION: Two specialized prompts for different phases:                    │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────┐      │
│  │  INITIALIZER AGENT (Session 1 only):                               │      │
│  │  ─────────────────────────────────────                              │      │
│  │  • Writes init.sh script (environment setup)                       │      │
│  │  • Creates claude-progress.txt (progress log)                      │      │
│  │  • Creates feature_list.json (200+ features, all "passes": false) │      │
│  │  • Makes initial git commit                                         │      │
│  │  • Sets up the foundation for ALL features                         │      │
│  └───────────────────────────────────────────────────────────────────┘      │
│       │                                                                      │
│       ▼                                                                      │
│  ┌───────────────────────────────────────────────────────────────────┐      │
│  │  CODING AGENT (Every subsequent session):                          │      │
│  │  ────────────────────────────────────────                           │      │
│  │  1. Run pwd                                                          │      │
│  │  2. Read claude-progress.txt + git log --oneline -20               │      │
│  │  3. Read feature_list.json → pick highest-priority failing feature │      │
│  │  4. Run init.sh → start dev server                                  │      │
│  │  5. Basic smoke test (verify app works)                             │      │
│  │  6. Implement ONE feature                                            │      │
│  │  7. End-to-end test (e.g., Puppeteer MCP)                          │      │
│  │  8. Mark feature as "passes": true                                  │      │
│  │  9. git commit with descriptive message                             │      │
│  │  10. Update claude-progress.txt                                      │      │
│  └───────────────────────────────────────────────────────────────────┘      │
│       │                                                                      │
│       ▼ (repeat for next session)                                            │
│                                                                             │
│  KEY INSIGHT: "Finding a way for agents to quickly understand the state     │
│  of work when starting with a fresh context window." Inspiration came       │
│  from knowing what effective software engineers do every day.               │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Design Choice: Feature list uses JSON (not Markdown) because models       │
│  are less likely to inappropriately change or overwrite JSON files.         │
│  Instruction: "It is unacceptable to remove or edit tests."                │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Memory Tool (Claude Developer Platform)

```python
class AnthropicMemoryTool:
    """
    Anthropic's Memory Tool (public beta, September 2025).
    
    File-based system enabling agents to store and consult information
    outside the context window. Operates entirely client-side through
    tool calls — developers manage the storage backend.
    
    Production Results:
    - Memory + context editing: 39% improvement over baseline
    - Context editing alone: 29% improvement
    - Enables knowledge bases that persist across sessions
    """
    
    def __init__(self, memory_dir: str = '/memory'):
        self.memory_dir = memory_dir
        
    def create(self, filename: str, content: str) -> str:
        """Create a new memory file."""
        path = os.path.join(self.memory_dir, filename)
        with open(path, 'w') as f:
            f.write(content)
        return f"Created {filename}"
    
    def read(self, filename: str) -> str:
        """Read a memory file back into context."""
        path = os.path.join(self.memory_dir, filename)
        with open(path, 'r') as f:
            return f.read()
    
    def update(self, filename: str, content: str) -> str:
        """Update an existing memory file."""
        path = os.path.join(self.memory_dir, filename)
        with open(path, 'w') as f:
            f.write(content)
        return f"Updated {filename}"
    
    def delete(self, filename: str) -> str:
        """Delete a memory file."""
        path = os.path.join(self.memory_dir, filename)
        os.remove(path)
        return f"Deleted {filename}"
    
    # Usage patterns:
    # 1. Agent creates notes.md to track progress
    # 2. Agent creates findings.md to store research results
    # 3. Agent reads notes.md at start of new session
    # 4. Agent updates notes.md at milestones
    # 5. Knowledge persists across compaction and session boundaries
```

### Technique 3: Sub-Agent Architectures

Sub-agent architectures divide work across clean context windows, achieving context isolation and compression through **parallel exploration with condensed returns**.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              SUB-AGENT ARCHITECTURE                                          │
│   (From "How We Built Our Multi-Agent Research System", June 2025)          │
│                                                                             │
│  ORCHESTRATOR-WORKER PATTERN:                                                │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │                     LEAD AGENT (Opus 4)                          │        │
│  │  • Analyzes query and develops strategy                          │        │
│  │  • Spawns subagents for parallel exploration                    │        │
│  │  • Synthesizes condensed returns                                 │        │
│  │  • Decides if more research needed                               │        │
│  │  • Saves plan to Memory (context may be truncated at 200K)      │        │
│  │  • Uses extended thinking for planning                           │        │
│  └────────┬──────────────┬──────────────┬──────────────────────────┘        │
│           │              │              │                                    │
│           ▼              ▼              ▼                                    │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐                        │
│  │ SUBAGENT 1   │ │ SUBAGENT 2   │ │ SUBAGENT 3   │  (Sonnet 4)           │
│  │ (Sonnet 4)   │ │ (Sonnet 4)   │ │ (Sonnet 4)   │                        │
│  │              │ │              │ │              │                        │
│  │ Clean ctx    │ │ Clean ctx    │ │ Clean ctx    │                        │
│  │ Own tools    │ │ Own tools    │ │ Own tools    │                        │
│  │ Own prompt   │ │ Own prompt   │ │ Own prompt   │                        │
│  │              │ │              │ │              │                        │
│  │ Explores:    │ │ Explores:    │ │ Explores:    │                        │
│  │ ~50K tokens  │ │ ~30K tokens  │ │ ~45K tokens  │                        │
│  │              │ │              │ │              │                        │
│  │ Returns:     │ │ Returns:     │ │ Returns:     │                        │
│  │ ~1.5K tokens │ │ ~1K tokens   │ │ ~2K tokens   │                        │
│  └──────────────┘ └──────────────┘ └──────────────┘                        │
│                                                                             │
│  COMPRESSION RATIO: ~125K explored → ~4.5K returned (28:1)                │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  "The essence of search is compression: distilling insights from a vast    │
│  corpus. Subagents facilitate compression by operating in parallel          │
│  with their own context windows."                                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Performance and Token Economics

```
┌─────────────────────────────────────────────────────────────────────────────┐
│          MULTI-AGENT PERFORMANCE DATA                                        │
│                                                                             │
│  Internal Evaluation Results:                                                │
│  ────────────────────────────                                                │
│  Multi-agent (Opus 4 lead + Sonnet 4 subagents) outperformed                │
│  single-agent Opus 4 by 90.2% on internal research eval                    │
│                                                                             │
│  BrowseComp Evaluation — Variance Explained:                                │
│  ────────────────────────────────────────────                                │
│  │ Token usage          ████████████████████████████████████████ 80% │      │
│  │ Tool calls           ████████████ 10%                         │      │
│  │ Model choice         ██████████ 5%                            │      │
│  │ Other factors        ██████████ 5%                            │      │
│  └───────────────────────────────────────────────────────────────┘      │
│                                                                             │
│  Token Usage Ratios:                                                         │
│  ───────────────────                                                         │
│  │ Chat interaction        █ 1x                                  │      │
│  │ Single agent            ████ 4x                               │      │
│  │ Multi-agent system      ███████████████ 15x                   │      │
│  └───────────────────────────────────────────────────────────────┘      │
│                                                                             │
│  "Multi-agent systems excel at valuable tasks that involve heavy            │
│  parallelization, information that exceeds single context windows,          │
│  and interfacing with numerous complex tools."                              │
│                                                                             │
│  Subagent Output Pattern:                                                    │
│  ────────────────────────                                                    │
│  Subagents write to filesystem, pass lightweight REFERENCES back            │
│  to coordinator — preventing information loss and reducing                  │
│  token overhead from copying large outputs through conversation history.   │
│                                                                             │
│  Scaling Guidelines:                                                         │
│  ───────────────────                                                         │
│  │ Simple fact-finding     │ 1 agent     │ 3-10 tool calls      │          │
│  │ Direct comparisons      │ 2-4 agents  │ 10-15 calls each     │          │
│  │ Complex research        │ 10+ agents  │ Clearly divided roles │          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Multi-Agent Context Isolation

```python
class AnthropicMultiAgentContext:
    """
    Anthropic's multi-agent context patterns.
    
    Key insight: separation of concerns — detailed search context
    stays isolated within subagents, lead agent focuses on synthesis.
    """
    
    def __init__(self, lead_model: str = "opus-4",
                 worker_model: str = "sonnet-4"):
        self.lead_model = lead_model
        self.worker_model = worker_model
        
    def research(self, query: str) -> dict:
        """
        Orchestrator-worker pattern for research.
        """
        # Lead agent plans approach (using extended thinking)
        plan = self._lead_plan(query)
        
        # Save plan to memory (persists if context truncated at 200K)
        self._save_to_memory('research_plan.md', plan)
        
        # Spawn subagents in parallel
        subagent_tasks = self._decompose_into_tasks(plan)
        
        # Each subagent gets:
        # - Clean context window
        # - Specific search task
        # - Own tools
        # - Interleaved thinking for evaluating results
        results = self._run_parallel_subagents(subagent_tasks)
        
        # Each subagent returns condensed summary (1-2K tokens)
        # from potentially tens of thousands of explored tokens
        condensed = [r['summary'] for r in results]
        
        # Lead agent synthesizes
        synthesis = self._lead_synthesize(query, condensed)
        
        # Decide if more research needed
        if self._needs_more_research(synthesis):
            additional_tasks = self._identify_gaps(synthesis)
            more_results = self._run_parallel_subagents(additional_tasks)
            synthesis = self._lead_refine(synthesis, more_results)
        
        # Citation agent processes final output
        cited = self._add_citations(synthesis)
        
        return cited
    
    def _run_parallel_subagents(self, tasks: list) -> list:
        """
        Run subagents in parallel with clean context windows.
        
        Each subagent:
        1. Plans using interleaved thinking
        2. Searches using 3+ tools in parallel
        3. Evaluates quality of results
        4. Identifies gaps and refines queries
        5. Returns condensed findings
        """
        import asyncio
        
        async def run_subagent(task):
            # Clean context — no cross-contamination
            agent = SubAgent(
                model=self.worker_model,
                task=task,
                tools=['web_search', 'read_page', 'file_write'],
                thinking_mode='interleaved'
            )
            
            # Subagent explores extensively
            exploration = await agent.execute()  # May use ~50K tokens
            
            # But returns only condensed summary
            return {
                'task': task,
                'summary': exploration.summary,  # ~1-2K tokens
                'confidence': exploration.confidence,
                'sources': exploration.sources
            }
        
        return asyncio.gather(*[run_subagent(t) for t in tasks])
```

### Technique Selection Guide

Anthropic provides guidance on when to use each technique:

| Technique | Best For | Key Strength | Trade-off |
|-----------|----------|--------------|-----------|
| **Compaction** | Tasks requiring extensive back-and-forth | Maintains conversational flow | Risk of losing subtle context |
| **Structured Note-Taking** | Iterative development with clear milestones | Persistence across sessions | Requires agent discipline |
| **Multi-Agent** | Complex research and analysis | Parallel exploration | 15x token usage vs. chat |

---

## 5. Production Patterns and Platform Features

### Context Editing (Claude Developer Platform)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              CONTEXT EDITING (Production Feature)                             │
│                                                                             │
│  Launched: September 29, 2025 (Public Beta)                                 │
│  Available: Claude Developer Platform, Amazon Bedrock, Vertex AI            │
│                                                                             │
│  Mechanism:                                                                   │
│  ──────────                                                                  │
│  Automatically clears stale tool calls and results from within              │
│  the context window when approaching token limits.                          │
│                                                                             │
│  Preserves conversation flow while removing content the agent               │
│  no longer needs.                                                            │
│                                                                             │
│  Production Metrics:                                                         │
│  ──────────────────                                                          │
│  │ Metric                        │ Result                   │              │
│  │──────────────────────────────│─────────────────────────│              │
│  │ Context editing alone         │ +29% performance         │              │
│  │ Context editing + memory      │ +39% performance         │              │
│  │ Token reduction (100-turn)    │ -84% consumption         │              │
│  │ Workflow completion           │ Enabled otherwise-failing│              │
│  │                               │ 100-turn workflows       │              │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Connection to Complexity Trap: Context editing is structurally              │
│  equivalent to observation masking — removing old tool outputs              │
│  without LLM-based summarization. The 84% token reduction mirrors          │
│  the finding that observations comprise ~84% of agent trajectories.         │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Long-Running Agent Failure Modes

Anthropic documents specific failure modes discovered during production development:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│           LONG-RUNNING AGENT FAILURE MODES AND SOLUTIONS                     │
│                                                                             │
│  FAILURE MODE 1: "One-Shotting"                                              │
│  ────────────────────────────────                                            │
│  Agent attempts to build entire application at once.                        │
│  Runs out of context mid-implementation.                                     │
│  Next session starts with half-implemented, undocumented features.          │
│                                                                             │
│  Solution: Feature list (JSON) + one-feature-at-a-time constraint          │
│                                                                             │
│  FAILURE MODE 2: "Declaring Victory"                                        │
│  ────────────────────────────────────                                        │
│  Agent sees progress has been made → declares job done.                     │
│  Remaining features never implemented.                                       │
│                                                                             │
│  Solution: Explicit feature checklist with "passes": false defaults         │
│                                                                             │
│  FAILURE MODE 3: "Premature Completion"                                     │
│  ──────────────────────────────────────                                      │
│  Agent marks feature as complete without proper testing.                    │
│  Makes code changes, does unit tests, but misses end-to-end failures.      │
│                                                                             │
│  Solution: End-to-end testing tools (e.g., Puppeteer MCP)                  │
│                                                                             │
│  FAILURE MODE 4: "Context Amnesia"                                          │
│  ─────────────────────────────────                                           │
│  After compaction, agent doesn't know what happened before.                 │
│  Spends tokens re-discovering environment state.                            │
│                                                                             │
│  Solution: Progress files + git history + init.sh script                    │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Connection to Complexity Trap — "Trajectory Elongation":                    │
│  Failure modes 1 and 4 echo the trajectory elongation effect: agents       │
│  run longer because compaction/summarization smooths over critical          │
│  signals. Anthropic's structured artifacts (progress files, feature         │
│  lists) address this by EXTERNALIZING state rather than relying on          │
│  in-context memory alone.                                                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Connection to Complexity Trap Research

### Direct Alignment

The Complexity Trap research and Anthropic's context engineering framework arrive at remarkably similar conclusions from different directions — one empirical, one production-driven:

| Complexity Trap Finding | Anthropic Production Pattern | Alignment |
|-------------------------|------------------------------|-----------|
| **Simple masking is effective** | Context editing (tool result clearing) = lightest-touch compaction | Direct: Both show that removing old tool outputs is the safest, most effective first step |
| **Observations are ~84% of trajectory** | Context editing reduces tokens by 84% in 100-turn eval | Striking numerical convergence |
| **Trajectory elongation from summarization** | "Overly aggressive compaction can result in the loss of subtle but critical context" | Both identify summarization risk; Anthropic adds structured note-taking as mitigation |
| **Hybrid approach wins** | Compaction + note-taking + multi-agent (three-technique arsenal) | Anthropic extends hybrid from two strategies to three complementary techniques |
| **Context management is essential** | "Context must be treated as a finite resource with diminishing marginal returns" | Universal agreement: unmanaged context is unsustainable |
| **Cost reduction through efficiency** | Multi-agent uses 15x tokens vs chat, but enables impossible tasks | Anthropic adds nuance: efficiency matters, but ROI justification also matters |

### Where Anthropic Extends the Research

```
┌─────────────────────────────────────────────────────────────────────────────┐
│        COMPLEXITY TRAP vs. ANTHROPIC CONTEXT ENGINEERING                     │
│                                                                             │
│  Complexity Trap Scope:              Anthropic Extensions:                  │
│  ──────────────────────              ─────────────────────                   │
│  • Observation masking               • Tool result clearing (platform API) │
│  • LLM summarization                 • Compaction (high-fidelity summary)  │
│  • Hybrid combination                • Structured note-taking (external    │
│  • Cost measurement                  │  state: progress files, JSON lists) │
│                                      • Multi-agent architecture             │
│  Single-agent focus                  • Sub-agent context isolation          │
│  SWE-bench evaluation                • Agentic search (just-in-time)       │
│                                      • Progressive disclosure               │
│  Academic analysis                   • Production deployment patterns       │
│                                      • Memory tool (platform feature)       │
│                                      • Long-running harness design          │
│                                                                             │
│  ═══════════════════════════════════════════════════════════════════════     │
│  Key Alignment:                                                              │
│  Both reject "just expand the context window" as a solution.                │
│  Both demonstrate that token curation beats token accumulation.             │
│  Both show simple approaches (masking / clearing) as strong baselines.     │
│                                                                             │
│  Key Extension:                                                              │
│  Anthropic addresses CROSS-SESSION continuity (the research only            │
│  considers within-session management). Structured note-taking and           │
│  progress files enable coherence across context resets — a dimension        │
│  the Complexity Trap research identifies as future work.                    │
│                                                                             │
│  Key Insight:                                                                │
│  Anthropic's three techniques map naturally onto the research strategies:   │
│  • Compaction ≈ LLM Summarization (but tuned for high recall)              │
│  • Context Editing ≈ Observation Masking (tool result clearing)            │
│  • Note-Taking ≈ Novel (external state, not in-context management)         │
│  • Multi-Agent ≈ Novel (context isolation through architecture)            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Implications for Agent Builders

| Decision | Complexity Trap Guidance | Anthropic Pattern | Combined Recommendation |
|----------|-------------------------|-------------------|------------------------|
| **First step** | Apply observation masking | Enable context editing (tool result clearing) | Always clear old tool results first — simplest, safest, highest ROI |
| **When to summarize** | Only as last resort (hybrid) | Compaction: maximize recall first, then precision | Defer summarization; when used, preserve architectural decisions and bugs |
| **Cross-session state** | Not addressed | Structured note-taking: progress files, feature lists, git history | Externalize state to files the agent reads at session start |
| **Parallel work** | Not addressed | Sub-agent architectures with condensed returns | Use multi-agent for research/exploration; ensure 28:1+ compression ratio |
| **Context retrieval** | Not addressed | Agentic search: lightweight references + just-in-time loading | Maintain references, not content; load on demand via tools |
| **Testing completeness** | Not addressed | End-to-end testing tools (Puppeteer MCP) | Agents must verify features as users would, not just via unit tests |

---

## 7. Comparison with Google ADK

| Dimension | Anthropic | Google ADK |
|-----------|-----------|------------|
| **Philosophy** | "Smallest set of high-signal tokens" | "Context is a compiled view over richer state" |
| **Compaction** | LLM-based summary + tool result clearing | Sliding window with overlap + LLM summary |
| **External memory** | Memory tool (file-based, client-side) | MemoryService (vector/keyword corpus) |
| **Multi-agent** | Orchestrator-worker with condensed returns | Agents-as-Tools vs. Agent Transfer |
| **Note-taking** | Agent-driven (NOTES.md, progress files) | Session state (key-value scratchpad) |
| **Caching** | Not discussed in detail | Prefix caching with static instructions |
| **Platform support** | Claude Developer Platform (context editing API) | ADK framework (processor pipeline) |
| **Design approach** | Practical patterns from production experience | Systematic architecture with typed abstractions |

---

## References

1. Anthropic Engineering, "Effective Context Engineering for AI Agents," September 2025 ([anthropic.com/engineering/effective-context-engineering-for-ai-agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents))
2. Anthropic Engineering, "Effective Harnesses for Long-Running Agents," November 2025 ([anthropic.com/engineering/effective-harnesses-for-long-running-agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents))
3. Anthropic Engineering, "How We Built Our Multi-Agent Research System," June 2025 ([anthropic.com/engineering/multi-agent-research-system](https://www.anthropic.com/engineering/multi-agent-research-system))
4. Anthropic, "Managing Context on the Claude Developer Platform," September 2025 ([anthropic.com/news/context-management](https://www.anthropic.com/news/context-management))
5. Anthropic Engineering, "Writing Effective Tools for Agents — with Agents," September 2025 ([anthropic.com/engineering/writing-tools-for-agents](https://www.anthropic.com/engineering/writing-tools-for-agents))
6. Anthropic Engineering, "Building Effective Agents," December 2024 ([anthropic.com/engineering/building-effective-agents](https://www.anthropic.com/engineering/building-effective-agents))
7. Hong et al., "Context Rot," Chroma Research, 2025 ([research.trychroma.com/context-rot](https://research.trychroma.com/context-rot))
8. Lindenbauer et al., "The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management," NeurIPS 2025 DL4C Workshop ([arXiv:2508.21433](https://arxiv.org/pdf/2508.21433))

---

## Next Steps

- **[Google ADK Context Architecture](01-google-adk-context.md)** - Production framework comparison
- **[Observation Masking](../strategies/01-observation-masking.md)** - The simple baseline Anthropic's context editing resembles
- **[Hybrid Approach](../strategies/03-hybrid-approach.md)** - Research hybrid mirrored by Anthropic's three-technique arsenal
- **[Trajectory Elongation](../experiments/03-trajectory-elongation.md)** - The hidden cost Anthropic mitigates with structured note-taking
- **[Advanced Strategies](../strategies/04-advanced-strategies.md)** - H-MEM, HiAgent, and other academic approaches
- **[Future Directions](../challenges/02-future-work.md)** - Open problems in production context management

---

*Based on Anthropic Engineering Blog posts and Claude Developer Platform documentation, 2024-2025*
