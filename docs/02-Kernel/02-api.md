# KERNEL PUBLIC API (COA INTERFACE)

This document defines the stable public API surface that COA (and the COA Simulator) will use. The Kernel exposes its functionality through traits to enable:

1. **Testability**: COA Simulator implements the caller side
2. **Versioning**: API evolution without breaking changes
3. **Mocking**: Test doubles for unit testing
4. **Type Safety**: Enforcement at compile time

**Critical Principle**: The API is split into **Construction Phase** (policy validation) and **Execution Phase** (integrity verification only).

---

## 22.1 Two-Phase API Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  CONSTRUCTION PHASE API                                          │
│  (Policy validation - "Is this allowed?")                       │
├─────────────────────────────────────────────────────────────────┤
│  GraphBuilder        - Build and validate graphs                │
│  ConstructionValidator - All policy validation logic            │
│  TokenIssuer         - Bind tokens to nodes at construction     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Produces: ValidatedGraph
┌─────────────────────────────────────────────────────────────────┐
│  EXECUTION PHASE API                                             │
│  (Integrity verification - "Is this authentic?")                │
├─────────────────────────────────────────────────────────────────┤
│  Executor            - Run pre-validated graphs                 │
│  TokenIntegrity      - Cryptographic verification               │
│  StateController     - Deterministic state evolution            │
└─────────────────────────────────────────────────────────────────┘
```

---

## 22.2 Construction Phase API

### GraphBuilder

The **only** way to create graphs. All policy validation happens here.

```rust
/// Builder for constructing validated execution graphs.
/// 
/// All policy decisions (autonomy, resources, security pipeline)
/// are made during build().validate() → produces ValidatedGraph.
pub struct GraphBuilder {
    // Opaque implementation
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new(graph_type: GraphType) -> Self;
    
    /// Add a node with full specification
    /// 
    /// NodeSpec includes autonomy_ceiling and resource_bounds which are
    /// ENCODED into the node type - no runtime checks needed.
    pub fn add_node(&mut self, spec: NodeSpec) -> NodeId;
    
    /// Add a dependency edge
    /// 
    /// # Construction-Time Enforcement
    /// - Self-loops rejected immediately
    /// - Cycles rejected at insertion for ProductionDAG
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), GraphError>;
    
    /// Add an expansion node (for dynamic graph growth)
    /// 
    /// The expansion node declares its output schema and resource budget
    /// for all possible expansions.
    pub fn add_expansion_node<T: ExpansionSchema>(
        &mut self,
        spec: NodeSpec,
        max_subgraph_resources: ResourceCaps,
        max_depth: u32,
    ) -> NodeId;
    
    /// Final validation - produces proof-carrying ValidatedGraph
    /// 
    /// This performs ALL policy validation:
    /// - Autonomy ceiling compliance
    /// - Resource bound proving
    /// - Security pipeline completeness
    /// - Token pre-binding
    pub fn validate(self) -> Result<ValidatedGraph, ValidationError>;
}

/// Enhanced NodeSpec with safe-by-construction fields
pub struct NodeSpec {
    pub directives: DirectiveSet,
    /// Autonomy ceiling ENCODED in node type
    /// No runtime check - this IS the ceiling
    pub autonomy_ceiling: AutonomyLevel,
    /// Resource bounds part of node type
    /// Proven at construction, enforced by container primitives
    pub resource_bounds: ResourceCaps,
    /// Optional expansion capability
    pub expansion_type: Option<ExpansionType>,
}
```

### ConstructionValidator

All policy validation logic concentrated here. **No runtime equivalent.**

```rust
/// Validates graphs at construction time ONLY.
/// There is no runtime "compliance check" - once ValidatedGraph exists,
/// all policy decisions have been made.
pub struct ConstructionValidator;

impl ConstructionValidator {
    /// Validate a complete graph
    pub fn validate_graph(
        &self,
        builder: GraphBuilder,
    ) -> Result<ValidationReport, ValidationError>;
    
