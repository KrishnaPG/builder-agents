use cog_kernel::logging::{Event, EventLog};
use cog_kernel::types::{AutonomyLevel, DirectiveProfileHash, EventId, NodeId};

#[test]
fn test_log_integrity() {
    let log = EventLog::default();
    
    let e1 = Event {
        event_id: EventId::new(),
        timestamp: 1,
        node_id: NodeId::new(),
        autonomy_level: AutonomyLevel::L0,
        directive_hash: DirectiveProfileHash([0u8; 32]),
        action: "create".to_string(),
        result: "ok".to_string(),
        prev_hash: [0u8; 32], // Will be ignored/overwritten by append
        hash: [0u8; 32], // Will be overwritten
    };
    
    let _ = log.append(e1.clone());
    
    let e2 = Event {
        event_id: EventId::new(),
        timestamp: 2,
        node_id: NodeId::new(),
        autonomy_level: AutonomyLevel::L0,
        directive_hash: DirectiveProfileHash([0u8; 32]),
        action: "update".to_string(),
        result: "ok".to_string(),
        prev_hash: [0u8; 32],
        hash: [0u8; 32],
    };
    
    let _ = log.append(e2);
    
    assert!(log.verify_integrity().is_ok());
}
