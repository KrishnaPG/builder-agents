//! Error types for COA Core
//!
//! Provides comprehensive error handling for:
//! - Intent parsing failures
//! - Task decomposition errors
//! - Agent pool exhaustion
//! - Construction/execution failures
//! - Human escalation requirements

use coa_composition::CompositionError;
use coa_symbol::{SymbolRef, SymbolRefError};

/// Main COA error type
#[derive(Debug, thiserror::Error)]
pub enum COAError {
    /// Invalid user intent
    #[error("invalid intent: {0}")]
    InvalidIntent(String),

    /// Task decomposition failed
    #[error("decomposition failed: {0}")]
    DecompositionFailed(#[from] DecompositionError),

    /// Graph construction failed
    #[error("construction failed: {0}")]
    ConstructionFailed(#[from] ConstructionError),

    /// Composition strategy failed
    #[error("composition failed: {0}")]
    CompositionFailed(#[from] CompositionError),

    /// Agent execution failed
    #[error("agent failed: {0}")]
    AgentFailed(String),

    /// Agent pool exhausted
    #[error("agent pool error: {0}")]
    PoolError(#[from] PoolError),

    /// Execution failed
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// Kernel integration error
    #[error("kernel error: {0}")]
    KernelError(String),

    /// Requires human intervention
    #[error("requires human intervention: {error}")]
    RequiresHumanIntervention {
        /// The underlying error
        error: Box<COAError>,
        /// Diagnostic information
        diagnostic: Diagnostic,
        /// Suggested fixes
        suggested_fixes: Vec<SuggestedFix>,
    },

    /// Symbol reference error
    #[error("symbol error: {0}")]
    SymbolError(#[from] SymbolRefError),

    /// Configuration error
    #[error("configuration error: {0}")]
    ConfigError(String),

    /// Timeout
    #[error("operation timed out after {duration_secs}s")]
    Timeout { duration_secs: u64 },

    /// Cancelled
    #[error("operation cancelled")]
    Cancelled,
}

impl COAError {
    /// Check if error requires human intervention
    #[inline]
    #[must_use]
    pub fn requires_human(&self) -> bool {
        matches!(self, Self::RequiresHumanIntervention { .. })
    }

    /// Check if error is retryable
    #[inline]
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::AgentFailed(_)
                | Self::ExecutionFailed(_)
                | Self::Timeout { .. }
                | Self::PoolError(PoolError::PoolExhausted(_))
        )
    }

    /// Create human intervention error
    #[inline]
    pub fn requires_human_intervention(
        error: impl Into<Box<COAError>>,
        diagnostic: Diagnostic,
        suggested_fixes: Vec<SuggestedFix>,
    ) -> Self {
        Self::RequiresHumanIntervention {
            error: error.into(),
            diagnostic,
            suggested_fixes,
        }
    }
}

/// Task decomposition errors
#[derive(Debug, thiserror::Error)]
pub enum DecompositionError {
    /// Specification is invalid or incomplete
    #[error("invalid specification: {0}")]
    InvalidSpecification(String),

    /// Failed to identify symbols from spec
    #[error("symbol identification failed: {0}")]
    SymbolIdentificationFailed(String),

    /// No strategy available for artifact type
    #[error("no strategy for artifact type: {0}")]
    NoStrategy(String),

    /// Recursion depth exceeded
    #[error("decomposition recursion depth exceeded")]
    RecursionDepthExceeded,

    /// Cannot decompose goal type
    #[error("cannot decompose goal: {0:?}")]
    UnsupportedGoal(Goal),
}

/// Construction errors
#[derive(Debug, thiserror::Error)]
pub enum ConstructionError {
    /// Validation failed
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// Cyclic dependency detected
    #[error("cyclic dependency: {path:?}")]
    CyclicDependency { path: Vec<String> },

    /// Resource bounds exceeded
    #[error("resource bounds exceeded: requested {requested:?}, limit {limit:?}")]
    ResourceBoundsExceeded {
        requested: ResourceAmount,
        limit: ResourceAmount,
    },

    /// Output integrity violation
    #[error("output integrity violation: {claimants:?}")]
    OutputIntegrityViolation { claimants: Vec<(String, SymbolRef)> },

    /// Referential integrity violation
    #[error("referential integrity violation: {unresolved} not found")]
    ReferentialIntegrityViolation { unresolved: SymbolRef },

    /// Invalid node configuration
    #[error("invalid node configuration: {0}")]
    InvalidNodeConfig(String),
}

/// Agent pool errors
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    /// Pool at capacity
    #[error("pool exhausted (max: {0})")]
    PoolExhausted(usize),

    /// Agent creation failed
    #[error("agent creation failed: {0}")]
    CreationFailed(String),

    /// Agent not found
    #[error("agent not found: {0}")]
    AgentNotFound(String),

    /// Communication failed
    #[error("communication failed: {0}")]
    CommunicationFailed(String),
}

/// Goal types for specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Goal {
    /// Create new artifact
    CreateNew,
    /// Modify existing artifact
    ModifyExisting,
    /// Refactor code
    Refactor,
    /// Analyze code
    Analyze,
    /// Optimize performance
    Optimize,
}

/// Resource amount specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceAmount {
    /// Memory in MB
    pub memory_mb: usize,
    /// CPU millicores
    pub cpu_millicores: usize,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl ResourceAmount {
    /// Create new resource amount
    #[inline]
    #[must_use]
    pub fn new(memory_mb: usize, cpu_millicores: usize, timeout_secs: u64) -> Self {
        Self {
            memory_mb,
            cpu_millicores,
            timeout_secs,
        }
    }
}

/// Diagnostic information for failures
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Error type classification
    pub error_type: ErrorType,
    /// Location in graph/task
    pub location: Location,
    /// Context information
    pub context: Context,
    /// Suggested fixes
    pub suggested_fixes: Vec<SuggestedFix>,
}

