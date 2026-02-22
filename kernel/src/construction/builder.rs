//! Graph Builder (v2.0)
//!
//! The primary interface for the construction phase.
//! Builds a graph and validates it, producing a `ValidatedGraph`.

use crate::error::ValidationError;
use crate::construction::validator::ValidationContext;
use crate::types::v2::{NodeSpecV2, SystemLimits, ValidatedGraph};
use crate::types::{GraphId, GraphType, NodeId};
use crate::construction::ConstructionValidator;
use ed25519_dalek::SigningKey;
use std::collections::HashMap;

/// Error type for graph builder operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphBuilderError {
    NodeNotFound(NodeId),
    EdgeAlreadyExists,
    SelfLoopNotAllowed,
    WouldCreateCycle,
    GraphTypeNotMutable,
}

impl std::fmt::Display for GraphBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for GraphBuilderError {}

/// Builder for constructing validated graphs
///
/// Usage:
/// ```rust,ignore
/// let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
/// let n1 = builder.add_node(spec1);
/// let n2 = builder.add_node(spec2);
/// builder.add_edge(n1, n2)?;
/// let validated: ValidatedGraph = builder.validate()?;
/// ```
pub struct GraphBuilder {
    graph_type: GraphType,
    graph_id: GraphId,
    nodes: HashMap<NodeId, NodeSpecV2>,
    edges: Vec<(NodeId, NodeId)>,
    system_limits: SystemLimits,
    adjacency: HashMap<NodeId, Vec<NodeId>>, // For cycle detection
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new(graph_type: GraphType) -> Self {
        Self {
            graph_type,
            graph_id: GraphId::new(),
            nodes: HashMap::new(),
            edges: Vec::new(),
            system_limits: SystemLimits::default(),
            adjacency: HashMap::new(),
        }
    }
    
    /// Create a new graph builder with custom system limits
    pub fn with_limits(graph_type: GraphType, limits: SystemLimits) -> Self {
        Self {
            graph_type,
            graph_id: GraphId::new(),
            nodes: HashMap::new(),
            edges: Vec::new(),
            system_limits: limits,
            adjacency: HashMap::new(),
        }
    }
    
    /// Get the graph ID
    pub fn graph_id(&self) -> GraphId {
        self.graph_id
    }
    
    /// Get the graph type
    pub fn graph_type(&self) -> GraphType {
        self.graph_type
    }
    
    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    
    /// Add a node to the graph
    ///
    /// Returns the node ID for use in edge construction.
    pub fn add_node(&mut self, spec: NodeSpecV2) -> NodeId {
        let node_id = NodeId::new();
        self.nodes.insert(node_id, spec);
        self.adjacency.insert(node_id, Vec::new());
        node_id
    }
    
