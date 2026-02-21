use crate::error::{KernelError, StateMachineError};
use crate::types::*;
use std::time::Duration;

pub struct ValidationReport;
pub struct TransitionReceipt;
pub struct ExecutionResult;
pub struct ExecutionError;
pub struct ProposedAction;
pub struct ComplianceReport;
pub struct ComplianceError;
pub struct PolicyScope;
pub struct PolicySnapshot;
pub struct ResourceAvailability;
pub struct EventFilter;
pub struct LogEntry;
pub struct IntegrityReport;
pub struct ScheduleToken;
pub struct SchedulerError;
pub struct GraphStats;
pub struct Compatibility;
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

pub trait Scheduler {
    fn schedule(
        &self,
        node_id: NodeId,
        token: &crate::autonomy::CapabilityToken,
    ) -> Result<ScheduleToken, SchedulerError>;
    fn cancel(&self, schedule_token: ScheduleToken) -> Result<(), SchedulerError>;
    fn wait_for_completion(
        &self,
        node_id: NodeId,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<ExecutionResult, SchedulerError>> + Send;
}
