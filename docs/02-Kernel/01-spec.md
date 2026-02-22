# COGNITIVE OS

# Constitutional Kernel Specification v2.0

**Safe-by-Construction with Staged Dynamic Expansion**

Language: Rust  
Goal: Zero Runtime Policy Validation—All Enforcement via Construction-Time Types

---

# 0. PURPOSE

The Kernel is a:

* Self-contained executable (CLI binary)
* Reusable Rust library (crate)
* **Construction-time validation engine** (policy validation occurs here)
* **Zero-policy-check execution runtime** (integrity verification only)
* Constitutionally immutable layer beneath COA

It enforces:

* **Construction Time:** DAG integrity, Autonomy ceilings, Resource bound provability, Policy compliance
* **Runtime:** Cryptographic integrity verification, State machine evolution, Container primitive enforcement

**Critical Distinction:**
- **Policy Validation**: "Is this allowed?" → Construction time only
- **Integrity Verification**: "Has this been tampered with?" → Runtime (cryptographic)
- **Primitive Enforcement**: Container limits, state transitions → Runtime (structural, not decision-based)

---

# 1. ARCHITECTURE OVERVIEW

## 1.1 Two-Phase Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONSTRUCTION PHASE                            │
│  (Policy validation occurs here - all "should this execute?")   │
├─────────────────────────────────────────────────────────────────┤
│  GraphBuilder                                                   │
│    ├── Type System Layer                                        │
│    ├── DAG Builder (cycle-free by construction)                 │
│    ├── Construction Validator                                   │
│    │     ├── Autonomy ceiling encoding                          │
│    │     ├── Resource bound proving                             │
│    │     ├── Security pipeline completeness                     │
│    │     └── Expansion type validation                          │
│    └── Expansion Handler (staged construction)                  │
│                                                                    │
│  Output: ValidatedGraph (proof-carrying)                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    EXECUTION PHASE                               │
│  (Zero policy checks - only integrity verification)             │
├─────────────────────────────────────────────────────────────────┤
│  Executor                                                       │
│    ├── Token Integrity Verification (cryptographic only)        │
│    ├── State Machine Engine (deterministic evolution)           │
│    ├── Isolation Executor (container primitives)                │
│    └── Immutable Event Log                                      │
│                                                                    │
│  No "validate_action" calls. No "check_policy" queries.         │
│  Execution follows pre-validated structure.                      │
└─────────────────────────────────────────────────────────────────┘
```

## 1.2 Module Structure

```
kernel/
 ├── src/
 │    ├── lib.rs
 │    ├── main.rs
 │    ├── types/              # Core type definitions
 │    ├── dag/                # DAG construction with cycle prevention
 │    ├── construction/       # Construction-time validation (NEW)
 │    │     ├── validator.rs  # Policy validation (was: compliance)
 │    │     ├── expansion.rs  # Dynamic expansion types
 │    │     └── token_issuer.rs # Token binding at construction
 │    ├── autonomy/           # Capability tokens
 │    ├── directives/         # Directive compilation
 │    ├── state_machine/      # Deterministic state transitions
 │    ├── scheduler/          # Topological execution
 │    ├── resource/           # Resource bound proving
 │    ├── logging/            # Hash-chain event log
 │    ├── isolation/          # Container execution
 │    ├── executor/           # Runtime execution (NEW)
 │    └── test_harness/
 ├── tests/
 ├── benches/
 └── Cargo.toml
