//! v2.0 Types for Safe-by-Construction Architecture
//!
//! This module contains the new types introduced in v2.0 that support
//! the two-phase architecture: Construction Phase → Execution Phase.

use crate::autonomy::CapabilityToken;
use crate::types::{AutonomyLevel, DirectiveSet, GraphId, GraphType, NodeId, ResourceCaps};
use ed25519_dalek::Signature;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;

/// v2.0 Node Specification with encoded policy constraints
///
/// All policy validation happens at construction time.
/// This type is immutable after creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSpecV2 {
    /// Node directives (compiled at construction time)
    pub directives: DirectiveSet,
    
    /// Maximum autonomy level (encoded constraint, not checked at runtime)
    pub autonomy_ceiling: AutonomyLevel,
    
    /// Resource bounds (part of type, proven at construction)
    pub resource_bounds: ResourceCaps,
    
    /// Optional expansion type for dynamic graph construction
    pub expansion_type: Option<ExpansionType>,
}

impl NodeSpecV2 {
    /// Create a new node specification
    pub fn new(
        directives: DirectiveSet,
        autonomy_ceiling: AutonomyLevel,
        resource_bounds: ResourceCaps,
    ) -> Self {
        Self {
            directives,
            autonomy_ceiling,
            resource_bounds,
            expansion_type: None,
        }
    }
    
    /// Create with expansion capability
    pub fn with_expansion(
        directives: DirectiveSet,
        autonomy_ceiling: AutonomyLevel,
        resource_bounds: ResourceCaps,
        expansion: ExpansionType,
    ) -> Self {
        Self {
            directives,
            autonomy_ceiling,
            resource_bounds,
            expansion_type: Some(expansion),
        }
    }
}

/// Expansion type for dynamic graph construction
///
/// Defines the schema and resource budget for subgraph expansions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionType {
    /// Type ID of the expansion schema (for type safety)
    pub schema_type_id: TypeIdWrapper,
    
    /// Maximum resources available to expansion subgraph
    pub max_subgraph_resources: ResourceCaps,
    
    /// Maximum recursion depth for nested expansions
    pub max_expansion_depth: u32,
}

/// Wrapper for TypeId
///
/// Note: TypeId doesn't have a stable as_u64() method.
/// We use a string-based representation for serialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeIdWrapper(pub String);

impl TypeIdWrapper {
    pub fn from_type_id(type_id: TypeId) -> Self {
        // Use the debug representation as a stable identifier
        Self(format!("{:?}", type_id))
    }
    
    pub fn to_type_id(&self) -> Option<TypeId> {
        // Cannot reconstruct TypeId from string, return None
        None
    }
    
    /// Create from a type directly
    pub fn of<T: 'static>() -> Self {
        Self::from_type_id(TypeId::of::<T>())
    }
}

/// Validation token - proof that a graph passed construction validation
///
/// This token is cryptographically signed and bound to the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationToken {
    pub graph_id: GraphId,
    pub validation_hash: [u8; 32],
    pub timestamp: u64,
    pub expires_at: u64,
    pub signature: Signature,
}

impl ValidationToken {
    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.expires_at
    }
}

/// A validated graph - proof-carrying type
///
/// This type can ONLY be constructed through `GraphBuilder::validate()`.
/// The private fields ensure type-level sealing.
#[derive(Debug, Clone)]
pub struct ValidatedGraph {
    pub(crate) graph_id: GraphId,
    pub(crate) validation_token: ValidationToken,
    pub(crate) graph_type: GraphType,
    pub(crate) nodes: HashMap<NodeId, NodeSpecV2>,
    pub(crate) edges: Vec<(NodeId, NodeId)>,
    pub(crate) node_tokens: HashMap<NodeId, CapabilityToken>,
}

impl ValidatedGraph {
    /// Get the validation token (for integrity verification)
    pub fn validation_token(&self) -> &ValidationToken {
        &self.validation_token
    }
    
    /// Get the graph ID
    pub fn graph_id(&self) -> GraphId {
        self.graph_id
    }
    
    /// Get the graph type
    pub fn graph_type(&self) -> GraphType {
        self.graph_type
    }
    
    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    
    /// Get a node's capability token
    pub fn get_node_token(&self, node_id: NodeId) -> Option<&CapabilityToken> {
        self.node_tokens.get(&node_id)
    }
    
    /// Get all node IDs
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }
    
    /// Get a node's specification
    pub fn get_node_spec(&self, node_id: NodeId) -> Option<&NodeSpecV2> {
        self.nodes.get(&node_id)
    }
}

/// Subgraph specification for expansion
///
/// The type parameter `T` ensures schema compliance at compile time.
#[derive(Debug, Clone)]
pub struct SubgraphSpec<T: ExpansionSchema> {
    pub nodes: Vec<NodeSpecV2>,
    pub edges: Vec<(NodeId, NodeId)>,
    pub _phantom: PhantomData<T>,
}

impl<T: ExpansionSchema> SubgraphSpec<T> {
    /// Create a new subgraph specification
    pub fn new(nodes: Vec<NodeSpecV2>, edges: Vec<(NodeId, NodeId)>) -> Self {
        Self {
            nodes,
            edges,
            _phantom: PhantomData,
        }
    }
}

/// Trait for expansion schemas
///
/// Implement this trait to define validation rules for subgraph expansions.
pub trait ExpansionSchema: Sized + 'static {
    /// Validate that a subgraph conforms to this schema
    fn validate_subgraph(subgraph: &SubgraphSpec<Self>) -> Result<(), ValidationError>;
    
    /// Get the type ID for this schema
    fn type_id() -> TypeIdWrapper {
        TypeIdWrapper::from_type_id(TypeId::of::<Self>())
    }
}

/// Validation error type (re-exported from error module)
pub use crate::error::ValidationError;

/// Expansion state for staged construction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionState {
    /// Waiting for subgraph specification
    AwaitingExpansion,
    /// Subgraph provided, pending validation
    Validating,
    /// Expansion complete, ready to continue
    Complete,
}

/// System limits for construction validation
#[derive(Debug, Clone, Copy)]
pub struct SystemLimits {
    pub max_autonomy: AutonomyLevel,
    pub max_resources: ResourceCaps,
    pub max_nodes: usize,
    pub max_edges: usize,
}

impl Default for SystemLimits {
    fn default() -> Self {
        Self {
            max_autonomy: AutonomyLevel::L5,
            max_resources: ResourceCaps {
                cpu_time_ms: u64::MAX,
                memory_bytes: u64::MAX,
                token_limit: u64::MAX,
                iteration_cap: u64::MAX,
            },
            max_nodes: 100_000,
            max_edges: 1_000_000,
        }
    }
}

/// Execution summary returned after graph execution
#[derive(Debug, Clone)]
pub struct ExecutionSummary {
    pub graph_id: GraphId,
    pub nodes_executed: usize,
    pub execution_time_ms: u64,
    pub resource_consumed: ResourceCaps,
}

/// Verification result for token integrity checks
#[derive(Debug, Clone)]
pub struct IntegrityVerification {
    pub valid: bool,
    pub node_binding_valid: bool,
    pub not_expired: bool,
}
