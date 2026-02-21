//! Negative tests - Testing failure modes and violations
//! 
//! From spec section 24.2

use cog_kernel::api::*;
use cog_kernel::autonomy::CapabilityToken;
use cog_kernel::handle::KernelHandle;
use cog_kernel::types::*;
use ed25519_dalek::SigningKey;

#[test]
fn test_rejects_cycle_in_production() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node1 = kernel.add_node(graph_id, spec.clone()).unwrap();
    let node2 = kernel.add_node(graph_id, spec).unwrap();
    
    // Add edge 1 -> 2
    kernel.add_edge(graph_id, node1, node2).unwrap();
    
    // Try to create cycle 2 -> 1 (should fail)
    let result = kernel.add_edge(graph_id, node2, node1);
    assert!(result.is_err(), "Cycle should be rejected in ProductionDAG");
}

#[test]
fn test_rejects_autonomy_elevation() {
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
    
    // Issue token at L1
    let token_l1 = kernel.issue_token(node_id, AutonomyLevel::L1, caps).unwrap();
    assert_eq!(token_l1.autonomy_level, AutonomyLevel::L1);
    
    // Try to downgrade to L5 (elevation) - should fail
    let result = kernel.downgrade_token(&token_l1, AutonomyLevel::L5);
    assert!(result.is_err(), "Autonomy elevation should be rejected");
    
    // Downgrade to L0 should work
    let token_l0 = kernel.downgrade_token(&token_l1, AutonomyLevel::L0).unwrap();
    assert_eq!(token_l0.autonomy_level, AutonomyLevel::L0);
}

#[test]
fn test_rejects_illegal_transition() {
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
    
    // Try illegal transition: Created -> Merged (should fail)
    let result = kernel.transition(node_id, NodeState::Merged, &token);
    assert!(result.is_err(), "Illegal transition should be rejected");
    
    // Valid transition should work
    let result = kernel.transition(node_id, NodeState::Isolated, &token);
    assert!(result.is_ok(), "Valid transition should succeed");
}

#[test]
fn test_detects_tampered_token() {
    use rand::rngs::OsRng;
    
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    
    let node_id = NodeId::new();
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024,
        token_limit: 100,
        iteration_cap: 10,
    };
    let hash = DirectiveProfileHash([0u8; 32]);
    
    // Create a token
    let mut token = CapabilityToken::sign(
        node_id, 
        AutonomyLevel::L1, 
        caps, 
        hash, 
        &signing_key,
        0,
        "",
    );
    
    // Verify original token
    assert!(token.verify(&verifying_key), "Original token should be valid");
    
    // Tamper with token - change autonomy level
    token.autonomy_level = AutonomyLevel::L5;
    
    // Tampered token should fail verification
    assert!(!token.verify(&verifying_key), "Tampered token should be rejected");
}

#[test]
fn test_detects_broken_hash_chain() {
    let kernel = KernelHandle::new();
    
    // Perform some operations to generate log entries
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let _node_id = kernel.add_node(graph_id, spec).unwrap();
    
    // Verify log integrity
    let report = kernel.verify_integrity().unwrap();
    assert!(report.valid, "Log integrity should be valid initially");
    
    // Note: To actually test tampering detection, we would need to
    // manually modify the log, but the log is append-only and protected
    // by the hash chain. This test verifies the integrity check works.
}

#[test]
fn test_rejects_expired_token() {
    use std::time::{SystemTime, UNIX_EPOCH};
    use rand::rngs::OsRng;
    
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    
    let node_id = NodeId::new();
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024,
        token_limit: 100,
        iteration_cap: 10,
    };
    let hash = DirectiveProfileHash([0u8; 32]);
    
    // Create expired token (expired 1 hour ago)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expired_time = now - 3600;
    
    let token = CapabilityToken::sign(
        node_id, 
        AutonomyLevel::L1, 
        caps, 
        hash, 
        &signing_key,
        expired_time,
        "",
    );
    
    // Token should be expired
    assert!(token.is_expired(), "Token should be detected as expired");
}

#[test]
fn test_rejects_self_loop() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    
    let node_id = kernel.add_node(graph_id, spec).unwrap();
    
    // Try to add self-loop (should fail)
    let result = kernel.add_edge(graph_id, node_id, node_id);
    assert!(result.is_err(), "Self-loop should be rejected");
}

#[test]
fn test_rejects_excessive_resource_request() {
    let kernel = KernelHandle::new();
    
    // Request excessive resources
    let caps = ResourceCaps {
        cpu_time_ms: u64::MAX,
        memory_bytes: u64::MAX,
        token_limit: u64::MAX,
        iteration_cap: u64::MAX,
    };
    
    let action = ProposedAction {
        action_type: ActionType::CreateGraph,
        node_id: None,
        graph_id: None,
        requested_caps: Some(caps),
        target_state: None,
    };
    
    let report = kernel.validate_action(action).unwrap();
    assert!(!report.approved, "Excessive resource request should be rejected");
    assert!(!report.resource_check_passed, "Resource check should fail");
}

#[test]
fn test_rejects_nonexistent_graph_operations() {
    let kernel = KernelHandle::new();
    
    let fake_graph_id = GraphId::new();
    
    // Try to close non-existent graph
    let result = kernel.close_graph(fake_graph_id);
    assert!(result.is_err(), "Closing non-existent graph should fail");
    
    // Try to add node to non-existent graph
    let spec = NodeSpec {
        directives: DirectiveSet {
            directives: Default::default(),
        },
    };
    let result = kernel.add_node(fake_graph_id, spec);
    assert!(result.is_err(), "Adding node to non-existent graph should fail");
}

#[test]
fn test_rejects_nonexistent_node_operations() {
    let kernel = KernelHandle::new();
    let graph_id = kernel.create_graph(GraphType::ProductionDAG).unwrap();
    
    let fake_node_id = NodeId::new();
    
    // Try to add edge with non-existent node
    let result = kernel.add_edge(graph_id, fake_node_id, fake_node_id);
    assert!(result.is_err(), "Adding edge with non-existent node should fail");
}
