# COA Implementation Plan: Constitutional Layer & Core Orchestrator

**Version**: 1.0  
**Date**: 2026-02-22  
**Scope**: `coa-constitutional`, `coa-core`  
**Status**: Design Phase

---

# Part 1: 30,000ft Overview

## System Context

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              USER INTERFACE                                  │
│                     (Natural Language Intent Input)                          │
└─────────────────────────────────┬───────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           COA CORE (coa-core)                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │   Intent     │  │   Task       │  │    Agent     │  │   Diagnostics   │  │
│  │   Parser     │──│ Decomposer   │──│    Pool      │──│   Collector     │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────────┘  │
│         │                 │                 │                   │           │
│         └─────────────────┴─────────────────┴───────────────────┘           │
│                                    │                                        │
│                                    ▼                                        │
│                         ┌─────────────────────┐                             │
│                         │  Graph Builder      │                             │
│                         │  (Validates at      │                             │
│                         │   Construction)     │                             │
│                         └─────────────────────┘                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      CONSTITUTIONAL LAYER (coa-constitutional)               │
│                                                                              │
│   ┌──────────────┐      ┌──────────────┐      ┌──────────────┐             │
│   │   Ingress    │      │   Transform  │      │    Egress    │             │
│   │   Parsers    │──────│   Engine     │──────│  Serializers │             │
│   │              │      │              │      │              │             │
│   │ • tree-sitter│      │ • Delta apply│      │ • Code gen   │             │
│   │ • serde      │      │ • Validation │      │ • Config emit│             │
│   │ • markdown   │      │ • Composition│      │ • Spec write │             │
│   └──────────────┘      └──────────────┘      └──────────────┘             │
│          │                       │                     │                   │
│          └───────────────────────┼─────────────────────┘                   │
│                                  ▼                                          │
│                        ┌─────────────────┐                                  │
│                        │  ArtifactCache  │                                  │
│                        │   (moka-based)  │                                  │
│                        └─────────────────┘                                  │
└─────────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    FOUNDATION LAYER (Already Built)                          │
│                                                                              │
│    coa-artifact          coa-symbol           coa-composition               │
│   ┌───────────┐         ┌───────────┐         ┌──────────────┐              │
│   │ Artifact<T>│         │ SymbolRef │         │ Composition  │              │
│   │ ContentHash│         │   Index   │         │  Strategies  │              │
│   │ SymbolPath │         │  Validator│         │   Registry   │              │
│   │Delta<T>   │         └───────────┘         └──────────────┘              │
│   └───────────┘                                                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Core Design Principles

1. **Safe-by-Construction**: All invalid states unrepresentable at type level
2. **Zero-Cost Abstractions**: Monomorphized generics for hot paths, trait objects at boundaries
3. **Async-First**: All I/O operations async with tokio
4. **High Performance**: 
   - Symbol lookup: O(log n)
   - Delta validation: O(k log n)
   - Throughput: 10k deltas/sec target
5. **Observability**: OpenTelemetry tracing, structured logging, Prometheus metrics

---

# Part 2: Requirements Analysis

## 2.1 Functional Requirements

### coa-constitutional

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| C1 | Parse source files into typed Artifacts | P0 | tree-sitter integration for code, serde for config |
| C2 | Apply StructuralDelta to produce new Artifact | P0 | All 4 operations (Add/Remove/Replace/Transform) |
| C3 | Serialize Artifacts back to source | P0 | Roundtrip: file → Artifact → file = equivalent |
| C4 | Content-addressed caching | P1 | moka cache with TTL, hit rate >90% |
| C5 | Multi-delta composition | P1 | Use composition strategies from coa-composition |
| C6 | Validation before apply | P0 | Check base_hash, target exists, invariants |

### coa-core

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| K1 | Parse user intent to Specification | P0 | LLM-based extraction |
| K2 | Decompose spec into Tasks | P0 | Hierarchical task breakdown |
| K3 | Build ValidatedGraph | P0 | Construction-time validation |
| K4 | Agent lifecycle management | P0 | Pool, spawn, collect results |
| K5 | Handle construction failures | P1 | Diagnostics with suggested fixes |
| K6 | Dynamic graph expansion | P2 | Staged construction protocol |

## 2.2 Non-Functional Requirements

