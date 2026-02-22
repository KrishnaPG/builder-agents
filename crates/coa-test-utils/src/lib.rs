//! Testing utilities for COA workspace
//! 
//! Shared test helpers, fixtures, and assertions.

#![allow(missing_docs)]

use coa_artifact::{Artifact, ArtifactType, ContentHash, DeltaOperation, StructuralDelta, SymbolPath};
use coa_artifact::__private::Sealed;
use coa_core::{COAConfig, CreatorOrchestratorAgent};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct TestCodeArtifact;

#[derive(Debug, Clone, PartialEq)]
pub struct TestCodeContent {
    pub source: String,
}

impl Sealed for TestCodeArtifact {}

impl ArtifactType for TestCodeArtifact {
    type Content = TestCodeContent;

    fn hash(content: &Self::Content) -> ContentHash {
        ContentHash::compute(content.source.as_bytes())
    }

    const TYPE_ID: &'static str = "test-code";
}

pub fn create_test_code_artifact_with_source(source: &str) -> Artifact<TestCodeArtifact> {
    let content = TestCodeContent {
        source: source.to_string(),
    };
    Artifact::<TestCodeArtifact>::new(content).unwrap()
}

pub fn create_test_code_artifact() -> Artifact<TestCodeArtifact> {
    create_test_code_artifact_with_source("fn main() {}")
}

pub fn create_delta(target: &str, operation: DeltaOperation<TestCodeArtifact>) -> StructuralDelta<TestCodeArtifact> {
    let artifact = create_test_code_artifact();
    let path = SymbolPath::from_str(target).unwrap();
    let base_hash = *artifact.hash();
    StructuralDelta::new(path, operation, base_hash)
}

pub fn create_delta_with_base(
    target: &str,
    operation: DeltaOperation<TestCodeArtifact>,
    base_hash: ContentHash,
) -> StructuralDelta<TestCodeArtifact> {
    let path = SymbolPath::from_str(target).unwrap();
    StructuralDelta::new(path, operation, base_hash)
}

pub fn setup_test_coa() -> CreatorOrchestratorAgent {
    let config = COAConfig::new();
    CreatorOrchestratorAgent::new(config)
}
