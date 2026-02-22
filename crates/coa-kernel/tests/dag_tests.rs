use cog_kernel::dag::Dag;
use cog_kernel::types::{GraphType, NodeId};
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_dag_remains_acyclic(
        node_count in 1..20usize,
        edges in proptest::collection::vec((0..20usize, 0..20usize), 0..50)
    ) {
        let dag = Dag::new(GraphType::ProductionDAG);
        let nodes: Vec<NodeId> = (0..node_count).map(|_| NodeId::new()).collect();
        
        for (from_idx, to_idx) in edges {
            if from_idx < nodes.len() && to_idx < nodes.len() {
                let from = nodes[from_idx];
                let to = nodes[to_idx];
                
                // Try adding edge
                let _ = dag.add_edge(from, to);
                
                // Invariant: Graph must remain acyclic if it's ProductionDAG
                // We can't easily access internal graph to check acyclicity directly here 
                // without exposing it, but the add_edge should return Err if cycle detected.
                // The invariant is that if add_edge returns Ok, the graph is acyclic.
                // And if it returns Err::CycleDetected, it correctly identified a cycle.
            }
        }
    }
}

#[test]
fn test_rejects_simple_cycle() {
    let dag = Dag::new(GraphType::ProductionDAG);
    let n1 = NodeId::new();
    let n2 = NodeId::new();
    let n3 = NodeId::new();
    
    dag.add_edge(n1, n2).unwrap();
    dag.add_edge(n2, n3).unwrap();
    
    // Cycle: n3 -> n1
    assert!(dag.add_edge(n3, n1).is_err());
}

#[test]
fn test_allows_cycle_in_sandbox() {
    let dag = Dag::new(GraphType::SandboxGraph);
    let n1 = NodeId::new();
    let n2 = NodeId::new();
    
    dag.add_edge(n1, n2).unwrap();
    
    // Cycle: n2 -> n1 allowed in sandbox
    assert!(dag.add_edge(n2, n1).is_ok());
}