    /// Validate a staged expansion
    /// 
    /// Called when an expansion node produces a subgraph specification.
    /// The expansion must pass full validation before being spliced.
    pub fn validate_expansion<T: ExpansionSchema>(
        &self,
        parent: &ValidatedGraph,
        expansion_point: NodeId,
        subgraph: &SubgraphSpec<T>,
    ) -> Result<ValidatedExpansion, ValidationError>;
    
    /// Verify resource bounds are provable
    /// 
    /// Sum of all node bounds must be ≤ system limits
    pub fn prove_resource_bounds(
        &self,
        nodes: &[NodeSpec],
    ) -> Result<ResourceProof, ResourceError>;
}

/// Proof that resource bounds have been validated
pub struct ResourceProof {
    pub total_bounds: ResourceCaps,
    pub system_limits: ResourceCaps,
}

/// Report from construction validation
pub struct ValidationReport {
    pub graph_id: GraphId,
    pub validation_token: ValidationToken,
    pub autonomy_compliance: Vec<(NodeId, AutonomyLevel)>,
    pub resource_proof: ResourceProof,
    pub security_pipeline_complete: bool,
}
```

### TokenIssuer

Issues tokens at construction time, bound to validated nodes.

```rust
/// Issues capability tokens during graph construction.
/// Tokens are embedded in ValidatedGraph - no issuance at runtime.
pub struct TokenIssuer {
    signing_key: ed25519::SigningKey,
}

impl TokenIssuer {
    /// Issue a token for a validated node
    /// 
    /// Called during GraphBuilder::validate()
    pub fn issue_token(
        &self,
        node_id: NodeId,
        spec: &NodeSpec,
        directive_hash: DirectiveProfileHash,
    ) -> CapabilityToken;
}
```

---

## 22.3 Execution Phase API

### Executor

Runs pre-validated graphs with **zero policy checks**.

```rust
/// Executes pre-validated graphs.
/// 
/// # Invariants
/// - Only accepts ValidatedGraph (proof of prior validation)
/// - Performs cryptographic integrity verification, NOT policy validation
/// - State machine evolution is deterministic
/// - Resource enforcement via container primitives
pub struct Executor {
    verifying_key: ed25519::VerifyingKey,
}

impl Executor {
    /// Create executor with the public key for token integrity verification
    pub fn new(verifying_key: ed25519::VerifyingKey) -> Self;
    
    /// Execute a validated graph to completion
    /// 
    /// # Runtime Checks (Integrity, NOT Policy)
    /// - Token signature verification (cryptographic)
    /// - Token expiration check (temporal integrity)
    /// - State transition determinism
    /// - Container resource enforcement
    pub async fn run(&self, graph: ValidatedGraph) -> Result<ExecutionSummary, ExecutionError>;
    
    /// Execute until an expansion point
    pub async fn run_until_expansion(
        &self,
        graph: ValidatedGraph,
    ) -> Result<ExpansionPoint, ExecutionError>;
}

/// Proof-carrying validated graph
/// 
/// Can ONLY be constructed by GraphBuilder::validate()
/// Contains embedded capability tokens pre-bound to nodes
pub struct ValidatedGraph {
    graph_id: GraphId,
    validation_token: ValidationToken,
    dag: Dag,
    node_tokens: HashMap<NodeId, CapabilityToken>,
    // Sealed - no public construction
}

/// Cryptographic proof of construction-time validation
pub struct ValidationToken {
    pub graph_id: GraphId,
    pub validation_hash: [u8; 32],
    pub timestamp: u64,
    pub signature: ed25519::Signature,
}
```

### TokenIntegrity

Cryptographic verification only—**NOT** "is this allowed?"

```rust
/// Token integrity verification (runtime)
/// 
/// These functions verify CRYPTOGRAPHIC integrity, not policy compliance.
/// Policy was validated at construction time.
pub struct TokenIntegrity;

impl TokenIntegrity {
    /// Verify token signature and expiration
    /// 
    /// # Distinction
    /// - This asks: "Has this token been tampered with?" (integrity)
    /// - NOT: "Should this node have this autonomy level?" (policy - construction time)
    pub fn verify_integrity(
        token: &CapabilityToken,
        verifying_key: &ed25519::VerifyingKey,
    ) -> Result<IntegrityVerification, TokenError>;
    
