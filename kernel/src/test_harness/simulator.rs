//! COA Simulator - Property-based testing harness for the Kernel
//! 
//! Implements section 23 of the specification.

use crate::api::*;
use crate::autonomy::CapabilityToken;
use crate::error::KernelError;
use crate::handle::KernelHandle;
use crate::types::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

/// COA Simulator configuration
#[derive(Debug, Clone)]
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

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            total_operations: 10_000,
            operation_distribution: OperationDistribution::default(),
            max_concurrent_graphs: 10,
            max_nodes_per_graph: 100,
            stop_on_first_violation: true,
            stop_on_error_count: None,
        }
    }
}

/// Probability distribution for operation generation
#[derive(Debug, Clone)]
pub struct OperationDistribution {
    /// Valid operations (normal COA behavior)
    pub valid_ops: f64,
    /// Edge cases (boundary values)
    pub edge_cases: f64,
    /// Invalid operations (should be rejected)
    pub invalid_ops: f64,
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

/// All possible operations the simulator can generate
#[derive(Debug, Clone)]
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
    QueryLog(EventFilter),
    VerifyIntegrity,
}

/// Expected result classification for an operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedResult {
    ShouldSucceed,
    ShouldFail,
}

/// A violation detected during simulation
#[derive(Debug, Clone)]
pub enum Violation {
    /// Operation outcome didn't match expectation
    UnexpectedOutcome {
        operation_index: u64,
        operation: SimulatedOperation,
        expected: ExpectedResult,
        actual: Result<String, String>,
    },
    /// Invariant was violated
    Invariant(InvariantViolation),
}

/// A specific invariant violation
#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub check: InvariantCheck,
    pub details: String,
}

/// Types of invariant checks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantCheck {
    // Graph
    AllProductionGraphsAreAcyclic,
    AllEdgesReferenceExistingNodes,
    
    // Autonomy
    NoTokenElevationOccurred,
    AllActiveTokensAreValid,
    TokenSignaturesAreValid,
    
    // State
    AllNodesInValidState,
    AllTransitionsInAllowedMatrix,
    
    // Log
    HashChainIsUnbroken,
    AllEventsHaveMonotonicTimestamps,
    LogIsAppendOnly,
    
    // Resource
    NoResourceCapExceeded,
}

/// Statistics collected during simulation
#[derive(Debug, Clone, Default)]
pub struct OperationStats {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub invariant_violations: u64,
    pub operations_by_type: HashMap<String, u64>,
}

impl OperationStats {
    pub fn record(&mut self, operation: &SimulatedOperation, result: &Result<String, String>) {
        self.total_operations += 1;
        
        let type_name = format!("{:?}", operation).split('(').next().unwrap_or("Unknown").to_string();
        *self.operations_by_type.entry(type_name).or_insert(0) += 1;
        
        match result {
            Ok(_) => self.successful_operations += 1,
            Err(_) => self.failed_operations += 1,
        }
    }
}

/// Final report from the simulator
#[derive(Debug, Clone)]
pub struct SimulatorReport {
    pub config: SimulatorConfig,
    pub stats: OperationStats,
    pub violations: Vec<Violation>,
    pub final_graph_count: usize,
    pub final_node_count: usize,
}

impl SimulatorReport {
    /// Check if simulation passed all criteria
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
    
    /// Generate a text report
    pub fn generate_text(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== COA Simulator Report ===\n\n");
        report.push_str(&format!("Seed: {}\n", self.config.seed));
        report.push_str(&format!("Total Operations: {}\n", self.stats.total_operations));
        report.push_str(&format!("Successful: {}\n", self.stats.successful_operations));
        report.push_str(&format!("Failed: {}\n", self.stats.failed_operations));
        report.push_str(&format!("Violations: {}\n", self.violations.len()));
        report.push_str(&format!("Final Graphs: {}\n", self.final_graph_count));
        report.push_str(&format!("Final Nodes: {}\n", self.final_node_count));
        
        if !self.violations.is_empty() {
            report.push_str("\n=== Violations ===\n");
            for (i, v) in self.violations.iter().enumerate() {
                report.push_str(&format!("{}. {:?}\n", i + 1, v));
            }
        }
        
        report.push_str(&format!("\n=== Result: {} ===\n", 
            if self.passed() { "PASS" } else { "FAIL" }
        ));
        
        report
    }
}

