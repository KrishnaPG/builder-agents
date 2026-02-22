//! GraphBuilder Tests (v2.0)
//!
//! Tests for the construction phase using GraphBuilder.
//!
use coa_kernel::prelude::*;
use coa_kernel::DirectiveSet;
use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

fn create_signing_key() -> SigningKey {
    let mut csprng = OsRng;
    SigningKey::generate(&mut csprng)
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

#[test]
fn test_graph_builder_create() {
    let builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    assert_eq!(builder.graph_type(), GraphType::ProductionDAG);
    assert_eq!(builder.node_count(), 0);
    assert_eq!(builder.edge_count(), 0);
}

#[test]
fn test_graph_builder_add_node() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = create_test_spec();
    let node_id = builder.add_node(spec);
    
    assert_eq!(builder.node_count(), 1);
    assert!(builder.get_node(node_id).is_some());
}

#[test]
fn test_graph_builder_add_edge() {
    let signing_key = create_signing_key();
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = create_test_spec();
    let node1 = builder.add_node(spec.clone());
    let node2 = builder.add_node(spec);
    
    builder.add_edge(node1, node2).unwrap();
    
    assert_eq!(builder.edge_count(), 1);
    
    // Validate should succeed
    let validated = builder.validate(&signing_key);
    assert!(validated.is_ok());
}

#[test]
fn test_graph_builder_rejects_self_loop() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = create_test_spec();
    let node = builder.add_node(spec);
    
    let result = builder.add_edge(node, node);
    assert!(result.is_err());
}

#[test]
fn test_graph_builder_rejects_cycle() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = create_test_spec();
    let n1 = builder.add_node(spec.clone());
    let n2 = builder.add_node(spec.clone());
    let n3 = builder.add_node(spec);
    
    builder.add_edge(n1, n2).unwrap();
    builder.add_edge(n2, n3).unwrap();
    
    // n3 -> n1 would create cycle
    let result = builder.add_edge(n3, n1);
    assert!(result.is_err());
}

#[test]
fn test_validated_graph_sealed() {
    let signing_key = create_signing_key();
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = create_test_spec();
    let node = builder.add_node(spec);
    
    let validated = builder.validate(&signing_key).unwrap();
    
    // ValidatedGraph should contain the node
    assert_eq!(validated.node_count(), 1);
    assert!(validated.get_node_spec(node).is_some());
    assert!(validated.get_node_token(node).is_some());
}

#[test]
fn test_sandbox_allows_cycle() {
    let signing_key = create_signing_key();
    let mut builder = GraphBuilder::new(GraphType::SandboxGraph);
    
    let spec = create_test_spec();
    let n1 = builder.add_node(spec.clone());
    let n2 = builder.add_node(spec);
    
    builder.add_edge(n1, n2).unwrap();
    
    // Cycle should be allowed in sandbox
    let result = builder.add_edge(n2, n1);
    assert!(result.is_ok());
    
    // Validation should succeed
    let validated = builder.validate(&signing_key);
    assert!(validated.is_ok());
}

#[test]
fn test_graph_builder_node_count() {
    let signing_key = create_signing_key();
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    for _ in 0..100 {
        builder.add_node(create_test_spec());
    }
    
    assert_eq!(builder.node_count(), 100);
    
    let validated = builder.validate(&signing_key).unwrap();
    assert_eq!(validated.node_count(), 100);
}

#[test]
fn test_validated_graph_has_tokens() {
    let signing_key = create_signing_key();
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = create_test_spec();
    let node = builder.add_node(spec);
    
    let validated = builder.validate(&signing_key).unwrap();
    
    // Each node should have a capability token
    let token = validated.get_node_token(node);
    assert!(token.is_some());
    
    let token = token.unwrap();
    assert_eq!(token.node_id, node);
    assert_eq!(token.autonomy_level, AutonomyLevel::L3);
}

#[test]
fn test_graph_builder_with_system_limits() {
    let limits = SystemLimits {
        max_autonomy: AutonomyLevel::L4,
        max_resources: ResourceCaps {
            cpu_time_ms: 10000,
            memory_bytes: 100 * 1024 * 1024,
            token_limit: 10000,
            iteration_cap: 1000,
        },
        max_nodes: 100,
        max_edges: 1000,
    };
    
    let builder = GraphBuilder::with_limits(GraphType::ProductionDAG, limits);
    assert_eq!(builder.graph_type(), GraphType::ProductionDAG);
}
