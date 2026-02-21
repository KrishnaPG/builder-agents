use crate::error::{ComplianceViolation, KernelError, StateMachineError};
use crate::types::*;
use std::time::Duration;

/// Report from validating a capability token
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub valid: bool,
    pub node_id: NodeId,
    pub autonomy_level: AutonomyLevel,
    pub timestamp_valid: bool,
    pub signature_valid: bool,
    pub resource_caps_valid: bool,
    pub reason: Option<String>,
}

/// Receipt for a successful state transition
#[derive(Debug, Clone)]
pub struct TransitionReceipt {
    pub node_id: NodeId,
    pub from_state: NodeState,
    pub to_state: NodeState,
    pub timestamp: u64,
    pub token_validated: bool,
}

/// Result of executing work in a node
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub node_id: NodeId,
    pub output: Option<String>,
    pub resource_usage: ResourceUsage,
}

/// Resource usage statistics from execution
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_bytes: u64,
    pub tokens_used: u64,
    pub iterations: u64,
}

/// Error from execution
#[derive(Debug, Clone)]
pub struct ExecutionError {
    pub node_id: Option<NodeId>,
    pub kind: ExecutionErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionErrorKind {
    ResourceExceeded,
    Timeout,
    IsolationFailure,
    TokenInvalid,
    Internal,
}

/// A proposed action to be validated by compliance
#[derive(Debug, Clone)]
pub struct ProposedAction {
    pub action_type: ActionType,
    pub node_id: Option<NodeId>,
    pub graph_id: Option<GraphId>,
    pub requested_caps: Option<ResourceCaps>,
    pub target_state: Option<NodeState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    CreateGraph,
    CloseGraph,
    AddNode,
    AddEdge,
    DeactivateNode,
    FreezeNode,
    IssueToken,
    TransitionState,
    ExecuteWork,
}

/// Report from compliance validation
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub approved: bool,
    pub violations: Vec<ComplianceViolation>,
    pub resource_check_passed: bool,
    pub policy_check_passed: bool,
    pub timestamp: u64,
}

/// Error from compliance check
#[derive(Debug, Clone)]
pub struct ComplianceError {
    pub violations: Vec<ComplianceViolation>,
    pub message: String,
}

/// Scope for policy queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyScope {
    Global,
    Graph(GraphId),
    Node(NodeId),
}

/// Snapshot of policy at a point in time
#[derive(Debug, Clone)]
pub struct PolicySnapshot {
    pub max_autonomy_level: AutonomyLevel,
    pub default_caps: ResourceCaps,
    pub require_token_for_all_actions: bool,
    pub timestamp: u64,
}

/// Resource availability status
#[derive(Debug, Clone)]
pub struct ResourceAvailability {
    pub available: bool,
    pub cpu_time_remaining_ms: u64,
    pub memory_remaining_bytes: u64,
    pub tokens_remaining: u64,
    pub iterations_remaining: u64,
}

/// Filter for querying log events
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub node_id: Option<NodeId>,
    pub action: Option<String>,
    pub since_timestamp: Option<u64>,
    pub until_timestamp: Option<u64>,
    pub autonomy_level: Option<AutonomyLevel>,
}

/// A single log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub event: crate::logging::Event,
    pub verified: bool,
}

/// Report from integrity verification
#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub valid: bool,
    pub events_checked: usize,
    pub first_invalid_index: Option<usize>,
    pub tamper_detected: bool,
}

/// Token representing a scheduled execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScheduleToken {
    pub node_id: NodeId,
    pub sequence: u64,
}

/// Error from scheduler operations
#[derive(Debug, Clone)]
pub struct SchedulerError {
    pub kind: SchedulerErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerErrorKind {
    NodeNotFound,
    DeadlockDetected,
    AlreadyScheduled,
    Timeout,
    Cancelled,
}

/// Statistics for a graph
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub graph_type: GraphType,
    pub is_closed: bool,
    pub created_at: u64,
}

/// API compatibility enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    Deprecated,
    BreakingChanges(Vec<String>),
    Incompatible(Vec<String>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

pub const KERNEL_API_VERSION: ApiVersion = ApiVersion {
    major: 1,
    minor: 0,
    patch: 0,
};

pub trait GraphManager {
    fn create_graph(&self, graph_type: GraphType) -> Result<GraphId, KernelError>;
    fn close_graph(&self, graph_id: GraphId) -> Result<(), KernelError>;
    fn graph_stats(&self, graph_id: GraphId) -> Result<GraphStats, KernelError>;
}

pub trait NodeOperations {
    fn add_node(&self, graph_id: GraphId, spec: NodeSpec) -> Result<NodeId, KernelError>;
    fn add_edge(&self, graph_id: GraphId, from: NodeId, to: NodeId) -> Result<(), KernelError>;
    fn deactivate_node(&self, node_id: NodeId) -> Result<(), KernelError>;
    fn freeze_node(&self, node_id: NodeId) -> Result<(), KernelError>;
}

pub trait AutonomyManager {
    fn issue_token(
        &self,
        node_id: NodeId,
        level: AutonomyLevel,
        caps: ResourceCaps,
    ) -> Result<crate::autonomy::CapabilityToken, KernelError>;

    fn downgrade_token(
        &self,
        token: &crate::autonomy::CapabilityToken,
        new_level: AutonomyLevel,
    ) -> Result<crate::autonomy::CapabilityToken, KernelError>;

    fn validate_token(&self, token: &crate::autonomy::CapabilityToken)
        -> Result<ValidationReport, KernelError>;
}

pub trait StateController {
    fn transition(
        &self,
        node_id: NodeId,
        to: NodeState,
        token: &crate::autonomy::CapabilityToken,
    ) -> Result<TransitionReceipt, StateMachineError>;

    fn current_state(&self, node_id: NodeId) -> Result<NodeState, KernelError>;
    fn allowed_transitions(&self, node_id: NodeId) -> Result<Vec<NodeState>, KernelError>;
}

pub trait ExecutionRuntime {
    fn execute(
        &self,
        node_id: NodeId,
        token: &crate::autonomy::CapabilityToken,
        work: WorkSpec,
    ) -> Result<ExecutionResult, ExecutionError>;
}

pub trait ComplianceInterface {
    fn validate_action(&self, action: ProposedAction) -> Result<ComplianceReport, ComplianceError>;
    fn query_policy(&self, scope: PolicyScope) -> Result<PolicySnapshot, KernelError>;
    fn check_resources(&self, caps: ResourceCaps) -> Result<ResourceAvailability, KernelError>;
}

pub trait EventLogger {
    fn log_event(&self, event: crate::logging::Event) -> Result<EventId, crate::error::LogError>;
    fn query_events(&self, filter: EventFilter, limit: usize) -> Result<Vec<LogEntry>, KernelError>;
    fn verify_integrity(&self) -> Result<IntegrityReport, KernelError>;
}

#[async_trait::async_trait]
pub trait Scheduler: Send + Sync {
    fn schedule(
        &self,
        node_id: NodeId,
        token: &crate::autonomy::CapabilityToken,
    ) -> Result<ScheduleToken, SchedulerError>;
    fn cancel(&self, schedule_token: ScheduleToken) -> Result<(), SchedulerError>;
    async fn wait_for_completion(
        &self,
        node_id: NodeId,
        timeout: Duration,
    ) -> Result<ExecutionResult, SchedulerError>;
}