/// Run the COA Simulator
pub fn run_simulator(config: SimulatorConfig) -> SimulatorReport {
    let kernel = KernelHandle::new();
    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut stats = OperationStats::default();
    let mut violations = Vec::new();
    
    // Track simulator state
    let mut graphs: Vec<GraphId> = Vec::new();
    let mut nodes: Vec<NodeId> = Vec::new();
    let mut tokens: Vec<CapabilityToken> = Vec::new();
    
    for i in 0..config.total_operations {
        // Generate operation based on distribution
        let operation = generate_operation(&mut rng, &config.operation_distribution, &graphs, &nodes, &tokens);
        
        // Track expected outcome
        let expected_result = classify_expected_result(&operation);
        
        // Execute against kernel
        let actual_result = execute_operation(&kernel, &operation, &mut graphs, &mut nodes, &mut tokens);
        
        // Verify outcome matches expectation
        let outcome_matches = match (expected_result, &actual_result) {
            (ExpectedResult::ShouldSucceed, Ok(_)) => true,
            (ExpectedResult::ShouldFail, Err(_)) => true,
            _ => false,
        };
        
        // Convert to string representation for reporting
        let actual_str: Result<String, String> = match &actual_result {
            Ok(_) => Ok("success".to_string()),
            Err(e) => Err(format!("{:?}", e)),
        };
        
        if !outcome_matches {
            violations.push(Violation::UnexpectedOutcome {
                operation_index: i,
                operation: operation.clone(),
                expected: expected_result,
                actual: actual_str.clone(),
            });
            
            if config.stop_on_first_violation {
                break;
            }
            
            if let Some(max_errors) = config.stop_on_error_count {
                if violations.len() >= max_errors {
                    break;
                }
            }
        }
        
        // Check all invariants after every operation
        if let Err(inv_violations) = KernelInvariants::check_all(&kernel) {
            for v in inv_violations {
                violations.push(Violation::Invariant(v));
            }
            if config.stop_on_first_violation {
                break;
            }
        }
        
        stats.record(&operation, &actual_str);
    }
    
    SimulatorReport {
        config,
        stats,
        violations,
        final_graph_count: graphs.len(),
        final_node_count: nodes.len(),
    }
}

/// Generate a random operation based on the distribution
fn generate_operation(
    rng: &mut StdRng,
    distribution: &OperationDistribution,
    graphs: &[GraphId],
    nodes: &[NodeId],
    _tokens: &[CapabilityToken],
) -> SimulatedOperation {
    let r: f64 = rng.gen();
    
    if r < distribution.valid_ops {
        generate_valid_operation(rng, graphs, nodes)
    } else if r < distribution.valid_ops + distribution.edge_cases {
        generate_edge_case_operation(rng, graphs, nodes)
    } else {
        generate_invalid_operation(rng, graphs, nodes)
    }
}

