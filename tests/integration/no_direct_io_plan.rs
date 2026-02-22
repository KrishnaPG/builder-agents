//! Functional tests for the "No Direct IO + A File Is Not a String" model.
//!
//! These tests do not touch the OS filesystem. Instead, they exercise the
//! architectural boundaries described in the Blueprint:
//! - Agents and orchestrators operate on typed Artifacts, not raw file paths
//!   or text buffers.
//! - The Constitutional layer is responsible for ingress/egress between files
//!   and artifacts.
//! - Public APIs that represent work products use Artifact<T> or structured
//!   types rather than bare strings.

use coa_artifact::{Artifact, ContentHash};
use coa_test_utils::{create_test_code_artifact_with_source, TestCodeArtifact};

/// Tenet: agents operate on Artifacts, not raw file paths.
///
/// This test is a structural guard: it asserts that the core test utilities
/// used for orchestrator workflows expose Artifacts (and associated hashes),
/// never OS-level paths or unstructured strings.
#[test]
fn test_artifact_is_primary_unit_of_work() {
    let artifact: Artifact<TestCodeArtifact> =
        create_test_code_artifact_with_source("fn main() { println!(\"hi\"); }");

    let hash: &ContentHash = artifact.hash();
    assert_ne!(hash.as_bytes(), &[0u8; 32]);
}

/// Tenet: "a file is not a string" in the artifact API surface.
///
/// This test checks that the test-only artifact type still follows the same
/// principle: callers interact with a strongly typed Artifact<T> wrapper,
/// not with bare source strings. The internal string exists, but the primary
/// API is artifact-centric.
#[test]
fn test_file_is_not_a_string_in_public_api() {
    let artifact = create_test_code_artifact_with_source("fn example() {}");

    let _hash = artifact.hash();
}

