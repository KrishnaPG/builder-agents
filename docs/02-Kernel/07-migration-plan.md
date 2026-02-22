# KERNEL MIGRATION PLAN

## v1.x → v2.0: Safe-by-Construction Architecture

**CRITICAL NOTE:** There is no COA implementation yet. The COA Simulator in `test_harness/simulator.rs` is our **only** mechanism for testing kernel behavior and will serve as the COA replacement during development. The simulator **must** be updated to use the new two-phase API.

This document details the gap analysis between the current kernel implementation and the v2.0 specification, along with the migration strategy.

---

# 1. GAP ANALYSIS

## 1.1 Architecture Deviations

### Current State (v1.x)

```
src/
├── lib.rs                 # Exports all modules
├── main.rs                # CLI entry point
├── api.rs                 # All traits (mixed construction + execution)
├── handle.rs              # KernelHandle (does EVERYTHING)
├── types/mod.rs           # Basic types
├── dag/mod.rs             # DAG construction
├── autonomy/mod.rs        # Token issuance + verification
├── compliance/mod.rs      # Runtime compliance checks ❌
├── directives/mod.rs      # Directive compilation
├── state_machine/mod.rs   # State transitions
├── scheduler/mod.rs       # Scheduling
├── resource/mod.rs        # Resource validation ❌
├── logging/mod.rs         # Event logging
├── isolation/mod.rs       # Execution isolation
└── test_harness/          # Testing utilities
```

### Problems

| Issue | Current | Spec Required | Severity |
|-------|---------|---------------|----------|
| **Mixed phases** | `KernelHandle` does construction + execution | Separate `GraphBuilder` and `Executor` | 🔴 Critical |
| **Runtime compliance** | `ComplianceInterface::validate_action()` called at runtime | Construction-time only validation | 🔴 Critical |
| **No proof-carrying** | `Dag` is just a graph | `ValidatedGraph` with `ValidationToken` | 🔴 Critical |
| **No expansion support** | No dynamic expansion types | `ExpansionNode<T>`, `SubgraphSpec<T>`, `StagedConstruction` | 🟡 High |
| **Token validation** | `validate_token()` checks policy | `verify_token_integrity()` - cryptographic only | 🟡 High |
| **Resource validation** | `check_resources()` at runtime | `prove_resource_bounds()` at construction | 🟡 High |
| **Autonomy check** | `AutonomyCeiling::check()` at token issuance | Encoded in `NodeSpec.autonomy_ceiling` | 🟡 High |

---

## 1.2 Module-by-Module Analysis

### `handle.rs` (800 lines) - REQUIRES MAJOR REFACTOR

**Current:** Single `KernelHandle` implements all traits

```rust
pub struct KernelHandle {
    config: KernelConfig,
    graphs: RwLock<HashMap<GraphId, GraphEntry>>,
    nodes: RwLock<HashMap<NodeId, NodeEntry>>,
    event_log: EventLog,
    // ...
}

impl GraphManager for KernelHandle { ... }
impl NodeOperations for KernelHandle { ... }
impl AutonomyManager for KernelHandle { ... }  // Issues tokens
impl StateController for KernelHandle { ... }
impl ExecutionRuntime for KernelHandle { ... }  // Validates tokens at runtime ❌
impl ComplianceInterface for KernelHandle { ... }  // Runtime validation ❌
impl Scheduler for KernelHandle { ... }
```

**Required:** Split into two separate structs

```rust
// Construction Phase
pub struct GraphBuilder {
    dag: Dag,
    node_specs: HashMap<NodeId, NodeSpec>,
    system_limits: SystemLimits,
}

impl GraphBuilder {
    pub fn validate(self) -> Result<ValidatedGraph, ValidationError>;
}

// Execution Phase
pub struct Executor {
    verifying_key: ed25519::VerifyingKey,
}

impl Executor {
    pub async fn run(&self, graph: ValidatedGraph) -> Result<ExecutionSummary, ExecutionError>;
}
```

**Changes needed:**
- Extract all construction logic to `GraphBuilder`
- Extract all execution logic to `Executor`
- Remove runtime `ComplianceInterface` implementation
- `Executor` only accepts `ValidatedGraph` type

---

### `compliance/mod.rs` (85 lines) - REWRITE REQUIRED

**Current:** Runtime compliance checking

```rust
impl ComplianceInterface for Compliance {
    fn validate_action(&self, action: ProposedAction) -> Result<ComplianceReport, ComplianceError>;
    fn query_policy(&self, scope: PolicyScope) -> Result<PolicySnapshot, KernelError>;
    fn check_resources(&self, caps: ResourceCaps) -> Result<ResourceAvailability, KernelError>;
}
```

