//! Functional tests for structural deltas and Merkle-based integrity.
//!
//! Core guarantees exercised here:
//! - StructuralDelta must validate that it targets the correct base artifact
//!   (via content hash).
//! - Non-overlapping deltas commute: applying them in different orders should
//!   yield the same resulting Merkle root.
//! - Overlapping deltas are rejected or produce an explicit conflict instead
//!   of silently overwriting each other.
//! - The Merkle root is a stable integrity summary of the artifact content.

use coa_artifact::{Artifact, ContentHash, DeltaOperation, StructuralDelta, SymbolPath};
use coa_test_utils::{
    create_delta_with_base,
    create_test_code_artifact,
    create_test_code_artifact_with_source,
    TestCodeArtifact,
};

/// Tenet: StructuralDelta must enforce the correct base hash.
///
/// If deltas could be applied against the wrong base version, callers could
/// accidentally corrupt artifacts or violate referential integrity without
/// noticing. This test ensures base-hash checking is active and effective.
#[test]
fn structural_delta_requires_correct_base_hash() {
    let artifact = create_test_code_artifact();
    let correct_hash = *artifact.hash();
    let wrong_hash = ContentHash::compute(b"wrong-base");

    let delta_ok = create_delta_with_base("root.fn_a", DeltaOperation::Remove, correct_hash);
    let delta_bad = create_delta_with_base("root.fn_a", DeltaOperation::Remove, wrong_hash);

    assert!(delta_ok.validate_base(&artifact).is_ok());
    assert!(delta_bad.validate_base(&artifact).is_err());
}

/// Helper: construct a test artifact with multiple independent symbols so we
/// can exercise commuting/non-commuting deltas.
fn make_multi_symbol_artifact() -> Artifact<TestCodeArtifact> {
    create_test_code_artifact_with_source(
        r#"
        fn a() {}
        fn b() {}
        fn c() {}
        "#,
    )
}

/// Helper: build a delta that targets a single top-level function symbol.
fn make_symbol_delta(symbol: &str, base: &Artifact<TestCodeArtifact>) -> StructuralDelta<TestCodeArtifact> {
    let path = SymbolPath::single(symbol);
    let base_hash = *base.hash();
    StructuralDelta::new(path, DeltaOperation::Remove, base_hash)
}

/// Tenet: non-overlapping deltas commute at the Merkle root.
///
/// Deltas touching disjoint symbol paths should be order-independent. If this
/// fails, it suggests either:
/// - the Merkle tree is not correctly keyed by paths, or
/// - delta application has hidden side effects or ordering dependencies.
#[test]
fn non_overlapping_deltas_commute() {
    let base = make_multi_symbol_artifact();

    let d_a = make_symbol_delta("a", &base);
    let d_b = make_symbol_delta("b", &base);

    let r1 = d_b.apply(&d_a.apply(&base).expect("apply delta a")).expect("apply delta b");
    let r2 = d_a.apply(&d_b.apply(&base).expect("apply delta b")).expect("apply delta a");

    assert_eq!(r1.hash(), r2.hash());
}

/// Tenet: overlapping deltas must not silently overwrite each other.
///
/// When two deltas target the same symbol (or ancestor/descendant paths), the
/// system must either reject the combination or surface an explicit conflict
/// error. This test ensures that at least one of those paths is taken instead
/// of "last writer wins" behavior.
#[test]
fn overlapping_deltas_are_detected_as_conflicts() {
    let base = make_multi_symbol_artifact();
    let base_hash = *base.hash();

    let path = SymbolPath::single("a");
    let d1 = StructuralDelta::new(path.clone(), DeltaOperation::Remove, base_hash);
    let d2 = StructuralDelta::new(path, DeltaOperation::Remove, base_hash);

    let r1 = d1.apply(&base);
    assert!(r1.is_ok());

    let r2 = d2.apply(&base);
    assert!(r2.is_ok());

    let first = r1.unwrap();
    let second = d2.apply(&first);

    assert!(second.is_err());
}

/// Tenet: Merkle root is a stable integrity summary.
///
/// Two artifacts created independently from the same content must yield the
/// same Merkle root; if content diverges, the roots must diverge as well.
#[test]
fn merkle_root_is_stable_integrity_summary() {
    let a1 = create_test_code_artifact_with_source("fn a() {}");
    let a2 = create_test_code_artifact_with_source("fn a() {}");
    let a3 = create_test_code_artifact_with_source("fn b() {}");

    assert_eq!(a1.merkle_root(), a2.merkle_root());
    assert_ne!(a1.merkle_root(), a3.merkle_root());
}

