# Architecture Overview

## 🏗️ System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    WORKFLOW TOOL ARCHITECTURE                          │
│                                                                     │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐│
│  │   CLI Layer     │    │  Agent Layer    │    │  Storage Layer  ││
│  │                 │    │                 │    │                 ││
│  │ • Clap Commands│◄──►│ • Supervisor    │◄──►│ • AgentFS       ││
│  │ • Interactive   │    │ • Research      │    │ • DuckDB+VSS   ││
│  │ • Session Mgmt  │    │ • POC           │    │ • Cloudflare R2 ││
│  │ • OpenCode Bridge│    │ • Documentation │    │ • Local Cache   ││
│  └─────────────────┘    │ • Validation   │    │                 ││
│           │              │                 │    │                 ││
│           ▼              │ • GraphFlow    │    │                 ││
│  ┌─────────────────┐    │                 │    │                 ││
│  │   Integration   │    └─────────────────┘    │                 ││
│  │                 │              │                      │                 ││
│  │ • OpenCode      │    ┌─────────────────┐    │                 ││
│  │ • Session Enhance│    │  Intelligence   │    │                 ││
│  │ • Context Builder│    │                 │    │                 ││
│  │ • Tool Registry  │    │ • Local LLMs     │    │                 ││
│  └─────────────────┘    │ • FastEmbed      │    │                 ││
│           │              │ • OpenRouter     │    │                 ││
│           ▼              │ • Context Mgmt    │    │                 ││
│  ┌─────────────────┐    │                 │    │                 ││
│  │   Performance   │    └─────────────────┘    │                 ││
│  │                 │              │                      │                 ││
│  │ • Hardware Detect│    ┌─────────────────┐    │                 ││
│  │ • Adaptive Concurrency│  │   Document     │    │                 ││
│  │ • Multi-Tier Cache│    │   Processing    │    │                 ││
│  │ • Resource Mgmt  │    │                 │    │                 ││
│  └─────────────────┘    │ • Tree-sitter    │    │                 ││
│           │              │ • Vector Store    │    │                 ││
│           ▼              │ • Global Index   │    │                 ││
│  ┌─────────────────────────────────────────────────────────────────────┐│
│  │                    USER WORKFLOW                           ││
│  │                                                             ││
│  │ Brainstorm → Research → Draft → Issues → POCs → Validate   ││
│  │    ↓           ↓        ↓       ↓       ↓         ↓        ││
│  │ Auto-track   Auto-fetch Auto-create Auto-exec Auto-log  Auto-doc  ││
│  │    ↓           ↓        ↓       ↓       ↓         ↓        ││
│  │ Save Session → Continue → Complete → Commit                    ││
│  └─────────────────────────────────────────────────────────────────────┘│
```

---

## 🧠 Core Components

### 1. **CLI Interface Layer**

**Responsibility**: User interaction and command orchestration  
**Key Technologies**: Clap, Tokio, Terminal UI  
**Features**:

- Interactive mode with TUI dashboard
- Command-line interface for scripting
- Session management and continuation
- Configuration management
- Progress tracking and visualization

### 2. **Agent Layer**

**Responsibility**: Specialized AI agents with coordination  
**Key Technologies**: Rig, Candle, OpenRouter, GraphFlow  
**Agents**:

- **ResearchAgent**: Library discovery, documentation analysis
- **POCAgent**: Proof-of-concept implementation and validation
- **DocumentationAgent**: Code parsing, documentation generation
- **SupervisorAgent**: Coordination, context building, OpenCode bridge

### 3. **Storage Layer**

**Responsibility**: Data persistence, vector search, file storage  
**Key Technologies**: AgentFS, DuckDB+VSS, Cloudflare R2, FastEmbed  
**Components**:

- **AgentFS**: Agent state management, audit trails
- **DuckDB**: Vector similarity search, metadata queries
- **Cloudflare R2**: Document storage, global index
- **Local Cache**: Multi-tier caching for performance

---

## 🔄 Data Flow Architecture

### Session Initialization

```
User starts → Detect project → Fetch dependencies → Index docs → Initialize agents → Ready state
```

### Research Phase

```
Brainstorm ideas → ResearchAgent discovers libraries → Download and parse docs → Update RAG index → Generate insights
```

### Implementation Phase

```
POCAgent creates implementation → Validate against assumptions → Log results → Update documentation → Commit changes
```

### OpenCode Enhancement

```
Supervisor collects agent states → Apply observation masking → Build compressed context → Inject into OpenCode → Monitor usage
```

---

## 🎯 Key Design Decisions

### 1. **Agent Specialization**

- **Exclusive tool sets** prevent conflicts and improve performance
- **Persistent agents** maintain learning across sessions
- **GraphFlow orchestration** handles complex workflows and dependencies

### 2. **Context Management Strategy**

- **Observation masking** (M=10) for immediate 50% token reduction
- **Hybrid approach** with summarization at N=43 for long sessions
- **AGENTS.md style** passive context beats active retrieval (100% vs 56%)

### 3. **Storage Architecture**

- **AgentFS** for agent state and coordination
- **DuckDB + VSS** for fast vector search with metadata
- **Cloudflare R2** for scalable document storage
- **Local caching** for offline capability and performance

### 4. **Performance Optimization**

- **Hardware-adaptive** concurrency and resource allocation
- **Multi-tier caching** (L1 memory, L2 AgentFS, L3 R2)
- **Intelligent prefetching** based on workflow patterns
- **Load shedding** for resource-constrained environments

---

## 🔗 Integration Points

### OpenCode Integration

- **Context injection** with retrieval-led reasoning prompts
- **Tool provision** for enhanced agent capabilities
- **Session monitoring** and optimization
- **Seamless user experience** with minimal friction

### External Services

- **OpenRouter API** for free model access and fallbacks
- **Model configuration database** for dynamic model management
- **Community documentation** sharing with opt-in privacy
- **Global knowledge base** for common patterns and solutions

---

## 🛡️ Security & Privacy

### Data Protection

- **Local-first** approach minimizes data exposure
- **Configurable privacy boundaries** for personal vs. shared data
- **Opt-in sharing** for community contributions
- **Encryption at rest** for sensitive data in R2

### Agent Safety

- **Tool boundaries** prevent unauthorized actions
- **Audit trails** for all agent operations via AgentFS
- **Supervisor oversight** for critical operations
- **Rollback capability** for error recovery

---

## 📊 Scalability Considerations

### Performance Scaling

- **Concurrent agents** with hardware-aware limits
- **Distributed processing** for large documentation sets
- **Intelligent caching** to minimize API calls and storage access
- **Background processing** for non-blocking operations

### Storage Scaling

- **R2 unlimited storage** for growing documentation index
- **DuckDB optimization** for millions of vector entries
- **AgentFS sync** for multi-device coordination
- **Compression strategies** to minimize storage costs

---

## 🔗 Cross-References

- Related to: [Agent System Design](./02-agent-system-design.md)
- Related to: [Data Management Strategy](./03-data-management-strategy.md)
- Related to: [Implementation Roadmap](./06-implementation-roadmap.md#phase-1-foundation)

