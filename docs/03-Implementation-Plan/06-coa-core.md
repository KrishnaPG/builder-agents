# COA Orchestrator Core Design

**Module**: `coa-core`  
**Lines Estimate**: ~2,000  
**Files**: 10-12

---

## Overview

The Creator-Orchestrator Agent (COA) is the central intelligence that:
- Parses user intent into structured specifications
- Decomposes work into tasks
- Manages the agent lifecycle
- Handles construction failures with diagnostics
- Coordinates multi-agent composition

---

## Core Types

### CreatorOrchestratorAgent

```rust
// File: src/coa.rs (400 lines)

use std::sync::Arc;

/// The central orchestrator
/// 
/// Owns the artifact namespace, manages agents, and coordinates work.
pub struct CreatorOrchestratorAgent {
    /// Configuration
    config: COAConfig,
    
    /// Symbol namespace
    symbol_index: Arc<SymbolRefIndex>,
    
    /// Constitutional layer for transformations
    constitutional: Arc<ConstitutionalLayer>,
    
    /// Active agent pool
    agent_pool: AgentPool,
    
    /// Task queue
    task_queue: TaskQueue,
    
    /// Composition strategy selector
    strategy_selector: StrategySelector,
    
    /// Diagnostics collector
    diagnostics: DiagnosticsCollector,
    
    /// Kernel integration
    kernel: KernelAdapter,
}

/// COA configuration
#[derive(Debug, Clone)]
pub struct COAConfig {
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    
    /// Default autonomy level for new agents
    pub default_autonomy: AutonomyLevel,
    
    /// System resource limits
    pub system_limits: SystemLimits,
    
    /// Whether to auto-apply fixes
    pub auto_apply_fixes: bool,
    
    /// Human escalation threshold
    pub escalation_threshold: EscalationThreshold,
}

impl CreatorOrchestratorAgent {
    /// Create new COA instance
    pub fn new(config: COAConfig) -> Self {
        Self {
            config: config.clone(),
            symbol_index: Arc::new(SymbolRefIndex::new()),
            constitutional: Arc::new(ConstitutionalLayer::new()),
            agent_pool: AgentPool::new(config.max_concurrent_agents),
            task_queue: TaskQueue::new(),
            strategy_selector: StrategySelector::default(),
            diagnostics: DiagnosticsCollector::new(),
            kernel: KernelAdapter::new(),
        }
    }
    
    /// Execute high-level user intent
    /// 
    /// This is the main entry point for user interactions.
    pub async fn execute_intent(
        &self,
        intent: UserIntent,
    ) -> Result<ExecutionResult, COAError> {
        // 1. Parse intent into structured specification
        let spec = self.parse_intent(intent).await?;
        
        // 2. Decompose into tasks
        let tasks = self.decompose(spec).await?;
        
        // 3. Create execution graph
        let graph = self.build_execution_graph(&tasks).await?;
        
        // 4. Validate and execute
        match self.execute_graph(graph).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // 5. Handle failure with diagnostics
                self.handle_execution_failure(e, &tasks).await
            }
        }
    }
    
    /// Parse natural language intent into structured spec
    async fn parse_intent(&self, intent: UserIntent) -> Result<Specification, COAError> {
        // Use LLM to extract structured information
        let prompt = format!(
            "Parse this user intent into a structured specification:\n{}\n\n\
             Output JSON with: goal, constraints, acceptance_criteria, artifact_type",
            intent.description
        );
        
        // Call LLM (placeholder)
        let response = self.call_llm(&prompt).await?;
        
        // Parse response
        let spec: Specification = serde_json::from_str(&response)?;
        
        Ok(spec)
    }
    
    /// Decompose specification into atomic tasks
    async fn decompose(&self, spec: Specification) -> Result<Vec<Task>, COAError> {
        let decomposer = TaskDecomposer::new(self.strategy_selector.clone());
        
        decomposer.decompose(spec, &self.symbol_index).await
    }
    
    /// Build execution graph from tasks
    async fn build_execution_graph(
        &self,
        tasks: &[Task],
    ) -> Result<ExecutionGraph, COAError> {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        for task in tasks {
            // Create node spec
            let node_spec = NodeSpecV2 {
                directives: task.directives.clone(),
                autonomy_ceiling: task.autonomy,
                resource_bounds: task.resources,
                expansion_type: task.expansion_type.clone(),
            };
            
            let node_id = builder.add_node(node_spec);
            
            // Add edges for dependencies
            for dep in &task.dependencies {
                if let Some(dep_id) = self.find_task_node(&builder, dep) {
                    builder.add_edge(dep_id, node_id)?;
                }
            }
        }
        
        // Validate through kernel
        let validated = builder.validate(&self.kernel.signing_key())?;
        
        Ok(ExecutionGraph::new(validated))
    }
    
    /// Execute validated graph
    async fn execute_graph(
        &self,
        graph: ExecutionGraph,
    ) -> Result<ExecutionResult, COAError> {
        // Use kernel executor
        let summary = self.kernel.execute(graph.validated).await?;
        
        Ok(ExecutionResult {
            nodes_executed: summary.nodes_executed,
            execution_time: summary.execution_time_ms,
            artifacts_produced: vec![], // Collect from agents
        })
    }
    
    /// Handle execution failure with diagnostics
    async fn handle_execution_failure(
        &self,
        error: COAError,
        tasks: &[Task],
    ) -> Result<ExecutionResult, COAError> {
        // Collect diagnostics
        let diagnostic = self.diagnostics.analyze(&error, tasks).await;
        
        // Generate suggested fixes
        let fixes = diagnostic.suggest_fixes();
        
        if self.config.auto_apply_fixes && fixes.iter().any(|f| f.auto_applicable) {
            // Apply automatic fix
            let fix = fixes.into_iter()
                .find(|f| f.auto_applicable)
                .unwrap();
            
            return self.apply_fix(fix).await;
        }
        
        // Escalate to human
        Err(COAError::RequiresHumanIntervention {
            error,
            diagnostic,
            suggested_fixes: fixes,
        })
    }
    
    /// Spawn an agent for a task
    async fn spawn_agent(&self, task: &Task) -> Result<AgentHandle, COAError> {
        // Generate agent specification
        let agent_spec = AgentSpec {
            role: task.role.clone(),
            directives: task.directives.clone(),
            autonomy: task.autonomy,
            resources: task.resources,
        };
        
        // Get from pool or create new
        let agent = self.agent_pool.acquire(agent_spec).await?;
        
        Ok(agent)
    }
    
    /// Collect delta from agent
    async fn collect_delta<T: ArtifactType>(
        &self,
        agent: &AgentHandle,
        task: &Task,
    ) -> Result<StructuralDelta<T>, COAError> {
        // Wait for agent to produce delta
        let proposal = agent.receive_proposal().await?;
        
        // Validate delta structure
        self.validate_delta_structure(&proposal)?;
        
        Ok(proposal.delta)
    }
    
    /// Apply composition strategy to collected deltas
    fn compose_deltas<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: Vec<StructuralDelta<T>>,
        strategy: &dyn CompositionStrategy,
    ) -> Result<Artifact<T>, COAError> {
        // Validate composition
        strategy.validate(&deltas, &self.symbol_index)
            .map_err(|e| COAError::CompositionFailed(e.to_string()))?;
        
        // Compose
        strategy.compose(base, &deltas)
            .map_err(|e| COAError::CompositionFailed(e.to_string()))
    }
}
```