| Metric | Target | Measurement |
|--------|--------|-------------|
| Latency (p99) | <10ms | Construction validation |
| Throughput | 10k deltas/sec | Single-threaded |
| Memory | <50 bytes/symbol | Including index |
| Concurrency | 10k+ concurrent agents | Agent pool |
| Availability | 99.9% | Under partial failure |

## 2.3 Security Requirements

| Requirement | Implementation |
|-------------|----------------|
| No Direct IO | Agents only produce deltas, Constitutional Layer handles files |
| Content Integrity | Blake3 hashes, Merkle trees |
| Token Verification | ed25519 signatures at runtime |
| Resource Isolation | Container primitives (enforced at runtime) |

---

# Part 3: Type System Design

## 3.1 Core Type Hierarchy

```rust
// ============================================================
// LAYER 1: Foundation (Already Built)
// ============================================================

// coa-artifact
crate::artifact::Artifact<T: ArtifactType>
crate::artifact::ArtifactType { type Content; fn hash() -> ContentHash; }
crate::delta::StructuralDelta<T: ArtifactType>
crate::delta::DeltaOperation<T: ArtifactType> { Add, Remove, Replace, Transform }
crate::hash::ContentHash([u8; 32])
crate::path::SymbolPath

// coa-symbol
crate::symbol::SymbolRef { path, parent_hash, revision }
crate::index::SymbolRefIndex

// coa-composition
crate::strategy::CompositionStrategy { validate(), compose() }

// ============================================================
// LAYER 2: Constitutional (To Build)
// ============================================================

/// The trusted boundary between external world and COA
pub struct ConstitutionalLayer {
    parsers: HashMap<String, Box<dyn ArtifactParser>>,
    serializers: HashMap<String, Box<dyn ArtifactSerializer>>,
    transformers: HashMap<String, Box<dyn ArtifactTransformer>>,
    cache: ArtifactCache,
}

/// Parser trait for ingress (file → Artifact)
pub trait ArtifactParser: Send + Sync {
    type Output: ArtifactType;
    fn parse(&self, content: &str) -> Result<Artifact<Self::Output>, ParseError>;
    fn can_parse(&self, path: &Path) -> bool;
}

/// Serializer trait for egress (Artifact → file)
pub trait ArtifactSerializer: Send + Sync {
    type Input: ArtifactType;
    fn serialize(&self, artifact: &Artifact<Self::Input>) -> Result<String, SerializeError>;
}

/// Transformer trait for delta application
pub trait ArtifactTransformer: Send + Sync {
    type T: ArtifactType;
    fn validate(&self, artifact: &Artifact<Self::T>, delta: &StructuralDelta<Self::T>) 
        -> Result<(), ValidationError>;
    fn apply(&self, artifact: &Artifact<Self::T>, operation: &DeltaOperation<Self::T>) 
        -> Result<Self::T::Content, TransformError>;
}

/// Content-addressed cache using moka
pub struct ArtifactCache {
    inner: Cache<ContentHash, Arc<dyn Any + Send + Sync>>,
    metrics: CacheMetrics,
}

// Parse/Apply/Serialize results
pub struct ParseResult<T: ArtifactType> {
    pub artifact: Artifact<T>,
    pub metadata: SourceMetadata,
}

pub struct ApplyResult<T: ArtifactType> {
    pub new_artifact: Artifact<T>,
    pub applied_deltas: Vec<StructuralDelta<T>>,
}

pub struct SourceMetadata {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub checksum: ContentHash,
}

// ============================================================
// LAYER 3: Core Orchestrator (To Build)
// ============================================================

/// The central orchestrator agent
pub struct CreatorOrchestratorAgent {
    config: COAConfig,
    symbol_index: Arc<SymbolRefIndex>,
    constitutional: Arc<ConstitutionalLayer>,
    agent_pool: AgentPool,
    task_queue: TaskQueue,
    strategy_selector: StrategySelector,
    diagnostics: DiagnosticsCollector,
    kernel: KernelAdapter,
}

/// COA configuration
pub struct COAConfig {
    pub max_concurrent_agents: usize,
    pub default_autonomy: AutonomyLevel,
    pub system_limits: SystemLimits,
    pub auto_apply_fixes: bool,
    pub escalation_threshold: EscalationThreshold,
}

/// User intent (natural language input)
pub struct UserIntent {
    pub description: String,
    pub context: Option<IntentContext>,
}

/// Structured specification (parsed from intent)
pub struct Specification {
    pub goal: Goal,
    pub artifact_type: String,
    pub target_path: SymbolPath,
    pub acceptance_criteria: Vec<String>,
    pub constraints: Vec<Constraint>,
}

/// Executable task
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

/// Agent handle for communication
pub struct AgentHandle {
    pub id: AgentId,
    pub spec: AgentSpec,
    pub sender: mpsc::Sender<AgentMessage>,
}

/// Agent pool for lifecycle management
pub struct AgentPool {
    max_size: usize,
    available: Mutex<Vec<AgentHandle>>,
    active: DashMap<AgentId, AgentHandle>,
    metrics: PoolMetrics,
}

/// Execution graph (validated)
pub struct ExecutionGraph {
    pub validated: ValidatedGraph,
    pub tasks: Vec<Task>,
}

/// Construction failure diagnostic
pub struct ConstructionFailure {
    pub failure_kind: IntegrityViolationKind,
    pub location: GraphLocation,
    pub involved_symbols: Vec<SymbolRef>,
    pub conflict_graph: ConflictGraph,
    pub suggested_fixes: Vec<SuggestedFix>,
    pub human_readable: String,
}

/// Suggested fix for recovery
pub struct SuggestedFix {
    pub description: String,
    pub confidence: f64,
    pub auto_applicable: bool,
    pub resulting_graph_diff: Option<GraphDiff>,
}
```

