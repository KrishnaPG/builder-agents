# COA SIMULATOR SPECIFICATION

The COA Simulator is a **test harness** that exercises the Kernel's public API to verify all invariants hold. It does not simulate COA intelligence—only the **input patterns** COA would generate.

## 23.1 Simulator Architecture

```rust
/// COA Simulator configuration
pub struct SimulatorConfig {
		/// Random seed for reproducibility
		pub seed: u64,
		
		/// Total operations to execute
		pub total_operations: u64,
		
		/// Distribution of operation types
		pub operation_distribution: OperationDistribution,
		
		/// Graph configuration
		pub max_concurrent_graphs: usize,
		pub max_nodes_per_graph: usize,
		
		/// Stop conditions
		pub stop_on_first_violation: bool,
		pub stop_on_error_count: Option<usize>,
}

/// Probability distribution for operation generation
pub struct OperationDistribution {
		/// Valid operations (normal COA behavior)
		pub valid_ops: f64,           // e.g., 0.70
		
		/// Edge cases (boundary values)
		pub edge_cases: f64,          // e.g., 0.20
		
		/// Invalid operations (should be rejected)
		pub invalid_ops: f64,         // e.g., 0.10
}

impl Default for OperationDistribution {
		fn default() -> Self {
				Self {
						valid_ops: 0.70,
						edge_cases: 0.20,
						invalid_ops: 0.10,
				}
		}
}
```

## 23.2 Operation Generators

The simulator generates operations using **property-based testing** (proptest) principles:

```rust
/// All possible operations the simulator can generate
pub enum SimulatedOperation {
		// Graph operations
		CreateGraph(GraphType),
		CloseGraph(GraphId),
		
		// Node operations  
		AddNode(GraphId, NodeSpec),
		AddEdge(GraphId, NodeId, NodeId),
		DeactivateNode(NodeId),
		FreezeNode(NodeId),
		
		// Token operations
		IssueToken(NodeId, AutonomyLevel, ResourceCaps),
		DowngradeToken(CapabilityToken, AutonomyLevel),
		ValidateToken(CapabilityToken),
		
		// State operations
		TransitionState(NodeId, NodeState, CapabilityToken),
		QueryState(NodeId),
		
		// Execution operations
		ExecuteWork(NodeId, CapabilityToken, WorkSpec),
		ScheduleExecution(NodeId, CapabilityToken),
		
		// Compliance operations
		ValidateAction(ProposedAction),
		QueryPolicy(PolicyScope),
		
		// Log operations
		LogEvent(Event),
		QueryLog(EventFilter),
		VerifyIntegrity,
}

/// Strategy for generating valid operations
pub fn valid_operation_strategy() -> impl Strategy<Value = SimulatedOperation>;

/// Strategy for generating edge cases
pub fn edge_case_strategy() -> impl Strategy<Value = SimulatedOperation>;

/// Strategy for generating invalid operations (should fail)
pub fn invalid_operation_strategy() -> impl Strategy<Value = SimulatedOperation>;
```

## 23.3 Invalid Operation Categories

The simulator MUST generate these specific invalid operation types:

| Category            | Invalid Operation                   | Expected Kernel Response                                           |
| ------------------- | ----------------------------------- | ------------------------------------------------------------------ |
| **Graph Integrity** | Create cycle in ProductionDAG       | `GraphError::CycleDetected`                                        |
|                     | Add edge to non-existent node       | `GraphError::NodeNotFound`                                         |
|                     | Add self-loop                       | `GraphError::SelfLoop`                                             |
| **Autonomy**        | Request level > policy ceiling      | `AutonomyError::CeilingExceeded`                                   |
|                     | Use expired token                   | `AutonomyError::TokenExpired`                                      |
|                     | Use token for wrong node            | `AutonomyError::TokenMismatch`                                     |
|                     | Attempt token elevation             | `AutonomyError::ElevationForbidden`                                |
|                     | Tampered token signature            | `AutonomyError::InvalidSignature`                                  |
| **State Machine**   | Illegal transition (Created→Merged) | `StateMachineError::IllegalTransition`                             |
|                     | Transition without valid token      | `AutonomyError::TokenRequired`                                     |
|                     | Concurrent transition attempts      | First succeeds, rest get `StateMachineError::TransitionInProgress` |
| **Resource**        | Request caps > limits               | `ResourceError::LimitExceeded`                                     |
|                     | Execute beyond token caps           | `ResourceError::CapExceeded` + node freeze                         |
| **Compliance**      | Bypass validation before execution  | `ComplianceViolation::ValidationRequired`                          |
|                     | Violate policy constraint           | `ComplianceViolation::PolicyViolation`                             |
| **Log**             | Attempt to modify existing entry    | `LogError::Immutable`                                              |
|                     | Break hash chain                    | `LogError::IntegrityViolation`                                     |

