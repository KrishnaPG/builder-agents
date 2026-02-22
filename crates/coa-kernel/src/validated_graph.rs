//! Validated Graph - Proof-Carrying Type (v2.0)
//!
//! This module defines the `ValidatedGraph` type which can ONLY be constructed
//! through the construction phase (`GraphBuilder::validate()`).
//!
//! The type is sealed - it has no public constructor, ensuring that:
//! 1. All graphs reaching execution have passed validation
//! 2. Validation cannot be bypassed
//! 3. The proof token is cryptographically bound to the graph

use crate::autonomy::CapabilityToken;
use crate::types::v2::{SystemLimits, ValidatedGraph, ValidationToken};
use crate::types::{GraphId, GraphType, NodeId};
use crate::types::v2::NodeSpecV2;
use std::collections::HashMap;

/// Sealed constructor for ValidatedGraph
///
/// This struct is only accessible within the crate, ensuring that
/// ValidatedGraph can only be created through the construction phase.
pub(crate) struct ValidatedGraphConstructor;

impl ValidatedGraphConstructor {
    /// Construct a ValidatedGraph (internal use only)
    ///
    /// # Safety
    /// This should only be called from `GraphBuilder::validate()` after
    /// all validation checks have passed.
    pub(crate) fn construct(
        graph_id: GraphId,
        validation_token: ValidationToken,
        graph_type: GraphType,
        nodes: HashMap<NodeId, NodeSpecV2>,
        edges: Vec<(NodeId, NodeId)>,
        node_tokens: HashMap<NodeId, CapabilityToken>,
    ) -> ValidatedGraph {
        ValidatedGraph {
            graph_id,
            validation_token,
            graph_type,
            nodes,
            edges,
            node_tokens,
        }
    }
}

/// Validation report returned after successful validation
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub graph_id: GraphId,
    pub node_count: usize,
    pub edge_count: usize,
    pub validation_duration_ms: u64,
}

/// Resource proof - evidence that resource bounds are satisfiable
#[derive(Debug, Clone)]
pub struct ResourceProof {
    pub total_cpu_ms: u64,
    pub total_memory_bytes: u64,
    pub total_tokens: u64,
    pub total_iterations: u64,
    pub within_system_limits: bool,
}

impl ResourceProof {
    /// Verify that resource bounds are provably satisfiable
    pub fn verify_bounds(
        nodes: &[NodeSpecV2],
        system_limits: &SystemLimits,
    ) -> Result<Self, crate::error::ValidationError> {
        let mut total_cpu = 0u64;
        let mut total_memory = 0u64;
        let mut total_tokens = 0u64;
        let mut total_iterations = 0u64;
        
        for node in nodes {
            let bounds = &node.resource_bounds;
            
            // Check for overflow
            total_cpu = total_cpu.checked_add(bounds.cpu_time_ms)
                .ok_or(crate::error::ValidationError::ResourceBoundsNotProvable)?;
            total_memory = total_memory.checked_add(bounds.memory_bytes)
                .ok_or(crate::error::ValidationError::ResourceBoundsNotProvable)?;
            total_tokens = total_tokens.checked_add(bounds.token_limit)
                .ok_or(crate::error::ValidationError::ResourceBoundsNotProvable)?;
            total_iterations = total_iterations.checked_add(bounds.iteration_cap)
                .ok_or(crate::error::ValidationError::ResourceBoundsNotProvable)?;
        }
        
        // Check against system limits
        let within_limits = total_cpu <= system_limits.max_resources.cpu_time_ms
            && total_memory <= system_limits.max_resources.memory_bytes
            && total_tokens <= system_limits.max_resources.token_limit
            && total_iterations <= system_limits.max_resources.iteration_cap;
        
        if !within_limits {
            return Err(crate::error::ValidationError::ResourceBoundsNotProvable);
        }
        
        Ok(Self {
            total_cpu_ms: total_cpu,
            total_memory_bytes: total_memory,
            total_tokens,
            total_iterations,
            within_system_limits: within_limits,
        })
    }
}

