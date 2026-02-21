use cog_kernel::api::*;
use cog_kernel::handle::KernelHandle;
use cog_kernel::types::*;

#[test]
fn test_kernel_handle_create_graph() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let stats = kernel.graph_stats(graph_id).unwrap();
    assert_eq!(stats.node_count, 0);
    assert_eq!(stats.edge_count, 0);
    assert_eq!(stats.graph_type, GraphType::ProductionDAG);
    assert!(!stats.is_closed);
}

#[test]
fn test_kernel_handle_close_graph() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    kernel.close_graph(graph_id).unwrap();
    
    let stats = kernel.graph_stats(graph_id).unwrap();
    assert!(stats.is_closed);
    
    // Can't add nodes to closed graph
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    let result = kernel.add_node(graph_id, spec);
    assert!(result.is_err());
}

#[test]
fn test_kernel_handle_add_node_and_edge() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node1 = kernel.add_node(graph_id, spec.clone()).unwrap();
    let node2 = kernel.add_node(graph_id, spec).unwrap();
    
    kernel.add_edge(graph_id, node1, node2).unwrap();
    
    let stats = kernel.graph_stats(graph_id).unwrap();
    assert_eq!(stats.node_count, 2);
    assert_eq!(stats.edge_count, 1);
}

#[test]
fn test_kernel_handle_rejects_cycle_in_production() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node1 = kernel.add_node(graph_id, spec.clone()).unwrap();
    let node2 = kernel.add_node(graph_id, spec).unwrap();
    
    kernel.add_edge(graph_id, node1, node2).unwrap();
    
    // Creating a cycle should fail
    let result = kernel.add_edge(graph_id, node2, node1);
    assert!(result.is_err());
}

#[test]
fn test_kernel_handle_allows_cycle_in_sandbox() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::SandboxGraph).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node1 = kernel.add_node(graph_id, spec.clone()).unwrap();
    let node2 = kernel.add_node(graph_id, spec).unwrap();
    
    kernel.add_edge(graph_id, node1, node2).unwrap();
    
    // Creating a cycle should succeed in sandbox
    let result = kernel.add_edge(graph_id, node2, node1);
    assert!(result.is_ok());
}

#[test]
fn test_kernel_handle_token_lifecycle() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node_id = kernel.add_node(graph_id, spec).unwrap();
    
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024 * 1024,
        token_limit: 1000,
        iteration_cap: 100,
    };
    
    // Issue token
    let token = kernel.issue_token(node_id, AutonomyLevel::L3, caps).unwrap();
    assert_eq!(token.node_id, node_id);
    assert_eq!(token.autonomy_level, AutonomyLevel::L3);
    
    // Validate token
    let report = kernel.validate_token(&token).unwrap();
    assert!(report.valid);
    assert!(report.signature_valid);
    
    // Downgrade token
    let downgraded = kernel.downgrade_token(&token, AutonomyLevel::L1).unwrap();
    assert_eq!(downgraded.autonomy_level, AutonomyLevel::L1);
    
    // Cannot upgrade via downgrade
    let result = kernel.downgrade_token(&downgraded, AutonomyLevel::L5);
    assert!(result.is_err());
}

#[test]
fn test_kernel_handle_state_transitions() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node_id = kernel.add_node(graph_id, spec).unwrap();
    
    // Initial state
    let state = kernel.current_state(node_id).unwrap();
    assert_eq!(state, NodeState::Created);
    
    // Get allowed transitions
    let allowed = kernel.allowed_transitions(node_id).unwrap();
    assert!(allowed.contains(&NodeState::Isolated));
    assert!(allowed.contains(&NodeState::Frozen));
    
    // Issue token for transition
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024 * 1024,
        token_limit: 1000,
        iteration_cap: 100,
    };
    let token = kernel.issue_token(node_id, AutonomyLevel::L2, caps).unwrap();
    
    // Perform transition
    let receipt = kernel.transition(node_id, NodeState::Isolated, &token).unwrap();
    assert_eq!(receipt.from_state, NodeState::Created);
    assert_eq!(receipt.to_state, NodeState::Isolated);
    assert!(receipt.token_validated);
    
    // Verify new state
    let state = kernel.current_state(node_id).unwrap();
    assert_eq!(state, NodeState::Isolated);
}