**Problems:**
1. `validate_action()` - runtime policy check ❌
2. `query_policy()` - runtime policy query ❌
3. `check_resources()` - runtime resource check ❌

**Required:** Construction-time validation only

```rust
pub struct ConstructionValidator;

impl ConstructionValidator {
    pub fn validate_graph(&self, builder: GraphBuilder) -> Result<ValidationReport, ValidationError>;
    pub fn validate_expansion<T: ExpansionSchema>(&self, ...) -> Result<ValidatedExpansion, ValidationError>;
    pub fn prove_resource_bounds(&self, nodes: &[NodeSpec]) -> Result<ResourceProof, ResourceError>;
}

// NO runtime policy query interface
```

---

### `types/mod.rs` (141 lines) - ENHANCEMENTS REQUIRED

**Current:**

```rust
pub struct NodeSpec {
    pub directives: DirectiveSet,
    // Missing: autonomy_ceiling, resource_bounds, expansion_type
}

pub struct AutonomyCeiling {
    pub max_level: AutonomyLevel,
}

impl AutonomyCeiling {
    pub fn check(&self, level: AutonomyLevel) -> bool;  // Runtime check
}
```

**Required:**

```rust
pub struct NodeSpec {
    pub directives: DirectiveSet,
    pub autonomy_ceiling: AutonomyLevel,  // ENCODED, not checked
    pub resource_bounds: ResourceCaps,     // Part of type
    pub expansion_type: Option<ExpansionType>,
}

pub struct ValidatedGraph {
    graph_id: GraphId,
    validation_token: ValidationToken,
    dag: Dag,
    node_tokens: HashMap<NodeId, CapabilityToken>,
    // Sealed type - only constructible via GraphBuilder::validate()
}

pub struct ValidationToken {
    pub graph_id: GraphId,
    pub validation_hash: [u8; 32],
    pub timestamp: u64,
    pub signature: ed25519::Signature,
}

pub struct ExpansionType {
    pub schema_type_id: TypeId,
    pub max_subgraph_resources: ResourceCaps,
    pub max_expansion_depth: u32,
}

pub struct SubgraphSpec<T: ExpansionSchema> {
    pub nodes: Vec<NodeSpec>,
    pub edges: Vec<(NodeId, NodeId)>,
    pub _phantom: PhantomData<T>,
}
```

---

### `autonomy/mod.rs` (117 lines) - MODIFICATIONS REQUIRED

**Current:** Token issuance and verification

```rust
pub struct CapabilityToken {
    pub node_id: NodeId,
    pub autonomy_level: AutonomyLevel,
    pub caps: ResourceCaps,
    pub directive_hash: DirectiveProfileHash,
    pub issued_at: u64,
    pub expires_at: u64,
    pub bound_operation: String,
    pub signature: Signature,
}

impl CapabilityToken {
    pub fn verify(&self, verifying_key: &VerifyingKey) -> bool;
    pub fn is_expired(&self) -> bool;
    pub fn is_bound_to(&self, operation: &str) -> bool;
}
```

**Status:** Token structure is good, but usage pattern changes:

**Current Usage (wrong):**
```rust
// In handle.rs - runtime verification
fn transition(&self, ..., token: &CapabilityToken) -> Result<...> {
    if !token.verify(&self.config.verifying_key) {  // OK - integrity
        return Err(...);
    }
    // ... then check policy ❌
}
```

**Required Usage:**
```rust
// In Executor - integrity verification only
fn execute(&self, ..., token: &CapabilityToken) -> Result<...> {
    if !TokenIntegrity::verify_integrity(token, &self.verifying_key)? {
        return Err(ExecutionError::TokenIntegrityFailure);
    }
    // No policy check - already validated at construction
}
```

**New Module Required:** `token_integrity.rs`

```rust
pub struct TokenIntegrity;

impl TokenIntegrity {
    pub fn verify_integrity(token: &CapabilityToken, key: &VerifyingKey) -> Result<IntegrityVerification, TokenError>;
    pub fn verify_node_binding(token: &CapabilityToken, node_id: NodeId) -> Result<(), TokenError>;
}
```

---

### `dag/mod.rs` (132 lines) - MINOR ENHANCEMENTS

**Current:** Basic DAG with cycle detection

**Status:** Core functionality correct, but needs:

1. Integration with `GraphBuilder` (construction phase only)
2. `freeze()` should produce immutable validated view
3. Remove `validate()` method (now in `ConstructionValidator`)

---

### `resource/mod.rs` (13 lines) - REWRITE REQUIRED

