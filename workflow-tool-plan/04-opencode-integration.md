# OpenCode Integration Strategy

## 🔗 Integration Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│               OPENCODE INTEGRATION ARCHITECTURE              │
│                                                             │
│  ┌─────────────────┐    ┌─────────────────┐               │
│  │  Workflow Tool  │    │   OpenCode      │               │
│  │                │    │                 │               │
│  │ • Supervisor   │◄──►│ • Enhanced       │               │
│  │ • Agent States  │    │   Sessions      │               │
│  │ • Context Data  │    │ • AI Agents     │               │
│  │ • Research DB   │    │ • Tool Access    │               │
│  └─────────────────┘    └─────────────────┘               │
│           │                      │                         │
│           ▼                      ▼                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              Integration Interface               ││
│  │                                                     ││
│  │ • Context Injection (Phase 1)                        ││
│  │ • Tool Provisioning (Phase 2)                       ││
│  │ • Real-time Updates (Phase 3)                         ││
│  │ • Session Monitoring (Phase 4)                        ││
│  └─────────────────────────────────────────────────────────────────────┘│
│                                                             │
│           ▼                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │           Context Management Research Applied       ││
│  │                                                        ││
│  │ • Observation Masking (50% reduction)                   ││
│  │ • Hybrid Strategy (59% cost reduction)                 ││
│  │ • AGENTS.md Passive Context (100% vs 56%)           ││
│  │ • Retrieval-Led Reasoning Instructions               ││
│  │ • Token Optimization                                    ││
│  └─────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

---

## 🎯 Integration Phases

### Phase 1: Context Injection Only (Recommended Start)

**Objective**: Provide enhanced context to OpenCode sessions with minimal complexity  
**Duration**: Weeks 1-2  
**Risk**: Low - simple integration point

#### Implementation

```rust
pub struct OpenCodeContextInjector {
    supervisor: SupervisorAgent,
    opencode_client: OpenCodeClient,
    context_builder: ContextBuilder,
}

impl OpenCodeContextInjector {
    pub async fn enhance_session(&self, session_id: &str) -> Result<()> {
        // 1. Collect comprehensive agent states
        let session_state = self.supervisor.get_full_session_state().await?;

        // 2. Apply research-backed context optimization
        let optimized_context = self.context_builder
            .build_from_agent_states(&session_state)
            .apply_observation_masking(masking_window: 10)
            .compress_documentation(compression_ratio: 0.2)  // 80% reduction
            .add_retrieval_led_reasoning()
            .optimize_for_token_efficiency()
            .await?;

        // 3. Inject into OpenCode session
        self.opencode_client
            .update_session_context(session_id, &optimized_context)
            .await?;

        // 4. Log integration for monitoring
        self.log_context_injection(session_id, &optimized_context).await?;

        Ok(())
    }
}
```

#### Context Building Strategy

```rust
pub struct ResearchBackedContextBuilder {
    doc_index: Arc<GlobalDocIndex>,
    findings_db: Arc<FindingsDatabase>,
    assumptions_store: Arc<AssumptionsStore>,
}

impl ResearchBackedContextBuilder {
    pub async fn build_optimal_context(&self, session_state: &SessionState) -> Result<OpenCodeContext> {
        let mut context_sections = Vec::new();

        // 1. Project Overview (AGENTS.md style)
        context_sections.push(ContextSection::ProjectOverview {
            title: "Current Project Analysis",
            content: self.build_project_overview(&session_state.project),
        });

        // 2. Research Findings (compressed)
        let compressed_findings = self.compress_findings(&session_state.research_findings)?;
        context_sections.push(ContextSection::ResearchFindings {
            title: "Relevant Research Findings",
            content: compressed_findings,
        });

        // 3. POC Results (key insights)
        let poc_insights = self.extract_poc_insights(&session_state.poc_results)?;
        context_sections.push(ContextSection::POCInsights {
            title: "Validated Implementation Insights",
            content: poc_insights,
        });

        // 4. Active Assumptions (high-confidence)
        let active_assumptions = self.filter_active_assumptions(&session_state.assumptions)?;
        context_sections.push(ContextSection::ActiveAssumptions {
            title: "Current Working Assumptions",
            content: active_assumptions,
        });

        // 5. Retrieval-Led Reasoning Instruction
        context_sections.push(ContextSection::Instruction {
            title: "Context Usage Instructions",
            content: PREFER_RETRIEVAL_LED_REASONING.to_string(),
        });

        Ok(OpenCodeContext::new(context_sections))
    }
}
```

