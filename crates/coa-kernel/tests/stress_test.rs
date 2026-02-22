//! Stress Test - 10,000 nodes (v2.0)
//! 
//! Run with: cargo nextest run --package coa-kernel -- stress
//! Or: cargo test --package coa-kernel --test stress_test
//!
use coa_kernel::prelude::*;
use coa_kernel::DirectiveSet;
use std::collections::BTreeMap;
use std::time::Instant;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

#[test]
fn stress_test_10k_nodes() {
    println!("\n[STRESS TEST] Creating 10,000 nodes with v2.0...");
    
    let start = Instant::now();
    
    // Create signing key for validation
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    
    // Create builder
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    // Create 10,000 nodes
    for i in 0..10_000 {
        let spec = NodeSpecV2 {
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
        };
        builder.add_node(spec);
        
        if i % 2000 == 0 {
            print!("\r  Progress: {}/10000", i);
        }
    }
    println!("\r  Progress: 10000/10000");
    
    // Validate (construction phase)
    let validated = builder.validate(&signing_key).expect("Validation should succeed");
    
    let duration = start.elapsed();
    let ops_per_sec = 10_000.0 / duration.as_secs_f64();
    
    println!("  Completed in {:.2}s ({:.0} ops/sec)", duration.as_secs_f64(), ops_per_sec);
    println!("  Final node count: {}", validated.node_count());
    
    // Performance requirement: under 2 seconds for 10k nodes
    assert!(
        duration < std::time::Duration::from_secs(2),
        "Stress test too slow: {:.2}s (target: <2s)",
        duration.as_secs_f64()
    );
    assert_eq!(validated.node_count(), 10_000, "Expected 10,000 nodes");
    
    println!("  ✓ Stress test passed\n");
}

#[test]
fn stress_test_graph_with_edges() {
    println!("\n[STRESS TEST] Creating graph with 1000 nodes and edges...");
    
    let start = Instant::now();
    
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    // Add nodes
    let mut node_ids = Vec::new();
    for _ in 0..1000 {
        let spec = NodeSpecV2 {
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
        };
        node_ids.push(builder.add_node(spec));
    }
    
    // Add edges (chain pattern)
    for i in 0..node_ids.len() - 1 {
        builder.add_edge(node_ids[i], node_ids[i + 1]).expect("Edge should be added");
    }
    
    let validated = builder.validate(&signing_key).expect("Validation should succeed");
    
    let duration = start.elapsed();
    
    println!("  Completed in {:.2}s", duration.as_secs_f64());
    println!("  Nodes: {}, Edges: {}", validated.node_count(), validated.edge_count());
    
    assert_eq!(validated.node_count(), 1000);
    assert_eq!(validated.edge_count(), 999);
    
    println!("  ✓ Graph with edges test passed\n");
}