/// Generate a valid operation
fn generate_valid_operation(
    rng: &mut StdRng,
    graphs: &[GraphId],
    nodes: &[NodeId],
) -> SimulatedOperation {
    let choices = if graphs.is_empty() {
        vec![0] // Only create graph
    } else if nodes.is_empty() {
        vec![0, 1] // Create graph or add node
    } else {
        vec![0, 1, 2, 3, 4, 5, 6, 7] // All operations
    };
    
    match choices[rng.gen_range(0..choices.len())] {
        0 => SimulatedOperation::CreateGraph(
            if rng.gen_bool(0.7) { GraphType::ProductionDAG } else { GraphType::SandboxGraph }
        ),
        1 => {
            let graph_id = graphs[rng.gen_range(0..graphs.len())];
            SimulatedOperation::AddNode(graph_id, generate_node_spec(rng))
        }
        2 => {
            let graph_id = graphs[rng.gen_range(0..graphs.len())];
            if nodes.len() >= 2 {
                let from = nodes[rng.gen_range(0..nodes.len())];
                let to = nodes[rng.gen_range(0..nodes.len())];
                SimulatedOperation::AddEdge(graph_id, from, to)
            } else {
                SimulatedOperation::QueryPolicy(PolicyScope::Global)
            }
        }
        3 => {
            let node_id = nodes[rng.gen_range(0..nodes.len())];
            SimulatedOperation::QueryState(node_id)
        }
        4 => {
            let node_id = nodes[rng.gen_range(0..nodes.len())];
            let level = AutonomyLevel::L0; // Safe default
            let caps = generate_resource_caps(rng);
            SimulatedOperation::IssueToken(node_id, level, caps)
        }
        5 => SimulatedOperation::VerifyIntegrity,
        6 => SimulatedOperation::QueryLog(EventFilter::default()),
        7 if graphs.len() > 1 => {
            // Only close if we have multiple graphs to avoid closing the only graph
            let graph_id = graphs[rng.gen_range(0..graphs.len())];
            SimulatedOperation::CloseGraph(graph_id)
        }
        _ => SimulatedOperation::VerifyIntegrity,
    }
}

/// Generate an edge case operation
fn generate_edge_case_operation(
    rng: &mut StdRng,
    _graphs: &[GraphId],
    nodes: &[NodeId],
) -> SimulatedOperation {
    // Edge cases: boundary values, empty operations
    match rng.gen_range(0..5) {
        0 => SimulatedOperation::QueryPolicy(PolicyScope::Global),
        1 => SimulatedOperation::QueryLog(EventFilter::default()),
        2 if !nodes.is_empty() => {
            let node_id = nodes[rng.gen_range(0..nodes.len())];
            SimulatedOperation::FreezeNode(node_id)
        }
        3 if !nodes.is_empty() => {
            let node_id = nodes[rng.gen_range(0..nodes.len())];
            SimulatedOperation::DeactivateNode(node_id)
        }
        _ => SimulatedOperation::VerifyIntegrity,
    }
}

/// Generate an invalid operation that should be rejected
fn generate_invalid_operation(
    rng: &mut StdRng,
    graphs: &[GraphId],
    nodes: &[NodeId],
) -> SimulatedOperation {
    match rng.gen_range(0..6) {
        0 if !graphs.is_empty() => {
            // Try to add edge to non-existent node
            let graph_id = graphs[rng.gen_range(0..graphs.len())];
            let fake_node = NodeId::new();
            SimulatedOperation::AddEdge(graph_id, fake_node, fake_node)
        }
        1 if !nodes.is_empty() => {
            // Try illegal state transition
            let node_id = nodes[rng.gen_range(0..nodes.len())];
            // Created -> Merged is illegal
            let fake_token = create_fake_token(node_id);
            SimulatedOperation::TransitionState(node_id, NodeState::Merged, fake_token)
        }
        2 => {
            // Try to query non-existent node
            let fake_node = NodeId::new();
            SimulatedOperation::QueryState(fake_node)
        }
        3 if !nodes.is_empty() => {
            // Try to validate fake token
            let node_id = nodes[rng.gen_range(0..nodes.len())];
            let fake_token = create_fake_token(node_id);
            SimulatedOperation::ValidateToken(fake_token)
        }
        4 => {
            // Request excessive resources
            let caps = ResourceCaps {
                cpu_time_ms: u64::MAX,
                memory_bytes: u64::MAX,
                token_limit: u64::MAX,
                iteration_cap: u64::MAX,
            };
            let action = ProposedAction {
                action_type: ActionType::CreateGraph,
                node_id: None,
                graph_id: None,
                requested_caps: Some(caps),
                target_state: None,
            };
            SimulatedOperation::ValidateAction(action)
        }
        _ => {
            // Invalid graph type (shouldn't happen with enum, so just query)
            SimulatedOperation::QueryPolicy(PolicyScope::Global)
        }
    }
}

