# Zenith: Studies Workflow

**Version**: 2026-02-08
**Status**: Design Document
**Purpose**: Structured learning workflow integrated into Zenith — learn about libraries, validate assumptions through code, produce durable knowledge artifacts
**Validated by**: Spike 0.11 (15/15 tests pass, Approach B selected). See [08-studies-spike-plan.md](./08-studies-spike-plan.md)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Workflow Steps](#2-workflow-steps)
3. [Step 1: Create Study](#3-step-1-create-study)
4. [Step 2: Form Assumptions](#4-step-2-form-assumptions)
5. [Step 3: Validate Assumptions](#5-step-3-validate-assumptions)
6. [Step 4: Conclude the Study](#6-step-4-conclude-the-study)
7. [Data Model Integration](#7-data-model-integration)
8. [CLI Commands](#8-cli-commands)
9. [Study Plan Template](#9-study-plan-template)
10. [Zenith Adaptations](#10-zenith-adaptations)

---

## 1. Overview

The studies workflow provides structured learning through LLM interaction:

```
Question -> Study Plan -> Assumptions -> Test -> Validate -> Conclude
```

When a developer says "I want to learn how tokio::spawn works," the LLM creates a study and systematically investigates the topic. Instead of producing a chat transcript that dies with the session, the study produces:

- **Validated assumptions** — hypotheses confirmed or debunked with evidence
- **Test results** — findings tagged `test-result` with code/output references
- **Structured conclusions** — an insight documenting what was learned, what was debunked, and what remains unknown

### What Makes Studies Different From Research

| Dimension | Research | Study |
|-----------|----------|-------|
| Goal | Find the answer | Build a mental model |
| Process | Search -> Synthesize -> Report | Hypothesize -> Test -> Refine -> Prove |
| Artifact | Findings + insight | Tests, validated assumptions, structured report |
| Duration | Can be one-shot | Multi-session, iterative |
| Validation | Citation quality | "Can I use this knowledge correctly?" |

### Test Execution Model

Zenith does NOT execute test code. The LLM uses its own host tools (bash, file I/O, etc.) to write and run test code, then records the results in Zenith via `zen study test`. Zenith is the knowledge store; the LLM's host environment handles execution. This preserves Zenith's "tool, not agent" philosophy.

---

## 2. Workflow Steps

```
1. User asks the LLM to learn about something
2. LLM calls `zen study create --topic "How tokio::spawn works" --library tokio`
   -> Zenith creates a study + linked research item, returns study ID
3. LLM forms assumptions about the topic
   -> `zen study assume <id> --content "spawn requires Send + 'static"`
   -> Creates hypotheses linked to the study
4. LLM explores: searches docs, reads source code
   -> `zen search "spawn" --package tokio`
   -> Uses host tools to write and run test code
5. LLM records test results
   -> `zen study test <id> --assumption <hyp-id> --result validated --evidence "..."`
   -> Creates findings, links to hypotheses, updates statuses
6. LLM checks progress
   -> `zen study get <id>`
   -> Shows N validated, M invalidated, K untested
7. LLM concludes the study
   -> `zen study conclude <id> --summary "..."`
   -> Creates insight, marks study completed
```

---

## 3. Step 1: Create Study

When the user asks to learn about something, the LLM:

**Phase 1: Identify Topic**

1. Create the study: `zen study create --topic "How tokio::spawn works" --library tokio`
2. Zenith auto-creates a research item linked to the study (unless `--research` provided)

**Phase 2: Plan**

The LLM forms a study plan — what questions to answer, what to test. This is captured as the research item's description:

```bash
zen research update <research-id> --description "## Study Plan

### Questions
1. What are the Send + 'static requirements for spawn?
2. What happens when a spawned task panics?
3. Is spawn zero-cost (no allocation)?

### Methodology
explore: read docs, write test code, validate assumptions"
```

**Phase 3: Search Context**

If the library is indexed, search for relevant documentation:

```bash
zen search "spawn" --package tokio
zen search "JoinHandle" --package tokio
```

If not indexed, install it first:

```bash
zen install tokio --ecosystem rust
```

---

## 4. Step 2: Form Assumptions

The LLM forms assumptions about the topic. Each assumption becomes a hypothesis linked to the study:

```bash
zen study assume stu-xxx --content "spawn requires Send + 'static bounds on the future"
zen study assume stu-xxx --content "spawned tasks can panic without crashing the runtime"
zen study assume stu-xxx --content "spawn is zero-cost (no allocation at spawn time)"
```

Each call:
1. Creates a hypothesis with `status: unverified`
2. Links it to the study via `entity_links` (`source_type: study, relation: relates-to`)
3. Also sets `research_id` on the hypothesis for direct FK queries
4. Writes an audit trail entry

---

## 5. Step 3: Validate Assumptions

For each assumption, the LLM:

1. **Writes test code** using its host tools (bash, file write)
2. **Runs the test** using its host tools
3. **Records the result** in Zenith

```bash
# Validated assumption: compile error proves Send is required
zen study test stu-xxx --assumption hyp-aaa \
    --result validated \
    --evidence "Compile error E0277: Rc<i32> cannot be sent between threads safely. Confirms Send bound is enforced at compile time."

# Invalidated assumption: spawn DOES allocate
zen study test stu-xxx --assumption hyp-ccc \
    --result invalidated \
    --evidence "spawn allocates ~200 bytes per task for JoinHandle + task harness on x86_64."
```

Each call:
1. Creates a finding tagged `test-result` with the evidence
2. Links the finding to the study via `entity_links`
3. Links the finding to the hypothesis via `entity_links` (`validates` or `debunks`)
4. Updates hypothesis status to `confirmed` / `debunked` / `inconclusive`
5. Writes audit trail entries

### Result Mapping

| `--result` | Hypothesis status | Entity link relation |
|------------|------------------|---------------------|
| `validated` | `confirmed` | `validates` |
| `invalidated` | `debunked` | `debunks` |
| `inconclusive` | `inconclusive` | `relates-to` |

### Progress Tracking

At any point, the LLM can check study progress:

```bash
zen study get stu-xxx
```

Returns:
```json
{
    "progress": {
        "total": 3,
        "confirmed": 2,
        "debunked": 1,
        "unverified": 0,
        "analyzing": 0,
        "inconclusive": 0
    }
}
```

---

## 6. Step 4: Conclude the Study

When all assumptions are tested (or the LLM decides to stop):

```bash
zen study conclude stu-xxx \
    --summary "## Study Conclusions: How tokio::spawn works

### Confirmed
- spawn requires Send + 'static bounds (compile-time enforcement via E0277)
- Spawned tasks can panic without crashing the runtime (JoinHandle returns JoinError)

### Debunked
- spawn is NOT zero-cost: allocates ~200 bytes per task for JoinHandle + harness

### Procedures Learned
- Use spawn_local on a LocalSet for non-Send data
- Always .await the JoinHandle or explicitly drop it

### Open Questions
- How does spawn interact with structured concurrency patterns?"
```

This:
1. Updates study status to `completed`
2. Stores the summary on the study
3. Creates an insight with the summary content
4. Links the insight to the study via `entity_links`
5. Resolves the linked research item (`status: resolved`)
6. Writes audit trail entries

---

## 7. Data Model Integration

The studies workflow uses the hybrid approach (Spike 0.11, Approach B):

| Study Concept | Zenith Entity | Details |
|--------------|--------------|---------|
| Study itself | `studies` | New table with `topic`, `library`, `methodology`, `status`, `summary` |
| Study question | `research_items` | Auto-created and linked via `research_id` FK |
| Assumption | `hypotheses` | Linked to study via `entity_links` + `research_id` FK |
| Test result | `findings` | Tagged `test-result`, linked to study + hypothesis |
| Evidence chain | `entity_links` | `validates` / `debunks` from findings to hypotheses |
| Conclusion | `insights` | Linked to study via `entity_links` + `research_id` FK |
| Study progress | Derived query | COUNT hypotheses by status WHERE linked to study |
| Study plan | `research_items.description` | Markdown study plan stored on the linked research item |

### No Additional Tables Beyond `studies`

The studies workflow reuses existing entities for all content:
- `hypotheses` for assumptions (with existing 6-state lifecycle)
- `findings` for test results (with existing tag system)
- `insights` for conclusions (with existing confidence levels)
- `entity_links` for evidence chains (with existing relation types)
- `audit_trail` for complete history

---

## 8. CLI Commands

### `zen study create`

```bash
zen study create --topic <topic> [--library <lib>] [--methodology explore|test-driven|compare] [--research <id>]
```

**Implementation**: Creates a `studies` row. If `--research` not provided, auto-creates a `research_items` row and links via `research_id`. Returns both IDs.

### `zen study assume <study-id>`

```bash
zen study assume <study-id> --content <assumption>
```

**Implementation**: Creates a `hypotheses` row with `research_id` from the study's linked research item. Creates an `entity_links` row linking study -> hypothesis. Returns hypothesis ID.

### `zen study test <study-id>`

```bash
zen study test <study-id> --assumption <hyp-id> --result validated|invalidated|inconclusive --evidence <text>
```

**Implementation**:
1. Creates a `findings` row tagged `test-result` with `research_id` from the study
2. Creates `entity_links`: study -> finding, finding -> hypothesis (validates/debunks/relates-to)
3. Updates hypothesis status based on `--result`
4. All in a transaction

### `zen study get <study-id>`

```bash
zen study get <study-id>
```

**Implementation**: Single query joining `studies` with linked hypotheses, findings, and insights via entity_links. Progress derived from hypothesis status counts.

### `zen study conclude <study-id>`

```bash
zen study conclude <study-id> --summary <text>
```

**Implementation**:
1. Updates study `status` to `completed`, sets `summary`
2. Creates insight with summary content
3. Links insight to study via entity_links
4. Updates linked research_item to `status: resolved`
5. All in a transaction

### `zen study list`

```bash
zen study list [--status active|concluding|completed|abandoned] [--library <lib>] [--limit 20]
```

---

## 9. Study Plan Template

When the LLM creates a study, it generates a plan as the research item's description:

```markdown
## Study Plan: [Topic]

### Library
[Library name and version]

### Questions
1. [First question to answer]
2. [Second question to answer]
3. [Third question to answer]

### Methodology
[explore | test-driven | compare]: [Brief description of approach]

### Initial Assumptions
- [Assumption 1] (to be formally added via `zen study assume`)
- [Assumption 2]
- [Assumption 3]

### Resources
- Indexed docs: `zen search "<query>" --package <lib>`
- Source code: [relevant files if known]
```

---

## 10. Zenith Adaptations

### Multi-Session Studies

Studies persist across sessions. At session start, `zen whats-next` includes active studies:

```bash
zen whats-next
# -> "Active studies: stu-xxx 'How tokio::spawn works' (2/3 assumptions tested)"
```

At session resume:

```bash
zen study get stu-xxx
# -> Full state: what's been validated, what remains
```

### Integration with Package Indexing

Studies naturally integrate with Zenith's documentation indexing:

```bash
# Index the library being studied
zen install tokio --ecosystem rust

# Search indexed docs for relevant APIs
zen search "spawn" --package tokio

# Record what you find as a finding
zen finding create --content "spawn signature: pub fn spawn<F>(future: F) -> JoinHandle<F::Output>" \
    --source "package:tokio" --research <research-id>
```

### Linking Studies to Other Entities

Studies can link to existing knowledge:

```bash
# Study derived from a previous finding
zen link stu-xxx fnd-yyy derived-from

# Study relates to an existing research investigation
zen link stu-xxx res-yyy relates-to

# Study findings trigger new tasks
zen link fnd-zzz tsk-www triggers
```

### Studies as PRD Input

Study conclusions can inform PRD creation:

```bash
# Complete a study about a library
zen study conclude stu-xxx --summary "..."

# Create a PRD based on study findings
zen prd create --title "Implement async task pool using tokio::spawn"
zen link iss-xxx stu-xxx derived-from
```

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md) (studies table, section 5)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md) (study commands, section 16)
- Crate designs: [05-crate-designs.md](./05-crate-designs.md) (StudyRepo in zen-db)
- PRD workflow: [06-prd-workflow.md](./06-prd-workflow.md) (composition pattern precedent)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md) (Phase 2: StudyRepo, Phase 5: CLI)
- Spike plan: [08-studies-spike-plan.md](./08-studies-spike-plan.md) (Approach A vs B evaluation)
