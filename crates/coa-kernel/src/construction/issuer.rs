//! Token Issuer (v2.0)
//!
//! Issues capability tokens during the construction phase.
//! All token parameters are encoded at construction time.

use crate::autonomy::CapabilityToken;
use crate::types::v2::NodeSpecV2;
use crate::types::{DirectiveProfileHash, GraphId, NodeId};
use ed25519_dalek::SigningKey;
use std::collections::HashMap;

/// Issued tokens collection
#[derive(Debug, Clone)]
pub struct IssuedTokens {
    pub graph_id: GraphId,
    pub tokens: HashMap<NodeId, CapabilityToken>,
    pub issued_at: u64,
}

impl IssuedTokens {
    /// Create a new issued tokens collection
    pub fn new(graph_id: GraphId) -> Self {
        let issued_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            graph_id,
            tokens: HashMap::new(),
            issued_at,
        }
    }
    
    /// Get a token for a specific node
    pub fn get_token(&self, node_id: NodeId) -> Option<&CapabilityToken> {
        self.tokens.get(&node_id)
    }
    
    /// Get all node IDs with tokens
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.tokens.keys().copied()
    }
    
    /// Count of tokens issued
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

/// Token issuer for construction phase
///
/// Issues capability tokens with all policy parameters encoded.
pub struct TokenIssuer {
    signing_key: SigningKey,
    default_expiry_secs: u64,
}

impl TokenIssuer {
    /// Create a new token issuer
    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            signing_key,
            default_expiry_secs: 3600, // 1 hour
        }
    }
    
    /// Create with custom expiry
    pub fn with_expiry(signing_key: SigningKey, expiry_secs: u64) -> Self {
        Self {
            signing_key,
            default_expiry_secs: expiry_secs,
        }
    }
    
    /// Issue tokens for all nodes in a graph
    pub fn issue_for_graph(
        &self,
        graph_id: GraphId,
        nodes: &HashMap<NodeId, NodeSpecV2>,
    ) -> IssuedTokens {
        let mut issued = IssuedTokens::new(graph_id);
        let expires_at = issued.issued_at + self.default_expiry_secs;
        
        for (node_id, spec) in nodes {
            let token = self.issue_single_token(
                *node_id,
                spec,
                expires_at,
                "execute",
            );
            
            issued.tokens.insert(*node_id, token);
        }
        
        issued
    }
    
    /// Issue a single capability token
    fn issue_single_token(
        &self,
        node_id: NodeId,
        spec: &NodeSpecV2,
        expires_at: u64,
        operation: &str,
    ) -> CapabilityToken {
        CapabilityToken::sign(
            node_id,
            spec.autonomy_ceiling,
            spec.resource_bounds,
            DirectiveProfileHash([0u8; 32]), // TODO: Compute actual directive hash
            &self.signing_key,
            expires_at,
            operation,
        )
    }
    
    /// Issue a token for a specific operation
    pub fn issue_bound_token(
        &self,
        node_id: NodeId,
        spec: &NodeSpecV2,
        operation: &str,
    ) -> CapabilityToken {
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + self.default_expiry_secs;
        
        self.issue_single_token(node_id, spec, expires_at, operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AutonomyLevel, DirectiveSet, ResourceCaps,
    };
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
    fn test_issue_tokens_for_graph() {
        let signing_key = create_signing_key();
        let issuer = TokenIssuer::new(signing_key);
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        nodes.insert(NodeId::new(), create_test_spec());
        nodes.insert(NodeId::new(), create_test_spec());
        
        let issued = issuer.issue_for_graph(graph_id, &nodes);
        
        assert_eq!(issued.token_count(), 2);
        assert_eq!(issued.graph_id, graph_id);
    }

    #[test]
    fn test_get_token_for_node() {
        let signing_key = create_signing_key();
        let issuer = TokenIssuer::new(signing_key);
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        let node_id = NodeId::new();
        nodes.insert(node_id, create_test_spec());
        
        let issued = issuer.issue_for_graph(graph_id, &nodes);
        
        assert!(issued.get_token(node_id).is_some());
        assert!(issued.get_token(NodeId::new()).is_none());
    }

    #[test]
    fn test_tokens_have_correct_autonomy_level() {
        let signing_key = create_signing_key();
        let issuer = TokenIssuer::new(signing_key);
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        let node_id = NodeId::new();
        
        let mut spec = create_test_spec();
        spec.autonomy_ceiling = AutonomyLevel::L4;
        nodes.insert(node_id, spec);
        
        let issued = issuer.issue_for_graph(graph_id, &nodes);
        let token = issued.get_token(node_id).unwrap();
        
        assert_eq!(token.autonomy_level, AutonomyLevel::L4);
    }

    #[test]
    fn test_tokens_have_correct_resource_caps() {
        let signing_key = create_signing_key();
        let issuer = TokenIssuer::new(signing_key);
        let graph_id = GraphId::new();
        
        let mut nodes = HashMap::new();
        let node_id = NodeId::new();
        
        let mut spec = create_test_spec();
        spec.resource_bounds.cpu_time_ms = 5000;
        spec.resource_bounds.memory_bytes = 10 * 1024 * 1024;
        nodes.insert(node_id, spec);
        
        let issued = issuer.issue_for_graph(graph_id, &nodes);
        let token = issued.get_token(node_id).unwrap();
        
        assert_eq!(token.caps.cpu_time_ms, 5000);
        assert_eq!(token.caps.memory_bytes, 10 * 1024 * 1024);
    }
}
