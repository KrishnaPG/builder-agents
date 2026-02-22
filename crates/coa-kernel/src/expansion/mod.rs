//! Dynamic Graph Expansion (v2.0)
//!
//! This module supports staged construction with dynamic graph expansion.
//!
//! # Usage Pattern
//!
//! ```rust,ignore
//! // Create graph with expansion point
//! let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
//! let expansion_node = builder.add_expansion_node::<MySchema>(...);
//! let validated = builder.validate()?;
//!
//! // Execute until expansion point
//! let mut staged = StagedConstruction::new(validated);
//! let expansion_point = staged.execute_until_expansion().await?;
//!
//! // Provide expansion subgraph
//! let subgraph = SubgraphSpec::<MySchema>::new(nodes, edges);
//! staged.provide_expansion(subgraph)?;
//!
//! // Complete expansion and continue
//! let expanded = staged.complete_expansion()?;
//! executor.run(expanded).await?;
//! ```

use crate::construction::GraphBuilder;
use crate::error::{ExecutionError, ValidationError};
use crate::types::v2::{
    ExpansionSchema, NodeSpecV2, SubgraphSpec, SystemLimits, TypeIdWrapper, ValidatedGraph,
};
use crate::types::{GraphId, NodeId};
use ed25519_dalek::SigningKey;



/// Staged construction for dynamic expansion
///
/// Manages the execution of a graph with expansion points.
pub struct StagedConstruction {
    graph: ValidatedGraph,
    expansion_stack: Vec<ExpansionFrame>,
    #[allow(dead_code)]
    signing_key: SigningKey,
    #[allow(dead_code)]
    system_limits: SystemLimits,
}

/// Stack frame for nested expansions
#[derive(Debug, Clone)]
struct ExpansionFrame {
    expansion_node: NodeId,
    #[allow(dead_code)]
    parent_graph_id: GraphId,
    #[allow(dead_code)]
    depth: u32,
}

/// Expansion point encountered during execution
#[derive(Debug, Clone)]
pub struct ExpansionPoint {
    pub node_id: NodeId,
    pub schema_type_id: TypeIdWrapper,
    pub remaining_depth: u32,
    pub available_resources: crate::types::ResourceCaps,
}

impl StagedConstruction {
    /// Create a new staged construction from a validated graph
    pub fn new(graph: ValidatedGraph, signing_key: SigningKey) -> Self {
        let system_limits = SystemLimits::default();
        Self {
            graph,
            expansion_stack: Vec::new(),
            signing_key,
            system_limits,
        }
    }
    
    /// Create with custom system limits
    pub fn with_limits(
        graph: ValidatedGraph,
        signing_key: SigningKey,
        limits: SystemLimits,
    ) -> Self {
        Self {
            graph,
            expansion_stack: Vec::new(),
            signing_key,
            system_limits: limits,
        }
    }
    
    /// Execute the graph until an expansion point is reached
    ///
    /// Returns `Ok(Some(ExpansionPoint))` if expansion is needed,
    /// `Ok(None)` if graph execution completed without expansion.
    pub async fn execute_until_expansion(
        &mut self,
    ) -> Result<Option<ExpansionPoint>, ExecutionError> {
        // Scan for expansion nodes that haven't been expanded
        for node_id in self.graph.node_ids() {
            let spec = self.graph.get_node_spec(node_id)
                .ok_or(ExecutionError::GraphNotValidated)?;
            
            if let Some(expansion) = &spec.expansion_type {
                // Check if already expanded
                if !self.is_expanded(node_id) {
                    let remaining_depth = self.calculate_remaining_depth(node_id);
                    
                    if remaining_depth == 0 {
                        return Err(ExecutionError::ResourceEnforcementTriggered);
                    }
                    
                    return Ok(Some(ExpansionPoint {
                        node_id,
                        schema_type_id: expansion.schema_type_id.clone(),
                        remaining_depth,
                        available_resources: expansion.max_subgraph_resources,
                    }));
                }
            }
        }
        
        // No expansion points found
        Ok(None)
    }
    
