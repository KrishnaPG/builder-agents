//! Functional tests for artifact foundations and content-addressed behavior.
//!
//! This module focuses on the lowest-level guarantees of the artifact system:
//! - A file is not a string: artifacts wrap structured content, not raw text.
//! - Content-addressed storage: identical content has identical hashes; small
//!   changes cause hash changes.
//! - Language-aware parsing: code artifacts are parsed with tree-sitter into
//!   a structured representation that supports symbol addressing.
//! - Symbol-path addressing: callers can refer to logical parts of artifacts
//!   using stable symbolic paths.
//!
//! These tests are intentionally "fat" compared to unit tests: each scenario
//! exercises multiple components end-to-end (hashing, parsing, symbol paths),
//! corresponding directly to Blueprint core tenets for artifacts.

use coa_artifact::types::{CodeArtifact, CodeContent, Language};
use coa_artifact::{Artifact, ContentHash, SymbolPath};

/// Helper: build a code artifact from a given source snippet and language.
///
/// This is intentionally small and deterministic: the same source-language
/// pair must always produce the same content hash.
fn make_code_artifact(source: &str, language: Language) -> Artifact<CodeArtifact> {
    let content = CodeContent::parse(source, language).expect("parse should succeed for test snippet");
    Artifact::<CodeArtifact>::new(content).expect("artifact creation should not fail for test snippet")
}

/// Tenet: identical inputs must produce identical content hashes.
///
/// If this fails, content-addressed storage is broken: downstream systems would
/// treat identical artifacts as different identities, violating deduplication
/// and integrity assumptions.
#[test]
fn identical_sources_yield_identical_hashes() {
    let a1 = make_code_artifact("fn a() {}", Language::Rust);
    let a2 = make_code_artifact("fn a() {}", Language::Rust);

    assert_eq!(a1.hash(), a2.hash());
}

/// Tenet: small changes in source must be visible at the hash level.
///
/// This checks that the hashing function is sensitive enough to detect even a
/// single-character change, which is required for any Merkle-based integrity
/// scheme to be meaningful.
#[test]
fn small_source_changes_change_hashes() {
    let a1 = make_code_artifact("fn a() {}", Language::Rust);
    let a2 = make_code_artifact("fn a() { }", Language::Rust);
    let a3 = make_code_artifact("fn b() {}", Language::Rust);

    assert_ne!(a1.hash(), a2.hash());
    assert_ne!(a1.hash(), a3.hash());
}

/// Tenet: language-aware parsing works for multiple supported languages.
///
/// We do not assert on the full AST shape here; instead we assert that parsing
/// succeeds and that language tags are set correctly for a diverse set of code
/// snippets. This ensures the artifact layer can safely host multi-language
/// workspaces.
#[test]
fn parses_multiple_languages_with_correct_language_tag() {
    let rust = make_code_artifact("fn main() {}", Language::Rust);
    let ts = make_code_artifact("function main() { return 1; }", Language::TypeScript);
    let py = make_code_artifact("def main():\n    return 1\n", Language::Python);
    let go = make_code_artifact("package main\nfunc main() {}", Language::Go);

    assert_eq!(rust.content().language(), Language::Rust);
    assert_eq!(ts.content().language(), Language::TypeScript);
    assert_eq!(py.content().language(), Language::Python);
    assert_eq!(go.content().language(), Language::Go);
}

/// Tenet: a file is not a string; callers address logical symbols, not byte
/// offsets.
///
/// This test uses `SymbolPath` to address a function inside a simple module.
/// The important guarantee is that:
/// - well-formed paths resolve to something meaningful
/// - malformed paths fail cleanly without corrupting the artifact
#[test]
fn symbol_paths_address_logical_parts_of_artifacts() {
    let source = r#"
        mod m {
            pub fn f() {}
        }
    "#;
    let artifact = make_code_artifact(source, Language::Rust);

    // Well-formed symbol path should resolve.
    let path_ok = SymbolPath::single("m::f");
    let symbol_ok = artifact
        .content()
        .resolve_symbol(&path_ok)
        .expect("valid symbol path should resolve");
    assert_eq!(symbol_ok.name(), "f");

    // Malformed/path should not resolve, but artifact remains valid.
    let path_bad = SymbolPath::single("m::missing");
    assert!(artifact.content().resolve_symbol(&path_bad).is_err());
    assert!(artifact.verify());
}

/// Tenet: content hash of the underlying artifact reflects the structured
/// content, not arbitrary string handling.
///
/// We construct two semantically equivalent snippets that differ only in
/// whitespace and verify whether the system treats them as distinct. The
/// expectation here is documented explicitly:
/// - If hashes are equal, it means the artifact layer normalizes whitespace
///   during parsing/hashing, emphasizing semantics over surface form.
/// - If hashes differ, callers must treat even formatting-only changes as
///   distinct artifact versions.
///
/// The test asserts the behavior we actually observe to guard against future
/// regressions: if the implementation currently treats whitespace changes as
/// significant, tightening that behavior would require updating this test and
/// the Blueprint.
#[test]
fn formatting_changes_have_consistent_hash_semantics() {
    let a1 = make_code_artifact("fn a() {}", Language::Rust);
    let a2 = make_code_artifact("fn  a( )  {}", Language::Rust);

    // We intentionally "snapshot" current behavior.
    assert_ne!(a1.hash(), a2.hash());
}

