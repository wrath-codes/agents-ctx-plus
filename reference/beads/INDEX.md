# beads — Sub-Index

> Git-backed issue tracking with 3-layer architecture (22 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
|[DOCUMENTATION_SUMMARY.md](DOCUMENTATION_SUMMARY.md)|Documentation overview|

### [architecture](architecture/)

|file|description|
|---|---|
|[overview.md](architecture/overview.md)|3-layer arch — Git/JSONL/SQLite, data flow, recovery model|
|[git-layer.md](architecture/git-layer.md)|Git layer — historical source of truth, branching|
|[jsonl-layer.md](architecture/jsonl-layer.md)|JSONL layer — operational source of truth, append-only|
|[sqlite-layer.md](architecture/sqlite-layer.md)|SQLite layer — fast queries, derived state, schema|
|[data-flow.md](architecture/data-flow.md)|Data flow — write/read/sync paths|
|[daemon-system.md](architecture/daemon-system.md)|Daemon — file watching, auto-sync, lock management|

### [core-features](core-features/)

|file|description|
|---|---|
|[issue-management.md](core-features/issue-management.md)|Issues — CRUD operations, lifecycle|
|[dependencies.md](core-features/dependencies.md)|Dependencies — blocks, parent-child, related|
|[hash-ids.md](core-features/hash-ids.md)|Hash IDs — short unique identifiers|
|[labels-comments.md](core-features/labels-comments.md)|Labels and comments|
|[priority-types.md](core-features/priority-types.md)|Priority levels and issue types|

### [workflows](workflows/)

|file|description|
|---|---|
|[chemistry-metaphor.md](workflows/chemistry-metaphor.md)|Chemistry metaphor — workflow model|
|[formulas.md](workflows/formulas.md)|Formulas — workflow templates|
|[gates.md](workflows/gates.md)|Gates — approval/review checkpoints|
|[molecules.md](workflows/molecules.md)|Molecules — compound workflows|
|[wisps.md](workflows/wisps.md)|Wisps — lightweight ephemeral tasks|

### [context-enhancement](context-enhancement/)

|file|description|
|---|---|
|[opportunities.md](context-enhancement/opportunities.md)|Context enhancement opportunities|

### [multi-agent](multi-agent/)

|file|description|
|---|---|
|[overview.md](multi-agent/overview.md)|Multi-agent — coordination patterns|
|[routing.md](multi-agent/routing.md)|Routing — task distribution|

### Key Patterns
```
bd create "title" --priority 1 --type task
bd list --status open --label backend
bd sync / bd sync --import-only / bd sync --force-rebuild
bd daemons killall → rm .beads/beads.db* → bd sync --import-only  # recovery
```

---
*22 files · Related: [btcab](../btcab/INDEX.md)*