    /// Provide an expansion subgraph for the current expansion point
    ///
    /// The subgraph is validated against the schema and resource constraints.
    pub fn provide_expansion<T: ExpansionSchema>(
        &mut self,
        subgraph: SubgraphSpec<T>,
    ) -> Result<(), ValidationError> {
        // Validate schema conformance
        T::validate_subgraph(&subgraph)?;
        
        // Get the expansion point
        let expansion_node = self.expansion_stack.last()
            .map(|f| f.expansion_node)
            .or_else(|| {
                // Find first unexpanded expansion node
                self.graph.node_ids().find(|&id| {
                    self.graph.get_node_spec(id)
                        .and_then(|s| s.expansion_type.as_ref())
                        .is_some() && !self.is_expanded(id)
                })
            })
            .ok_or(ValidationError::InvalidGraphStructure)?;
        
        let spec = self.graph.get_node_spec(expansion_node)
            .ok_or(ValidationError::InvalidGraphStructure)?;
        
        let expansion_type = spec.expansion_type
            .as_ref()
            .ok_or(ValidationError::InvalidGraphStructure)?;
        
        // Verify schema type matches
        let expected_type_id = T::type_id();
        if expansion_type.schema_type_id.0 != expected_type_id.0 {
            return Err(ValidationError::ExpansionSchemaMismatch);
        }
        
        // Validate resource budget
        self.validate_expansion_budget(&subgraph, expansion_type.max_subgraph_resources)?;
        
        // Validate autonomy ceiling propagation
        self.validate_autonomy_propagation(&subgraph, spec.autonomy_ceiling)?;
        
        // Push expansion frame
        let frame = ExpansionFrame {
            expansion_node,
            parent_graph_id: self.graph.graph_id(),
            depth: self.expansion_stack.len() as u32 + 1,
        };
        self.expansion_stack.push(frame);
        
        // TODO: Merge subgraph into graph
        // This would create a new ValidatedGraph with the expansion
        
        Ok(())
    }
    
    /// Complete the expansion and return the expanded graph
    pub fn complete_expansion(mut self) -> Result<ValidatedGraph, ValidationError> {
        // Pop the expansion frame
        if let Some(_frame) = self.expansion_stack.pop() {
            // TODO: Finalize the expanded graph
            // This would involve re-validating the merged graph
        }
        
        Ok(self.graph)
    }
    
    /// Check if a node has been expanded
    fn is_expanded(&self, _node_id: NodeId) -> bool {
        // TODO: Track expanded nodes
        false
    }
    
    /// Calculate remaining expansion depth for a node
    fn calculate_remaining_depth(&self, node_id: NodeId) -> u32 {
        let spec = match self.graph.get_node_spec(node_id) {
            Some(s) => s,
            None => return 0,
        };
        
        let expansion_type = match &spec.expansion_type {
            Some(e) => e,
            None => return 0,
        };
        
        let current_depth = self.expansion_stack.len() as u32;
        expansion_type.max_expansion_depth.saturating_sub(current_depth)
    }
    
    /// Validate that expansion subgraph is within budget
    fn validate_expansion_budget<T: ExpansionSchema>(
        &self,
        subgraph: &SubgraphSpec<T>,
        budget: crate::types::ResourceCaps,
    ) -> Result<(), ValidationError> {
        let mut total_cpu = 0u64;
        let mut total_memory = 0u64;
        let mut total_tokens = 0u64;
        let mut total_iterations = 0u64;
        
        for node in &subgraph.nodes {
            let bounds = &node.resource_bounds;
            
            total_cpu = total_cpu.checked_add(bounds.cpu_time_ms)
                .ok_or(ValidationError::ExpansionBudgetExceeded)?;
            total_memory = total_memory.checked_add(bounds.memory_bytes)
                .ok_or(ValidationError::ExpansionBudgetExceeded)?;
            total_tokens = total_tokens.checked_add(bounds.token_limit)
                .ok_or(ValidationError::ExpansionBudgetExceeded)?;
            total_iterations = total_iterations.checked_add(bounds.iteration_cap)
                .ok_or(ValidationError::ExpansionBudgetExceeded)?;
        }
        
        if total_cpu > budget.cpu_time_ms
            || total_memory > budget.memory_bytes
            || total_tokens > budget.token_limit
            || total_iterations > budget.iteration_cap
        {
            return Err(ValidationError::ExpansionBudgetExceeded);
        }
        
        Ok(())
    }
    
    /// Validate that expansion subgraph respects parent autonomy ceiling
    fn validate_autonomy_propagation<T: ExpansionSchema>(
        &self,
        subgraph: &SubgraphSpec<T>,
        parent_ceiling: crate::types::AutonomyLevel,
    ) -> Result<(), ValidationError> {
        for node in &subgraph.nodes {
            if node.autonomy_ceiling.as_u8() > parent_ceiling.as_u8() {
                return Err(ValidationError::AutonomyCeilingExceeded);
            }
        }
        
        Ok(())
    }
    
    /// Get current expansion depth
    pub fn current_depth(&self) -> u32 {
        self.expansion_stack.len() as u32
    }
    
    /// Get the underlying validated graph
    pub fn graph(&self) -> &ValidatedGraph {
        &self.graph
    }
}

/// Extension trait for GraphBuilder to add expansion nodes
pub trait ExpansionBuilder {
    /// Add an expansion node with a specific schema
    fn add_expansion_node<T: ExpansionSchema>(
        &mut self,
        spec: NodeSpecV2,
        max_subgraph_resources: crate::types::ResourceCaps,
        max_expansion_depth: u32,
    ) -> NodeId;
}