```

---

# 2. TYPE SYSTEM REQUIREMENTS

## 2.1 Core Types

### NodeId
* UUID v4
* Immutable

### AutonomyLevel
Enum 0 to 5.

Encoded in **NodeType**, not checked at runtime.

### ResourceCaps
```rust
pub struct ResourceCaps {
    pub cpu_time_ms: u64,
    pub memory_bytes: u64,
    pub token_limit: u64,
    pub iteration_cap: u64,
}
```

Part of **NodeType** declaration. Bounds proven at construction time.

### GraphType
* `ProductionDAG`: Cycles rejected at edge insertion
* `SandboxGraph`: Cycles allowed

---

## 2.2 Safe-by-Construction Types (NEW)

### ValidatedGraph
A **proof-carrying type** that can only be constructed by passing all validation.

```rust
/// A graph that has passed construction-time validation.
/// The only type accepted by Executor::run().
pub struct ValidatedGraph {
    graph_id: GraphId,
    validation_token: ValidationToken, // Cryptographic proof
    dag: Dag,
    // ...
}

impl ValidatedGraph {
    /// Only callable from construction module
    pub(crate) fn new_unchecked(...) -> Self { ... }
}
```

### NodeSpec (Enhanced)
```rust
pub struct NodeSpec {
    pub directives: DirectiveSet,
    pub autonomy_ceiling: AutonomyLevel,    // ENCODED here, not checked later
    pub resource_bounds: ResourceCaps,       // Part of node type
    pub expansion_type: Option<ExpansionType>, // For dynamic expansion
}
```

### ExpansionNode<T> (NEW)
Typed dynamic expansion support.

```rust
/// An expansion node declares it will produce a subgraph conforming to schema T.
pub struct ExpansionNode<T> {
    pub node_id: NodeId,
    pub max_subgraph_resources: ResourceCaps,
    pub max_expansion_depth: u32,
    _phantom: PhantomData<T>,
}

/// Output of an expansion node - a subgraph specification that must be validated.
pub struct SubgraphSpec<T> {
    pub nodes: Vec<NodeSpec>,
    pub edges: Vec<(NodeId, NodeId)>,
    pub validation_schema: PhantomData<T>,
}

/// Marker trait for valid expansion schemas.
pub trait ExpansionSchema {
    fn validate_subgraph(spec: &SubgraphSpec<Self>) -> Result<(), ValidationError>;
}
```

### ValidationToken (NEW)
Cryptographic proof of construction-time validation.

```rust
pub struct ValidationToken {
    pub graph_id: GraphId,
    pub validation_hash: [u8; 32],  // Hash of all validated constraints
    pub timestamp: u64,
    pub signature: ed25519::Signature,
}
```

---

# 3. CONSTRUCTION PHASE (NEW SECTION)

## 3.1 GraphBuilder

The **only** way to create a `ValidatedGraph`.

```rust
pub struct GraphBuilder {
    dag: Dag,
    node_specs: HashMap<NodeId, NodeSpec>,
    system_limits: SystemLimits,
}

impl GraphBuilder {
    pub fn new(graph_type: GraphType) -> Self;
    
    pub fn add_node(&mut self, spec: NodeSpec) -> NodeId;
    
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), GraphError>;
    // Cycle detection happens HERE - construction time
    
    /// Final validation - produces proof-carrying ValidatedGraph
    pub fn validate(self) -> Result<ValidatedGraph, ValidationError>;
}
```

## 3.2 Construction-Time Validation

The `validate()` method performs ALL policy validation:

1. **DAG Integrity**: Cycle detection (already done at each edge insertion)
2. **Autonomy Ceiling Compliance**: 
   - Each node's `autonomy_ceiling` ≤ system policy ceiling
   - Encoded into node type—no runtime check needed
3. **Resource Bound Proving**:
   - Sum of all `node.resource_bounds` ≤ `system_limits`
   - Expansion nodes: prove max_subgraph_resources ≤ allocated budget
4. **Security Pipeline Completeness**:
   - Verify all mandatory pipeline stages present in graph topology
   - No optional security stages—enforced by structure
5. **Token Pre-binding**:
   - Issue capability tokens bound to each node
   - Tokens embedded in ValidatedGraph

## 3.3 Staged Construction for Dynamic Expansion

Research tasks require dynamic graph expansion. This is supported through **typed staged construction**.

### Expansion Protocol

```rust
pub enum ConstructionStage {
    /// Building the initial graph
    Building,
    /// Executing up to an expansion point
    ExecutingUntilExpansion(NodeId),
    /// Expansion node produced subgraph spec
    ExpansionProduced(SubgraphSpec<dyn ExpansionSchema>),
    /// Validating the expansion
    ValidatingExpansion,
    /// Splicing validated subgraph into graph
    Splicing,
    /// Resuming execution
    Executing,
}

