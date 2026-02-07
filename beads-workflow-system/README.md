# Beads + Tempolite Workflow System

## 🎯 Project Overview

A hybrid workflow system that combines **Beads** for coordination and **Tempolite** for execution, creating a powerful workflow engine for AI agents with Git-backed persistence and SQLite-based durability.

**Core Vision**: Provide persistent, coordinated workflow automation for multi-agent development workflows while leveraging proven battle-tested components.

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│              HYBRID WORKFLOW ARCHITECTURE           │
│                                                     │
│  ┌─────────────────┐    ┌─────────────────┐         │
│  │     BEADS      │    │    TEMPOLITE   │         │
│  │   (Coordination)│    │   (Execution)   │         │
│  │                 │    │                 │         │
│  │ • Issue tracking│◄──►│ • Activities    │         │
│  │ • Dependencies  │    │ • Sagas         │         │
│  │ • Multi-agent   │    │ • Signals        │         │
│  │ • Git storage   │    │ • Checkpoints    │         │
│  └─────────────────┘    └─────────────────┘         │
│           │                      │                   │
│           └──────────────┬─────────┘                   │
│                          ▼                           │
│  ┌─────────────────────────────────────────────┐│
│  │           AGENT LAYER                   ││
│  │                                           ││
│  │ • ResearchAgent  ← Activity            ││
│  │ • POCAgent       ← Activity            ││
│  │ • DocumentationAgent ← Activity        ││
│  │ • ValidationAgent ← Activity           ││
│  └─────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

## 📚 Documentation Structure

- [Architecture Overview](./architecture/) - Technical architecture and design decisions
- [Implementation Guide](./implementation/) - Step-by-step implementation instructions
- [User Guides](./guides/) - User documentation and tutorials
- [Examples](./examples/) - Code examples and workflow templates
- [API Reference](./api/) - Complete API documentation
- [CLI Commands](./cli-commands/) - Command-line interface reference
- [Deployment Guide](./deployment/) - Production deployment instructions

## 🎯 Core Features

### Beads Integration
- **Issue Management**: Hash-based IDs prevent multi-agent conflicts
- **Dependency Awareness**: Always know what to work on next
- **Multi-Agent Coordination**: Work assignment and handoff patterns
- **Git-Backed Persistence**: Context travels with codebase
- **Formula System**: Declarative workflow templates

### Tempolite Execution Engine
- **Activity-Based Workflows**: Composable, reusable workflow steps
- **Saga Support**: Transactional operations with compensation
- **Signal System**: Async coordination between workflows
- **Checkpoint & Recovery**: Durable execution with automatic recovery
- **SQLite Storage**: Fast, reliable local persistence

### Custom Coordination Layer
- **Workflow Mapping**: Bridge between beads issues and tempolite workflows
- **Agent Assignment**: Intelligent agent workload distribution
- **Result Storage**: Structured storage for research findings, POC results, etc.
- **Performance Analytics**: Comprehensive workflow performance tracking

## 🔄 Workflow Process

```
Brainstorm → Research → Draft → Issues → POCs → Validate → Document
    ↓           ↓        ↓       ↓       ↓         ↓         ↓
Auto-track   Auto-fetch Auto-create Auto-exec Auto-log   Auto-gen  Auto-sync
```

### Research Phase
```bash
bd workflow start research "Analyze tokio vs async-std performance"
# → Creates beads issue + tempolite research workflow
# → Discovers libraries automatically
# → Analyzes documentation and stores findings
```

### Implementation Phase
```bash
bd workflow start poc bd-123 --based-on research-456
# → Creates POC workflow saga with compensation
# → Builds implementation, runs tests, benchmarks performance
# → Stores results with confidence scores
```

### Documentation Phase
```bash
bd workflow start docs bd-123.4 --template comprehensive
# → Generates documentation using findings and POC results
# → Stores structured metadata for future reference
# → Updates beads issue with generated artifacts
```

## 🚀 Quick Start

### Installation
```bash
# 1. Install dependencies
go install github.com/steveyegge/beads@latest
go install github.com/davidroman0o/tempolite@latest

# 2. Initialize workflow system
git clone https://github.com/your-org/beads-workflow-system.git
cd beads-workflow-system
make setup
```

### First Workflow
```bash
# Initialize in your project
cd your-project
bd init --quiet
workflow setup

# Create first research workflow
bd workflow start research "Find best Rust web framework" \
  --agent research \
  --output-format comprehensive

# Check progress
bd workflow status
bd workflow logs wf-research-001

# Continue with next step
bd workflow start poc wf-research-001.finding-1 \
  --agent poc \
  --template web-server
```

## 📊 Success Metrics

| Metric | Target | Measurement Method |
|---------|---------|-------------------|
| **Workflow Automation** | >80% steps automated | Manual intervention tracking |
| **Agent Coordination** | <5s handoff time | Tempolite signal timing |
| **Data Integrity** | Zero data loss | Coordination DB consistency checks |
| **Performance** | <2s workflow steps | Tempolite activity execution timing |
| **Beads Integration** | 100% compatibility | Beads command success rate |
| **Recovery** | <30s recovery time | Tempolite checkpoint restoration |

## 🔧 Technology Stack

- **Beads**: Issue tracking and multi-agent coordination
- **Tempolite**: SQLite-based workflow execution engine  
- **Go**: Primary implementation language
- **SQLite**: Local persistence for coordination and execution
- **Git**: Distributed version control and backup
- **CLI**: Command-line interface for automation

## 🎯 Design Principles

1. **Local-First**: Everything works offline with optional sync
2. **Fault-Tolerant**: Automatic recovery from failures
3. **Composable**: Reusable workflow components
4. **Observable**: Full visibility into workflow execution
5. **Backward Compatible**: Graceful migration from existing tools
6. **Extensible**: Plugin architecture for custom agents

## 📋 Implementation Status

### Phase 1: Foundation (Weeks 1-2)
- [x] Beads integration layer
- [x] Tempolite workflow definitions
- [x] Custom coordination database
- [x] Basic CLI commands

### Phase 2: Agent Implementation (Weeks 3-4)  
- [x] ResearchAgent workflows
- [x] POCAgent sagas
- [x] DocumentationAgent activities
- [x] ValidationAgent checklists

### Phase 3: Advanced Features (Weeks 5-6)
- [x] Performance monitoring
- [x] Workflow templates
- [x] Agent handoff optimization
- [x] Analytics dashboard

### Phase 4: Production Ready (Weeks 7-8)
- [x] Error handling & recovery
- [x] Deployment automation
- [x] Comprehensive testing
- [x] Documentation completion

## 🔗 Related Projects

- [Beads](https://github.com/steveyegge/beads) - Issue tracking for AI agents
- [Tempolite](https://github.com/davidroman0o/tempolite) - SQLite-based workflow engine
- [cr-sqlite](https://github.com/vlcn-io/cr-sqlite) - Multi-writer SQLite with CRDT
- [Backlite](https://github.com/mikestefanello/backlite) - SQLite job queue

## 📞 Getting Help

- **Documentation**: Check the [guides](./guides/) directory
- **Examples**: See [examples](./examples/) for workflow templates
- **API Reference**: Full API docs in [api/](./api/)
- **Issues**: Report bugs at [GitHub Issues](https://github.com/your-org/beads-workflow-system/issues)

---

**Next Steps**: Read the [Architecture Overview](./architecture/01-system-design.md) to understand the technical details.