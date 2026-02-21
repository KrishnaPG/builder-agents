//! Stress Test - 10,000 nodes
//! 
//! Run with: cargo nextest run --package cog_kernel -- stress
//! Or: cargo test --package cog_kernel --test stress_test

use cog_kernel::*;
use std::time::Instant;

#[test]
fn stress_test_10k_nodes() {
    println!("\n[STRESS TEST] Creating 10,000 nodes...");
    
    let start = Instant::now();
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    let spec = NodeSpec { 
        directives: DirectiveSet { directives: Default::default() } 
    };
    
    // Create 10,000 nodes
    for i in 0..10_000 {
        let _ = kernel.add_node(graph_id, spec.clone());
        if i % 2000 == 0 {
            print!("\r  Progress: {}/10000", i);
        }
    }
    println!("\r  Progress: 10000/10000");
    
    let duration = start.elapsed();
    let ops_per_sec = 10_000.0 / duration.as_secs_f64();
    let stats = kernel.graph_stats(graph_id).unwrap();
    
    println!("  Completed in {:.2}s ({:.0} ops/sec)", duration.as_secs_f64(), ops_per_sec);
    println!("  Final node count: {}", stats.node_count);
    
    // Performance requirement: under 2 seconds for 10k nodes
    assert!(
        duration < std::time::Duration::from_secs(2),
        "Stress test too slow: {:.2}s (target: <2s)",
        duration.as_secs_f64()
    );
    assert_eq!(stats.node_count, 10_000, "Expected 10,000 nodes");
    
    println!("  ✓ Stress test passed\n");
}