pub struct StagedConstruction {
    validated_base: ValidatedGraph,
    pending_expansion: Option<ExpansionPoint>,
}

impl StagedConstruction {
    /// Execute until an expansion point is reached
    pub async fn execute_until_expansion(&mut self) -> Result<ExpansionPoint, ExecutionError>;
    
    /// Provide the subgraph specification from expansion
    pub fn provide_expansion<T: ExpansionSchema>(
        &mut self,
        spec: SubgraphSpec<T>,
    ) -> Result<(), ValidationError>;
    
    /// Complete validation and resume
    pub fn complete_expansion(self) -> Result<ValidatedGraph, ValidationError>;
}
```

### Expansion Invariants

1. **Type Safety**: `ExpansionNode<T>` can only produce `SubgraphSpec<T>`
2. **Resource Bounded**: Parent graph allocates budget; expansion cannot exceed
3. **Depth Limited**: Recursive expansion limited by `max_expansion_depth`
4. **Validation Required**: Expanded subgraph must pass full construction validation before execution resumes

---

# 4. EXECUTION PHASE (MODIFIED)

## 4.1 Executor

Accepts only `ValidatedGraph`—proof of prior validation.

```rust
pub struct Executor {
    verifying_key: ed25519::VerifyingKey,
}

impl Executor {
    pub fn new(verifying_key: ed25519::VerifyingKey) -> Self;
    
    /// Execute a pre-validated graph
    /// 
    /// # Runtime Checks (NOT policy validation)
    /// - Token integrity verification (cryptographic)
    /// - State machine evolution (deterministic)
    /// - Container primitive enforcement
    pub async fn run(&self, graph: ValidatedGraph) -> Result<ExecutionSummary, ExecutionError>;
}
```

## 4.2 Token Integrity Verification (NOT Validation)

At construction time: Token issued and bound to node  
At runtime: Verify token has not been tampered with

```rust
/// Verify token integrity - NOT "is this allowed?"
/// This is cryptographic integrity verification.
pub fn verify_token_integrity(token: &CapabilityToken, key: &VerifyingKey) -> bool {
    token.verify(key) && !token.is_expired()
}
```

**Critical distinction**: We're not asking "should this node have L3 autonomy?"—that was decided at construction. We're asking "is this token authentic?"

## 4.3 State Machine Engine

Deterministic evolution based on pre-defined contract.

```rust
/// State transition - enforced by structure, not decision
pub fn transition(from: NodeState, to: NodeState) -> Result<NodeState, StateError> {
    match (from, to) {
        (Created, Isolated) => Ok(to),
        (Isolated, Testing) => Ok(to),
        (Testing, Executing) => Ok(to),
        (Executing, Validating) => Ok(to),
        (Validating, Merged) => Ok(to),
        // ... defined transitions only
        _ => Err(StateError::IllegalTransition),
    }
}
```

No policy decision—just deterministic state evolution.

## 4.4 Resource Enforcement (NOT Validation)

At construction time: Resource bounds proven and encoded  
At runtime: Container primitives enforce declared bounds

```rust
pub struct Container {
    cpu_limit_ms: u64,      // From NodeSpec.resource_bounds
    memory_limit_bytes: u64,
}

