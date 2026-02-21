use crate::error::StateMachineError;
use crate::types::NodeState;

pub fn validate_transition(from: NodeState, to: NodeState) -> Result<(), StateMachineError> {
    if allowed(from, to) {
        Ok(())
    } else {
        Err(StateMachineError::IllegalTransition)
    }
}

pub fn allowed_transitions(from: NodeState) -> Vec<NodeState> {
    use NodeState::*;
    match from {
        Created => vec![Isolated, Frozen, Escalated],
        Isolated => vec![Testing, Frozen, Escalated],
        Testing => vec![Executing, Frozen, Escalated],
        Executing => vec![Validating, Frozen, Escalated],
        Validating => vec![Merged, Frozen, Escalated],
        Merged => vec![],
        Escalated => vec![],
        Frozen => vec![Escalated],
    }
}

fn allowed(from: NodeState, to: NodeState) -> bool {
    allowed_transitions(from).into_iter().any(|s| s == to)
}
