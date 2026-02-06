# beads — Sub-Index

> Git-backed issue tracking with 3-layer architecture (21 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [🎯 Why Beads?](README.md#why-beads) · [⚡ Quick Start](README.md#quick-start) · [🔑 Essential Commands](README.md#essential-commands) · [🏗️ Core Architecture](README.md#core-architecture) · [🧬 Workflow Chemistry Metaphor](README.md#workflow-chemistry-metaphor) · [🤖 For AI Agents](README.md#for-ai-agents) · [🎯 Integration Options](README.md#integration-options) · [🔄 Multi-Agent Coordination](README.md#multi-agent-coordination) · +6 more|
|[DOCUMENTATION_SUMMARY.md](DOCUMENTATION_SUMMARY.md)|Documentation overview|
| |↳ [📚 Documentation Complete](DOCUMENTATION_SUMMARY.md#documentation-complete) · [📁 Documentation Structure](DOCUMENTATION_SUMMARY.md#documentation-structure) · [✅ What's Documented](DOCUMENTATION_SUMMARY.md#whats-documented) · [🎯 Highlights for Context Enhancement CLI](DOCUMENTATION_SUMMARY.md#highlights-for-context-enhancement-cli) · [📖 Quick Navigation](DOCUMENTATION_SUMMARY.md#quick-navigation) · [🚀 Next Steps](DOCUMENTATION_SUMMARY.md#next-steps) · [📊 Documentation Statistics](DOCUMENTATION_SUMMARY.md#documentation-statistics) · [🔗 Key External Resources](DOCUMENTATION_SUMMARY.md#key-external-resources) · +2 more|

### [architecture](architecture/)

|file|description|
|---|---|
|[overview.md](architecture/overview.md)|3-layer arch — Git/JSONL/SQLite, data flow, recovery model|
| |↳ [🏗️ Three-Layer Architecture](architecture/overview.md#three-layer-architecture) · [🔄 Data Flow](architecture/overview.md#data-flow) · [🔄 Sync Modes](architecture/overview.md#sync-modes) · [🛡️ Recovery Model](architecture/overview.md#recovery-model) · [🎯 Design Trade-offs](architecture/overview.md#design-trade-offs) · [🔧 The Daemon System](architecture/overview.md#the-daemon-system) · [🏢 Multi-Machine Considerations](architecture/overview.md#multi-machine-considerations) · [🔗 Related Documentation](architecture/overview.md#related-documentation) · +1 more|
|[git-layer.md](architecture/git-layer.md)|Git layer — historical source of truth, branching|
| |↳ [🗂️ Role in Three-Layer Architecture](architecture/git-layer.md#role-in-three-layer-architecture) · [📁 Git-Tracked Files](architecture/git-layer.md#git-tracked-files) · [🔄 Git Integration Benefits](architecture/git-layer.md#git-integration-benefits) · [📝 JSONL in Git](architecture/git-layer.md#jsonl-in-git) · [🔧 Git Hooks Integration](architecture/git-layer.md#git-hooks-integration) · [🔄 Git Workflow Patterns](architecture/git-layer.md#git-workflow-patterns) · [🔍 Git History Analysis](architecture/git-layer.md#git-history-analysis) · [🛡️ Backup and Recovery](architecture/git-layer.md#backup-and-recovery) · +4 more|
|[jsonl-layer.md](architecture/jsonl-layer.md)|JSONL layer — operational source of truth, append-only|
| |↳ [📄 Role in Three-Layer Architecture](architecture/jsonl-layer.md#role-in-three-layer-architecture) · [📝 JSONL Format Specification](architecture/jsonl-layer.md#jsonl-format-specification) · [📁 JSONL Files Structure](architecture/jsonl-layer.md#jsonl-files-structure) · [🔄 Append-Only Benefits](architecture/jsonl-layer.md#append-only-benefits) · [🔄 SQLite Rebuild Process](architecture/jsonl-layer.md#sqlite-rebuild-process) · [📊 File Size and Growth](architecture/jsonl-layer.md#file-size-and-growth) · [🛡️ Data Integrity](architecture/jsonl-layer.md#data-integrity) · [🔧 Operational Commands](architecture/jsonl-layer.md#operational-commands) · +3 more|
|[sqlite-layer.md](architecture/sqlite-layer.md)|SQLite layer — fast queries, derived state, schema|
| |↳ [⚡ Role in Three-Layer Architecture](architecture/sqlite-layer.md#role-in-three-layer-architecture) · [🗃️ Database Structure](architecture/sqlite-layer.md#database-structure) · [📊 Database Schema](architecture/sqlite-layer.md#database-schema) · [🔄 Query Performance](architecture/sqlite-layer.md#query-performance) · [🏗️ Database Rebuild Process](architecture/sqlite-layer.md#database-rebuild-process) · [🔧 Database Operations](architecture/sqlite-layer.md#database-operations) · [📈 Performance Optimization](architecture/sqlite-layer.md#performance-optimization) · [🔍 Database Analysis](architecture/sqlite-layer.md#database-analysis) · +3 more|
|[data-flow.md](architecture/data-flow.md)|Data flow — write/read/sync paths|
| |↳ [🔄 Overview of Data Flow](architecture/data-flow.md#overview-of-data-flow) · [📝 Write Operations Flow](architecture/data-flow.md#write-operations-flow) · [📖 Read Operations Flow](architecture/data-flow.md#read-operations-flow) · [🔄 Sync Operations Flow](architecture/data-flow.md#sync-operations-flow) · [🔀 Multi-Agent Data Flow](architecture/data-flow.md#multi-agent-data-flow) · [🌐 Integration Data Flow](architecture/data-flow.md#integration-data-flow) · [🛡️ Error Handling Flow](architecture/data-flow.md#error-handling-flow) · [📊 Performance Flow Analysis](architecture/data-flow.md#performance-flow-analysis) · +4 more|
|[daemon-system.md](architecture/daemon-system.md)|Daemon — file watching, auto-sync, lock management|
| |↳ [🔄 Daemon Role in Architecture](architecture/daemon-system.md#daemon-role-in-architecture) · [🏗️ Daemon Architecture](architecture/daemon-system.md#daemon-architecture) · [🚀 Daemon Lifecycle](architecture/daemon-system.md#daemon-lifecycle) · [🔄 Sync Operations](architecture/daemon-system.md#sync-operations) · [🔒 Lock Management](architecture/daemon-system.md#lock-management) · [🛠️ Daemon Management](architecture/daemon-system.md#daemon-management) · [🔍 Monitoring and Logging](architecture/daemon-system.md#monitoring-and-logging) · [🔧 Configuration and Tuning](architecture/daemon-system.md#configuration-and-tuning) · +3 more|

### [core-features](core-features/)

|file|description|
|---|---|
|[issue-management.md](core-features/issue-management.md)|Issues — CRUD operations, lifecycle|
| |↳ [📋 Issue Overview](core-features/issue-management.md#issue-overview) · [🎯 Issue Types](core-features/issue-management.md#issue-types) · [📊 Priority Levels](core-features/issue-management.md#priority-levels) · [📝 Status Lifecycle](core-features/issue-management.md#status-lifecycle) · [🏷️ Label Management](core-features/issue-management.md#label-management) · [👥 Hierarchical Issues](core-features/issue-management.md#hierarchical-issues) · [📖 Issue Operations](core-features/issue-management.md#issue-operations) · [🔍 Query and Filtering](core-features/issue-management.md#query-and-filtering) · +3 more|
|[dependencies.md](core-features/dependencies.md)|Dependencies — blocks, parent-child, related|
| |↳ [🔗 Dependency Types](core-features/dependencies.md#dependency-types) · [🎯 Dependency Management Commands](core-features/dependencies.md#dependency-management-commands) · [🌳 Dependency Trees](core-features/dependencies.md#dependency-trees) · [⚡ Ready Work Calculation](core-features/dependencies.md#ready-work-calculation) · [🔄 Circular Dependencies](core-features/dependencies.md#circular-dependencies) · [📊 Dependency Statistics](core-features/dependencies.md#dependency-statistics) · [🎯 Multi-Agent Dependencies](core-features/dependencies.md#multi-agent-dependencies) · [🔧 Dependency Workflows](core-features/dependencies.md#dependency-workflows) · +4 more|
|[hash-ids.md](core-features/hash-ids.md)|Hash IDs — short unique identifiers|
| |↳ [🔑 ID System Overview](core-features/hash-ids.md#id-system-overview) · [🎯 How Hash-Based IDs Work](core-features/hash-ids.md#how-hash-based-ids-work) · [🌳 Hierarchical IDs](core-features/hash-ids.md#hierarchical-ids) · [🔄 Multi-Agent Collision Prevention](core-features/hash-ids.md#multi-agent-collision-prevention) · [📊 ID Management](core-features/hash-ids.md#id-management) · [🔍 ID Operations](core-features/hash-ids.md#id-operations) · [🎛️ Advanced ID Features](core-features/hash-ids.md#advanced-id-features) · [📈 ID Analytics](core-features/hash-ids.md#id-analytics) · +5 more|
|[labels-comments.md](core-features/labels-comments.md)|Labels and comments|
| |↳ [🏷️ Labels](core-features/labels-comments.md#labels) · [💬 Comments](core-features/labels-comments.md#comments) · [🎯 Agent Communication Patterns](core-features/labels-comments.md#agent-communication-patterns) · [🔍 Search and Discovery](core-features/labels-comments.md#search-and-discovery) · [📊 Metadata Management](core-features/labels-comments.md#metadata-management) · [🎛️ Automation and Workflows](core-features/labels-comments.md#automation-and-workflows) · [📈 Analytics and Reporting](core-features/labels-comments.md#analytics-and-reporting) · [🔗 Related Documentation](core-features/labels-comments.md#related-documentation) · +1 more|
|[priority-types.md](core-features/priority-types.md)|Priority levels and issue types|
| |↳ [📊 Priority Levels](core-features/priority-types.md#priority-levels) · [🎯 Issue Types](core-features/priority-types.md#issue-types) · [🔄 Priority & Type Interactions](core-features/priority-types.md#priority-type-interactions) · [📊 Analytics and Reporting](core-features/priority-types.md#analytics-and-reporting) · [🎛️ Workflow Automation](core-features/priority-types.md#workflow-automation) · [🎯 Best Practices](core-features/priority-types.md#best-practices) · [🔗 Related Documentation](core-features/priority-types.md#related-documentation) · [📚 See Also](core-features/priority-types.md#see-also)|

### [workflows](workflows/)

|file|description|
|---|---|
|[chemistry-metaphor.md](workflows/chemistry-metaphor.md)|Chemistry metaphor — workflow model|
| |↳ [🧪 Chemistry-Inspired Workflow System](workflows/chemistry-metaphor.md#chemistry-inspired-workflow-system) · [🧬 Phase 1: Proto (Solid) - Formulas](workflows/chemistry-metaphor.md#phase-1-proto-solid-formulas) · [💧 Phase 2: Mol (Liquid) - Molecules](workflows/chemistry-metaphor.md#phase-2-mol-liquid-molecules) · [☁️ Phase 3: Wisp (Vapor) - Ephemeral Operations](workflows/chemistry-metaphor.md#phase-3-wisp-vapor-ephemeral-operations) · [🔄 Phase Transitions](workflows/chemistry-metaphor.md#phase-transitions) · [🎯 When to Use Each Phase](workflows/chemistry-metaphor.md#when-to-use-each-phase) · [📊 Phase Comparison](workflows/chemistry-metaphor.md#phase-comparison) · [🔄 Complete Workflow Example](workflows/chemistry-metaphor.md#complete-workflow-example) · +5 more|
|[formulas.md](workflows/formulas.md)|Formulas — workflow templates|
| |↳ [📝 Formula Structure](workflows/formulas.md#formula-structure) · [🎯 Formula Types](workflows/formulas.md#formula-types) · [📋 Step Definition](workflows/formulas.md#step-definition) · [🔄 Step Dependencies](workflows/formulas.md#step-dependencies) · [📊 Variables](workflows/formulas.md#variables) · [🚪 Gates](workflows/formulas.md#gates) · [🔗 Bond Points](workflows/formulas.md#bond-points) · [🎣 Hooks](workflows/formulas.md#hooks) · +6 more|
|[gates.md](workflows/gates.md)|Gates — approval/review checkpoints|
| |↳ [🚪 What are Gates?](workflows/gates.md#what-are-gates) · [🎯 Gate Types](workflows/gates.md#gate-types) · [🔄 Gate States](workflows/gates.md#gate-states) · [🎛️ Gate Operations](workflows/gates.md#gate-operations) · [📋 Gate Configuration](workflows/gates.md#gate-configuration) · [🔄 waits-for Dependency](workflows/gates.md#waits-for-dependency) · [🎯 Gate Examples](workflows/gates.md#gate-examples) · [🔔 Gate Notifications](workflows/gates.md#gate-notifications) · +5 more|
|[molecules.md](workflows/molecules.md)|Molecules — compound workflows|
| |↳ [🧬 What is a Molecule?](workflows/molecules.md#what-is-a-molecule) · [🔄 Molecule Lifecycle](workflows/molecules.md#molecule-lifecycle) · [🎯 Creating Molecules](workflows/molecules.md#creating-molecules) · [📋 Working with Molecules](workflows/molecules.md#working-with-molecules) · [🔗 Step Dependencies](workflows/molecules.md#step-dependencies) · [🎛️ Advanced Molecule Features](workflows/molecules.md#advanced-molecule-features) · [📊 Progress Tracking](workflows/molecules.md#progress-tracking) · [🏷️ Pinning and Assignment](workflows/molecules.md#pinning-and-assignment) · +6 more|
|[wisps.md](workflows/wisps.md)|Wisps — lightweight ephemeral tasks|
| |↳ [☁️ What are Wisps?](workflows/wisps.md#what-are-wisps) · [🎯 When to Use Wisps](workflows/wisps.md#when-to-use-wisps) · [📝 Creating Wisps](workflows/wisps.md#creating-wisps) · [🔧 Working with Wisps](workflows/wisps.md#working-with-wisps) · [🔄 Wisp Lifecycle](workflows/wisps.md#wisp-lifecycle) · [🎛️ Wisp Configuration](workflows/wisps.md#wisp-configuration) · [🔄 Wisp Transitions](workflows/wisps.md#wisp-transitions) · [📊 Wisp Analytics](workflows/wisps.md#wisp-analytics) · +6 more|

### [context-enhancement](context-enhancement/)

|file|description|
|---|---|
|[opportunities.md](context-enhancement/opportunities.md)|Context enhancement opportunities|
| |↳ [🎯 Overview](context-enhancement/opportunities.md#overview) · [🚀 Key Opportunities](context-enhancement/opportunities.md#key-opportunities) · [🔧 Implementation Patterns](context-enhancement/opportunities.md#implementation-patterns) · [📊 Context Metrics](context-enhancement/opportunities.md#context-metrics) · [🎛️ Advanced Features](context-enhancement/opportunities.md#advanced-features) · [🔗 Integration Guide](context-enhancement/opportunities.md#integration-guide) · [🎯 Best Practices](context-enhancement/opportunities.md#best-practices) · [📚 Implementation Examples](context-enhancement/opportunities.md#implementation-examples) · +2 more|

### [multi-agent](multi-agent/)

|file|description|
|---|---|
|[overview.md](multi-agent/overview.md)|Multi-agent — coordination patterns|
| |↳ [🤖 Overview](multi-agent/overview.md#overview) · [🎯 Key Concepts](multi-agent/overview.md#key-concepts) · [🏗️ Architecture](multi-agent/overview.md#architecture) · [📁 Documentation Sections](multi-agent/overview.md#documentation-sections) · [🚀 Quick Start](multi-agent/overview.md#quick-start) · [🔗 See Also](multi-agent/overview.md#see-also)|
|[routing.md](multi-agent/routing.md)|Routing — task distribution|
| |↳ [🎯 Overview](multi-agent/routing.md#overview) · [📋 Configuration](multi-agent/routing.md#configuration) · [🛠️ Commands](multi-agent/routing.md#commands) · [🔄 Cross-Repo Dependencies](multi-agent/routing.md#cross-repo-dependencies) · [💧 Hydration](multi-agent/routing.md#hydration) · [✅ Best Practices](multi-agent/routing.md#best-practices) · [🔗 Related Documentation](multi-agent/routing.md#related-documentation)|

### Key Patterns
```
bd create "title" --priority 1 --type task
bd list --status open --label backend
bd sync / bd sync --import-only / bd sync --force-rebuild
bd daemons killall → rm .beads/beads.db* → bd sync --import-only  # recovery
```

---
*21 files · Related: [btcab](../btcab/INDEX.md)*