### Phase 2: Selective Tool Provisioning

**Objective**: Provide specific tools for OpenCode agents to call directly  
**Duration**: Weeks 3-4  
**Risk**: Medium - potential for circular dependencies

#### Tool Registry

```rust
pub struct OpenCodeToolRegistry {
    available_tools: HashMap<String, Box<dyn OpenCodeTool>>,
    usage_stats: HashMap<String, ToolUsageStats>,
}

#[async_trait]
pub trait OpenCodeTool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value, context: &OpenCodeContext) -> Result<ToolResult>;
}

// Specific tools for OpenCode integration
pub struct GetProjectContextTool;
pub struct GetActiveAssumptionsTool;
pub struct GetResearchFindingsTool;
pub struct GetPOCResultsTool;

impl OpenCodeTool for GetProjectContextTool {
    fn name(&self) -> &str { "get_project_context" }
    fn description(&self) -> &str { "Get current project analysis and structure" }

    async fn execute(&self, params: serde_json::Value, context: &OpenCodeContext) -> Result<ToolResult> {
        let project_analysis = self.supervisor.get_project_analysis().await?;
        Ok(ToolResult::success(serde_json::to_value(project_analysis)?))
    }
}
```

#### Tool Exposure Strategy

```rust
impl OpenCodeToolRegistry {
    pub async fn provide_tools_to_opencode(&self, session_id: &str) -> Result<()> {
        let context = self.opencode_client.get_session_context(session_id).await?;

        // Determine which tools are relevant based on context
        let relevant_tools = self.select_relevant_tools(&context)?;

        // Register tools with OpenCode
        for tool in relevant_tools {
            self.opencode_client
                .register_tool(session_id, tool)
                .await?;
        }

        // Monitor tool usage for optimization
        self.track_tool_provisioning(session_id, &relevant_tools).await?;

        Ok(())
    }

    fn select_relevant_tools(&self, context: &OpenCodeContext) -> Result<Vec<Box<dyn OpenCodeTool>>> {
        let mut tools = Vec::new();

        // Always provide project context
        tools.push(Box::new(GetProjectContextTool::new()));

        // Add research findings if available
        if context.has_research_findings() {
            tools.push(Box::new(GetResearchFindingsTool::new()));
        }

        // Add POC results if implementation phase
        if context.is_implementation_phase() {
            tools.push(Box::new(GetPOCResultsTool::new()));
            tools.push(Box::new(GetActiveAssumptionsTool::new()));
        }

        Ok(tools)
    }
}
```

### Phase 3: Real-time Updates

**Objective**: Provide live context updates as agents generate new insights  
**Duration**: Weeks 5-6  
**Risk**: Medium-High - requires careful coordination

#### Real-time Update System

```rust
pub struct RealtimeUpdateManager {
    update_channel: broadcast::Sender<ContextUpdate>,
    subscribers: HashMap<String, OpenCodeSession>,
    update_queue: VecDeque<ContextUpdate>,
}

#[derive(Debug, Clone)]
pub struct ContextUpdate {
    pub update_type: UpdateType,
    pub content: serde_json::Value,
    pub priority: UpdatePriority,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum UpdateType {
    NewResearchFinding { finding: Finding },
    POCResult { result: POCResult },
    AssumptionValidated { validation: ValidationResult },
    DocumentationGenerated { doc: Documentation },
}

impl RealtimeUpdateManager {
    pub async fn start_realtime_updates(&self, session_id: &str) -> Result<()> {
        // Subscribe to agent state changes
        self.supervisor.subscribe_to_agent_states().await?;

        // Subscribe to session changes
        self.opencode_client.subscribe_to_session_updates(session_id).await?;

        // Start update processing loop
        tokio::spawn(async move {
            while let Some(update) = self.update_channel.recv().await {
                if self.should_send_update(&update) {
                    self.send_update_to_opencode(&update).await?;
                }
            }
        });

        Ok(())
    }

    fn should_send_update(&self, update: &ContextUpdate) -> bool {
        match update.priority {
            UpdatePriority::Critical => true,  // Always send
            UpdatePriority::High => {
                // Rate limit: max 5 per minute
                self.update_queue.len() < 5
            },
            UpdatePriority::Normal => {
                // Throttle: max 10 per minute
                self.update_queue.len() < 10
            },
        }
    }
}
```