impl Container {
    pub fn spawn(&self, work: WorkSpec) -> Result<Child, Error> {
        // Use cgroups/ulimit to enforce declared bounds
        // NOT checking "should this run?" - enforcing declared limits
    }
}
```

---

# 5. AUTONOMY ENGINE (MODIFIED)

## 5.1 Token Structure

```rust
pub struct CapabilityToken {
    pub node_id: NodeId,
    pub autonomy_level: AutonomyLevel,  // ENCODED at construction
    pub caps: ResourceCaps,             // ENCODED at construction
    pub directive_hash: DirectiveProfileHash,
    pub issued_at: u64,
    pub expires_at: u64,
    pub signature: ed25519::Signature,
}
```

## 5.2 Token Lifecycle

**Construction Time:**
```rust
// In GraphBuilder::validate()
let token = CapabilityToken::sign(
    node_id,
    spec.autonomy_ceiling,  // From NodeSpec - already validated
    spec.resource_bounds,   // From NodeSpec - already proven
    directive_hash,
    &signing_key,
);
```

**Runtime:**
```rust
// In Executor - integrity verification only
if !verify_token_integrity(token, &self.verifying_key) {
    return Err(ExecutionError::TokenIntegrityFailure);
}
// No "check if autonomy allowed" - that was construction time
```

---

# 6. RESOURCE GOVERNANCE (MODIFIED)

## 6.1 Construction-Time: Resource Bound Proving

```rust
pub fn prove_resource_bounds(
    nodes: &[NodeSpec],
    system_limits: &ResourceCaps,
) -> Result<ResourceProof, ResourceError> {
    let total: ResourceCaps = nodes.iter()
        .map(|n| n.resource_bounds)
        .sum();
    
    if total.cpu_time_ms > system_limits.cpu_time_ms
        || total.memory_bytes > system_limits.memory_bytes
        // ...
    {
        return Err(ResourceError::BoundsNotProvable);
    }
    
    Ok(ResourceProof { total_bounds: total })
}
```

## 6.2 Runtime: Container Primitive Enforcement

NOT validation—just enforcement of declared bounds:

```rust
pub fn enforce_container_limits(caps: &ResourceCaps) -> Result<(), Error> {
    // Set cgroup cpu.max
    // Set cgroup memory.max
    // Setrlimit for process
    // These ENFORCE the pre-declared bounds
}
```

---

# 7. DIRECTIVE COMPILER

Unchanged—compiles DirectiveSet to ExecutionProfile at construction time.

---

# 8. STATE MACHINE ENGINE

Unchanged—but clarify: deterministic evolution, not policy validation.

---

# 9. ISOLATION EXECUTOR

Two modes based on **pre-encoded** autonomy level:

* L0-L2: Thread isolation
* L3-L5: Subprocess isolation with cleared environment

Isolation level determined by NodeSpec.autonomy_ceiling (construction time), not runtime check.

---

# 10. IMMUTABLE LOGGING

Unchanged—hash-chain integrity verification.

---

# 11. COMPLIANCE MODULE → CONSTRUCTION VALIDATOR (RENAMED & REFOCUSED)

## OLD (Runtime Compliance):
```rust
pub trait ComplianceInterface {
    fn validate_action(&self, action: ProposedAction) -> Result<ComplianceReport, ComplianceError>;
    fn query_policy(&self, scope: PolicyScope) -> Result<PolicySnapshot, KernelError>;
    fn check_resources(&self, caps: ResourceCaps) -> Result<ResourceAvailability, KernelError>;
}
```

## NEW (Construction-Time Validation):
```rust
/// All policy validation happens here, at construction time only.
pub struct ConstructionValidator;

impl ConstructionValidator {
    /// Validate complete graph construction
    pub fn validate_graph(
        &self,
        builder: GraphBuilder,
    ) -> Result<ValidatedGraph, ValidationError>;
    
    /// Validate staged expansion
    pub fn validate_expansion<T: ExpansionSchema>(
        &self,
        parent: &ValidatedGraph,
        expansion: &SubgraphSpec<T>,
    ) -> Result<ValidatedExpansion, ValidationError>;
}

