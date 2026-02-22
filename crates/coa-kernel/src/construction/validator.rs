//! Construction Validator (v2.0)
//!
//! Performs all policy validation at construction time.
//! No policy validation happens at runtime - only integrity verification.

use crate::error::ValidationError;
use crate::types::v2::{NodeSpecV2, SystemLimits, ValidatedGraph, ValidationToken};
use crate::validated_graph::ResourceProof;
use crate::types::{GraphId, GraphType, NodeId};
use crate::validated_graph::{compute_validation_hash, ValidatedGraphConstructor};
use ed25519_dalek::{Signer, SigningKey};
use std::collections::{HashMap, HashSet};

/// Context for validation
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub system_limits: SystemLimits,
    pub graph_type: GraphType,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            system_limits: SystemLimits::default(),
            graph_type: GraphType::ProductionDAG,
        }
    }
}

/// Construction-time policy validator
pub struct ConstructionValidator {
    context: ValidationContext,
}

impl ConstructionValidator {
    /// Create a new validator with default context
    pub fn new() -> Self {
        Self {
            context: ValidationContext::default(),
        }
    }
    
    /// Create a new validator with custom context
    pub fn with_context(context: ValidationContext) -> Self {
        Self { context }
    }
    
    /// Validate a complete graph
    ///
    /// Performs all construction-time validations:
    /// 1. Graph structure (cycles, self-loops in production)
    /// 2. Autonomy ceilings
    /// 3. Resource bounds provability
    /// 4. Security pipeline completeness
    pub fn validate_graph(
        &self,
        graph_id: GraphId,
        graph_type: GraphType,
        nodes: &HashMap<NodeId, NodeSpecV2>,
        edges: &[(NodeId, NodeId)],
        signing_key: &SigningKey,
    ) -> Result<ValidatedGraph, ValidationError> {
        // 1. Validate graph structure
        self.validate_graph_structure(graph_type, nodes, edges)?;
        
        // 2. Validate node specifications
        let node_specs: Vec<_> = nodes.values().collect();
        self.validate_node_specs(&node_specs)?;
        
        // 3. Prove resource bounds
        let node_specs_ref: Vec<_> = node_specs.iter().map(|&n| n.clone()).collect();
        ResourceProof::verify_bounds(&node_specs_ref, &self.context.system_limits)?;
        
        // 4. Issue capability tokens
        let node_tokens = self.issue_node_tokens(graph_id, nodes, signing_key);
        
        // 5. Create validation token
        let validation_token = self.create_validation_token(
            graph_id,
            nodes,
            edges,
            signing_key,
        );
        
        // 6. Construct ValidatedGraph (sealed type)
        Ok(ValidatedGraphConstructor::construct(
            graph_id,
            validation_token,
            graph_type,
            nodes.clone(),
            edges.to_vec(),
            node_tokens,
        ))
    }
    
    /// Validate graph structure
    fn validate_graph_structure(
        &self,
        graph_type: GraphType,
        nodes: &HashMap<NodeId, NodeSpecV2>,
        edges: &[(NodeId, NodeId)],
    ) -> Result<(), ValidationError> {
        // Check for self-loops
        for (from, to) in edges {
            if from == to {
                return Err(ValidationError::SelfLoop);
            }
        }
        
        // Check for cycles in production DAG
        if matches!(graph_type, GraphType::ProductionDAG) {
            if self.has_cycle(nodes, edges) {
                return Err(ValidationError::CycleDetected);
            }
        }
        
        // Check that all edge endpoints exist
        for (from, to) in edges {
            if !nodes.contains_key(from) {
                return Err(ValidationError::InvalidGraphStructure);
            }
            if !nodes.contains_key(to) {
                return Err(ValidationError::InvalidGraphStructure);
            }
        }
        
        Ok(())
    }
    
