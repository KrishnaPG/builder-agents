use std::fmt;

#[derive(Debug)]
pub enum KernelError {
    Graph(GraphError),
    Node(NodeError),
    Autonomy(AutonomyError),
    Compliance(ComplianceViolation),
    Resource(ResourceError),
    StateMachine(StateMachineError),
    Log(LogError),
    Config(ConfigError),
    Internal(InternalError),
}

impl KernelError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            KernelError::Internal(_) => false,
            KernelError::Config(_) => false,
            KernelError::Log(LogError::IntegrityViolation) => false,
            KernelError::Compliance(_) => true,
            KernelError::Autonomy(_) => true,
            KernelError::Graph(_) => true,
            KernelError::Node(_) => true,
            KernelError::Resource(_) => true,
            KernelError::StateMachine(_) => true,
            KernelError::Log(_) => true,
        }
    }

    pub fn is_system_error(&self) -> bool {
        matches!(
            self,
            KernelError::Internal(_)
                | KernelError::Config(_)
                | KernelError::Log(LogError::IntegrityViolation)
        )
    }

    pub fn should_escalate(&self) -> bool {
        self.is_system_error()
            || matches!(
                self,
                KernelError::Compliance(_) | KernelError::Autonomy(_) | KernelError::Resource(_)
            )
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::Graph(e) => write!(f, "Graph error: {e}"),
            KernelError::Node(e) => write!(f, "Node error: {e}"),
            KernelError::Autonomy(e) => write!(f, "Autonomy violation: {e}"),
            KernelError::Compliance(e) => write!(f, "Compliance violation: {e}"),
            KernelError::Resource(e) => write!(f, "Resource exhausted: {e}"),
            KernelError::StateMachine(e) => write!(f, "State machine error: {e}"),
            KernelError::Log(e) => write!(f, "Log error: {e}"),
            KernelError::Config(e) => write!(f, "Configuration error: {e}"),
            KernelError::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl std::error::Error for KernelError {}

impl From<GraphError> for KernelError {
    fn from(value: GraphError) -> Self {
        KernelError::Graph(value)
    }
}

impl From<NodeError> for KernelError {
    fn from(value: NodeError) -> Self {
        KernelError::Node(value)
    }
}

impl From<AutonomyError> for KernelError {
    fn from(value: AutonomyError) -> Self {
        KernelError::Autonomy(value)
    }
}

impl From<StateMachineError> for KernelError {
    fn from(value: StateMachineError) -> Self {
        KernelError::StateMachine(value)
    }
}

impl From<ResourceError> for KernelError {
    fn from(value: ResourceError) -> Self {
        KernelError::Resource(value)
    }
}

impl From<LogError> for KernelError {
    fn from(value: LogError) -> Self {
        KernelError::Log(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    GraphNotFound,
    GraphClosed,
    CycleDetected,
    NodeNotFound,
    SelfLoop,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeError {
    NodeNotFound,
    NodeDeactivated,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomyError {
    CeilingExceeded,
    TokenExpired,
    TokenMismatch,
    ElevationForbidden,
    InvalidSignature,
    TokenRequired,
}

impl fmt::Display for AutonomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceViolation {
    ValidationRequired,
    PolicyViolation,
}

impl fmt::Display for ComplianceViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    LimitExceeded,
    CapExceeded,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMachineError {
    IllegalTransition,
    TransitionInProgress,
}

impl fmt::Display for StateMachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogError {
    Immutable,
    IntegrityViolation,
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    InvalidConfiguration,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalError(pub String);

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