#[test]
fn test_kernel_handle_freeze_node() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node_id = kernel.add_node(graph_id, spec).unwrap();
    
    // Freeze node
    kernel.freeze_node(node_id).unwrap();
    
    // Verify frozen state
    let state = kernel.current_state(node_id).unwrap();
    assert_eq!(state, NodeState::Frozen);
}

#[test]
fn test_kernel_handle_deactivate_node() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node_id = kernel.add_node(graph_id, spec).unwrap();
    
    // Deactivate node (should succeed)
    kernel.deactivate_node(node_id).unwrap();
    
    // Deactivating again should also succeed (idempotent)
    kernel.deactivate_node(node_id).unwrap();
}

#[test]
fn test_kernel_handle_compliance_validation() {
    let kernel = KernelHandle::new();
    
    let action = ProposedAction {
        action_type: ActionType::CreateGraph,
        node_id: None,
        graph_id: None,
        requested_caps: None,
        target_state: None,
    };
    
    let report = kernel.validate_action(action).unwrap();
    assert!(report.approved);
    assert!(report.resource_check_passed);
    assert!(report.policy_check_passed);
}

#[test]
fn test_kernel_handle_event_logging() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    // Events are logged automatically during operations
    let stats = kernel.graph_stats(graph_id).unwrap();
    assert_eq!(stats.graph_type, GraphType::ProductionDAG);
    
    // Verify integrity
    let report = kernel.verify_integrity().unwrap();
    assert!(report.valid);
    assert!(report.events_checked > 0);
}

#[test]
fn test_kernel_handle_schedule() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node_id = kernel.add_node(graph_id, spec).unwrap();
    
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024 * 1024,
        token_limit: 1000,
        iteration_cap: 100,
    };
    let token = kernel.issue_token(node_id, AutonomyLevel::L2, caps).unwrap();
    
    // Schedule node
    let schedule_token = kernel.schedule(node_id, &token).unwrap();
    assert_eq!(schedule_token.node_id, node_id);
    
    // Cancel schedule
    kernel.cancel(schedule_token).unwrap();
}

#[test]
fn test_kernel_handle_api_version() {
    use cog_kernel::api::Compatibility;
    
    let kernel = KernelHandle::new();
    
    let version = kernel.api_version();
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 0);
    assert_eq!(version.patch, 0);
    
    // Check compatibility with same version
    let compat = kernel.check_compatibility(ApiVersion {
        major: 1,
        minor: 0,
        patch: 0,
    });
    assert_eq!(compat, Compatibility::Compatible);
    
    // Check with lower minor/patch version (we're newer, so compatible)
    let compat = kernel.check_compatibility(ApiVersion {
        major: 1,
        minor: 0,
        patch: 0, // same
    });
    assert_eq!(compat, Compatibility::Compatible);
    
    // Check with higher minor (we're older than expected, so deprecated but works)
    let compat = kernel.check_compatibility(ApiVersion {
        major: 1,
        minor: 5,
        patch: 0,
    });
    assert_eq!(compat, Compatibility::Deprecated);
    
    // Check incompatibility with different major version
    let compat = kernel.check_compatibility(ApiVersion {
        major: 2,
        minor: 0,
        patch: 0,
    });
    match compat {
        Compatibility::Incompatible(changes) => {
            assert!(!changes.is_empty());
        }
        _ => panic!("Expected Incompatible"),
    }
}

#[test]
fn test_kernel_handle_policy_query() {
    let kernel = KernelHandle::new();
    
    let policy = kernel.query_policy(PolicyScope::Global).unwrap();
    assert!(policy.require_token_for_all_actions);
}

#[test]
fn test_kernel_handle_resource_check() {
    let kernel = KernelHandle::new();
    
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024 * 1024,
        token_limit: 1000,
        iteration_cap: 100,
    };
    
    let availability = kernel.check_resources(caps).unwrap();
    assert!(availability.available);
    assert!(availability.cpu_time_remaining_ms > 0);
    assert!(availability.memory_remaining_bytes > 0);
}

#[tokio::test]
async fn test_kernel_handle_wait_for_completion() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node_id = kernel.add_node(graph_id, spec).unwrap();
    
    // Just test that wait_for_completion works
    let result = kernel.wait_for_completion(
        node_id, 
        std::time::Duration::from_millis(1)
    ).await.unwrap();
    
    assert!(result.success);
    assert_eq!(result.node_id, node_id);
}