### Phase 4: Session Monitoring & Optimization

**Objective**: Track integration effectiveness and optimize based on usage patterns  
**Duration**: Weeks 7-8  
**Risk**: Low - analytics and optimization only

#### Monitoring System

```rust
pub struct IntegrationMonitor {
    metrics_collector: MetricsCollector,
    pattern_analyzer: PatternAnalyzer,
    optimization_engine: OptimizationEngine,
}

#[derive(Debug)]
pub struct IntegrationMetrics {
    // Context effectiveness
    context_relevance_score: f64,
    token_usage_before: u32,
    token_usage_after: u32,
    context_reduction_percentage: f64,

    // Tool usage
    tools_provided: Vec<String>,
    tools_used: Vec<String>,
    tool_success_rate: f64,

    // User satisfaction
    user_feedback_scores: Vec<f64>,
    session_completion_rate: f64,
}

impl IntegrationMonitor {
    pub async fn track_session_effectiveness(&self, session_id: &str) -> Result<IntegrationMetrics> {
        let session_data = self.collect_session_data(session_id).await?;

        // Calculate metrics
        let metrics = IntegrationMetrics {
            context_relevance_score: self.calculate_relevance(&session_data),
            token_usage_before: session_data.initial_token_usage,
            token_usage_after: session_data.enhanced_token_usage,
            context_reduction_percentage: self.calculate_reduction(&session_data),
            tools_provided: session_data.provided_tools.clone(),
            tools_used: session_data.used_tools.clone(),
            tool_success_rate: self.calculate_tool_success(&session_data),
            user_feedback_scores: session_data.feedback_scores.clone(),
            session_completion_rate: self.calculate_completion_rate(&session_data),
        };

        // Store for analysis
        self.metrics_collector.store_session_metrics(session_id, &metrics).await?;

        Ok(metrics)
    }

    pub async fn generate_optimization_recommendations(&self) -> Result<OptimizationPlan> {
        let patterns = self.pattern_analyzer.analyze_usage_patterns().await?;
        let recommendations = self.optimization_engine.generate_recommendations(&patterns).await?;

        Ok(OptimizationPlan::new(patterns, recommendations))
    }
}
```

---

## 📊 Context Management Research Applied

### Observation Masking Implementation

**Based on Research**: M=10 provides optimal balance of context retention and token reduction

```rust
impl ContextBuilder {
    pub fn apply_observation_masking(&self, session: &Session) -> MaskedContext {
        let current_turn = session.conversation_history.len();
        let masking_window = 10; // Research-optimized value

        let masked_history = session.conversation_history
            .iter()
            .enumerate()
            .map(|(i, turn)| {
                let turns_ago = current_turn - i;

                if turns_ago > masking_window {
                    // Keep reasoning and actions, mask verbose observations
                    Turn {
                        reasoning: turn.reasoning.clone(),
                        action: turn.action.clone(),
                        observation: "[Observation omitted for brevity]".to_string(),
                    }
                } else {
                    // Keep full turn for recent history
                    turn.clone()
                }
            })
            .collect();

        MaskedContext::new(masked_history)
    }
}
```

**Expected Token Reduction**: ~50% with preserved reasoning chain

### AGENTS.md Style Passive Context

**Based on Research**: Passive context beats active retrieval (100% vs 56% success rate)