    /// Detect cycles using DFS
    fn has_cycle(
        &self,
        nodes: &HashMap<NodeId, NodeSpecV2>,
        edges: &[(NodeId, NodeId)],
    ) -> bool {
        let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        
        // Build adjacency list
        for (from, to) in edges {
            adjacency.entry(*from).or_default().push(*to);
        }
        
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        
        fn dfs(
            node: NodeId,
            adjacency: &HashMap<NodeId, Vec<NodeId>>,
            visiting: &mut HashSet<NodeId>,
            visited: &mut HashSet<NodeId>,
        ) -> bool {
            if visiting.contains(&node) {
                return true; // Cycle detected
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
        
        for &node_id in nodes.keys() {
            if !visited.contains(&node_id) {
                if dfs(node_id, &adjacency, &mut visiting, &mut visited) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Validate all node specifications
    fn validate_node_specs(
        &self,
        nodes: &[&NodeSpecV2],
    ) -> Result<(), ValidationError> {
        for node in nodes {
            // Check autonomy ceiling
            if node.autonomy_ceiling.as_u8() > self.context.system_limits.max_autonomy.as_u8() {
                return Err(ValidationError::AutonomyCeilingExceeded);
            }
            
            // Check security pipeline completeness
            // (This is a placeholder - actual implementation would check directive structure)
            if !self.has_security_pipeline(node) {
                return Err(ValidationError::SecurityPipelineIncomplete);
            }
        }
        
        Ok(())
    }
    
    /// Check if node has complete security pipeline
    fn has_security_pipeline(&self, _node: &NodeSpecV2) -> bool {
        // Placeholder: In real implementation, check that directives contain
        // all required security stages
        true
    }
    
    /// Issue capability tokens for all nodes
    fn issue_node_tokens(
        &self,
        _graph_id: GraphId,
        nodes: &HashMap<NodeId, NodeSpecV2>,
        signing_key: &SigningKey,
    ) -> HashMap<NodeId, crate::autonomy::CapabilityToken> {
        use crate::autonomy::CapabilityToken;
        use crate::types::DirectiveProfileHash;
        
        let mut tokens = HashMap::new();
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + 3600; // 1 hour expiration
        
        for (node_id, spec) in nodes {
            let token = CapabilityToken::sign(
                *node_id,
                spec.autonomy_ceiling,
                spec.resource_bounds,
                DirectiveProfileHash([0u8; 32]), // TODO: Compute actual hash
                signing_key,
                expires_at,
                "execute",
            );
            
            tokens.insert(*node_id, token);
        }
        
        tokens
    }
    
    /// Create validation token for the graph
    fn create_validation_token(
        &self,
        graph_id: GraphId,
        nodes: &HashMap<NodeId, NodeSpecV2>,
        edges: &[(NodeId, NodeId)],
        signing_key: &SigningKey,
    ) -> ValidationToken {
        let validation_hash = compute_validation_hash(graph_id, nodes, edges);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expires_at = timestamp + 3600; // 1 hour expiration
        
        // Create message to sign
        let mut message = Vec::with_capacity(16 + 32 + 8 + 8);
        message.extend_from_slice(graph_id.0.as_bytes());
        message.extend_from_slice(&validation_hash);
        message.extend_from_slice(&timestamp.to_le_bytes());
        message.extend_from_slice(&expires_at.to_le_bytes());
        
        let signature = signing_key.sign(&message);
        
        ValidationToken {
            graph_id,
            validation_hash,
            timestamp,
            expires_at,
            signature,
        }
    }
}

impl Default for ConstructionValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AutonomyLevel, DirectiveSet, ResourceCaps};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;

    fn create_test_spec(autonomy: AutonomyLevel, cpu_ms: u64) -> NodeSpecV2 {
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

    fn create_signing_key() -> SigningKey {
        let mut csprng = OsRng;
        SigningKey::generate(&mut csprng)
    }

    #[test]
    fn test_valid_graph_passes() {
        let validator = ConstructionValidator::new();
        let signing_key = create_signing_key();
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        
        nodes.insert(n1, create_test_spec(AutonomyLevel::L3, 1000));
        nodes.insert(n2, create_test_spec(AutonomyLevel::L3, 2000));
        
        let edges = vec![(n1, n2)];
        
        let result = validator.validate_graph(
            graph_id,
            GraphType::ProductionDAG,
            &nodes,
            &edges,
            &signing_key,
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_cycle_detection() {
        let validator = ConstructionValidator::new();
        let signing_key = create_signing_key();
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let n3 = NodeId::new();
        
        nodes.insert(n1, create_test_spec(AutonomyLevel::L3, 1000));
        nodes.insert(n2, create_test_spec(AutonomyLevel::L3, 2000));
        nodes.insert(n3, create_test_spec(AutonomyLevel::L3, 3000));
        
        // n1 -> n2 -> n3 -> n1 (cycle)
        let edges = vec![(n1, n2), (n2, n3), (n3, n1)];
        
        let result = validator.validate_graph(
            graph_id,
            GraphType::ProductionDAG,
            &nodes,
            &edges,
            &signing_key,
        );
        
        assert!(matches!(result, Err(ValidationError::CycleDetected)));
    }

    #[test]
    fn test_autonomy_ceiling_check() {
        let context = ValidationContext {
            system_limits: SystemLimits {
                max_autonomy: AutonomyLevel::L3,
                ..SystemLimits::default()
            },
            graph_type: GraphType::ProductionDAG,
        };
        let validator = ConstructionValidator::with_context(context);
        let signing_key = create_signing_key();
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        let n1 = NodeId::new();
        
        // L5 exceeds L3 ceiling
        nodes.insert(n1, create_test_spec(AutonomyLevel::L5, 1000));
        
        let result = validator.validate_graph(
            graph_id,
            GraphType::ProductionDAG,
            &nodes,
            &[],
            &signing_key,
        );
        
        assert!(matches!(result, Err(ValidationError::AutonomyCeilingExceeded)));
    }

    #[test]
    fn test_sandbox_allows_cycle() {
        let validator = ConstructionValidator::new();
        let signing_key = create_signing_key();
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        
        nodes.insert(n1, create_test_spec(AutonomyLevel::L3, 1000));
        nodes.insert(n2, create_test_spec(AutonomyLevel::L3, 2000));
        
        // Cycle in sandbox
        let edges = vec![(n1, n2), (n2, n1)];
        
        let result = validator.validate_graph(
            graph_id,
            GraphType::SandboxGraph, // Sandbox allows cycles
            &nodes,
            &edges,
            &signing_key,
        );
        
        assert!(result.is_ok());
    }
}