**Current:**

```rust
pub fn validate_caps(caps: &ResourceCaps, limits: &ResourceCaps) -> Result<(), ResourceError>;
```

**Problems:** Called at runtime ❌

**Required:**

```rust
// Construction time: prove bounds are satisfiable
pub fn prove_resource_bounds(
    nodes: &[NodeSpec],
    system_limits: &ResourceCaps,
) -> Result<ResourceProof, ResourceError>;

// Runtime: container primitive enforcement (NOT validation)
pub struct Container {
    cpu_limit_ms: u64,
    memory_limit_bytes: u64,
}

impl Container {
    pub fn enforce_limits(caps: &ResourceCaps) -> Result<(), Error>;
}
```

---

### `state_machine/mod.rs` (39 lines) - MINIMAL CHANGES

**Current:** State transition validation

**Status:** Correct - deterministic state evolution

**Note:** Clarify in documentation that this is **deterministic evolution**, not policy validation.

---

### `scheduler/mod.rs` (45 lines) - MODIFICATIONS REQUIRED

**Current:** Accepts any node

**Required:** Only accepts `ValidatedGraph`

```rust
pub trait Scheduler {
    fn schedule(&self, graph: &ValidatedGraph, node_id: NodeId) -> Result<ScheduleToken, SchedulerError>;
    // ...
}
```

---

### `isolation/mod.rs` (79 lines) - MODIFICATIONS REQUIRED

**Current:** Determines isolation level at runtime

```rust
fn execute(&self, ..., token: &CapabilityToken) -> Result<...> {
    match token.autonomy_level {  // Reading from token
        L0|L1|L2 => thread_isolation(),
        L3|L4|L5 => subprocess_isolation(),
    }
}
```

**Required:** Isolation level comes from `NodeSpec` (construction time)

```rust
fn execute(&self, node_spec: &NodeSpec, ...) -> Result<...> {
    match node_spec.autonomy_ceiling {  // From pre-validated spec
        L0|L1|L2 => thread_isolation(),
        L3|L4|L5 => subprocess_isolation(),
    }
}
```

---

### `logging/mod.rs` (70 lines) - NO CHANGES REQUIRED

**Status:** Immutable hash-chain logging - correct as-is.

---

### `directives/mod.rs` (48 lines) - NO CHANGES REQUIRED

**Status:** Directive compilation at construction time - correct as-is.

---

## 1.3 Missing Modules

| Module | Purpose | Complexity |
|--------|---------|------------|
| `construction/mod.rs` | `ConstructionValidator`, `TokenIssuer` | High |
| `construction/expansion.rs` | `StagedConstruction`, `ExpansionSchema` | High |
| `executor/mod.rs` | `Executor` struct | Medium |
| `token_integrity.rs` | Runtime integrity verification | Low |
| `validated_graph.rs` | `ValidatedGraph` type + sealing | Medium |

---

# 2. MIGRATION STRATEGY

## Phase 1: Foundation (1-2 days)

### 2.1.1 Create New Types

**File:** `src/types/v2.rs` (new types alongside old)

```rust
pub struct NodeSpecV2 {
    pub directives: DirectiveSet,
    pub autonomy_ceiling: AutonomyLevel,
    pub resource_bounds: ResourceCaps,
    pub expansion_type: Option<ExpansionType>,
}

pub struct ValidatedGraph {
    // Sealed - private fields
    pub(crate) graph_id: GraphId,
    pub(crate) validation_token: ValidationToken,
    // ...
}

// ValidationToken, ExpansionType, etc.
```

### 2.1.2 Create Construction Module Skeleton

**File:** `src/construction/mod.rs`

```rust
pub struct GraphBuilder { ... }
pub struct ConstructionValidator { ... }
pub struct TokenIssuer { ... }

impl GraphBuilder {
    pub fn validate(self) -> Result<ValidatedGraph, ValidationError> {
        // Call ConstructionValidator
    }
}
```

### 2.1.3 Create Executor Module Skeleton

**File:** `src/executor/mod.rs`

```rust
pub struct Executor { ... }

impl Executor {
    pub async fn run(&self, graph: ValidatedGraph) -> Result<...> {
        // Only accepts ValidatedGraph
    }
}
```

## Phase 2: Migration (2-3 days)

### 2.2.1 Refactor handle.rs

Split `KernelHandle`:

```rust
// Move to src/construction/builder.rs
pub struct GraphBuilder { ... }

// Move to src/executor/mod.rs
pub struct Executor { ... }

// Keep minimal compatibility layer
pub struct KernelHandle { ... }  // Deprecated, delegates to new types
```