---

## Task Decomposition

```rust
// File: src/decomposition.rs (300 lines)

/// Decomposes specifications into executable tasks
pub struct TaskDecomposer {
    strategy_selector: StrategySelector,
}

/// An executable task
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub role: String,
    pub description: String,
    pub directives: DirectiveSet,
    pub autonomy: AutonomyLevel,
    pub resources: ResourceCaps,
    pub dependencies: Vec<TaskId>,
    pub target_artifact: SymbolPath,
    pub expected_output: OutputSpec,
    pub expansion_type: Option<ExpansionType>,
}

/// Task output specification
#[derive(Debug, Clone)]
pub enum OutputSpec {
    Code { language: Language },
    Config { schema: String },
    Spec { format: SpecFormat },
    Binary { mime_type: String },
}

impl TaskDecomposer {
    /// Decompose specification into tasks
    pub async fn decompose(
        &self,
        spec: Specification,
        index: &SymbolRefIndex,
    ) -> Result<Vec<Task>, DecompositionError> {
        match spec.goal {
            Goal::CreateNew => self.decompose_create(&spec, index).await,
            Goal::ModifyExisting => self.decompose_modify(&spec, index).await,
            Goal::Refactor => self.decompose_refactor(&spec, index).await,
            Goal::Analyze => self.decompose_analyze(&spec, index).await,
        }
    }
    
    async fn decompose_create(
        &self,
        spec: &Specification,
        index: &SymbolRefIndex,
    ) -> Result<Vec<Task>, DecompositionError> {
        let mut tasks = Vec::new();
        
        // 1. Design task
        tasks.push(Task {
            id: TaskId::new(),
            role: "architect".to_string(),
            description: format!("Design {} structure", spec.artifact_type),
            directives: DirectiveSet::default(),
            autonomy: AutonomyLevel::L3,
            resources: ResourceCaps::default(),
            dependencies: vec![],
            target_artifact: spec.target_path.clone(),
            expected_output: OutputSpec::Spec { format: SpecFormat::DesignDoc },
            expansion_type: None,
        });
        
        // 2. Implementation tasks (decomposed by symbol)
        let symbols = self.identify_symbols(&spec).await?;
        
        for symbol in symbols {
            let task = Task {
                id: TaskId::new(),
                role: "implementer".to_string(),
                description: format!("Implement {}", symbol),
                directives: DirectiveSet::default(),
                autonomy: AutonomyLevel::L4,
                resources: ResourceCaps::default(),
                dependencies: vec![tasks[0].id], // Depends on design
                target_artifact: spec.target_path.child(&symbol),
                expected_output: spec.output_spec(),
                expansion_type: None,
            };
            tasks.push(task);
        }
        
        // 3. Test task
        tasks.push(Task {
            id: TaskId::new(),
            role: "tester".to_string(),
            description: "Generate tests".to_string(),
            directives: DirectiveSet::default(),
            autonomy: AutonomyLevel::L3,
            resources: ResourceCaps::default(),
            dependencies: tasks[1..].iter().map(|t| t.id).collect(),
            target_artifact: spec.target_path.child("tests"),
            expected_output: OutputSpec::Code { language: Language::Rust },
            expansion_type: None,
        });
        
        // Select composition strategy for each group
        for task in &mut tasks {
            task.directives = self.select_strategy(&spec, task);
        }
        
        Ok(tasks)
    }
    
    /// Identify symbols to implement
    async fn identify_symbols(
        &self,
        spec: &Specification,
    ) -> Result<Vec<String>, DecompositionError> {
        // Use LLM or spec analysis
        let prompt = format!(
            "Given this spec, identify the main symbols to implement:\n{}\n\
             Output as JSON array of symbol names",
            spec.acceptance_criteria
        );
        
        // Placeholder
        Ok(vec![
            "main".to_string(),
            "helper".to_string(),
        ])
    }
    
    fn select_strategy(&self, spec: &Specification, task: &Task) -> DirectiveSet {
        let strategy = match spec.artifact_type.as_str() {
            "code" => "single_writer",
            "svg" | "image" => "hybrid",
            _ => "single_writer",
        };
        
        let mut directives = DirectiveSet::default();
        directives.insert("composition_strategy".to_string(), strategy.into());
        directives
    }
}
```

