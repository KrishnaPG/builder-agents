//! Public API Types (v2.0)
//!
//! This module contains the public API types for the kernel.
//! Note: Most functionality is now accessed through the `prelude` module
//! which provides the v2.0 two-phase architecture types.

use crate::types::{AutonomyLevel, GraphId, GraphType, NodeId, NodeSpec, NodeState, ResourceCaps};
use crate::error::StateMachineError;
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

/// Error from execution (legacy API - use ExecutionError from error module instead)
#[derive(Debug, Clone)]
pub struct ApiExecutionError {
    pub node_id: Option<NodeId>,
    pub kind: ApiExecutionErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiExecutionErrorKind {
    ResourceExceeded,
    Timeout,
    IsolationFailure,
    TokenInvalid,
    Internal,
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
    major: 2,
    minor: 0,
    patch: 0,
};

/// Graph management trait (legacy - use GraphBuilder directly)
pub trait GraphManager {
    fn create_graph(&self, graph_type: GraphType) -> Result<GraphId, crate::error::KernelError>;
    fn close_graph(&self, graph_id: GraphId) -> Result<(), crate::error::KernelError>;
    fn graph_stats(&self, graph_id: GraphId) -> Result<GraphStats, crate::error::KernelError>;
}

/// Node operations trait (legacy - use GraphBuilder directly)
pub trait NodeOperations {
    fn add_node(&self, graph_id: GraphId, spec: NodeSpec) -> Result<NodeId, crate::error::KernelError>;
    fn add_edge(&self, graph_id: GraphId, from: NodeId, to: NodeId) -> Result<(), crate::error::KernelError>;
    fn deactivate_node(&self, node_id: NodeId) -> Result<(), crate::error::KernelError>;
    fn freeze_node(&self, node_id: NodeId) -> Result<(), crate::error::KernelError>;
}

/// Autonomy management trait (legacy - handled by ConstructionValidator)
pub trait AutonomyManager {
    fn issue_token(
        &self,
        node_id: NodeId,
        level: AutonomyLevel,
        caps: ResourceCaps,
    ) -> Result<crate::autonomy::CapabilityToken, crate::error::KernelError>;

    fn downgrade_token(
        &self,
        token: &crate::autonomy::CapabilityToken,
        new_level: AutonomyLevel,
    ) -> Result<crate::autonomy::CapabilityToken, crate::error::KernelError>;

    fn validate_token(&self, token: &crate::autonomy::CapabilityToken)
        -> Result<ValidationReport, crate::error::KernelError>;
}

/// State controller trait
pub trait StateController {
    fn transition(
        &self,
        node_id: NodeId,
        to: NodeState,
        token: &crate::autonomy::CapabilityToken,
    ) -> Result<TransitionReceipt, StateMachineError>;

    fn current_state(&self, node_id: NodeId) -> Result<NodeState, crate::error::KernelError>;
    fn allowed_transitions(&self, node_id: NodeId) -> Result<Vec<NodeState>, crate::error::KernelError>;
}

/// Execution runtime trait (legacy - use Executor instead)
pub trait ExecutionRuntime {
    fn execute(
        &self,
        node_id: NodeId,
        token: &crate::autonomy::CapabilityToken,
        work: crate::types::WorkSpec,
    ) -> Result<ExecutionResult, ApiExecutionError>;
}

/// Event logger trait
pub trait EventLogger {
    fn log_event(&self, event: crate::logging::Event) -> Result<crate::types::EventId, crate::error::LogError>;
    fn query_events(&self, filter: EventFilter, limit: usize) -> Result<Vec<LogEntry>, crate::error::KernelError>;
    fn verify_integrity(&self) -> Result<IntegrityReport, crate::error::KernelError>;
}

/// Scheduler trait
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