### 2.2.2 Replace Compliance Module

```rust
// Delete: src/compliance/mod.rs
// Create: src/construction/validator.rs

pub struct ConstructionValidator;
impl ConstructionValidator {
    // Move logic from old Compliance, but construction-time only
}
```

### 2.2.3 Add Token Integrity Module

```rust
// Create: src/token_integrity.rs

pub struct TokenIntegrity;
impl TokenIntegrity {
    pub fn verify_integrity(...) -> Result<...>;
}
```

### 2.2.4 Update Resource Module

```rust
// src/resource/mod.rs

// Rename: validate_caps -> prove_resource_bounds
// Add: Container::enforce_limits (runtime enforcement)
```

## Phase 3: Expansion Support (2-3 days)

### 2.3.1 Add Expansion Types

**File:** `src/construction/expansion.rs`

```rust
pub struct ExpansionNode<T> { ... }
pub struct SubgraphSpec<T: ExpansionSchema> { ... }
pub struct StagedConstruction { ... }

pub trait ExpansionSchema {
    fn validate_subgraph(...) -> Result<...>;
}
```

### 2.3.2 Update DAG Module

Add expansion node support to DAG builder.

## Phase 4: Testing & Cleanup (1-2 days)

### 2.4.1 Update Tests

- Replace `KernelHandle` usage with `GraphBuilder` + `Executor`
- Add tests for `ValidatedGraph` sealing
- Add tests for staged construction
- Add tests for token integrity vs policy validation

### 2.4.2 Update Simulator (CRITICAL - COA Replacement)

**Current Issue:** Simulator uses `KernelHandle` directly with runtime compliance checks.

**Required Changes:**

```rust
// Current (v1.x)
pub fn run_simulator(config: SimulatorConfig) -> SimulatorReport {
    let kernel = KernelHandle::new();  // Mixed construction + execution
    // ...
    SimulatedOperation::ValidateAction(action)  // Runtime compliance ❌
}

// Required (v2.0)
pub fn run_simulator(config: SimulatorConfig) -> SimulatorReport {
    // Simulate COA behavior with two-phase architecture
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    // Construction phase: build and validate graph
    // ... add nodes, edges ...
    
    let validated_graph = match builder.validate() {
        Ok(graph) => graph,
        Err(e) => {
            // Track construction failure
            return report_construction_failure(e);
        }
    };
    
    // Execution phase: run pre-validated graph
    let executor = Executor::new(verifying_key);
    let result = executor.run(validated_graph).await;
    
    // Test expansion if configured
    if config.test_expansion {
        test_staged_expansion(&executor, validated_graph).await;
    }
}
```

**New Simulation Scenarios:**

| Scenario | Current | New |
|----------|---------|-----|
| Invalid graph | `ValidateAction` rejects | `GraphBuilder::validate()` fails |
| Valid construction | N/A (implicit) | `ValidatedGraph` produced |
| Execution on valid | `execute()` with token | `Executor::run()` with `ValidatedGraph` |
| Expansion | Not tested | `StagedConstruction` tested |
| Token integrity | `validate_token()` | `TokenIntegrity::verify_integrity()` |

**Simulator Modes:**

1. **ConstructionTest Mode**: Test `GraphBuilder` rejection of invalid graphs
2. **ExecutionTest Mode**: Test `Executor` on pre-validated graphs  
3. **ExpansionTest Mode**: Test `StagedConstruction` with dynamic expansion
4. **FullSimulation Mode**: End-to-end construction → execution → expansion

**New Operations to Test:**

```rust
pub enum SimulatedOperationV2 {
    // Construction phase
    ConstructionStart(GraphType),
    ConstructionAddNode(NodeSpecV2),
    ConstructionAddEdge(NodeId, NodeId),
    ConstructionValidate,  // Should produce ValidatedGraph or fail
    
    // Execution phase
    ExecutionRun(ValidatedGraph),
    ExecutionUntilExpansion,
    
    // Expansion
    ExpansionProvideSubgraph(SubgraphSpec<dyn ExpansionSchema>),
    ExpansionComplete,
    
    // Integrity (runtime)
    VerifyTokenIntegrity(CapabilityToken),
    VerifyLogIntegrity,
}
```

### 2.4.3 Simulator Invariant Updates

**New Invariants to Check:**

```rust
pub enum InvariantCheckV2 {
    // Construction invariants
    AllGraphsValidatedBeforeExecution,
    NoRuntimePolicyValidationCalls,
    ValidatedGraphsAreImmutable,
    
    // Expansion invariants
    ExpansionSubgraphsAreValidated,
    ResourceBoundsPropagatedToExpansions,
    
    // Integrity invariants
    TokenIntegrityVerifiedBeforeUse,
    LogHashChainUnbroken,
}
```