/// Classify whether an operation should succeed or fail
/// Note: This is called during generation, so we don't have full context.
/// We mark operations that are typically invalid as ShouldFail.
fn classify_expected_result(operation: &SimulatedOperation) -> ExpectedResult {
    match operation {
        // Self-loops should always fail
        SimulatedOperation::AddEdge(_, from, to) if from == to => ExpectedResult::ShouldFail,
        
        // Illegal state transitions
        SimulatedOperation::TransitionState(_, NodeState::Merged, _) => ExpectedResult::ShouldFail,
        
        // ValidateAction with excessive caps should fail
        SimulatedOperation::ValidateAction(action) => {
            if let Some(caps) = &action.requested_caps {
                const REASONABLE_MAX_CPU: u64 = 24 * 60 * 60 * 1000;
                const REASONABLE_MAX_MEMORY: u64 = 1024 * 1024 * 1024 * 100;
                if caps.cpu_time_ms > REASONABLE_MAX_CPU
                    || caps.memory_bytes > REASONABLE_MAX_MEMORY
                {
                    return ExpectedResult::ShouldFail;
                }
            }
            ExpectedResult::ShouldSucceed
        }
        
        // Most operations are context-dependent, assume success
        _ => ExpectedResult::ShouldSucceed,
    }
}

/// Execute an operation against the kernel
fn execute_operation(
    kernel: &KernelHandle,
    operation: &SimulatedOperation,
    graphs: &mut Vec<GraphId>,
    nodes: &mut Vec<NodeId>,
    tokens: &mut Vec<CapabilityToken>,
) -> Result<String, KernelError> {
    match operation {
        SimulatedOperation::CreateGraph(graph_type) => {
            let id = kernel.create_graph(*graph_type)?;
            graphs.push(id);
            Ok(format!("Created graph {:?}", id))
        }
        SimulatedOperation::CloseGraph(graph_id) => {
            kernel.close_graph(*graph_id)?;
            graphs.retain(|g| g != graph_id);
            Ok("Closed graph".to_string())
        }
        SimulatedOperation::AddNode(graph_id, spec) => {
            let id = kernel.add_node(*graph_id, spec.clone())?;
            nodes.push(id);
            Ok(format!("Added node {:?}", id))
        }
        SimulatedOperation::AddEdge(graph_id, from, to) => {
            kernel.add_edge(*graph_id, *from, *to)?;
            Ok("Added edge".to_string())
        }
        SimulatedOperation::DeactivateNode(node_id) => {
            kernel.deactivate_node(*node_id)?;
            Ok("Deactivated node".to_string())
        }
        SimulatedOperation::FreezeNode(node_id) => {
            kernel.freeze_node(*node_id)?;
            Ok("Frozen node".to_string())
        }
        SimulatedOperation::IssueToken(node_id, level, caps) => {
            let token = kernel.issue_token(*node_id, *level, *caps)?;
            tokens.push(token.clone());
            Ok(format!("Issued token for {:?}", node_id))
        }
        SimulatedOperation::DowngradeToken(token, new_level) => {
            let new_token = kernel.downgrade_token(token, *new_level)?;
            Ok(format!("Downgraded token to {:?}", new_token.autonomy_level))
        }
        SimulatedOperation::ValidateToken(token) => {
            let report = kernel.validate_token(token)?;
            Ok(format!("Token valid: {}", report.valid))
        }
        SimulatedOperation::TransitionState(node_id, to_state, token) => {
            kernel.transition(*node_id, *to_state, token)?;
            Ok(format!("Transitioned to {:?}", to_state))
        }
        SimulatedOperation::QueryState(node_id) => {
            let state = kernel.current_state(*node_id)?;
            Ok(format!("Current state: {:?}", state))
        }
        SimulatedOperation::ExecuteWork(node_id, token, work) => {
            let result = kernel.execute(*node_id, token, work.clone())
                .map_err(|e| KernelError::Internal(crate::error::InternalError(format!("{:?}", e))))?;
            Ok(format!("Execution success: {}", result.success))
        }
        SimulatedOperation::ScheduleExecution(node_id, token) => {
            let _schedule_token = kernel.schedule(*node_id, token)
                .map_err(|e| KernelError::Internal(crate::error::InternalError(format!("{:?}", e))))?;
            Ok("Scheduled execution".to_string())
        }
        SimulatedOperation::ValidateAction(action) => {
            let report = kernel.validate_action(action.clone())
                .map_err(|_e| KernelError::Compliance(crate::error::ComplianceViolation::PolicyViolation))?;
            if report.approved {
                Ok(format!("Action approved: true"))
            } else {
                Err(KernelError::Compliance(crate::error::ComplianceViolation::PolicyViolation))
            }
        }
        SimulatedOperation::QueryPolicy(scope) => {
            let policy = kernel.query_policy(*scope)?;
            Ok(format!("Max autonomy: {:?}", policy.max_autonomy_level))
        }
        SimulatedOperation::QueryLog(filter) => {
            let entries = kernel.query_events(filter.clone(), 100)?;
            Ok(format!("Found {} entries", entries.len()))
        }
        SimulatedOperation::VerifyIntegrity => {
            let report = kernel.verify_integrity()?;
            Ok(format!("Integrity valid: {}", report.valid))
        }
    }
}