impl ExpansionBuilder for GraphBuilder {
    fn add_expansion_node<T: ExpansionSchema>(
        &mut self,
        spec: NodeSpecV2,
        max_subgraph_resources: crate::types::ResourceCaps,
        max_expansion_depth: u32,
    ) -> NodeId {
        let expansion_type = crate::types::v2::ExpansionType {
            schema_type_id: T::type_id(),
            max_subgraph_resources,
            max_expansion_depth,
        };
        
        let node_spec = NodeSpecV2 {
            expansion_type: Some(expansion_type),
            ..spec
        };
        
        self.add_node(node_spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AutonomyLevel, DirectiveSet, GraphType, ResourceCaps,
    };
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;

    // Define a test expansion schema
    struct TestSchema;
    impl ExpansionSchema for TestSchema {
        fn validate_subgraph(_subgraph: &SubgraphSpec<Self>) -> Result<(), ValidationError> {
            // Test schema accepts any subgraph
            Ok(())
        }
    }

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
    fn test_expansion_builder_adds_node() {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        let spec = create_test_spec();
        let resources = ResourceCaps {
            cpu_time_ms: 5000,
            memory_bytes: 10 * 1024 * 1024,
            token_limit: 5000,
            iteration_cap: 500,
        };
        
        let node_id = builder.add_expansion_node::<TestSchema>(spec, resources, 2);
        
        assert_eq!(builder.node_count(), 1);
        
        let node_spec = builder.get_node(node_id).unwrap();
        assert!(node_spec.expansion_type.is_some());
        assert_eq!(node_spec.expansion_type.as_ref().unwrap().max_expansion_depth, 2);
    }

    #[test]
    fn test_staged_construction_current_depth() {
        let signing_key = create_signing_key();
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        builder.add_node(create_test_spec());
        
        let validated = builder.validate(&signing_key).unwrap();
        let staged = StagedConstruction::new(validated, signing_key);
        
        assert_eq!(staged.current_depth(), 0);
    }

    #[test]
    fn test_validate_expansion_budget_within_limit() {
        let signing_key = create_signing_key();
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        // Add first node before expansion node
        let first_node = builder.add_node(create_test_spec());
        
        let spec = create_test_spec();
        let budget = ResourceCaps {
            cpu_time_ms: 10000,
            memory_bytes: 100 * 1024 * 1024,
            token_limit: 10000,
            iteration_cap: 1000,
        };
        
        let expansion_node = builder.add_expansion_node::<TestSchema>(spec, budget, 2);
        builder.add_edge(first_node, expansion_node).unwrap();
        
        let validated = builder.validate(&signing_key).unwrap();
        let staged = StagedConstruction::new(validated, signing_key);
        
        // Create subgraph within budget
        let subgraph = SubgraphSpec::<TestSchema>::new(
            vec![
                create_test_spec(),
                create_test_spec(),
            ],
            vec![],
        );
        
        // Should pass budget validation
        assert!(staged.validate_expansion_budget(&subgraph, budget).is_ok());
    }

    #[test]
    fn test_validate_expansion_budget_exceeds_limit() {
        let budget = ResourceCaps {
            cpu_time_ms: 500, // Too low
            memory_bytes: 100 * 1024 * 1024,
            token_limit: 10000,
            iteration_cap: 1000,
        };
        
        // Create subgraph that exceeds budget
        let mut spec = create_test_spec();
        spec.resource_bounds.cpu_time_ms = 1000; // Exceeds budget
        
        let subgraph = SubgraphSpec::<TestSchema>::new(
            vec![spec],
            vec![],
        );
        
        let signing_key = create_signing_key();
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        builder.add_node(create_test_spec());
        let validated = builder.validate(&signing_key).unwrap();
        let staged = StagedConstruction::new(validated, signing_key);
        
        assert!(staged.validate_expansion_budget(&subgraph, budget).is_err());
    }

    #[test]
    fn test_autonomy_propagation_respected() {
        let signing_key = create_signing_key();
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        builder.add_node(create_test_spec());
        let validated = builder.validate(&signing_key).unwrap();
        let staged = StagedConstruction::new(validated, signing_key);
        
        // Subgraph with autonomy at or below parent ceiling (L3)
        let mut spec = create_test_spec();
        spec.autonomy_ceiling = AutonomyLevel::L2;
        
        let subgraph = SubgraphSpec::<TestSchema>::new(vec![spec], vec![]);
        
        assert!(staged.validate_autonomy_propagation(&subgraph, AutonomyLevel::L3).is_ok());
    }

    #[test]
    fn test_autonomy_propagation_violated() {
        let signing_key = create_signing_key();
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        builder.add_node(create_test_spec());
        let validated = builder.validate(&signing_key).unwrap();
        let staged = StagedConstruction::new(validated, signing_key);
        
        // Subgraph with autonomy exceeding parent ceiling
        let mut spec = create_test_spec();
        spec.autonomy_ceiling = AutonomyLevel::L5;
        
        let subgraph = SubgraphSpec::<TestSchema>::new(vec![spec], vec![]);
        
        // Parent ceiling is L3, child wants L5
        assert!(staged.validate_autonomy_propagation(&subgraph, AutonomyLevel::L3).is_err());
    }
}