/// Compute validation hash for a graph
///
/// This hash cryptographically binds the validation to the graph structure.
pub fn compute_validation_hash(
    graph_id: GraphId,
    nodes: &HashMap<NodeId, NodeSpecV2>,
    edges: &[(NodeId, NodeId)],
) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    
    let mut hasher = Sha256::new();
    
    // Hash graph ID
    hasher.update(graph_id.0.as_bytes());
    
    // Hash nodes (sorted for determinism)
    let mut node_ids: Vec<_> = nodes.keys().collect();
    node_ids.sort();
    
    for node_id in node_ids {
        hasher.update(node_id.0.as_bytes());
        let node = &nodes[node_id];
        hasher.update(&[node.autonomy_ceiling.as_u8()]);
        hasher.update(&node.resource_bounds.cpu_time_ms.to_le_bytes());
        hasher.update(&node.resource_bounds.memory_bytes.to_le_bytes());
        hasher.update(&node.resource_bounds.token_limit.to_le_bytes());
        hasher.update(&node.resource_bounds.iteration_cap.to_le_bytes());
    }
    
    // Hash edges (sorted for determinism)
    let mut sorted_edges: Vec<_> = edges.to_vec();
    sorted_edges.sort();
    
    for (from, to) in sorted_edges {
        hasher.update(from.0.as_bytes());
        hasher.update(to.0.as_bytes());
    }
    
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AutonomyLevel, DirectiveSet, ResourceCaps,
    };
    use std::collections::BTreeMap;

    fn create_test_node_spec(autonomy: AutonomyLevel, cpu_ms: u64) -> NodeSpecV2 {
        NodeSpecV2 {
            directives: DirectiveSet {
                directives: BTreeMap::new(),
            },
            autonomy_ceiling: autonomy,
            resource_bounds: ResourceCaps {
                cpu_time_ms: cpu_ms,
                memory_bytes: 1024 * 1024,
                token_limit: 1000,
                iteration_cap: 100,
            },
            expansion_type: None,
        }
    }

    #[test]
    fn test_resource_proof_within_limits() {
        let nodes = vec![
            create_test_node_spec(AutonomyLevel::L3, 1000),
            create_test_node_spec(AutonomyLevel::L3, 2000),
        ];
        
        let limits = SystemLimits {
            max_autonomy: AutonomyLevel::L5,
            max_resources: ResourceCaps {
                cpu_time_ms: 10000,
                memory_bytes: 10 * 1024 * 1024,
                token_limit: 10000,
                iteration_cap: 1000,
            },
            max_nodes: 100,
            max_edges: 1000,
        };
        
        let proof = ResourceProof::verify_bounds(&nodes, &limits);
        assert!(proof.is_ok());
        
        let proof = proof.unwrap();
        assert!(proof.within_system_limits);
        assert_eq!(proof.total_cpu_ms, 3000);
    }

    #[test]
    fn test_resource_proof_exceeds_limits() {
        let nodes = vec![
            create_test_node_spec(AutonomyLevel::L3, 10000),
            create_test_node_spec(AutonomyLevel::L3, 20000),
        ];
        
        let limits = SystemLimits {
            max_autonomy: AutonomyLevel::L5,
            max_resources: ResourceCaps {
                cpu_time_ms: 1000, // Too low
                memory_bytes: 10 * 1024 * 1024,
                token_limit: 10000,
                iteration_cap: 1000,
            },
            max_nodes: 100,
            max_edges: 1000,
        };
        
        let proof = ResourceProof::verify_bounds(&nodes, &limits);
        assert!(proof.is_err());
    }

    #[test]
    fn test_validation_hash_deterministic() {
        let graph_id = GraphId::new();
        let mut nodes = HashMap::new();
        
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        
        nodes.insert(n1, create_test_node_spec(AutonomyLevel::L3, 1000));
        nodes.insert(n2, create_test_node_spec(AutonomyLevel::L3, 2000));
        
        let edges = vec![(n1, n2)];
        
        let hash1 = compute_validation_hash(graph_id, &nodes, &edges);
        let hash2 = compute_validation_hash(graph_id, &nodes, &edges);
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_validation_hash_different_graphs() {
        let graph_id1 = GraphId::new();
        let graph_id2 = GraphId::new();
        
        let mut nodes = HashMap::new();
        let n1 = NodeId::new();
        nodes.insert(n1, create_test_node_spec(AutonomyLevel::L3, 1000));
        
        let edges = vec![];
        
        let hash1 = compute_validation_hash(graph_id1, &nodes, &edges);
        let hash2 = compute_validation_hash(graph_id2, &nodes, &edges);
        
        // Different graph IDs should produce different hashes
        assert_ne!(hash1, hash2);
    }
}
