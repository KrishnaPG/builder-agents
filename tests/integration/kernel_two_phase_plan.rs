//! Functional tests for the v2.0 two-phase kernel architecture.
//!
//! These tests exercise the construction → execution split described in the
//! kernel Blueprint:
//! - All structural and policy validation happens in the construction phase
//!   (GraphBuilder + ConstructionValidator).
//! - The execution phase sees only ValidatedGraph and performs zero policy
//!   validation, focusing purely on integrity verification and state updates.
//! - The COA Simulator is the authoritative harness for checking these
//!   invariants across many randomized operations.

use coa_kernel::construction::GraphBuilder;
use coa_kernel::error::ValidationError;
use coa_kernel::prelude::*;
use coa_kernel::test_harness::{run_simulator, SimulatorConfig};
use coa_kernel::types::{AutonomyLevel, GraphType, ResourceCaps};
use ed25519_dalek::SigningKey;
use rand::{rngs::StdRng, SeedableRng};

/// Helper: create a minimal NodeSpecV2 with bounded resources.
fn make_node_spec() -> NodeSpecV2 {
    let directives = DirectiveSet {
        directives: std::collections::BTreeMap::new(),
    };
    let resources = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024 * 1024,
        token_limit: 1000,
        iteration_cap: 100,
    };
    NodeSpecV2::new(directives, AutonomyLevel::L3, resources)
}

/// Tenet: a simple acyclic production DAG validates successfully.
///
/// If this fails, the construction phase is rejecting graphs that should be
/// allowed, which indicates a bug in GraphBuilder or ConstructionValidator.
#[test]
fn production_dag_validates_successfully() {
    let mut rng = StdRng::seed_from_u64(1);
    let signing_key = SigningKey::generate(&mut rng);

    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    let spec = make_node_spec();

    let n1 = builder.add_node(spec.clone());
    let n2 = builder.add_node(spec);

    builder.add_edge(n1, n2).expect("edge in DAG should be valid");

    let validated = builder
        .validate(&signing_key)
        .expect("acyclic production DAG must validate");

    assert_eq!(validated.node_count(), 2);
    assert_eq!(validated.edge_count(), 1);
}

/// Tenet: cycles are rejected at construction time for production DAGs.
///
/// A production graph that would introduce a cycle must never validate; this
/// is a core part of the DAG integrity guarantees.
#[test]
fn production_dag_rejects_cycles() {
    let mut rng = StdRng::seed_from_u64(2);
    let signing_key = SigningKey::generate(&mut rng);

    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    let spec = make_node_spec();

    let n1 = builder.add_node(spec.clone());
    let n2 = builder.add_node(spec);

    builder.add_edge(n1, n2).expect("first edge should be valid");
    let cycle_result = builder.add_edge(n2, n1);
    assert!(cycle_result.is_err(), "cycle edge must be rejected");

    let validated = builder
        .validate(&signing_key)
        .expect("builder with cycle rejected should still validate remaining DAG");

    assert_eq!(validated.node_count(), 2);
    assert_eq!(validated.edge_count(), 1);
}

/// Tenet: sandbox graphs can contain cycles (no DAG requirement).
///
/// SandboxGraph is explicitly allowed to be cyclic; the construction-phase
/// validator should respect this and allow cycles as long as other constraints
/// (resources, autonomy) are satisfied.
#[test]
fn sandbox_graph_allows_cycles() {
    let mut rng = StdRng::seed_from_u64(3);
    let signing_key = SigningKey::generate(&mut rng);

    let mut builder = GraphBuilder::new(GraphType::SandboxGraph);
    let spec = make_node_spec();

    let n1 = builder.add_node(spec.clone());
    let n2 = builder.add_node(spec);

    builder.add_edge(n1, n2).expect("edge 1 should be valid");
    builder.add_edge(n2, n1).expect("edge 2 should be valid in sandbox");

    let validated = builder
        .validate(&signing_key)
        .expect("sandbox graph with cycle must validate");

    assert_eq!(validated.node_count(), 2);
    assert_eq!(validated.edge_count(), 2);
}

/// Tenet: resource bounds are enforced at construction time.
///
/// Graphs that exceed configured SystemLimits must fail validation instead of
/// slipping through to execution and failing there.
#[test]
fn system_limits_enforced_at_construction_time() {
    let mut rng = StdRng::seed_from_u64(4);
    let signing_key = SigningKey::generate(&mut rng);

    let limits = SystemLimits {
        max_autonomy: AutonomyLevel::L3,
        max_resources: ResourceCaps {
            cpu_time_ms: 10,
            memory_bytes: 1024,
            token_limit: 10,
            iteration_cap: 10,
        },
        max_nodes: 1,
        max_edges: 0,
    };

    let mut builder = GraphBuilder::with_limits(GraphType::ProductionDAG, limits);
    let spec = make_node_spec();

    let _n1 = builder.add_node(spec.clone());
    let _n2 = builder.add_node(spec);

    let result = builder.validate(&signing_key);
    match result {
        Err(ValidationError::ResourceBoundsExceeded { .. }) => {}
        other => panic!("expected ResourceBoundsExceeded, got {:?}", other),
    }
}

/// Tenet: the simulator enforces the zero-runtime-policy invariant.
///
/// This is the direct implementation of the Blueprint requirement that all
/// policy validation happens in construction, with runtime only performing
/// integrity verification. The simulator runs many randomized operations and
/// must never record a runtime policy validation.
#[tokio::test]
async fn simulator_respects_zero_runtime_policy_invariant() {
    let config = SimulatorConfig {
        seed: 42,
        total_constructions: 100,
        total_executions: 100,
        stop_on_first_violation: true,
        verify_zero_runtime_policy: true,
    };

    let report = run_simulator(config).await;

    assert!(
        report.passed(),
        "simulator detected violations: {:?}",
        report.violations
    );
    assert!(
        !report.zero_runtime_policy_violated(),
        "runtime policy validation detected (architecture violation)"
    );
    assert_eq!(
        report.stats.runtime_policy_validation_count, 0,
        "runtime policy validation count must be zero"
    );
}