    /// Add an edge between two nodes
    ///
    /// For production DAGs, this will reject edges that would create a cycle.
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), GraphBuilderError> {
        // Check nodes exist
        if !self.nodes.contains_key(&from) {
            return Err(GraphBuilderError::NodeNotFound(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(GraphBuilderError::NodeNotFound(to));
        }
        
        // Check for self-loop
        if from == to {
            return Err(GraphBuilderError::SelfLoopNotAllowed);
        }
        
        // Check if edge already exists
        if self.edges.contains(&(from, to)) {
            return Err(GraphBuilderError::EdgeAlreadyExists);
        }
        
        // Check for cycles in production DAG
        if matches!(self.graph_type, GraphType::ProductionDAG) {
            // Temporarily add edge and check for cycle
            self.adjacency.get_mut(&from).unwrap().push(to);
            
            if self.has_cycle() {
                // Remove the edge we just added
                let neighbors = self.adjacency.get_mut(&from).unwrap();
                neighbors.retain(|&n| n != to);
                return Err(GraphBuilderError::WouldCreateCycle);
            }
        } else {
            // Sandbox - just add edge
            self.adjacency.get_mut(&from).unwrap().push(to);
        }
        
        self.edges.push((from, to));
        Ok(())
    }
    
    /// Check if the current graph has a cycle
    fn has_cycle(&self) -> bool {
        let mut visiting = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();
        
        fn dfs(
            node: NodeId,
            adjacency: &HashMap<NodeId, Vec<NodeId>>,
            visiting: &mut std::collections::HashSet<NodeId>,
            visited: &mut std::collections::HashSet<NodeId>,
        ) -> bool {
            if visiting.contains(&node) {
                return true;
            }
            if visited.contains(&node) {
                return false;
            }
            
            visiting.insert(node);
            
            if let Some(neighbors) = adjacency.get(&node) {
                for &neighbor in neighbors {
                    if dfs(neighbor, adjacency, visiting, visited) {
                        return true;
                    }
                }
            }
            
            visiting.remove(&node);
            visited.insert(node);
            false
        }
        
        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id) {
                if dfs(node_id, &self.adjacency, &mut visiting, &mut visited) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Get a reference to a node's specification
    pub fn get_node(&self, node_id: NodeId) -> Option<&NodeSpecV2> {
        self.nodes.get(&node_id)
    }
    
    /// Get all node IDs
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }
    
    /// Get all edges
    pub fn edges(&self) -> &[(NodeId, NodeId)] {
        &self.edges
    }
    
    /// Validate the graph and produce a ValidatedGraph
    ///
    /// This performs all construction-time validation:
    /// - Graph structure validation
    /// - Policy compliance checks
    /// - Resource bounds proving
    /// - Token issuance
    ///
    /// Once validated, the graph cannot be modified.
    pub fn validate(self, signing_key: &SigningKey) -> Result<ValidatedGraph, ValidationError> {
        let validator = ConstructionValidator::with_context(ValidationContext {
            system_limits: self.system_limits,
            graph_type: self.graph_type,
        });
        
        validator.validate_graph(
            self.graph_id,
            self.graph_type,
            &self.nodes,
            &self.edges,
            signing_key,
        )
    }
    
    /// Check if adding an edge would create a cycle
    ///
    /// This is a preview method that doesn't modify the builder.
    pub fn would_create_cycle(&self, from: NodeId, to: NodeId) -> bool {
        if from == to {
            return true;
        }
        
        // Temporarily add edge and check
        let mut temp_adjacency = self.adjacency.clone();
        temp_adjacency.entry(from).or_default().push(to);
        
        // Check if 'to' can reach 'from' (would create cycle)
        self.can_reach(&temp_adjacency, to, from)
    }
    
    /// Check if target is reachable from source
    fn can_reach(
        &self,
        adjacency: &HashMap<NodeId, Vec<NodeId>>,
        source: NodeId,
        target: NodeId,
    ) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![source];
        
        while let Some(node) = stack.pop() {
            if node == target {
                return true;
            }
            
            if visited.insert(node) {
                if let Some(neighbors) = adjacency.get(&node) {
                    for &neighbor in neighbors {
                        stack.push(neighbor);
                    }
                }
            }
        }
        
        false
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new(GraphType::ProductionDAG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AutonomyLevel, DirectiveSet, ResourceCaps};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;

    fn create_test_spec() -> NodeSpecV2 {
        NodeSpecV2 {
            directives: DirectiveSet {
                directives: BTreeMap::new(),
            },
            autonomy_ceiling: AutonomyLevel::L3,
            resource_bounds: ResourceCaps {
                cpu_time_ms: 1000,
                memory_bytes: 1024 * 1024,
                token_limit: 1000,
                iteration_cap: 100,
            },
            expansion_type: None,
        }
    }

    fn create_signing_key() -> SigningKey {
        let mut csprng = OsRng;
        SigningKey::generate(&mut csprng)
    }

    #[test]
    fn test_builder_creates_nodes() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        let n1 = builder.add_node(create_test_spec());
        let n2 = builder.add_node(create_test_spec());
        
        assert_eq!(builder.node_count(), 2);
        assert_ne!(n1, n2);
    }

    #[test]
    fn test_add_edge_valid() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        let n1 = builder.add_node(create_test_spec());
        let n2 = builder.add_node(create_test_spec());
        
        assert!(builder.add_edge(n1, n2).is_ok());
        assert_eq!(builder.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_rejects_self_loop() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        let n1 = builder.add_node(create_test_spec());
        
        assert!(matches!(
            builder.add_edge(n1, n1),
            Err(GraphBuilderError::SelfLoopNotAllowed)
        ));
    }

    #[test]
    fn test_add_edge_rejects_cycle_in_production() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        let n1 = builder.add_node(create_test_spec());
        let n2 = builder.add_node(create_test_spec());
        let n3 = builder.add_node(create_test_spec());
        
        builder.add_edge(n1, n2).unwrap();
        builder.add_edge(n2, n3).unwrap();
        
        // n3 -> n1 would create cycle
        assert!(matches!(
            builder.add_edge(n3, n1),
            Err(GraphBuilderError::WouldCreateCycle)
        ));
    }

    #[test]
    fn test_add_edge_allows_cycle_in_sandbox() {
        let mut builder = GraphBuilder::new(GraphType::SandboxGraph);
        
        let n1 = builder.add_node(create_test_spec());
        let n2 = builder.add_node(create_test_spec());
        
        builder.add_edge(n1, n2).unwrap();
        
        // Cycles allowed in sandbox
        assert!(builder.add_edge(n2, n1).is_ok());
    }

    #[test]
    fn test_validate_produces_validated_graph() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        let signing_key = create_signing_key();
        
        let n1 = builder.add_node(create_test_spec());
        let n2 = builder.add_node(create_test_spec());
        builder.add_edge(n1, n2).unwrap();
        
        let result = builder.validate(&signing_key);
        
        assert!(result.is_ok());
        
        let validated = result.unwrap();
        assert_eq!(validated.node_count(), 2);
        assert_eq!(validated.edge_count(), 1);
    }

    #[test]
    fn test_would_create_cycle_preview() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        let n1 = builder.add_node(create_test_spec());
        let n2 = builder.add_node(create_test_spec());
        let n3 = builder.add_node(create_test_spec());
        
        builder.add_edge(n1, n2).unwrap();
        builder.add_edge(n2, n3).unwrap();
        
        // Preview: n3 -> n1 would create cycle
        assert!(builder.would_create_cycle(n3, n1));
        
        // Preview: n1 -> n3 is fine
        assert!(!builder.would_create_cycle(n1, n3));
        
        // Verify builder not modified
        assert_eq!(builder.edge_count(), 2);
    }

    #[test]
    fn test_node_not_found_error() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        let n1 = builder.add_node(create_test_spec());
        let n2 = NodeId::new(); // Not in builder
        
        assert!(matches!(
            builder.add_edge(n1, n2),
            Err(GraphBuilderError::NodeNotFound(_))
        ));
    }
}