## 3.2 Enum Definitions

```rust
/// Goal types for specification
pub enum Goal {
    CreateNew,
    ModifyExisting,
    Refactor,
    Analyze,
    Optimize,
}

/// Autonomy levels (embedded in node types)
pub enum AutonomyLevel {
    L0, // Full HITL
    L1, // Approval before merge
    L2, // Auto code, human merge
    L3, // Auto merge in sandbox
    L4, // Auto merge + test deploy
    L5, // Full autonomous
}

/// Artifact output specification
pub enum OutputSpec {
    Code { language: Language },
    Config { schema: String },
    Spec { format: SpecFormat },
    Binary { mime_type: String },
}

/// Types of construction failures
pub enum IntegrityViolationKind {
    OutputIntegrityViolation { claimants: Vec<(TaskNodeId, SymbolRef)> },
    ReferentialIntegrityViolation { unresolved: SymbolRef, available_similar: Vec<SymbolRef> },
    CompositionStrategyViolation { strategy: StrategyType, violation: StrategyViolationDetail },
    CyclicDependency { cycle_path: Vec<NodeId> },
    ResourceBoundViolation { requested: ResourceAmount, declared_limit: ResourceAmount },
}

/// Expansion types for dynamic graph
pub enum ExpansionType {
    Conditional { condition: ExpandsTo<SubgraphSchema> },
    Recursive { max_depth: usize },
    Parallel { branches: Vec<BranchSpec> },
}

/// Agent message types
pub enum AgentMessage {
    Execute(Task),
    Shutdown,
    Pause,
    Resume,
}
```

## 3.3 Type Aliases and Newtypes

```rust
/// Task identifier (ULID for sortability)
pub struct TaskId(pub Ulid);

/// Agent identifier
pub struct AgentId(pub Ulid);

/// Resource capacity specification
pub struct ResourceCaps {
    pub memory_mb: usize,
    pub cpu_millicores: usize,
    pub timeout_secs: u64,
}

/// System limits
pub struct SystemLimits {
    pub max_memory_gb: usize,
    pub max_cpu_cores: usize,
    pub max_agents: usize,
}

/// Escalation threshold configuration
pub struct EscalationThreshold {
    pub max_test_failures: u32,
    pub max_security_violations: u32,
    pub max_autonomy_violations: u32,
}

/// Directive set (behavioral modifiers)
pub type DirectiveSet = HashMap<String, DirectiveValue>;

pub enum DirectiveValue {
    Bool(bool),
    Int(i64),
    String(String),
    List(Vec<DirectiveValue>),
}
```

---

# Part 4: Module Breakdown

## 4.1 coa-constitutional Module Structure