---

## Agent Pool

```rust
// File: src/agent_pool.rs (200 lines)

/// Pool of reusable agents
pub struct AgentPool {
    max_size: usize,
    available: Arc<Mutex<Vec<AgentHandle>>>,
    active: Arc<DashMap<AgentId, AgentHandle>>,
}

/// Agent handle
#[derive(Debug, Clone)]
pub struct AgentHandle {
    id: AgentId,
    spec: AgentSpec,
    channel: mpsc::Channel<AgentMessage>,
}

/// Agent specification
#[derive(Debug, Clone)]
pub struct AgentSpec {
    pub role: String,
    pub directives: DirectiveSet,
    pub autonomy: AutonomyLevel,
    pub resources: ResourceCaps,
}

impl AgentPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            available: Arc::new(Mutex::new(Vec::new())),
            active: Arc::new(DashMap::new()),
        }
    }
    
    /// Acquire an agent (create or reuse)
    pub async fn acquire(&self, spec: AgentSpec) -> Result<AgentHandle, PoolError> {
        // Try to find matching available agent
        let mut available = self.available.lock().await;
        
        if let Some(idx) = available.iter().position(|a| a.spec == spec) {
            let agent = available.remove(idx);
            self.active.insert(agent.id, agent.clone());
            return Ok(agent);
        }
        
        // Create new agent
        if self.active.len() >= self.max_size {
            return Err(PoolError::PoolExhausted);
        }
        
        let agent = self.create_agent(spec).await?;
        self.active.insert(agent.id, agent.clone());
        
        Ok(agent)
    }
    
    /// Release agent back to pool
    pub async fn release(&self, agent: AgentHandle) {
        self.active.remove(&agent.id);
        
        let mut available = self.available.lock().await;
        if available.len() < self.max_size {
            available.push(agent);
        }
        // Else: drop agent
    }
    
    async fn create_agent(&self, spec: AgentSpec) -> Result<AgentHandle, PoolError> {
        let id = AgentId::new();
        let (tx, rx) = mpsc::channel(100);
        
        // Spawn agent task
        tokio::spawn(agent_task(id, spec.clone(), rx));
        
        Ok(AgentHandle {
            id,
            spec,
            channel: tx,
        })
    }
}

/// Agent task (runs in separate tokio task)
async fn agent_task(
    id: AgentId,
    spec: AgentSpec,
    mut rx: mpsc::Receiver<AgentMessage>,
) {
    // Agent lifecycle
    while let Some(msg) = rx.recv().await {
        match msg {
            AgentMessage::Execute(task) => {
                // Execute task
                let result = execute_task(&spec, task).await;
                // Send result back
            }
            AgentMessage::Shutdown => break,
        }
    }
}
```

