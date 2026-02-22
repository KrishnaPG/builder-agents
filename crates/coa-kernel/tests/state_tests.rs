use coa_kernel::state_machine::{allowed_transitions, validate_transition};
use coa_kernel::types::NodeState;
use proptest::prelude::*;

#[test]
fn test_created_transitions() {
    assert!(validate_transition(NodeState::Created, NodeState::Isolated).is_ok());
    assert!(validate_transition(NodeState::Created, NodeState::Frozen).is_ok());
    assert!(validate_transition(NodeState::Created, NodeState::Escalated).is_ok());
    
    // Invalid
    assert!(validate_transition(NodeState::Created, NodeState::Merged).is_err());
    assert!(validate_transition(NodeState::Created, NodeState::Executing).is_err());
}

#[test]
fn test_frozen_transitions() {
    // Frozen can only go to Escalated (or maybe Unfrozen back to previous state, but spec says Escalated)
    // Spec: Frozen -> Escalated.
    assert!(validate_transition(NodeState::Frozen, NodeState::Escalated).is_ok());
    
    assert!(validate_transition(NodeState::Frozen, NodeState::Executing).is_err());
}

proptest! {
    #[test]
    fn prop_all_transitions_are_subset_of_allowed(
        from in prop_oneof![
            Just(NodeState::Created),
            Just(NodeState::Isolated),
            Just(NodeState::Testing),
            Just(NodeState::Executing),
            Just(NodeState::Validating),
            Just(NodeState::Merged),
            Just(NodeState::Escalated),
            Just(NodeState::Frozen),
        ],
        to in prop_oneof![
            Just(NodeState::Created),
            Just(NodeState::Isolated),
            Just(NodeState::Testing),
            Just(NodeState::Executing),
            Just(NodeState::Validating),
            Just(NodeState::Merged),
            Just(NodeState::Escalated),
            Just(NodeState::Frozen),
        ]
    ) {
        let res = validate_transition(from, to);
        let allowed = allowed_transitions(from);
        
        if res.is_ok() {
            assert!(allowed.contains(&to));
        } else {
            assert!(!allowed.contains(&to));
        }
    }
}