```rust
impl ContextBuilder {
    pub fn build_agents_md_style_context(&self, session_state: &SessionState) -> Result<String> {
        let mut context = String::new();

        // Add retrieval-led reasoning instruction (critical from AGENTS.md research)
        context.push_str(&format!(
            "{}\n\n",
            PREFER_RETRIEVAL_LED_REASONING
        ));

        // Add compressed documentation index (80% reduction)
        context.push_str(&format!(
            "## Project Documentation Index\n\n{}\n\n",
            self.compress_documentation_index(&session_state)?
        ));

        // Add recent findings and insights
        context.push_str(&format!(
            "## Recent Research Findings\n\n{}\n\n",
            self.summarize_findings(&session_state.research_findings)?
        ));

        // Add active POC insights
        context.push_str(&format!(
            "## Implementation Insights\n\n{}\n\n",
            self.extract_key_insights(&session_state.poc_results)?
        ));

        Ok(context)
    }
}

const PREFER_RETRIEVAL_LED_REASONING: &str = r#"
<agent_instructions>
When working with this project context, prefer retrieval-led reasoning over pre-training-led reasoning.

Retrieval-led reasoning means:
1. Check the documentation index for relevant patterns first
2. Use the research findings and POC insights as primary guidance
3. Apply validated assumptions rather than making new assumptions
4. Use pre-training knowledge only for edge cases not covered here

Pre-training-led reasoning means:
1. Relying on what you learned during training (often outdated)
2. Making assumptions without validation
3. Ignoring proven research findings
4. Should be the fallback, not the default

The documentation and findings above have been validated through research and POC testing.
Prioritize this information over general training knowledge.
</agent_instructions>
"#;
```

### Hybrid Context Management

**Based on Research**: Switch to summarization at N=43 for optimal cost-efficiency

```rust
pub struct HybridContextManager {
    masking_window: usize,
    summarize_at: usize,
    summary_prompt: String,
}

impl HybridContextManager {
    pub fn get_optimized_context(&self, session: &Session) -> Result<String> {
        let conversation_length = session.conversation_history.len();

        if conversation_length < self.summarize_at {
            // Phase 1: Use observation masking only
            self.apply_observation_masking(session)
        } else {
            // Phase 2: Create summary and keep recent tail
            let summary = self.create_session_summary(session).await?;
            let tail = session.conversation_history
                .iter()
                .skip(conversation_length - self.masking_window)
                .collect();

            self.build_hybrid_context(summary, tail)
        }
    }
}
```

**Expected Performance**: -59% cost reduction with +2.6pp solve rate improvement

---

## 📈 Integration Success Metrics

### Key Performance Indicators

| Metric                    | Target     | Measurement Method                  |
| ------------------------- | ---------- | ----------------------------------- |
| **Context Relevance**     | >90%       | User feedback + relevance scoring   |
| **Token Reduction**       | >50%       | Before/after token usage comparison |
| **Tool Adoption**         | >70%       | Tools provided vs. tools used       |
| **Session Enhancement**   | <30s       | Time to enhance new session         |
| **User Satisfaction**     | >4.0/5.0   | Regular feedback collection         |
| **Integration Stability** | <1% errors | Error rate monitoring               |

### Monitoring Dashboard

```rust
pub struct IntegrationDashboard {
    real_time_metrics: Arc<RwLock<RealTimeMetrics>>,
    historical_trends: Arc<RwLock<TrendData>>,
    alert_system: AlertSystem,
}

impl IntegrationDashboard {
    pub fn get_current_metrics(&self) -> RealTimeMetrics {
        self.real_time_metrics.read().clone()
    }

    pub fn generate_performance_report(&self, period: TimePeriod) -> PerformanceReport {
        let trends = self.historical_trends.read().clone();

        PerformanceReport::new()
            .with_token_efficiency(trends.token_reduction_trend)
            .with_context_relevance(trends.relevance_scores)
            .with_tool_adoption(trends.tool_usage_rates)
            .with_user_satisfaction(trends.satisfaction_trend)
            .with_recommendations(self.generate_improvement_recommendations())
    }
}
```

---

## 🔗 Cross-References

- Related to: [Architecture Overview](./01-architecture-overview.md#integration-layer)
- Related to: [Agent System Design](./02-agent-system-design.md#supervisoragent)
- Related to: [Data Management Strategy](./03-data-management-strategy.md#sync-strategy)
- Related to: [Task List](./TASKLIST.md#phase-4-integration-optimization)