---

## Diagnostics

```rust
// File: src/diagnostics.rs (300 lines)

/// Collects and analyzes construction/execution failures
pub struct DiagnosticsCollector {
    history: Vec<DiagnosticEvent>,
}

/// Diagnostic information for a failure
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub error_type: ErrorType,
    pub location: Location,
    pub context: Context,
    pub suggested_fixes: Vec<SuggestedFix>,
}

/// Suggested fix for a failure
#[derive(Debug, Clone)]
pub struct SuggestedFix {
    pub description: String,
    pub confidence: f64,
    pub auto_applicable: bool,
    pub resulting_graph_diff: Option<GraphDiff>,
}

impl DiagnosticsCollector {
    /// Analyze error and generate diagnostic
    pub async fn analyze(
        &self,
        error: &COAError,
        tasks: &[Task],
    ) -> Diagnostic {
        match error {
            COAError::ConstructionFailed(e) => {
                self.analyze_construction_failure(e, tasks).await
            }
            COAError::CompositionFailed(e) => {
                self.analyze_composition_failure(e, tasks).await
            }
            COAError::AgentFailed(e) => {
                self.analyze_agent_failure(e, tasks).await
            }
            _ => Diagnostic {
                error_type: ErrorType::Unknown,
                location: Location::Unknown,
                context: Context::empty(),
                suggested_fixes: vec![],
            },
        }
    }
    
    async fn analyze_construction_failure(
        &self,
        error: &ConstructionError,
        tasks: &[Task],
    ) -> Diagnostic {
        match error {
            ConstructionError::ValidationFailed(v) => {
                let fixes = match &v.kind {
                    ConflictKind::OverlappingTargets => vec![
                        SuggestedFix {
                            description: "Decompose into disjoint subtrees".to_string(),
                            confidence: 0.9,
                            auto_applicable: true,
                            resulting_graph_diff: self.suggest_decomposition(v),
                        },
                        SuggestedFix {
                            description: "Use OrderedCompositionStrategy".to_string(),
                            confidence: 0.7,
                            auto_applicable: false,
                            resulting_graph_diff: None,
                        },
                    ],
                    _ => vec![],
                };
                
                Diagnostic {
                    error_type: ErrorType::Construction,
                    location: Location::GraphConstruction,
                    context: Context::from_validation(v),
                    suggested_fixes: fixes,
                }
            }
            _ => Diagnostic::default(),
        }
    }
    
    fn suggest_decomposition(&self, validation: &ValidationDiagnostic) -> Option<GraphDiff> {
        // Generate decomposition diff
        // ...
        None
    }
}
```