```
crates/coa-constitutional/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Public API exports
    ├── layer.rs                  # ConstitutionalLayer (main entry)
    ├── cache.rs                  # ArtifactCache (moka wrapper)
    ├── error.rs                  # Error types
    ├── types/
    │   ├── mod.rs                # Type exports
    │   ├── code.rs               # CodeArtifact, CodeContent
    │   ├── config.rs             # ConfigArtifact
    │   └── spec.rs               # SpecArtifact
    ├── parsers/
    │   ├── mod.rs                # Parser exports
    │   ├── trait.rs              # ArtifactParser trait
    │   ├── code.rs               # Tree-sitter parsers
    │   ├── json.rs               # JSON parser
    │   ├── yaml.rs               # YAML parser
    │   └── markdown.rs           # Markdown parser
    ├── transformers/
    │   ├── mod.rs                # Transformer exports
    │   ├── trait.rs              # ArtifactTransformer trait
    │   ├── code.rs               # Code transformations
    │   ├── config.rs             # Config transformations
    │   └── spec.rs               # Spec transformations
    └── serializers/
        ├── mod.rs                # Serializer exports
        ├── trait.rs              # ArtifactSerializer trait
        ├── code.rs               # Code serialization
        ├── config.rs             # Config serialization
        └── spec.rs               # Spec serialization
```

## 4.2 coa-core Module Structure

```
crates/coa-core/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Public API exports
    ├── coa.rs                    # CreatorOrchestratorAgent
    ├── config.rs                 # COAConfig
    ├── intent.rs                 # UserIntent, Specification parsing
    ├── decomposition.rs          # TaskDecomposer
    ├── task.rs                   # Task, TaskId
    ├── agent_pool.rs             # AgentPool, AgentHandle
    ├── agent.rs                  # Agent lifecycle, messaging
    ├── graph_builder.rs          # ExecutionGraph builder
    ├── execution.rs              # Graph execution orchestration
    ├── diagnostics.rs            # DiagnosticsCollector
    ├── kernel_adapter.rs         # KernelAdapter
    ├── expansion.rs              # Dynamic graph expansion
    └── policy.rs                 # Policy validation
```

---

# Part 5: Function Contracts and Signatures

## 5.1 coa-constitutional Public API

