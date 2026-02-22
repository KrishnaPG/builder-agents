//! Performance and scaling smoke-tests for graph construction and simulation.
//!
//! These are not full benchmarks (Criterion is used at the crate level for
//! that), but they exercise the core paths under moderately larger loads:
//! - Constructing graphs with increasing node counts.
//! - Running the COA simulator with larger construction/execution counts.
//!
//! The goal is to ensure the system remains functionally correct and does not
//! trivially blow up (panic, overflow, etc.) under typical scalability
//! scenarios described in the Blueprint.

use coa_kernel::construction::GraphBuilder;
use coa_kernel::prelude::*;
use coa_kernel::test_harness::{run_simulator, SimulatorConfig};
use coa_kernel::types::{AutonomyLevel, GraphType, ResourceCaps};
use ed25519_dalek::SigningKey;
use rand::{rngs::StdRng, SeedableRng};

/// Helper: create a minimal NodeSpecV2 used for scaling tests.
fn make_node_spec() -> NodeSpecV2 {
    let directives = DirectiveSet {
        directives: std::collections::BTreeMap::new(),
    };
    let resources = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024 * 1024,
        token_limit: 10_000,
        iteration_cap: 1_000,
    };
    NodeSpecV2::new(directives, AutonomyLevel::L3, resources)
}

/// Scaling scenario: construct a linear DAG with N nodes.
///
/// This is a simple but representative stress pattern: each node depends on
/// the previous one, yielding a chain whose size we can grow. The test
/// asserts only that validation succeeds and node/edge counts match.
fn construct_linear_dag(node_count: usize) {
    let mut rng = StdRng::seed_from_u64(10 + node_count as u64);
    let signing_key = SigningKey::generate(&mut rng);

    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    let spec = make_node_spec();

    let mut last = builder.add_node(spec.clone());
    for _ in 1..node_count {
        let next = builder.add_node(spec.clone());
        builder
            .add_edge(last, next)
            .expect("edge in linear DAG should be valid");
        last = next;
    }

    let validated = builder
        .validate(&signing_key)
        .expect("linear DAG must validate at scale");

    assert_eq!(validated.node_count(), node_count);
    assert_eq!(validated.edge_count(), node_count.saturating_sub(1));
}

/// Tenet: graph construction remains correct for moderately large DAGs.
///
/// We do not assert on timing here; instead we assert that the builder and
/// validator handle larger sizes without panicking or violating invariants.
#[test]
fn construct_medium_sized_dag() {
    construct_linear_dag(128);
}

/// Tenet: the simulator can execute mid-sized certification-style runs without
/// violating invariants.
///
/// This mirrors a scaled-down version of the certification configuration from
/// the Blueprint, asserting that all invariants still hold.
#[tokio::test]
async fn simulator_handles_medium_scale_run() {
    let config = SimulatorConfig {
        seed: 123,
        total_constructions: 500,
        total_executions: 500,
        stop_on_first_violation: true,
        verify_zero_runtime_policy: true,
    };

    let report = run_simulator(config).await;

    assert!(
        report.passed(),
        "simulator detected violations at medium scale: {:?}",
        report.violations
    );
    assert_eq!(
        report.stats.runtime_policy_validation_count, 0,
        "runtime policy validation count must remain zero"
    );
}