**Simulator Test Cases:**

1. **Construction Rejection**: Invalid graphs fail at `GraphBuilder::validate()`, never reach `Executor`
2. **Zero Runtime Policy**: Verify no `ComplianceInterface` calls during execution phase
3. **Token Integrity**: Verify `TokenIntegrity::verify_integrity()` called before token use
4. **Expansion Validation**: Verify expanded subgraphs pass validation before execution resumes

### 2.4.4 Deprecate Old APIs

Mark old traits as deprecated:

```rust
#[deprecated(since = "0.2.0", note = "Use GraphBuilder instead")]
pub trait GraphManager { ... }

#[deprecated(since = "0.2.0", note = "Use Executor instead")]
pub trait ExecutionRuntime { ... }
```

---

# 3. BREAKING CHANGES

## API Breaking Changes

### Core API Changes

| Old API | New API | Migration |
|---------|---------|-----------|
| `KernelHandle::new()` | `GraphBuilder::new()` + `Executor::new()` | Split usage |
| `GraphManager::create_graph()` | `GraphBuilder::new()` | Direct replacement |
| `NodeOperations::add_node()` | `GraphBuilder::add_node()` | Direct replacement |
| `ComplianceInterface::validate_action()` | **REMOVED** | Move to construction phase |
| `AutonomyManager::issue_token()` | `TokenIssuer::issue_token()` | Internal to construction |
| `ExecutionRuntime::execute()` | `Executor::run()` | Accepts `ValidatedGraph` |
| `Scheduler::schedule(node_id, token)` | `Scheduler::schedule(graph, node_id)` | Pass graph reference |

### Simulator-Specific Changes (CRITICAL)

| Old Simulator | New Simulator | Impact |
|---------------|---------------|--------|
| Uses `KernelHandle` directly | Uses `GraphBuilder` → `Executor` | Architecture testable |
| `SimulatedOperation::ValidateAction` | `SimulatedOperation::ConstructionValidate` | Tests construction rejection |
| Tests runtime compliance | Tests construction validation | Aligns with spec |
| Single-phase simulation | Two-phase simulation (build → run) | Validates architecture |
| No expansion testing | `StagedConstruction` testing | Dynamic graph support |

## Type Breaking Changes

| Old Type | New Type | Notes |
|----------|----------|-------|
| `NodeSpec` | `NodeSpecV2` | Added fields |
| `Dag` | `ValidatedGraph` | Proof-carrying |
| `ComplianceReport` | `ValidationReport` | Construction-time |

---

# 4. ESTIMATED EFFORT

| Phase | Duration | Files Modified | New Files | Notes |
|-------|----------|----------------|-----------|-------|
| 1: Foundation | 2-3 days | 2 | 6 | Types, builder, executor skeleton |
| 2: Migration | 3-4 days | 9 | 3 | Handle split, compliance → validator |
| 3: Expansion | 2-3 days | 3 | 2 | StagedConstruction, expansion types |
| 4: Simulator Update | 2-3 days | 2 | 0 | **Critical: COA replacement** |
| 5: Testing & Cleanup | 1-2 days | 5 | 0 | Full integration test |
| **Total** | **10-15 days** | **21** | **11** | Includes simulator rewrite |

---

# 5. VERIFICATION CHECKLIST

## 5.1 Core Kernel Verification

- [ ] `GraphBuilder::validate()` produces `ValidatedGraph`
- [ ] `Executor` only accepts `ValidatedGraph`
- [ ] No `ComplianceInterface` usage at runtime
- [ ] Token integrity verification (not policy validation) at runtime
- [ ] Resource bounds proven at construction, enforced at runtime
- [ ] Expansion nodes work with staged construction
- [ ] All existing tests pass
- [ ] New expansion tests pass
- [ ] Binary size < 15 MB
- [ ] Stress test 10k nodes < 2s

## 5.2 COA Simulator Verification (Critical)

- [ ] Simulator uses `GraphBuilder` + `Executor` (not `KernelHandle`)
- [ ] Simulator tests construction-phase rejection of invalid graphs
- [ ] Simulator tests execution with `ValidatedGraph`
- [ ] Simulator tests `StagedConstruction` for expansion
- [ ] Simulator verifies zero runtime policy calls
- [ ] Simulator invariants include "no runtime compliance checks"
- [ ] `cargo run -- simulate` passes with new architecture
- [ ] `cargo run -- stress` passes with new architecture

---

# END OF MIGRATION PLAN
