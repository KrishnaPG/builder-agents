//! Core types for COA
//!
//! Defines the fundamental types for the orchestrator:
//! - COA configuration
//! - User intent and specifications
//! - Tasks and their properties
//! - Agent specifications

use crate::error::Goal;
use coa_artifact::SymbolPath;
use coa_composition::StrategyHint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use ulid::Ulid;

/// Unique task identifier (ULID for sortability)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Ulid);

impl TaskId {
    /// Generate new task ID
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique agent identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Ulid);

impl AgentId {
    /// Generate new agent ID
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// COA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct COAConfig {
    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,
    /// Default autonomy level for new agents
    pub default_autonomy: AutonomyLevel,
    /// System resource limits
    pub system_limits: SystemLimits,
    /// Whether to auto-apply fixes
    pub auto_apply_fixes: bool,
    /// Human escalation threshold
    pub escalation_threshold: EscalationThreshold,
    /// Task timeout in seconds
    pub task_timeout_secs: u64,
    /// Maximum decomposition depth
    pub max_decomposition_depth: usize,
}

impl COAConfig {
    /// Create default configuration
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With max concurrent agents
    #[inline]
    #[must_use]
    pub fn with_max_agents(mut self, max: usize) -> Self {
        self.max_concurrent_agents = max;
        self
    }

    /// With default autonomy
    #[inline]
    #[must_use]
    pub fn with_default_autonomy(mut self, autonomy: AutonomyLevel) -> Self {
        self.default_autonomy = autonomy;
        self
    }
}

impl Default for COAConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            default_autonomy: AutonomyLevel::L3,
            system_limits: SystemLimits::default(),
            auto_apply_fixes: false,
            escalation_threshold: EscalationThreshold::default(),
            task_timeout_secs: 300,
            max_decomposition_depth: 5,
        }
    }
}

/// System resource limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SystemLimits {
    /// Maximum memory in GB
    pub max_memory_gb: usize,
    /// Maximum CPU cores
    pub max_cpu_cores: usize,
    /// Maximum number of agents
    pub max_agents: usize,
}

impl Default for SystemLimits {
    fn default() -> Self {
        Self {
            max_memory_gb: 32,
            max_cpu_cores: 8,
            max_agents: 100,
        }
    }
}

/// Escalation threshold configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EscalationThreshold {
    /// Max test failures before escalation
    pub max_test_failures: u32,
    /// Max security violations before escalation
    pub max_security_violations: u32,
    /// Max autonomy violations before escalation
    pub max_autonomy_violations: u32,
}

impl Default for EscalationThreshold {
    fn default() -> Self {
        Self {
            max_test_failures: 3,
            max_security_violations: 1,
            max_autonomy_violations: 1,
        }
    }
}

/// Autonomy levels (embedded in node types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AutonomyLevel {
    /// Level 0: Full human-in-the-loop
    L0,
    /// Level 1: Approval before merge
    L1,
    /// Level 2: Auto code, human merge
    L2,
    /// Level 3: Auto merge in sandbox
    L3,
    /// Level 4: Auto merge + test deploy
    L4,
    /// Level 5: Full autonomous within boundary
    L5,
}

impl AutonomyLevel {
    /// Get numeric value
    #[inline]
    #[must_use]
    pub fn value(&self) -> u8 {
        match self {
            AutonomyLevel::L0 => 0,
            AutonomyLevel::L1 => 1,
            AutonomyLevel::L2 => 2,
            AutonomyLevel::L3 => 3,
            AutonomyLevel::L4 => 4,
            AutonomyLevel::L5 => 5,
        }
    }

    /// Check if this level can auto-merge
    #[inline]
    #[must_use]
    pub fn can_auto_merge(&self) -> bool {
        self.value() >= 3
    }

    /// Check if this level requires human approval
    #[inline]
    #[must_use]
    pub fn requires_human_approval(&self) -> bool {
        self.value() < 2
    }
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        AutonomyLevel::L3
    }
}

/// User intent (natural language input)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIntent {
    /// Natural language description
    pub description: String,
    /// Optional context
    pub context: Option<IntentContext>,
}

impl UserIntent {
    /// Create new intent
    #[inline]
    #[must_use]
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            context: None,
        }
    }

    /// With context
    #[inline]
    #[must_use]
    pub fn with_context(mut self, context: IntentContext) -> Self {
        self.context = Some(context);
        self
    }
}

/// Intent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentContext {
    /// Project/scope identifier
    pub project: Option<String>,
    /// Target files/paths
    pub targets: Vec<String>,
    /// Additional constraints
    pub constraints: Vec<String>,
    /// Preferred mode
    pub mode: Option<String>,
}