```rust
// ============================================================
// layer.rs - ConstitutionalLayer
// ============================================================

impl ConstitutionalLayer {
    /// Creates new layer with default parsers/serializers
    pub fn new() -> Self;
    
    /// Parse file into typed artifact (Ingress)
    /// 
    /// # Errors
    /// - `ParseError::NoParserForExtension` if no parser registered
    /// - `ParseError::SyntaxError` if content invalid
    /// - `ParseError::Io` if file read fails
    pub async fn parse_ingress<T: ArtifactType>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ParseResult<T>, ParseError>;
    
    /// Apply single delta to artifact
    ///
    /// # Preconditions
    /// - delta.base_hash must match artifact.hash()
    /// - target must exist (for Replace/Remove/Transform)
    /// - target must not exist (for Add)
    ///
    /// # Errors
    /// - `ApplyError::InvalidBase` if hash mismatch
    /// - `ApplyError::TargetNotFound` if target missing
    /// - `ApplyError::ValidationFailed` if operation invalid
    pub fn apply_delta<T: ArtifactType>(
        &self,
        artifact: &Artifact<T>,
        delta: &StructuralDelta<T>,
    ) -> Result<Artifact<T>, ApplyError>;
    
    /// Apply multiple deltas with composition strategy
    ///
    /// # Preconditions
    /// - All deltas must be validated by strategy
    ///
    /// # Performance
    /// - O(k log n) for validation where k=deltas, n=symbols
    /// - O(k) for application
    pub fn apply_deltas<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
        strategy: &dyn CompositionStrategy,
        index: &SymbolRefIndex,
    ) -> Result<Artifact<T>, ApplyError>;
    
    /// Serialize artifact to file (Egress)
    ///
    /// # Errors
    /// - `SerializeError::NoSerializer` if type not supported
    /// - `SerializeError::Io` if file write fails
    pub async fn serialize_egress<T: ArtifactType>(
        &self,
        artifact: &Artifact<T>,
        path: impl AsRef<Path>,
    ) -> Result<(), SerializeError>;
    
    /// Register custom parser
    pub fn register_parser(
        &mut self,
        extension: impl Into<String>,
        parser: Box<dyn ArtifactParser>,
    );
    
    /// Register custom serializer
    pub fn register_serializer(
        &mut self,
        type_id: impl Into<String>,
        serializer: Box<dyn ArtifactSerializer>,
    );
}

// ============================================================
// cache.rs - ArtifactCache
// ============================================================

impl ArtifactCache {
    /// Create cache with max capacity
    pub fn new(max_capacity: u64) -> Self;
    
    /// Create cache with TTL eviction
    pub fn with_ttl(max_capacity: u64, ttl: Duration) -> Self;
    
    /// Insert artifact (async for moka)
    pub async fn insert<T: ArtifactType>(&self, hash: ContentHash, artifact: Artifact<T>);
    
    /// Get artifact by hash
    pub async fn get<T: ArtifactType>(&self, hash: &ContentHash) -> Option<Artifact<T>>;
    
    /// Get or compute with fallback
    pub async fn get_or_insert_with<T: ArtifactType, F, Fut>(
        &self,
        hash: ContentHash,
        f: F,
    ) -> Artifact<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Artifact<T>>;
    
    /// Invalidate entry
    pub async fn invalidate(&self, hash: &ContentHash);
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats;
}

// ============================================================
// parsers/trait.rs - ArtifactParser
// ============================================================

pub trait ArtifactParser: Send + Sync + 'static {
    /// The artifact type this parser produces
    type Output: ArtifactType;
    
    /// Parse content into artifact
    fn parse(&self, content: &str) -> Result<Artifact<Self::Output>, ParseError>;
    
    /// Check if this parser can handle given path
    fn can_parse(&self, path: &Path) -> bool;
    
    /// Supported file extensions
    fn extensions(&self) -> &[&str];
}

// ============================================================
// transformers/trait.rs - ArtifactTransformer
// ============================================================

pub trait ArtifactTransformer: Send + Sync + 'static {
    type T: ArtifactType;
    
    /// Validate delta can be applied to artifact
    fn validate(
        &self,
        artifact: &Artifact<Self::T>,
        delta: &StructuralDelta<Self::T>,
    ) -> Result<(), ValidationError>;
    
    /// Apply operation to content
    fn apply(
        &self,
        content: &Self::T::Content,
        operation: &DeltaOperation<Self::T>,
    ) -> Result<Self::T::Content, TransformError>;
}
```

## 5.2 coa-core Public API

