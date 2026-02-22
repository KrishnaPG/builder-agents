//! Commutative batch composition strategy
//!
//! CRDT-style commutative operations for maximum parallelism.

use crate::strategy::{
    CompositionCost, CompositionError, CompositionStrategy, ConflictKind, DeltaClass,
    Granularity, Parallelism, ResolutionSuggestion, TimeComplexity, Validation,
    ValidationDiagnostic, ValidationMetadata,
};
use coa_artifact::{Artifact, ArtifactType, StructuralDelta};
use coa_symbol::SymbolRefIndex;
use std::collections::HashSet;

/// Commutative batch composition strategy
///
/// # Characteristics
/// - Maximum parallelism (order-independent)
/// - Requires operations to be naturally commutative
/// - Good for set-like operations (add/remove layers, tags)
#[derive(Debug, Clone, Copy, Default)]
pub struct CommutativeBatchStrategy;

impl CommutativeBatchStrategy {
    /// Create new commutative batch strategy
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validate that all operations are commutative
    fn validate_commutative<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
    ) -> Result<(), CompositionError> {
        for (i, delta) in deltas.iter().enumerate() {
            if !Self::is_operation_commutative(delta) {
                return Err(CompositionError::validation_failed(
                    ValidationDiagnostic {
                        kind: ConflictKind::NonCommutativeOperations,
                        involved_deltas: vec![i],
                        description: format!(
                            "Delta {} contains non-commutative operation: {:?}",
                            i,
                            delta.operation()
                        ),
                        suggestions: vec![
                            ResolutionSuggestion::UseOrdered,
                            ResolutionSuggestion::UseSingleWriter,
                        ],
                    },
                ));
            }
        }

        Ok(())
    }

    /// Check if delta operation is commutative
    fn is_operation_commutative<T: ArtifactType>(delta: &StructuralDelta<T>) -> bool {
        matches!(
            delta.operation(),
            coa_artifact::DeltaOperation::Add(_) | coa_artifact::DeltaOperation::Remove
        )
    }

    /// Check for duplicate targets (still not allowed in commutative mode)
    fn validate_unique_targets<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
    ) -> Result<(), CompositionError> {
        let mut seen = HashSet::new();

        for (i, delta) in deltas.iter().enumerate() {
            let key = delta.target().to_string();
            if !seen.insert(key.clone()) {
                return Err(CompositionError::validation_failed(
                    ValidationDiagnostic {
                        kind: ConflictKind::OverlappingTargets,
                        involved_deltas: vec![i],
                        description: format!("Duplicate target in commutative batch: {}", key),
                        suggestions: vec![ResolutionSuggestion::UseSingleWriter],
                    },
                ));
            }
        }

        Ok(())
    }

    /// Apply deltas in parallel (order doesn't matter)
    fn apply_commutative<T: ArtifactType>(
        &self,
        _base: &Artifact<T>,
        _deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError> {
        // In a real implementation, this would:
        // 1. Apply all deltas in parallel using rayon
        // 2. Merge results (since operations are commutative, order doesn't matter)
        //
        // For now, placeholder:
        Err(CompositionError::CompositionFailed(
            "CommutativeBatchStrategy requires ConstitutionalLayer".to_string(),
        ))
    }
}

impl CompositionStrategy for CommutativeBatchStrategy {
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        _index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        if deltas.len() <= 1 {
            return Ok(Validation::minimal());
        }

        // All operations must be commutative
        self.validate_commutative(deltas)?;

        // Targets must still be unique
        self.validate_unique_targets(deltas)?;

        let mut metadata = ValidationMetadata::default();
        metadata.set_batch_count(1);

        let cost = CompositionCost {
            time: TimeComplexity::ON,
            space: crate::strategy::SpaceComplexity::ON,
            parallelism_factor: 1.0,
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

        self.apply_commutative(base, deltas)
    }

    fn parallelism(&self) -> Parallelism {
        Parallelism::Full
    }

    fn granularity(&self) -> Granularity {
        Granularity::Node
    }

    fn name(&self) -> &'static str {
        "CommutativeBatch"
    }
}

/// Classifier for commutative strategy
pub struct CommutativeClassifier;

impl CommutativeClassifier {
    /// Check if delta operation is commutative
    #[inline]
    #[must_use]
    pub fn is_commutative<T: ArtifactType>(delta: &StructuralDelta<T>) -> bool {
        matches!(
            delta.operation(),
            coa_artifact::DeltaOperation::Add(_) | coa_artifact::DeltaOperation::Remove
        )
    }

    /// Classify delta for hybrid strategy
    #[inline]
    #[must_use]
    pub fn classify<T: ArtifactType>(delta: &StructuralDelta<T>) -> DeltaClass {
        if Self::is_commutative(delta) {
            DeltaClass::Commutative
        } else {
            DeltaClass::Ordered(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coa_artifact::{ArtifactType, ContentHash, DeltaOperation, SymbolPath};
    use std::str::FromStr;

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

    fn make_add_delta(target: &str, base_hash: ContentHash) -> StructuralDelta<TestArtifact> {
        StructuralDelta::new(
            SymbolPath::from_str(target).unwrap(),
            DeltaOperation::Add(TestContent),
            base_hash,
        )
    }

    fn make_remove_delta(target: &str, base_hash: ContentHash) -> StructuralDelta<TestArtifact> {
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
    fn commutative_strategy_new() {
        let strategy = CommutativeBatchStrategy::new();
        assert_eq!(strategy.name(), "CommutativeBatch");
    }

    #[test]
    fn commutative_validates_add_remove() {
        let strategy = CommutativeBatchStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_add_delta("layer1", test_hash()),
            make_add_delta("layer2", test_hash()),
            make_remove_delta("layer3", test_hash()),
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn commutative_rejects_replace() {
        let strategy = CommutativeBatchStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_add_delta("layer1", test_hash()),
            StructuralDelta::new(
                SymbolPath::from_str("layer2").unwrap(),
                DeltaOperation::Replace(TestContent),
                test_hash(),
            ),
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompositionError::ValidationFailed { .. }
        ));
    }

    #[test]
    fn commutative_rejects_duplicate_targets() {
        let strategy = CommutativeBatchStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_add_delta("layer1", test_hash()),
            make_add_delta("layer1", test_hash()), // Same target
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_err());
    }

    #[test]
    fn commutative_classifier() {
        let add = make_add_delta("test", test_hash());
        let remove = make_remove_delta("test", test_hash());

        assert!(CommutativeClassifier::is_commutative(&add));
        assert!(CommutativeClassifier::is_commutative(&remove));

        assert!(matches!(
            CommutativeClassifier::classify(&add),
            DeltaClass::Commutative
        ));
    }
}