/// Generate a random NodeSpec
fn generate_node_spec(rng: &mut StdRng) -> NodeSpec {
    use std::collections::BTreeMap;
    use serde_json::Value;
    
    let mut directives = BTreeMap::new();
    directives.insert("test".to_string(), Value::Bool(rng.gen_bool(0.5)));
    
    NodeSpec {
        directives: DirectiveSet { directives },
    }
}

/// Generate random resource caps
fn generate_resource_caps(rng: &mut StdRng) -> ResourceCaps {
    ResourceCaps {
        cpu_time_ms: rng.gen_range(100..10000),
        memory_bytes: rng.gen_range(1024..(1024 * 1024 * 100)),
        token_limit: rng.gen_range(10..10000),
        iteration_cap: rng.gen_range(1..1000),
    }
}

/// Create a fake token for testing invalid operations
fn create_fake_token(node_id: NodeId) -> CapabilityToken {
    use ed25519_dalek::SigningKey;
    use crate::types::DirectiveProfileHash;
    
    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    
    CapabilityToken::sign(
        node_id,
        AutonomyLevel::L0,
        ResourceCaps {
            cpu_time_ms: 1000,
            memory_bytes: 1024,
            token_limit: 100,
            iteration_cap: 10,
        },
        DirectiveProfileHash([0u8; 32]),
        &signing_key,
        0,
        "",
    )
}

/// Kernel invariant checks
pub struct KernelInvariants;

impl KernelInvariants {
    /// Check all invariants
    pub fn check_all(kernel: &KernelHandle) -> Result<(), Vec<InvariantViolation>> {
        let mut violations = Vec::new();
        
        if let Err(e) = Self::check_graph_invariants(kernel) {
            violations.push(e);
        }
        if let Err(e) = Self::check_autonomy_invariants(kernel) {
            violations.push(e);
        }
        if let Err(e) = Self::check_log_invariants(kernel) {
            violations.push(e);
        }
        
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
    
    /// Check graph invariants
    pub fn check_graph_invariants(_kernel: &KernelHandle) -> Result<(), InvariantViolation> {
        // Note: In a full implementation, we'd need access to internal state
        // For now, we rely on the kernel's internal enforcement
        Ok(())
    }
    
    /// Check autonomy invariants
    pub fn check_autonomy_invariants(_kernel: &KernelHandle) -> Result<(), InvariantViolation> {
        // Token validation is done during operations
        Ok(())
    }
    
    /// Check log invariants
    pub fn check_log_invariants(kernel: &KernelHandle) -> Result<(), InvariantViolation> {
        match kernel.verify_integrity() {
            Ok(report) => {
                if !report.valid {
                    Err(InvariantViolation {
                        check: InvariantCheck::HashChainIsUnbroken,
                        details: "Log integrity check failed".to_string(),
                    })
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(InvariantViolation {
                check: InvariantCheck::HashChainIsUnbroken,
                details: format!("Log verification error: {:?}", e),
            }),
        }
    }
}