---

## Kernel Adapter

```rust
// File: src/kernel_adapter.rs (150 lines)

use cog_kernel::prelude::*;

/// Adapter between COA and kernel
pub struct KernelAdapter {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KernelAdapter {
    pub fn new() -> Self {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        
        Self {
            signing_key,
            verifying_key,
        }
    }
    
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
    
    pub async fn execute(
        &self,
        graph: ValidatedGraph,
    ) -> Result<ExecutionSummary, KernelError> {
        let executor = Executor::new(self.verifying_key);
        executor.run(graph).await
            .map_err(|e| KernelError::ExecutionFailed(e.to_string()))
    }
    
    pub fn create_expansion_builder(&self) -> ExpansionBuilder {
        ExpansionBuilder::new(self.signing_key.clone())
    }
}
```

---

## Tests

```rust
// File: tests/coa_tests.rs (400 lines)

#[cfg(test)]
mod tests {
    use super::*;
    use coa_core::*;

    #[tokio::test]
    async fn coa_executes_simple_intent() {
        let config = COAConfig::default();
        let coa = CreatorOrchestratorAgent::new(config);
        
        let intent = UserIntent {
            description: "Create a hello world function".to_string(),
            context: None,
        };
        
        let result = coa.execute_intent(intent).await;
        assert!(result.is_ok());
    }

    #[test]
    fn task_decomposer_creates_tasks() {
        let decomposer = TaskDecomposer::default();
        let spec = Specification {
            goal: Goal::CreateNew,
            artifact_type: "code".to_string(),
            target_path: SymbolPath::from_str("app.greeter"),
            acceptance_criteria: "Has hello function".to_string(),
        };
        
        let index = SymbolRefIndex::new();
        
        // Would need async runtime in real test
        // let tasks = decomposer.decompose(spec, &index).await.unwrap();
        // assert!(!tasks.is_empty());
    }

    #[tokio::test]
    async fn agent_pool_acquires_and_releases() {
        let pool = AgentPool::new(2);
        
        let spec = AgentSpec {
            role: "tester".to_string(),
            directives: DirectiveSet::default(),
            autonomy: AutonomyLevel::L3,
            resources: ResourceCaps::default(),
        };
        
        let agent1 = pool.acquire(spec.clone()).await.unwrap();
        let agent2 = pool.acquire(spec.clone()).await.unwrap();
        
        // Third acquire should fail (pool size 2)
        let agent3 = pool.acquire(spec.clone()).await;
        assert!(matches!(agent3, Err(PoolError::PoolExhausted)));
        
        // Release one
        pool.release(agent1).await;
        
        // Now acquire should succeed
        let agent3 = pool.acquire(spec).await;
        assert!(agent3.is_ok());
    }

    #[test]
    fn diagnostics_suggests_fixes() {
        let collector = DiagnosticsCollector::new();
        
        let error = COAError::ConstructionFailed(
            ConstructionError::ValidationFailed(ValidationDiagnostic {
                kind: ConflictKind::OverlappingTargets,
                // ...
            })
        );
        
        let diagnostic = collector.analyze(&error, &[]);
        
        assert!(!diagnostic.suggested_fixes.is_empty());
    }
}
```

---

## File Structure

```
crates/coa-core/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── coa.rs              # CreatorOrchestratorAgent
    ├── config.rs           # COAConfig
    ├── decomposition.rs    # TaskDecomposer
    ├── agent_pool.rs       # AgentPool, AgentHandle
    ├── task.rs             # Task, TaskId
    ├── diagnostics.rs      # DiagnosticsCollector
    ├── kernel_adapter.rs   # KernelAdapter
    └── intent.rs           # UserIntent, Specification
```

---

## Cargo.toml

```toml
[package]
name = "coa-core"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core
coa-artifact = { path = "../coa-artifact" }
coa-symbol = { path = "../coa-symbol" }
coa-composition = { path = "../coa-composition" }
coa-constitutional = { path = "../coa-constitutional" }
cog-kernel = { path = "../../kernel" }

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Collections
dashmap = "5"

# LLM (placeholder - replace with actual client)
# reqwest = { version = "0.11", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Cryptography
ed25519-dalek = "2"
rand = "0.8"

# Error handling
thiserror = "1"

[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"
```
