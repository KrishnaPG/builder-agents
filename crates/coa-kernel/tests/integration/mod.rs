use coa_artifact::{ContentHash, DeltaOperation, StructuralDelta, SymbolPath};
use coa_composition::SingleWriterStrategy;
use coa_core::UserIntent;
use coa_core::error::COAError;
use coa_kernel::prelude::*;
use coa_kernel::test_harness::{run_simulator, SimulatorConfig};
use coa_test_utils::{
    create_delta_with_base,
    create_test_code_artifact,
    create_test_code_artifact_with_source,
    setup_test_coa,
};
use ed25519_dalek::SigningKey;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::str::FromStr;

fn make_remove_delta(target: &str, base_hash: ContentHash) -> StructuralDelta<coa_test_utils::TestCodeArtifact> {
    let path = SymbolPath::from_str(target).unwrap();
    StructuralDelta::new(path, DeltaOperation::Remove, base_hash)
}

#[test]
fn artifact_hash_is_deterministic_and_sensitive_to_source() {
    let a1 = create_test_code_artifact_with_source("fn a() {}");
    let a2 = create_test_code_artifact_with_source("fn a() {}");
    let a3 = create_test_code_artifact_with_source("fn b() {}");

    assert_eq!(a1.hash(), a2.hash());
    assert_ne!(a1.hash(), a3.hash());
}

#[test]
fn structural_delta_enforces_correct_base_hash() {
    let artifact = create_test_code_artifact();
    let correct_hash = *artifact.hash();
    let wrong_hash = ContentHash::compute(b"wrong");

    let delta_ok = create_delta_with_base("module.fn_a", DeltaOperation::Remove, correct_hash);
    let delta_bad = create_delta_with_base("module.fn_a", DeltaOperation::Remove, wrong_hash);

    assert!(delta_ok.validate_base(&artifact).is_ok());
    assert!(delta_bad.validate_base(&artifact).is_err());
}

#[test]
fn single_writer_accepts_disjoint_paths_and_rejects_overlaps() {
    let index = coa_symbol::SymbolRefIndex::new();
    let strategy = SingleWriterStrategy::new();
    let base_hash = ContentHash::compute(b"base");

    let d1 = make_remove_delta("service.fn_a", base_hash);
    let d2 = make_remove_delta("service.fn_b", base_hash);
    let d3 = make_remove_delta("service", base_hash);

    assert!(strategy.validate(&[d1.clone(), d2.clone()], &index).is_ok());
    assert!(strategy.validate(&[d1, d3], &index).is_err());
}

#[test]
fn production_graph_rejects_cycle_and_sandbox_allows_it() {
    let mut rng = StdRng::seed_from_u64(1);
    let signing_key = SigningKey::generate(&mut rng);

    let directives = DirectiveSet { directives: std::collections::BTreeMap::new() };
    let resource_bounds = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024 * 1024,
        token_limit: 1000,
        iteration_cap: 100,
    };

    let spec = NodeSpecV2::new(directives.clone(), AutonomyLevel::L3, resource_bounds);

    let mut prod_builder = GraphBuilder::new(GraphType::ProductionDAG);
    let p1 = prod_builder.add_node(spec.clone());
    let p2 = prod_builder.add_node(spec.clone());
    prod_builder.add_edge(p1, p2).unwrap();
    let result = prod_builder.add_edge(p2, p1);
    assert!(result.is_err());

    let mut sandbox_builder = GraphBuilder::new(GraphType::SandboxGraph);
    let s1 = sandbox_builder.add_node(spec.clone());
    let s2 = sandbox_builder.add_node(spec);
    sandbox_builder.add_edge(s1, s2).unwrap();
    let result_sandbox = sandbox_builder.add_edge(s2, s1);
    assert!(result_sandbox.is_ok());

    let validated = sandbox_builder.validate(&signing_key).unwrap();
    assert_eq!(validated.node_count(), 2);
}

#[tokio::test]
async fn simulator_respects_two_phase_invariant() {
    let config = SimulatorConfig {
        seed: 42,
        total_constructions: 50,
        total_executions: 50,
        stop_on_first_violation: true,
        verify_zero_runtime_policy: true,
    };

    let report = run_simulator(config).await;
    assert!(report.passed());
    assert!(!report.zero_runtime_policy_violated());
    assert_eq!(report.stats.runtime_policy_validation_count, 0);
}

#[tokio::test]
async fn coa_execute_intent_requires_human_intervention() {
    let coa = setup_test_coa();
    let intent = UserIntent::new("Create a simple function");
    let result = coa.execute_intent(intent).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, COAError::RequiresHumanIntervention { .. }));
    assert!(error.requires_human());
}