## 23.4 Invariant Assertions

After every operation, the simulator asserts:

```rust
/// Invariants that must always hold
pub struct KernelInvariants;

impl KernelInvariants {
		/// Graph invariants
		pub fn check_graph_invariants(kernel: &KernelHandle) -> Result<(), InvariantViolation>;
		
		/// Autonomy invariants
		pub fn check_autonomy_invariants(kernel: &KernelHandle) -> Result<(), InvariantViolation>;
		
		/// State machine invariants
		pub fn check_state_invariants(kernel: &KernelHandle) -> Result<(), InvariantViolation>;
		
		/// Log integrity invariants
		pub fn check_log_invariants(kernel: &KernelHandle) -> Result<(), InvariantViolation>;
		
		/// Resource accounting invariants
		pub fn check_resource_invariants(kernel: &KernelHandle) -> Result<(), InvariantViolation>;
		
		/// Run all invariants
		pub fn check_all(kernel: &KernelHandle) -> Result<(), Vec<InvariantViolation>>;
}

/// Specific invariant checks
pub enum InvariantCheck {
		// Graph
		AllProductionGraphsAreAcyclic,
		NoDeletedNodesExist,
		AllEdgesReferenceExistingNodes,
		
		// Autonomy
		NoTokenElevationOccurred,
		AllActiveTokensAreValid,
		TokenSignaturesAreValid,
		
		// State
		AllNodesInValidState,
		AllTransitionsInAllowedMatrix,
		NoConcurrentTransitionsOnSameNode,
		
		// Log
		HashChainIsUnbroken,
		AllEventsHaveMonotonicTimestamps,
		LogIsAppendOnly,
		
		// Resource
		NoResourceCapExceeded,
		ResourceAccountingIsConsistent,
}
```

## 23.5 Simulator Execution

```rust
/// Run the COA Simulator
pub fn run_simulator(config: SimulatorConfig) -> SimulatorReport {
		let kernel = KernelHandle::new();
		let mut rng = StdRng::seed_from_u64(config.seed);
		let mut stats = OperationStats::default();
		let mut violations = Vec::new();
		
		for i in 0..config.total_operations {
				// Generate operation based on distribution
				let operation = generate_operation(&mut rng, &config.operation_distribution);
				
				// Track expected outcome
				let expected_result = classify_expected_result(&operation);
				
				// Execute against kernel
				let actual_result = execute_operation(&kernel, operation);
				
				// Verify outcome matches expectation
				if !outcome_matches_expectation(expected_result, &actual_result) {
						violations.push(Violation {
								operation_index: i,
								operation,
								expected: expected_result,
								actual: actual_result,
						});
						
						if config.stop_on_first_violation {
								break;
						}
				}
				
				// Check all invariants after every operation
				if let Err(e) = KernelInvariants::check_all(&kernel) {
						violations.extend(e.into_iter().map(|inv| Violation::Invariant(inv)));
						if config.stop_on_first_violation {
								break;
						}
				}
				
				stats.record(&operation, &actual_result);
		}
		
		SimulatorReport {
				config,
				stats,
				violations,
				final_state: kernel.export_state(),
		}
}
```

---