impl Diagnostic {
    /// Create new diagnostic
    #[inline]
    #[must_use]
    pub fn new(error_type: ErrorType, location: Location) -> Self {
        Self {
            error_type,
            location,
            context: Context::empty(),
            suggested_fixes: Vec::new(),
        }
    }

    /// Add context
    #[inline]
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }

    /// Add suggested fixes
    #[inline]
    pub fn with_suggestions(mut self, fixes: Vec<SuggestedFix>) -> Self {
        self.suggested_fixes = fixes;
        self
    }
}

/// Error type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// Parsing/intent error
    Intent,
    /// Decomposition error
    Decomposition,
    /// Construction validation error
    Construction,
    /// Composition error
    Composition,
    /// Agent execution error
    Agent,
    /// Resource error
    Resource,
    /// System error
    System,
    /// Unknown
    Unknown,
}

/// Location in graph/task
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// Graph construction
    GraphConstruction,
    /// Specific task
    Task(String),
    /// Specific node
    Node(String),
    /// Specific agent
    Agent(String),
    /// Unknown location
    Unknown,
}

/// Context information for diagnostics
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Key-value context pairs
    pub entries: Vec<(String, String)>,
}

impl Context {
    /// Create empty context
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add context entry
    #[inline]
    pub fn add(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.entries.push((key.into(), value.into()));
        self
    }
}

/// Suggested fix for recovery
#[derive(Debug, Clone)]
pub struct SuggestedFix {
    /// Human-readable description
    pub description: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Whether fix can be auto-applied
    pub auto_applicable: bool,
    /// Resulting graph diff (if applicable)
    pub resulting_graph_diff: Option<GraphDiff>,
}

impl SuggestedFix {
    /// Create new suggested fix
    #[inline]
    #[must_use]
    pub fn new(description: impl Into<String>, confidence: f64) -> Self {
        Self {
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            auto_applicable: false,
            resulting_graph_diff: None,
        }
    }

    /// Mark as auto-applicable
    #[inline]
    #[must_use]
    pub fn auto_applicable(mut self) -> Self {
        self.auto_applicable = true;
        self
    }

    /// With graph diff
    #[inline]
    #[must_use]
    pub fn with_diff(mut self, diff: GraphDiff) -> Self {
        self.resulting_graph_diff = Some(diff);
        self
    }
}

/// Graph difference for suggested fixes
#[derive(Debug, Clone)]
pub struct GraphDiff {
    /// Added nodes
    pub added: Vec<String>,
    /// Removed nodes
    pub removed: Vec<String>,
    /// Modified nodes
    pub modified: Vec<String>,
}

impl GraphDiff {
    /// Create empty diff
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coa_error_display() {
        let err = COAError::InvalidIntent("test".to_string());
        assert!(err.to_string().contains("invalid intent"));
    }

    #[test]
    fn coa_error_requires_human() {
        let err = COAError::InvalidIntent("test".to_string());
        assert!(!err.requires_human());

        let human_err = COAError::RequiresHumanIntervention {
            error: Box::new(COAError::AgentFailed("test".to_string())),
            diagnostic: Diagnostic::new(ErrorType::Agent, Location::Unknown),
            suggested_fixes: vec![],
        };
        assert!(human_err.requires_human());
    }

    #[test]
    fn coa_error_is_retryable() {
        assert!(COAError::AgentFailed("test".to_string()).is_retryable());
        assert!(COAError::Timeout { duration_secs: 30 }.is_retryable());
        assert!(!COAError::InvalidIntent("test".to_string()).is_retryable());
    }

    #[test]
    fn suggested_fix_creation() {
        let fix = SuggestedFix::new("test fix", 0.9).auto_applicable();
        assert_eq!(fix.description, "test fix");
        assert_eq!(fix.confidence, 0.9);
        assert!(fix.auto_applicable);
    }

    #[test]
    fn diagnostic_builder() {
        let diag = Diagnostic::new(ErrorType::Construction, Location::GraphConstruction)
            .with_context(Context::empty().add("key", "value"))
            .with_suggestions(vec![SuggestedFix::new("fix", 0.8)]);

        assert!(matches!(diag.error_type, ErrorType::Construction));
        assert_eq!(diag.context.entries.len(), 1);
    }
}