impl IntentContext {
    /// Create new context
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for IntentContext {
    fn default() -> Self {
        Self {
            project: None,
            targets: Vec::new(),
            constraints: Vec::new(),
            mode: None,
        }
    }
}

/// Structured specification (parsed from intent)
#[derive(Debug, Clone)]
pub struct Specification {
    /// Goal type
    pub goal: crate::error::Goal,
    /// Artifact type (code, config, spec, etc.)
    pub artifact_type: String,
    /// Target path/symbol
    pub target_path: SymbolPath,
    /// Acceptance criteria
    pub acceptance_criteria: Vec<String>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Output specification
    pub output_spec: Option<OutputSpec>,
}

impl Specification {
    /// Create new specification
    #[inline]
    #[must_use]
    pub fn new(
        goal: crate::error::Goal,
        artifact_type: impl Into<String>,
        target_path: SymbolPath,
    ) -> Self {
        Self {
            goal,
            artifact_type: artifact_type.into(),
            target_path,
            acceptance_criteria: Vec::new(),
            constraints: Vec::new(),
            output_spec: None,
        }
    }

    /// With acceptance criteria
    #[inline]
    #[must_use]
    pub fn with_criteria(mut self, criteria: Vec<String>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }

    /// Get composition strategy hint based on goal
    #[inline]
    #[must_use]
    pub fn strategy_hint(&self) -> StrategyHint {
        match self.goal {
            Goal::CreateNew | Goal::ModifyExisting => StrategyHint::Balanced,
            Goal::Refactor => StrategyHint::Ordered,
            Goal::Analyze => StrategyHint::Safety,
            Goal::Optimize => StrategyHint::Parallelism,
        }
    }
}

/// Constraint on specification
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Must use specific technology
    Technology(String),
    /// Must follow pattern
    Pattern(String),
    /// Must not exceed resource limit
    ResourceLimit { memory_mb: usize, cpu_cores: usize },
    /// Must complete by deadline
    Deadline(chrono::DateTime<chrono::Utc>),
    /// Custom constraint
    Custom(String, String),
}

/// Output specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputSpec {
    /// Code output with language
    Code { language: String },
    /// Config output with schema
    Config { schema: String },
    /// Spec output with format
    Spec { format: SpecFormat },
    /// Binary output with MIME type
    Binary { mime_type: String },
}

/// Spec format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecFormat {
    /// Markdown
    Markdown,
    /// Design document
    DesignDoc,
    /// API specification
    ApiSpec,
    /// Test specification
    TestSpec,
}

/// Executable task
#[derive(Debug, Clone)]
pub struct Task {
    /// Task identifier
    pub id: TaskId,
    /// Agent role
    pub role: String,
    /// Task description
    pub description: String,
    /// Directives (behavioral modifiers)
    pub directives: DirectiveSet,
    /// Autonomy level
    pub autonomy: AutonomyLevel,
    /// Resource caps
    pub resources: ResourceCaps,
    /// Task dependencies (task IDs)
    pub dependencies: Vec<TaskId>,
    /// Target artifact path
    pub target_artifact: SymbolPath,
    /// Expected output type
    pub expected_output: Option<OutputSpec>,
    /// Expansion type for dynamic graphs
    pub expansion_type: Option<ExpansionType>,
}

impl Task {
    /// Create new task
    #[inline]
    #[must_use]
    pub fn new(
        role: impl Into<String>,
        description: impl Into<String>,
        target_artifact: SymbolPath,
    ) -> Self {
        Self {
            id: TaskId::new(),
            role: role.into(),
            description: description.into(),
            directives: new_directive_set(),
            autonomy: AutonomyLevel::L3,
            resources: ResourceCaps::default(),
            dependencies: Vec::new(),
            target_artifact,
            expected_output: None,
            expansion_type: None,
        }
    }

    /// With autonomy level
    #[inline]
    #[must_use]
    pub fn with_autonomy(mut self, autonomy: AutonomyLevel) -> Self {
        self.autonomy = autonomy;
        self
    }

    /// With dependency
    #[inline]
    #[must_use]
    pub fn depends_on(mut self, task_id: TaskId) -> Self {
        self.dependencies.push(task_id);
        self
    }

    /// With resource caps
    #[inline]
    #[must_use]
    pub fn with_resources(mut self, resources: ResourceCaps) -> Self {
        self.resources = resources;
        self
    }

    /// With directive
    #[inline]
    #[must_use]
    pub fn with_directive(mut self, key: impl Into<String>, value: DirectiveValue) -> Self {
        self.directives.insert(key.into(), value);
        self
    }

    /// With expansion type
    #[inline]
    #[must_use]
    pub fn with_expansion(mut self, expansion: ExpansionType) -> Self {
        self.expansion_type = Some(expansion);
        self
    }
}

