//! Single writer composition strategy
//!
//! Default strategy: each agent claims a disjoint subtree.
//! Maximum safety, universal applicability.

use crate::strategy::{
    CompositionCost, CompositionError, CompositionStrategy, ConflictKind, DeltaClass,
    Granularity, Parallelism, ResolutionSuggestion, TimeComplexity, Validation,
    ValidationDiagnostic, ValidationMetadata,
};
use coa_artifact::{Artifact, ArtifactType, StructuralDelta};
use coa_symbol::{SingleWriterValidator, SymbolRefIndex};

/// Single writer strategy: disjoint subtree claims
///
/// # Characteristics
/// - Maximum safety (no conflicts possible)
/// - Universal applicability
/// - Requires fine-grained COA decomposition
/// - Fully parallel composition
#[derive(Debug, Clone, Copy, Default)]
pub struct SingleWriterStrategy;

impl SingleWriterStrategy {
    /// Create new single writer strategy
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validate deltas have disjoint targets
    fn validate_disjoint<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<(), CompositionError> {
        let validator = SingleWriterValidator::new();

        validator
            .validate_deltas(deltas, index)
            .map_err(|e| CompositionError::validation_failed_simple(
                ConflictKind::OverlappingTargets,
                e.to_string(),
            ))?;

        validator
            .validate_against_index(deltas, index)
            .map_err(|e| CompositionError::validation_failed_simple(
                ConflictKind::OverlappingTargets,
                e.to_string(),
            ))?;

        Ok(())
    }

    /// Apply deltas in any order (they're independent)
    fn apply_parallel<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError> {
        // Since all deltas target disjoint paths, we can apply in any order
        // For now, sequential fold (can be parallelized with rayon later)
        deltas
            .iter()
            .try_fold(base.clone(), |acc, delta| {
                self.apply_single(&acc, delta)
            })
    }

    /// Apply single delta
    fn apply_single<T: ArtifactType>(
        &self,
        _artifact: &Artifact<T>,
        _delta: &StructuralDelta<T>,
    ) -> Result<Artifact<T>, CompositionError> {
        // Note: This is a placeholder - actual application requires
        // artifact-type-specific logic that would be provided by
        // the ConstitutionalLayer
        Err(CompositionError::CompositionFailed(
            "SingleWriterStrategy requires ConstitutionalLayer for delta application".to_string(),
        ))
    }
}

impl CompositionStrategy for SingleWriterStrategy {
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        if deltas.is_empty() {
            return Ok(Validation::minimal());
        }

        // Check for disjoint paths
        self.validate_disjoint(deltas, index)?;

        // Build validation metadata
        let mut metadata = ValidationMetadata::default();
        metadata.set_batch_count(1); // Single batch, all parallel

        let cost = CompositionCost {
            time: TimeComplexity::ONLogN,
            space: crate::strategy::SpaceComplexity::ON,
            parallelism_factor: 1.0, // Fully parallel
        };

        Ok(Validation::with_metadata(metadata).with_cost(cost))
    }

    fn compose<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError> {
        if deltas.is_empty() {
            return Ok(base.clone());
        }

        self.apply_parallel(base, deltas)
    }

    fn parallelism(&self) -> Parallelism {
        Parallelism::Full
    }

    fn granularity(&self) -> Granularity {
        Granularity::Subtree
    }

    fn name(&self) -> &'static str {
        "SingleWriter"
    }
}

/// Classifier for single writer compatibility
pub struct SingleWriterClassifier;

impl SingleWriterClassifier {
    /// Check if delta is compatible with single writer strategy
    ///
    /// All operations are compatible since we enforce disjoint paths.
    #[inline]
    #[must_use]
    pub fn is_compatible<T: ArtifactType>(_delta: &StructuralDelta<T>) -> bool {
        true
    }

    /// Classify delta for hybrid strategy
    #[inline]
    #[must_use]
    pub fn classify<T: ArtifactType>(_delta: &StructuralDelta<T>) -> DeltaClass {
        // Single writer treats all operations as commutative
        // (because paths are guaranteed disjoint)
        DeltaClass::Commutative
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coa_artifact::{ArtifactType, ContentHash, DeltaOperation, SymbolPath};
    use std::str::FromStr;

    // Test artifact type
    #[derive(Debug, Clone)]
    struct TestArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct TestContent;

    impl coa_artifact::__private::Sealed for TestArtifact {}

    impl ArtifactType for TestArtifact {
        type Content = TestContent;

        fn hash(_content: &Self::Content) -> ContentHash {
            ContentHash::compute(b"test")
        }

        const TYPE_ID: &'static str = "test";
    }

    fn make_delta(target: &str, base_hash: ContentHash) -> StructuralDelta<TestArtifact> {
        StructuralDelta::new(
            SymbolPath::from_str(target).unwrap(),
            DeltaOperation::Remove,
            base_hash,
        )
    }

    fn test_hash() -> ContentHash {
        ContentHash::compute(b"base")
    }

    #[test]
    fn single_writer_strategy_new() {
        let strategy = SingleWriterStrategy::new();
        assert_eq!(strategy.name(), "SingleWriter");
        assert!(matches!(strategy.parallelism(), Parallelism::Full));
    }

    #[test]
    fn single_writer_validates_disjoint() {
        let strategy = SingleWriterStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_delta("auth.login", test_hash()),
            make_delta("auth.register", test_hash()),
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.metadata.batch_count, Some(1));
        assert_eq!(validation.cost_estimate.parallelism_factor, 1.0);
    }

    #[test]
    fn single_writer_rejects_overlapping() {
        let strategy = SingleWriterStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_delta("auth", test_hash()),
            make_delta("auth.login", test_hash()),
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompositionError::ValidationFailed { .. }
        ));
    }

    #[test]
    fn single_writer_empty_deltas() {
        let strategy = SingleWriterStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas: Vec<StructuralDelta<TestArtifact>> = vec![];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn single_writer_classifier_compatible() {
        let delta = make_delta("test.path", test_hash());
        assert!(SingleWriterClassifier::is_compatible(&delta));
    }

    #[test]
    fn single_writer_classifier_commutative() {
        let delta = make_delta("test.path", test_hash());
        assert!(matches!(
            SingleWriterClassifier::classify(&delta),
            DeltaClass::Commutative
        ));
    }
}