```rust
// ============================================================
// coa.rs - CreatorOrchestratorAgent
// ============================================================

impl CreatorOrchestratorAgent {
    /// Create new COA instance with configuration
    pub fn new(config: COAConfig) -> Self;
    
    /// Execute high-level user intent (main entry point)
    ///
    /// # Workflow
    /// 1. Parse intent → Specification
    /// 2. Decompose spec → Tasks
    /// 3. Build execution graph
    /// 4. Validate and execute
    /// 5. Handle failures with diagnostics
    ///
    /// # Errors
    /// - `COAError::InvalidIntent` if parsing fails
    /// - `COAError::DecompositionFailed` if tasks cannot be created
    /// - `COAError::ConstructionFailed` if graph validation fails
    /// - `COAError::RequiresHumanIntervention` if auto-fix not possible
    pub async fn execute_intent(
        &self,
        intent: UserIntent,
    ) -> Result<ExecutionResult, COAError>;
    
    /// Parse intent into structured specification
    async fn parse_intent(&self, intent: UserIntent) -> Result<Specification, COAError>;
    
    /// Decompose specification into tasks
    async fn decompose(&self, spec: Specification) -> Result<Vec<Task>, COAError>;
    
    /// Build execution graph from tasks
    async fn build_execution_graph(
        &self,
        tasks: &[Task],
    ) -> Result<ExecutionGraph, COAError>;
    
    /// Execute validated graph through kernel
    async fn execute_graph(
        &self,
        graph: ExecutionGraph,
    ) -> Result<ExecutionResult, COAError>;
    
    /// Handle execution failure with diagnostics
    async fn handle_execution_failure(
        &self,
        error: COAError,
        tasks: &[Task],
    ) -> Result<ExecutionResult, COAError>;
    
    /// Spawn agent for task
    async fn spawn_agent(&self, task: &Task) -> Result<AgentHandle, COAError>;
    
    /// Collect delta from agent
    async fn collect_delta<T: ArtifactType>(
        &self,
        agent: &AgentHandle,
    ) -> Result<StructuralDelta<T>, COAError>;
    
    /// Apply composition strategy to deltas
    fn compose_deltas<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: Vec<StructuralDelta<T>>,
        strategy: &dyn CompositionStrategy,
    ) -> Result<Artifact<T>, COAError>;
}

// ============================================================
// decomposition.rs - TaskDecomposer
// ============================================================

impl TaskDecomposer {
    /// Create new decomposer with strategy selector
    pub fn new(strategy_selector: StrategySelector) -> Self;
    
    /// Decompose specification into executable tasks
    ///
    /// # Algorithm
    /// 1. Analyze spec goal type
    /// 2. Dispatch to specialized decomposer
    /// 3. Identify symbols to implement
    /// 4. Create dependency graph
    /// 5. Assign composition strategies
    ///
    /// # Performance
    /// O(n) where n = symbols identified
    pub async fn decompose(
        &self,
        spec: Specification,
        index: &SymbolRefIndex,
    ) -> Result<Vec<Task>, DecompositionError>;
    
    /// Decompose "create new" goal
    async fn decompose_create(
        &self,
        spec: &Specification,
        index: &SymbolRefIndex,
    ) -> Result<Vec<Task>, DecompositionError>;
    
    /// Decompose "modify existing" goal
    async fn decompose_modify(
        &self,
        spec: &Specification,
        index: &SymbolRefIndex,
    ) -> Result<Vec<Task>, DecompositionError>;
    
    /// Identify symbols from specification
    async fn identify_symbols(
        &self,
        spec: &Specification,
    ) -> Result<Vec<String>, DecompositionError>;
    
    /// Select composition strategy for task
    fn select_strategy(&self, spec: &Specification, task: &Task) -> DirectiveSet;
}

// ============================================================
// agent_pool.rs - AgentPool
// ============================================================

impl AgentPool {
    /// Create new pool with max size
    pub fn new(max_size: usize) -> Self;
    
    /// Acquire agent (reuse or create)
    ///
    /// # Errors
    /// - `PoolError::PoolExhausted` if max agents active
    pub async fn acquire(&self, spec: AgentSpec) -> Result<AgentHandle, PoolError>;
    
    /// Release agent back to pool
    pub async fn release(&self, agent: AgentHandle);
    
    /// Shutdown all agents
    pub async fn shutdown(&self);
    
    /// Get pool statistics
    pub fn stats(&self) -> PoolStats;
}

// ============================================================
// diagnostics.rs - DiagnosticsCollector
// ============================================================

impl DiagnosticsCollector {
    /// Create new diagnostics collector
    pub fn new() -> Self;
    
    /// Analyze error and generate diagnostic with fixes
    ///
    /// # Returns
    /// Diagnostic with structured error info and suggested fixes
    pub async fn analyze(
        &self,
        error: &COAError,
        tasks: &[Task],
    ) -> Diagnostic;
    
    /// Analyze construction failure
    async fn analyze_construction_failure(
        &self,
        error: &ConstructionError,
        tasks: &[Task],
    ) -> Diagnostic;
    
    /// Analyze composition failure
    async fn analyze_composition_failure(
        &self,
        error: &CompositionError,
        tasks: &[Task],
    ) -> Diagnostic;
    
    /// Suggest decomposition for overlapping claims
    fn suggest_decomposition(
        &self,
        validation: &ValidationDiagnostic,
    ) -> Option<GraphDiff>;
}

// ============================================================
// graph_builder.rs - ExecutionGraph
// ============================================================

impl ExecutionGraph {
    /// Create new execution graph from validated kernel graph
    pub fn new(validated: ValidatedGraph, tasks: Vec<Task>) -> Self;
    
    /// Get task by node ID
    pub fn task_for_node(&self, node_id: NodeId) -> Option<&Task>;
    
    /// Get all tasks in dependency order
    pub fn tasks_in_order(&self) -> Vec<&Task>;
    
    /// Get execution statistics
    pub fn stats(&self) -> GraphStats;
}
```

---

# Part 6: Error Types