/// NO runtime policy query interface. 
/// Once ValidatedGraph exists, all policy decisions are made.
```

---

# 12. SCHEDULER

Unchanged—but operates on `ValidatedGraph` only.

---

# 13. DYNAMIC EXPANSION EXAMPLE

Scientific hypothesis exploration with safe-by-construction:

```rust
// Stage 1: Construct base graph with expansion stub
let mut builder = GraphBuilder::new(GraphType::ProductionDAG);

let benchmark = builder.add_node(NodeSpec {
    autonomy_ceiling: AutonomyLevel::L3,
    resource_bounds: ResourceCaps { cpu_time_ms: 60000, ... },
    expansion_type: None,
    ...
});

// Expansion node declares it will produce ArchitectureBranch schema
let analyzer: NodeId = builder.add_node(NodeSpec {
    autonomy_ceiling: AutonomyLevel::L4,
    resource_bounds: ResourceCaps { cpu_time_ms: 300000, ... },
    expansion_type: Some(ExpansionType::new::<ArchitectureBranch>(
        ResourceCaps { cpu_time_ms: 1800000, ... }, // Max for ALL branches
        2, // max depth
    )),
    ...
});

builder.add_edge(benchmark, analyzer)?;

// Validate base graph
let base_graph: ValidatedGraph = builder.validate()?;

// Stage 2: Execute until expansion point
let mut staged = StagedConstruction::new(base_graph);
let expansion_point = staged.execute_until_expansion().await?;

// Stage 3: Agent produces subgraph specification
let subgraph_spec: SubgraphSpec<ArchitectureBranch> = expansion_point.generate_spec(|output| {
    // Based on benchmark failure, generate alternative architectures
    SubgraphSpec {
        nodes: vec![
            NodeSpec { /* Architecture A */ },
            NodeSpec { /* Architecture B */ },
            NodeSpec { /* Comparative Testing */ },
        ],
        edges: vec![...],
        ...
    }
});

// Stage 4: Validate expansion (construction time!)
staged.provide_expansion(subgraph_spec)?;
let expanded_graph: ValidatedGraph = staged.complete_expansion()?;

// Stage 5: Resume execution (zero runtime policy checks)
let result = executor.run(expanded_graph).await?;
```

---

# 14. SELF-SUSTAINED EXECUTABLE MODE

Binary supports:

```
cog-kernel simulate
og-kernel stress
cog-kernel validate-log
cog-kernel report
```

---

# 15-20. TESTING, STRESS, CERTIFICATION

Unchanged structure—but tests verify:
- **Construction phase**: Invalid graphs rejected, autonomy bounds enforced
- **Execution phase**: Token integrity verified, state machine deterministic
- **Expansion**: Staged construction validates before resumption

---

# 21. SAFE BY CONSTRUCTION GUARANTEE (UPDATED)

The Kernel guarantees:

**Construction Time:**
* Illegal graph cannot be constructed (cycle rejection at insertion)
* Autonomy ceiling encoded in node type (no runtime ceiling check)
* Resource bounds proven before execution (no runtime "should this run?")
* Invalid state transitions impossible at type level
* Security pipeline complete by graph topology

**Runtime:**
* Token integrity cryptographically verifiable (not policy validation)
* State machine evolution deterministic (not policy decision)
* Resource limits enforced by container primitives (enforcing declared bounds)
* Log tampering detectable

**Dynamic Expansion:**
* Expansion output typed at construction
* Expanded subgraph validated before execution resumes
* Recursive expansion depth bounded

COA operates within type constraints enforced at construction.

---

# 22-26. CROSS-REFERENCES

* API details: [02-api.md](./02-api.md)
* Simulator: [03-simulator.md](./03-simulator.md)
* Test spec: [04-test-spec.md](./04-test-spec.md)
* Certification: [05-certify.md](./05-certify.md)
* Impl notes: [06-impl-notes.md](./06-impl-notes.md)

# END OF SPECIFICATION