/// Resource capacity specification for a task
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResourceCaps {
    /// Memory limit in MB
    pub memory_mb: usize,
    /// CPU limit in millicores
    pub cpu_millicores: usize,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl Default for ResourceCaps {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpu_millicores: 500,
            timeout_secs: 60,
        }
    }
}

/// Directive set (behavioral modifiers)
pub type DirectiveSet = HashMap<String, DirectiveValue>;

/// Directive value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DirectiveValue {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// String value
    String(String),
    /// List of values
    List(Vec<DirectiveValue>),
}

/// Create new empty directive set
#[inline]
#[must_use]
pub fn new_directive_set() -> DirectiveSet {
    DirectiveSet::new()
}

/// Get string value from directive set
#[inline]
#[must_use]
pub fn get_directive_string<'a>(directives: &'a DirectiveSet, key: &str) -> Option<&'a str> {
    directives.get(key).and_then(|v| match v {
        DirectiveValue::String(s) => Some(s.as_str()),
        _ => None,
    })
}

/// Get bool value from directive set
#[inline]
#[must_use]
pub fn get_directive_bool(directives: &DirectiveSet, key: &str) -> Option<bool> {
    directives.get(key).and_then(|v| match v {
        DirectiveValue::Bool(b) => Some(*b),
        _ => None,
    })
}

/// Expansion types for dynamic graph generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpansionType {
    /// Conditional expansion based on condition
    Conditional { condition: String },
    /// Recursive expansion with max depth
    Recursive { max_depth: usize },
    /// Parallel branches
    Parallel { branches: Vec<BranchSpec> },
}

/// Branch specification for parallel expansion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSpec {
    /// Branch name
    pub name: String,
    /// Branch condition
    pub condition: String,
    /// Branch-specific configuration
    pub config: HashMap<String, String>,
}

/// Agent specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Agent role
    pub role: String,
    /// Directives
    pub directives: DirectiveSet,
    /// Autonomy level
    pub autonomy: AutonomyLevel,
    /// Resource limits
    pub resources: ResourceCaps,
}

impl AgentSpec {
    /// Create new agent spec
    #[inline]
    #[must_use]
    pub fn new(role: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            directives: new_directive_set(),
            autonomy: AutonomyLevel::L3,
            resources: ResourceCaps::default(),
        }
    }

    /// From task
    #[inline]
    #[must_use]
    pub fn from_task(task: &Task) -> Self {
        Self {
            role: task.role.clone(),
            directives: task.directives.clone(),
            autonomy: task.autonomy,
            resources: task.resources,
        }
    }
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Number of nodes executed
    pub nodes_executed: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Artifacts produced
    pub artifacts_produced: Vec<ArtifactSummary>,
    /// Tasks completed
    pub tasks_completed: Vec<TaskId>,
}

/// Artifact summary in execution result
#[derive(Debug, Clone)]
pub struct ArtifactSummary {
    /// Artifact type
    pub artifact_type: String,
    /// Artifact path/symbol
    pub path: String,
    /// Content hash
    pub hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn autonomy_level_value() {
        assert_eq!(AutonomyLevel::L0.value(), 0);
        assert_eq!(AutonomyLevel::L5.value(), 5);
    }

    #[test]
    fn autonomy_can_auto_merge() {
        assert!(!AutonomyLevel::L2.can_auto_merge());
        assert!(AutonomyLevel::L3.can_auto_merge());
        assert!(AutonomyLevel::L5.can_auto_merge());
    }

    #[test]
    fn task_builder() {
        let task = Task::new("tester", "test task", SymbolPath::from_str("test.path").unwrap())
            .with_autonomy(AutonomyLevel::L4)
            .depends_on(TaskId::new());

        assert_eq!(task.role, "tester");
        assert_eq!(task.autonomy, AutonomyLevel::L4);
        assert_eq!(task.dependencies.len(), 1);
    }

    #[test]
    fn directive_set_operations() {
        let mut directives = new_directive_set();
        directives.insert("key".to_string(), DirectiveValue::String("value".to_string()));
        directives.insert("flag".to_string(), DirectiveValue::Bool(true));

        assert_eq!(get_directive_string(&directives, "key"), Some("value"));
        assert_eq!(get_directive_bool(&directives, "flag"), Some(true));
    }

    #[test]
    fn agent_spec_from_task() {
        let task = Task::new("dev", "implement feature", SymbolPath::from_str("api.login").unwrap())
            .with_autonomy(AutonomyLevel::L4);

        let spec = AgentSpec::from_task(&task);
        assert_eq!(spec.role, "dev");
        assert_eq!(spec.autonomy, AutonomyLevel::L4);
    }
}