```rust
// ============================================================
// coa-constitutional errors
// ============================================================

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("no parser for extension: {0}")]
    NoParserForExtension(String),
    
    #[error("syntax error: {0}")]
    SyntaxError(String),
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("invalid type: expected {expected}, got {actual}")]
    InvalidType { expected: String, actual: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("invalid base hash: {0}")]
    InvalidBase(String),
    
    #[error("target not found: {0}")]
    TargetNotFound(SymbolPath),
    
    #[error("target already exists: {0}")]
    TargetAlreadyExists(SymbolPath),
    
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("composition failed: {0}")]
    CompositionFailed(String),
    
    #[error("transformer not found: {0}")]
    NoTransformer(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    #[error("no serializer for type: {0}")]
    NoSerializer(String),
    
    #[error("serialization failed: {0}")]
    SerializationFailed(String),
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ============================================================
// coa-core errors
// ============================================================

#[derive(Debug, thiserror::Error)]
pub enum COAError {
    #[error("invalid intent: {0}")]
    InvalidIntent(String),
    
    #[error("decomposition failed: {0}")]
    DecompositionFailed(String),
    
    #[error("construction failed: {0}")]
    ConstructionFailed(#[from] ConstructionError),
    
    #[error("composition failed: {0}")]
    CompositionFailed(String),
    
    #[error("agent failed: {0}")]
    AgentFailed(String),
    
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("kernel error: {0}")]
    KernelError(String),
    
    #[error("requires human intervention: {error}")]
    RequiresHumanIntervention {
        error: Box<COAError>,
        diagnostic: Diagnostic,
        suggested_fixes: Vec<SuggestedFix>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum DecompositionError {
    #[error("specification invalid: {0}")]
    InvalidSpecification(String),
    
    #[error("symbol identification failed: {0}")]
    SymbolIdentificationFailed(String),
    
    #[error("no strategy for artifact type: {0}")]
    NoStrategy(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("pool exhausted (max: {0})")]
    PoolExhausted(usize),
    
    #[error("agent creation failed: {0}")]
    CreationFailed(String),
}
```

---

# Part 7: Implementation Roadmap

## Phase 1: coa-constitutional Foundation (Week 1)

### Week 1, Days 1-2: Core Types and Cache
- [ ] `ArtifactCache` with moka integration
- [ ] Error types (`ParseError`, `ApplyError`, `SerializeError`)
- [ ] Module structure and public API skeleton

### Week 1, Days 3-4: Parser Framework
- [ ] `ArtifactParser` trait
- [ ] Tree-sitter integration setup
- [ ] `CodeParser` for Rust
- [ ] `JsonParser`, `YamlParser`

### Week 1, Days 5-7: Transformer Framework
- [ ] `ArtifactTransformer` trait
- [ ] `CodeTransformer` with tree-sitter edits
- [ ] Delta validation logic
- [ ] Composition integration

## Phase 2: coa-constitutional Completion (Week 2)

### Week 2, Days 1-2: Serializers
- [ ] `ArtifactSerializer` trait
- [ ] Code serialization (pretty-print AST)
- [ ] Config serializers

### Week 2, Days 3-4: Integration
- [ ] `ConstitutionalLayer` implementation
- [ ] Ingress → Transform → Egress pipeline
- [ ] Cache integration

### Week 2, Days 5-7: Testing
- [ ] Unit tests for parsers
- [ ] Unit tests for transformers
- [ ] Integration tests (roundtrip)
- [ ] Benchmarks

## Phase 3: coa-core Foundation (Week 3)

### Week 3, Days 1-2: Core Types and Config
- [ ] `COAConfig`, `UserIntent`, `Specification`
- [ ] `Task`, `TaskId`, `AgentSpec`
- [ ] Error types

### Week 3, Days 3-4: Intent Parsing
- [ ] LLM integration (placeholder)
- [ ] Specification extraction
- [ ] Validation

### Week 3, Days 5-7: Task Decomposition
- [ ] `TaskDecomposer`
- [ ] Goal-based decomposition
- [ ] Symbol identification

## Phase 4: coa-core Completion (Week 4)

### Week 4, Days 1-2: Agent Pool
- [ ] `AgentPool` implementation
- [ ] `AgentHandle` messaging
- [ ] Agent lifecycle management

### Week 4, Days 3-4: Graph Building
- [ ] `ExecutionGraph` builder
- [ ] Construction-time validation
- [ ] Kernel adapter

