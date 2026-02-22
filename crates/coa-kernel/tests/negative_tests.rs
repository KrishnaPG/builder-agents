//! Negative Tests - v2.0
//!
//! Tests for construction-phase rejection of invalid graphs.

use cog_kernel::prelude::*;
use cog_kernel::DirectiveSet;
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
fn test_rejects_cycle_in_production() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let n1 = builder.add_node(create_test_spec());
    let n2 = builder.add_node(create_test_spec());
    
    // Add edge 1 -> 2
    builder.add_edge(n1, n2).unwrap();
    
    // Try to create cycle 2 -> 1 (should fail)
    let result = builder.add_edge(n2, n1);
    assert!(result.is_err(), "Cycle should be rejected in ProductionDAG");
}

#[test]
fn test_rejects_self_loop() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let n1 = builder.add_node(create_test_spec());
    
    // Self-loop should fail
    let result = builder.add_edge(n1, n1);
    assert!(result.is_err(), "Self-loop should be rejected");
}

#[test]
fn test_rejects_edge_to_nonexistent_node() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let n1 = builder.add_node(create_test_spec());
    let fake_node = NodeId::new(); // Not in builder
    
    let result = builder.add_edge(n1, fake_node);
    assert!(result.is_err(), "Edge to non-existent node should be rejected");
}

#[test]
fn test_validation_rejects_autonomy_above_ceiling() {
    let signing_key = create_signing_key();
    
    // Create builder with L3 max autonomy
    let limits = SystemLimits {
        max_autonomy: AutonomyLevel::L3,
        max_resources: ResourceCaps {
            cpu_time_ms: 10000,
            memory_bytes: 100 * 1024 * 1024,
            token_limit: 10000,
            iteration_cap: 1000,
        },
        max_nodes: 100,
        max_edges: 1000,
    };
    
    let mut builder = GraphBuilder::with_limits(GraphType::ProductionDAG, limits);
    
    // Add node with L5 autonomy (exceeds L3 ceiling)
    let spec = NodeSpecV2 {
        autonomy_ceiling: AutonomyLevel::L5,
        ..create_test_spec()
    };
    builder.add_node(spec);
    
    // Validation should fail
    let result = builder.validate(&signing_key);
    assert!(result.is_err(), "Autonomy above ceiling should be rejected at validation");
}

#[test]
fn test_validation_rejects_impossible_resource_bounds() {
    let signing_key = create_signing_key();
    
    let limits = SystemLimits {
        max_autonomy: AutonomyLevel::L5,
        max_resources: ResourceCaps {
            cpu_time_ms: 5000,
            memory_bytes: 100 * 1024 * 1024,
            token_limit: 10000,
            iteration_cap: 1000,
        },
        max_nodes: 100,
        max_edges: 1000,
    };
    
    let mut builder = GraphBuilder::with_limits(GraphType::ProductionDAG, limits);
    
    // Add node with resource bounds exceeding system limits
    let spec = NodeSpecV2 {
        resource_bounds: ResourceCaps {
            cpu_time_ms: 10000, // Exceeds 5000 limit
            memory_bytes: 1024 * 1024,
            token_limit: 1000,
            iteration_cap: 100,
        },
        ..create_test_spec()
    };
    builder.add_node(spec);
    
    // Validation should fail
    let result = builder.validate(&signing_key);
    assert!(result.is_err(), "Impossible resource bounds should be rejected at validation");
}

#[test]
fn test_sandbox_allows_cycles() {
    let signing_key = create_signing_key();
    let mut builder = GraphBuilder::new(GraphType::SandboxGraph);
    
    let n1 = builder.add_node(create_test_spec());
    let n2 = builder.add_node(create_test_spec());
    
    builder.add_edge(n1, n2).unwrap();
    
    // Cycle should be allowed in sandbox
    let result = builder.add_edge(n2, n1);
    assert!(result.is_ok(), "Cycle should be allowed in SandboxGraph");
    
    // Validation should succeed
    let result = builder.validate(&signing_key);
    assert!(result.is_ok(), "Sandbox graph with cycle should validate");
}

#[test]
fn test_construction_rejects_invalid_graph_structure() {
    let signing_key = create_signing_key();
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    // Create a more complex graph that would be valid structurally
    // but we want to test that validation catches issues
    let n1 = builder.add_node(create_test_spec());
    let n2 = builder.add_node(create_test_spec());
    let n3 = builder.add_node(create_test_spec());
    
    // Valid edges
    builder.add_edge(n1, n2).unwrap();
    builder.add_edge(n2, n3).unwrap();
    
    // Should validate successfully
    let validated = builder.validate(&signing_key);
    assert!(validated.is_ok());
    
    let validated = validated.unwrap();
    assert_eq!(validated.node_count(), 3);
    assert_eq!(validated.edge_count(), 2);
}

#[test]
fn test_empty_graph_validates() {
    let signing_key = create_signing_key();
    let builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    // Empty graph should validate
    let result = builder.validate(&signing_key);
    assert!(result.is_ok());
    
    let validated = result.unwrap();
    assert_eq!(validated.node_count(), 0);
    assert_eq!(validated.edge_count(), 0);
}
