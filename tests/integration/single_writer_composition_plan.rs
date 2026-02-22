//! Functional tests for single-writer composition and integrity.
//!
//! This module exercises the "single writer per symbol" invariant that underpins
//! composition safety. It focuses on:
//! - Accepting disjoint writers (no overlapping symbol paths).
//! - Rejecting overlapping or ancestor/descendant write paths.
//! - Catching referential integrity issues early.

use coa_artifact::{ContentHash, DeltaOperation, StructuralDelta, SymbolPath};
use coa_composition::SingleWriterStrategy;
use coa_symbol::SymbolRefIndex;
use coa_test_utils::{create_test_code_artifact_with_source, TestCodeArtifact};

/// Helper: build a StructuralDelta for a given symbol path on a shared base.
fn make_delta(symbol_path: &str, base_hash: ContentHash) -> StructuralDelta<TestCodeArtifact> {
    let path = SymbolPath::single(symbol_path);
    StructuralDelta::new(path, DeltaOperation::Remove, base_hash)
}

/// Tenet: single-writer allows multiple writers as long as they are disjoint.
///
/// Deltas targeting distinct symbol paths (e.g. `a` and `b`) must be accepted
/// together by the strategy. If this fails, the system is overly strict and
/// prevents safe parallelization.
#[test]
fn disjoint_writers_are_accepted() {
    let index = SymbolRefIndex::new();
    let strategy = SingleWriterStrategy::new();

    let artifact = create_test_code_artifact_with_source(
        r#"
        fn a() {}
        fn b() {}
        "#,
    );
    let base_hash = *artifact.hash();

    let d1 = make_delta("a", base_hash);
    let d2 = make_delta("b", base_hash);

    assert!(strategy.validate(&[d1, d2], &index).is_ok());
}

/// Tenet: overlapping paths are rejected as integrity violations.
///
/// When one delta targets a parent path (e.g. `module`) and another targets a
/// child (e.g. `module.a`), the strategy must treat this as a conflict rather
/// than letting one silently overwrite the other.
#[test]
fn ancestor_descendant_paths_are_rejected() {
    let index = SymbolRefIndex::new();
    let strategy = SingleWriterStrategy::new();

    let artifact = create_test_code_artifact_with_source(
        r#"
        mod module {
            fn a() {}
        }
        "#,
    );
    let base_hash = *artifact.hash();

    let parent = make_delta("module", base_hash);
    let child = make_delta("module::a", base_hash);

    assert!(strategy.validate(&[parent, child], &index).is_err());
}

/// Tenet: same-path writers are rejected.
///
/// Two deltas both targeting the same symbol path must not be accepted
/// together, regardless of their individual operation types.
#[test]
fn identical_paths_are_rejected() {
    let index = SymbolRefIndex::new();
    let strategy = SingleWriterStrategy::new();

    let artifact = create_test_code_artifact_with_source("fn a() {}");
    let base_hash = *artifact.hash();

    let d1 = make_delta("a", base_hash);
    let d2 = make_delta("a", base_hash);

    assert!(strategy.validate(&[d1, d2], &index).is_err());
}