    /// Verify token is bound to specific node
    pub fn verify_node_binding(
        token: &CapabilityToken,
        node_id: NodeId,
    ) -> Result<(), TokenError>;
}

pub struct IntegrityVerification {
    pub signature_valid: bool,
    pub not_expired: bool,
    pub bound_to_node: NodeId,
}
```

### StateController

Deterministic state evolution—not policy decisions.

```rust
/// Controls node state transitions.
/// 
/// Transitions are DETERMINISTIC based on pre-defined contract.
/// No policy decisions at runtime.
pub trait StateController {
    /// Perform state transition
    /// 
    /// # Safety
    /// Transition validity is enforced by the state machine matrix.
    /// This is deterministic evolution, not policy validation.
    fn transition(
        &self,
        node_id: NodeId,
        to: NodeState,
        token: &CapabilityToken, // Proves node was pre-validated
    ) -> Result<TransitionReceipt, StateError>;
    
    fn current_state(&self, node_id: NodeId) -> Result<NodeState, KernelError>;
    fn allowed_transitions(&self, node_id: NodeId) -> Result<Vec<NodeState>, KernelError>;
}
```

### IsolationExecutor

Container primitive enforcement—not policy validation.

```rust
/// Executes work in isolated contexts.
/// 
/// Isolation level determined by NodeSpec.autonomy_ceiling (construction time).
/// Runtime only ENFORCES the pre-declared isolation, not decides it.
pub trait IsolationExecutor {
    /// Execute work with container primitive enforcement
    /// 
    /// Resource limits come from NodeSpec.resource_bounds (construction time).
    /// Runtime enforces via cgroups/setrlimit, not validates.
    fn execute(
        &self,
        node_id: NodeId,
        token: &CapabilityToken,
        work: WorkSpec,
    ) -> Result<ExecutionResult, ExecutionError>;
}
```

---

## 22.4 Dynamic Expansion API

### StagedConstruction

For research workflows requiring dynamic graph growth.

```rust
/// Handles typed staged construction for dynamic expansion.
/// 
/// Each expansion requires full re-validation before execution resumes.
pub struct StagedConstruction {
    validated_base: ValidatedGraph,
    stage: ConstructionStage,
}

impl StagedConstruction {
    /// Create from validated base graph
    pub fn new(base: ValidatedGraph) -> Self;
    
    /// Execute until reaching an expansion node
    pub async fn execute_until_expansion(&mut self) -> Result<ExpansionPoint, ExecutionError>;
    
    /// Provide subgraph specification from expansion
    /// 
    /// # Validation
    /// The subgraph must pass full construction validation.
    /// This is NOT optional - execution cannot resume without it.
    pub fn provide_expansion<T: ExpansionSchema>(
        &mut self,
        spec: SubgraphSpec<T>,
    ) -> Result<PendingExpansion, ValidationError>;
    
    /// Complete validation and return expanded graph
    pub fn complete_expansion(self) -> Result<ValidatedGraph, ValidationError>;
}

/// Point in execution where expansion is required
pub struct ExpansionPoint {
    pub node_id: NodeId,
    pub expansion_type_id: TypeId,
    pub parent_resources_remaining: ResourceCaps,
}

/// Typed subgraph specification
pub struct SubgraphSpec<T: ExpansionSchema> {
    pub nodes: Vec<NodeSpec>,
    pub edges: Vec<(NodeId, NodeId)>,
    pub _phantom: PhantomData<T>,
}

/// Marker trait for valid expansion schemas
pub trait ExpansionSchema {
    /// Validate a subgraph conforms to this schema
    fn validate_subgraph(spec: &SubgraphSpec<Self>) -> Result<(), SchemaError>;
    
    /// Get resource requirements for this schema
    fn resource_requirements() -> ResourceSchema;
}

/// Expansion type declared in NodeSpec
pub struct ExpansionType {
    pub schema_type_id: TypeId,
    pub max_subgraph_resources: ResourceCaps,
    pub max_expansion_depth: u32,
}

