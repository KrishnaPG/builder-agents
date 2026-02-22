//! Functional tests for COA orchestration and error semantics.
//!
//! These tests exercise the CreatorOrchestratorAgent and the end-to-end error
//! model described in the Blueprint:
//! - execute_intent performs intent parsing, decomposition, and execution.
//! - Errors that require human intervention are surfaced explicitly.
//! - Retryable vs non-retryable errors are classified via COAError::is_retryable.

use coa_core::error::{COAError, DecompositionError, Goal, PoolError};
use coa_core::{COAConfig, CreatorOrchestratorAgent, UserIntent};

/// Helper: create a default test orchestrator.
fn make_orchestrator() -> CreatorOrchestratorAgent {
    let config = COAConfig::new();
    CreatorOrchestratorAgent::new(config)
}

/// Tenet: unimplemented execution surfaces RequiresHumanIntervention instead of
/// failing silently or panicking.
///
/// Current implementation returns a RequiresHumanIntervention wrapper when
/// execute_task is not fully implemented; this test anchors that behavior so
/// future changes keep the "human in the loop" semantics explicit.
#[tokio::test]
async fn execute_intent_requires_human_intervention() {
    let coa = make_orchestrator();
    let intent = UserIntent::new("Create a simple function");

    let result = coa.execute_intent(intent).await;
    assert!(result.is_err(), "execution should not silently succeed yet");

    let error = result.unwrap_err();
    match error {
        COAError::RequiresHumanIntervention { .. } => {}
        other => panic!("expected RequiresHumanIntervention, got {:?}", other),
    }

    assert!(error.requires_human());
}

/// Tenet: invalid intents are surfaced as InvalidIntent errors.
///
/// The exact parsing logic may evolve, but structurally invalid or unsupported
/// intents must result in COAError::InvalidIntent, rather than generic failures.
#[tokio::test]
async fn invalid_intent_produces_invalid_intent_error() {
    let coa = make_orchestrator();
    let intent = UserIntent::new("");

    let result = coa.execute_intent(intent).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        COAError::InvalidIntent(_) => {}
        other => panic!("expected InvalidIntent, got {:?}", other),
    }
}

/// Tenet: decomposition failures are wrapped as DecompositionFailed and are
/// treated as non-retryable.
#[test]
fn decomposition_failures_are_non_retryable() {
    let underlying = DecompositionError::UnsupportedGoal(Goal::Analyze);
    let err = COAError::DecompositionFailed(underlying);

    assert!(
        !err.is_retryable(),
        "decomposition failures should not be auto-retried"
    );
}

/// Tenet: certain runtime errors are marked retryable, so the orchestrator
/// (or higher layers) can safely attempt recovery strategies.
#[test]
fn runtime_errors_are_correctly_marked_retryable() {
    let agent_failed = COAError::AgentFailed("transient".into());
    let execution_failed = COAError::ExecutionFailed("transient".into());
    let timeout = COAError::Timeout { duration_secs: 30 };
    let pool_exhausted = COAError::PoolError(PoolError::PoolExhausted(10));

    assert!(agent_failed.is_retryable());
    assert!(execution_failed.is_retryable());
    assert!(timeout.is_retryable());
    assert!(pool_exhausted.is_retryable());
}

