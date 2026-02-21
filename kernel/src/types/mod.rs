use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphId(pub Uuid);

impl GraphId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomyLevel {
    L0,
    L1,
    L2,
    L3,
    L4,
    L5,
}

impl AutonomyLevel {
    pub fn as_u8(self) -> u8 {
        match self {
            AutonomyLevel::L0 => 0,
            AutonomyLevel::L1 => 1,
            AutonomyLevel::L2 => 2,
            AutonomyLevel::L3 => 3,
            AutonomyLevel::L4 => 4,
            AutonomyLevel::L5 => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphType {
    ProductionDAG,
    SandboxGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Created,
    Isolated,
    Testing,
    Executing,
    Validating,
    Merged,
    Escalated,
    Frozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceCaps {
    pub cpu_time_ms: u64,
    pub memory_bytes: u64,
    pub token_limit: u64,
    pub iteration_cap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveSet {
    pub directives: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProfile {
    pub required_test_coverage_percent: u8,
    pub security_scan_depth: u8,
    pub max_debate_iterations: u32,
    pub merge_gating_policy: String,
    pub resource_multipliers: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DirectiveProfileHash(pub [u8; 32]);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSpec {
    pub directives: DirectiveSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSpec {
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