impl ExpansionType {
    pub fn new<T: ExpansionSchema>(
        max_resources: ResourceCaps,
        max_depth: u32,
    ) -> Self;
}
```

---

## 22.5 Event Logging API

Unchanged—immutable append-only logging.

```rust
pub trait EventLogger {
    fn log_event(&self, event: Event) -> Result<EventId, LogError>;
    fn query_events(&self, filter: EventFilter, limit: usize) -> Result<Vec<LogEntry>, KernelError>;
    fn verify_integrity(&self) -> Result<IntegrityReport, KernelError>;
}
```

---

## 22.6 Error Types

### Construction Phase Errors

```rust
/// Errors during graph construction (all recoverable)
#[derive(Debug, thiserror::Error)]
pub enum ConstructionError {
    #[error("Cycle detected in ProductionDAG")]
    CycleDetected,
    
    #[error("Autonomy ceiling exceeded: node wants {requested:?}, max is {ceiling:?}")]
    AutonomyCeilingExceeded { requested: AutonomyLevel, ceiling: AutonomyLevel },
    
    #[error("Resource bounds not provable: {0}")]
    ResourceBoundsNotProvable(ResourceError),
    
    #[error("Security pipeline incomplete: missing {0}")]
    SecurityPipelineIncomplete(String),
    
    #[error("Invalid expansion type: {0}")]
    InvalidExpansionType(String),
    
    #[error("Expansion exceeds resource budget")]
    ExpansionBudgetExceeded,
}
```

### Execution Phase Errors

```rust
/// Errors during execution (integrity failures, not policy violations)
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Token integrity failure: {0}")]
    TokenIntegrityFailure(TokenError),  // Cryptographic, not policy
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Illegal state transition: {from:?} -> {to:?}")]
    IllegalStateTransition { from: NodeState, to: NodeState },
    
    #[error("Container isolation failure: {0}")]
    IsolationFailure(String),
    
    #[error("Resource enforcement trigger: {cap} limit reached")]
    ResourceEnforcementTriggered { cap: String }, // Enforcing declared bounds
    
    #[error("Execution timeout")]
    Timeout,
}
```

---

## 22.7 API Versioning

```rust
pub const KERNEL_API_VERSION: ApiVersion = ApiVersion {
    major: 2,  // Breaking change: Two-phase architecture
    minor: 0,
    patch: 0,
};

pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

pub enum Compatibility {
    Compatible,
    Deprecated,
    BreakingChanges(Vec<String>),
    Incompatible(Vec<String>),
}
```

---

## 22.8 Usage Example

```rust
use cog_kernel::{GraphBuilder, Executor, AutonomyLevel, ResourceCaps};

// ===== CONSTRUCTION PHASE =====
// All policy validation happens here

let mut builder = GraphBuilder::new(GraphType::ProductionDAG);

let node_a = builder.add_node(NodeSpec {
    directives: directives,
    autonomy_ceiling: AutonomyLevel::L3,  // ENCODED in type
    resource_bounds: ResourceCaps {       // Part of type
        cpu_time_ms: 60000,
        memory_bytes: 1024 * 1024 * 1024,
        token_limit: 100000,
        iteration_cap: 1000,
    },
    expansion_type: None,
});

// Edge insertion checks for cycles HERE - construction time
builder.add_edge(node_a, node_b)?;

// Final validation - produces proof-carrying ValidatedGraph
let validated: ValidatedGraph = builder.validate()?;

// ===== EXECUTION PHASE =====
// Zero policy checks - only integrity verification

let executor = Executor::new(verifying_key);
let result = executor.run(validated).await?;

// ===== DYNAMIC EXPANSION =====
let mut staged = StagedConstruction::new(validated);
let expansion = staged.execute_until_expansion().await?;

let subgraph: SubgraphSpec<MySchema> = generate_subgraph(expansion);
staged.provide_expansion(subgraph)?;  // Validates before resuming

let expanded: ValidatedGraph = staged.complete_expansion()?;
let final_result = executor.run(expanded).await?;
```

---

# END OF API SPECIFICATION