### Week 4, Days 5-6: Diagnostics
- [ ] `DiagnosticsCollector`
- [ ] Suggested fix generation
- [ ] Human escalation

### Week 4, Day 7: Integration
- [ ] End-to-end flow
- [ ] Integration tests
- [ ] Documentation

## Phase 5: Polish and Optimization (Week 5)

- [ ] Performance benchmarks
- [ ] Documentation and examples
- [ ] Error message improvements
- [ ] Edge case handling

---

# Part 8: Testing Strategy

## Unit Tests

| Module | Coverage Target | Key Tests |
|--------|-----------------|-----------|
| cache.rs | 100% | hit/miss, TTL eviction, concurrent access |
| parsers/ | 90% | valid parse, syntax error, roundtrip |
| transformers/ | 90% | all operations, validation, error cases |
| layer.rs | 85% | ingress/egress, composition |
| decomposition.rs | 90% | goal dispatch, symbol identification |
| agent_pool.rs | 90% | acquire/release, exhaustion, shutdown |
| diagnostics.rs | 85% | error analysis, fix suggestions |

## Integration Tests

```rust
// tests/roundtrip.rs
#[tokio::test]
async fn code_roundtrip() {
    // file → Artifact → file
}

// tests/composition_flow.rs
#[tokio::test]
async fn multi_agent_composition() {
    // Multiple deltas composed correctly
}

// tests/coa_e2e.rs
#[tokio::test]
async fn coa_executes_intent() {
    // Full flow: intent → tasks → graph → result
}
```

## Benchmarks

```rust
// benches/symbol_lookup.rs
fn symbol_lookup_benchmark(c: &mut Criterion) {
    // 1k, 10k, 100k, 1M symbols
}

// benches/delta_apply.rs
fn delta_apply_benchmark(c: &mut Criterion) {
    // Single delta, batch deltas
}

// benches/graph_construction.rs
fn graph_construction_benchmark(c: &mut Criterion) {
    // 10, 100, 1000 tasks
}
```

---

# Part 9: Dependencies

## coa-constitutional Cargo.toml

```toml
[dependencies]
# Core (workspace members)
coa-artifact = { path = "../coa-artifact" }
coa-symbol = { path = "../coa-symbol" }
coa-composition = { path = "../coa-composition" }

# Async
tokio = { version = "1.35", features = ["fs", "rt", "macros"] }

# Parsing
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-typescript = "0.20"
tree-sitter-python = "0.20"
serde_json = "1"
serde_yaml = "0.9"
pulldown-cmark = "0.9"

# Caching
moka = { version = "0.12", features = ["future"] }

# Hashing
blake3 = "1.5"

# Error handling
thiserror = "1"

# Tracing
tracing = "0.1"

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3"
criterion = "0.5"
```

## coa-core Cargo.toml

```toml
[dependencies]
# Core (workspace members)
coa-artifact = { path = "../coa-artifact" }
coa-symbol = { path = "../coa-symbol" }
coa-composition = { path = "../coa-composition" }
coa-constitutional = { path = "../coa-constitutional" }
cog-kernel = { path = "../../kernel" }

# Async
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# Collections
dashmap = "5.5"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# IDs
ulid = { version = "1", features = ["serde"] }

# Cryptography (tokens)
ed25519-dalek = "2"
rand = "0.8"

# Error handling
thiserror = "1"
anyhow = "1"

# Tracing
tracing = "0.1"
opentelemetry = "0.21"

[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"
criterion = { version = "0.5", features = ["async_tokio"] }
```

---

# Appendix: Glossary

| Term | Definition |
|------|------------|
| **Artifact** | Typed, content-addressed representation of work product |
| **Constitutional Layer** | Trusted boundary for parse/apply/serialize |
| **COA** | Creator-Orchestrator Agent (central intelligence) |
| **Delta** | Semantic transformation operation |
| **Ingress** | File → Artifact parsing |
| **Egress** | Artifact → File serialization |
| **Expansion** | Dynamic graph generation at runtime |
| **Intent** | Natural language user request |
| **Specification** | Structured representation of intent |
| **Strategy** | Composition algorithm (SingleWriter, etc.) |
| **SymbolRef** | Hash-bound symbolic reference |
| **Task** | Executable unit of work |
