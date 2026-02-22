//! Hybrid composition strategy
//!
//! Combines commutative batch with ordered refinement.

use crate::commutative::CommutativeClassifier;
use crate::ordered::OrderedClassifier;
use crate::strategy::{
    CompositionCost, CompositionError, CompositionStrategy, ConflictKind, DeltaClass,
    Granularity, Parallelism, ResolutionSuggestion, TimeComplexity, Validation,
    ValidationDiagnostic, ValidationMetadata,
};
use coa_artifact::{Artifact, ArtifactType, StructuralDelta};
use coa_symbol::SymbolRefIndex;
use std::collections::HashSet;

/// Combines commutative batch with ordered refinement
///
/// # Characteristics
/// - Two-phase: parallel commutative → sequential ordered
/// - Best of both worlds
/// - Recommended for creative tools
#[derive(Clone)]
pub struct HybridCompositionStrategy<C = ()>
where
    C: Classifier,
{
    /// Custom classifier
    classifier: C,
}

impl<C: Classifier> std::fmt::Debug for HybridCompositionStrategy<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridCompositionStrategy")
            .field("classifier", &"<classifier>")
            .finish()
    }
}

/// Classifier trait for hybrid strategy
pub trait Classifier: Send + Sync {
    /// Classify a delta
    fn classify<T: ArtifactType>(&self, delta: &StructuralDelta<T>) -> DeltaClass;
}

impl Classifier for () {
    fn classify<T: ArtifactType>(&self, delta: &StructuralDelta<T>) -> DeltaClass {
        HybridCompositionStrategy::<()>::default_classifier(delta)
    }
}

impl HybridCompositionStrategy<()> {
    /// Create with default classifier
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { classifier: () }
    }
}

impl<C: Classifier> HybridCompositionStrategy<C> {
    /// Create with custom classifier
    #[inline]
    #[must_use]
    pub fn with_classifier(classifier: C) -> Self {
        Self { classifier }
    }

    /// Default classifier based on operation type
    fn default_classifier<T: ArtifactType>(delta: &StructuralDelta<T>) -> DeltaClass {
        if CommutativeClassifier::is_commutative(delta) {
            DeltaClass::Commutative
        } else {
            // Use the delta's order if set, otherwise default to 1
            match delta.order() {
                Some(order) => DeltaClass::Ordered(order),
                None => DeltaClass::Ordered(1),
            }
        }
    }

    /// Partition deltas into commutative and ordered batches
    fn partition_deltas<'a, T: ArtifactType>(
        &self,
        deltas: &'a [StructuralDelta<T>],
    ) -> (Vec<&'a StructuralDelta<T>>, Vec<(u32, &'a StructuralDelta<T>)>) {
        let mut commutative = Vec::new();
        let mut ordered = Vec::new();

        for delta in deltas {
            match self.classifier.classify(delta) {
                DeltaClass::Commutative => commutative.push(delta),
                DeltaClass::Ordered(order) => ordered.push((order, delta)),
            }
        }

        // Sort ordered by order value
        ordered.sort_by_key(|(o, _)| *o);

        (commutative, ordered)
    }

    /// Validate commutative batch
    fn validate_commutative_batch<T: ArtifactType>(
        &self,
        batch: &[&StructuralDelta<T>],
    ) -> Result<(), CompositionError> {
        // Check for duplicates
        let mut seen = HashSet::new();
        for delta in batch {
            let key = delta.target().to_string();
            if !seen.insert(key.clone()) {
                return Err(CompositionError::validation_failed(
                    ValidationDiagnostic {
                        kind: ConflictKind::OverlappingTargets,
                        involved_deltas: vec![],
                        description: format!(
                            "Duplicate target in commutative batch: {}",
                            key
                        ),
                        suggestions: vec![ResolutionSuggestion::UseSingleWriter],
                    },
                ));
            }
        }

        // All must actually be commutative operations
        for delta in batch {
            if !CommutativeClassifier::is_commutative(delta) {
                return Err(CompositionError::validation_failed(
                    ValidationDiagnostic {
                        kind: ConflictKind::NonCommutativeOperations,
                        involved_deltas: vec![],
                        description: "Non-commutative operation in commutative batch".to_string(),
                        suggestions: vec![ResolutionSuggestion::UseOrdered],
                    },
                ));
            }
        }

        Ok(())
    }

    /// Validate ordered sequence
    fn validate_ordered_sequence<T: ArtifactType>(
        &self,
        sequence: &[(u32, &StructuralDelta<T>)],
    ) -> Result<(), CompositionError> {
        // Check all have explicit ordering
        for (order, delta) in sequence {
            if delta.order() != Some(*order) {
                return Err(CompositionError::validation_failed(
                    ValidationDiagnostic {
                        kind: ConflictKind::MissingOrdering,
                        involved_deltas: vec![],
                        description: format!("Delta has mismatched order: expected {:?}, got {:?}", delta.order(), order),
                        suggestions: vec![ResolutionSuggestion::AddOrdering {
                            suggested_order: vec![],
                        }],
                    },
                ));
            }
        }

        Ok(())
    }

    /// Compute parallelism factor
    fn compute_parallelism_factor(&self, commutative_len: usize, total_len: usize) -> f64 {
        if total_len == 0 {
            return 1.0;
        }
        commutative_len as f64 / total_len as f64
    }
}

impl Default for HybridCompositionStrategy<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Classifier> CompositionStrategy for HybridCompositionStrategy<C> {
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        _index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        if deltas.len() <= 1 {
            return Ok(Validation::minimal());
        }

        let (commutative, ordered) = self.partition_deltas(deltas);

        // Validate commutative batch
        self.validate_commutative_batch(&commutative)?;

        // Validate ordered sequence
        self.validate_ordered_sequence(&ordered)?;

        let mut metadata = ValidationMetadata::default();
        metadata.set_batch_count(2); // Commutative + Ordered

        let cost = CompositionCost {
            time: TimeComplexity::ON,
            space: crate::strategy::SpaceComplexity::ON,
            parallelism_factor: self.compute_parallelism_factor(commutative.len(), deltas.len()),
        };

        Ok(Validation::with_metadata(metadata).with_cost(cost))
    }

    fn compose<T: ArtifactType>(
        &self,
        _base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError> {
        if deltas.is_empty() {
            return Err(CompositionError::NotValidated);
        }

        let (commutative, ordered) = self.partition_deltas(deltas);

        // Phase 1: Apply commutative in parallel (placeholder)
        let _after_commutative = commutative.len();

        // Phase 2: Apply ordered sequentially (placeholder)
        let _after_ordered = ordered.len();

        Err(CompositionError::CompositionFailed(
            "HybridCompositionStrategy requires ConstitutionalLayer".to_string(),
        ))
    }

    fn parallelism(&self) -> Parallelism {
        Parallelism::Partial
    }

    fn granularity(&self) -> Granularity {
        Granularity::Attribute
    }

    fn name(&self) -> &'static str {
        "HybridComposition"
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

    fn make_ordered_delta(
        target: &str,
        order: u32,
        base_hash: ContentHash,
    ) -> StructuralDelta<TestArtifact> {
        use coa_artifact::Transformation;

        struct DummyTransform;
        impl std::fmt::Debug for DummyTransform {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "DummyTransform")
            }
        }
        impl Transformation<TestArtifact> for DummyTransform {
            fn apply(
                &self,
                _content: &TestContent,
            ) -> Result<TestContent, coa_artifact::TransformError> {
                Ok(TestContent)
            }
            fn describe(&self) -> String {
                "dummy".to_string()
            }
        }

        StructuralDelta::with_order(
            SymbolPath::from_str(target).unwrap(),
            DeltaOperation::Transform(Box::new(DummyTransform)),
            base_hash,
            order,
        )
    }

    fn test_hash() -> ContentHash {
        ContentHash::compute(b"base")
    }

    #[test]
    fn hybrid_strategy_new() {
        let strategy = HybridCompositionStrategy::new();
        assert_eq!(strategy.name(), "HybridComposition");
    }

    #[test]
    fn hybrid_partition_deltas() {
        let strategy = HybridCompositionStrategy::new();

        let deltas = vec![
            make_add_delta("layer1", test_hash()),      // Commutative
            make_add_delta("layer2", test_hash()),      // Commutative
            make_ordered_delta("effect", 1, test_hash()), // Ordered
        ];

        let (commutative, ordered) = strategy.partition_deltas(&deltas);

        assert_eq!(commutative.len(), 2);
        assert_eq!(ordered.len(), 1);
    }

    #[test]
    fn hybrid_validates_mixed() {
        let strategy = HybridCompositionStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_add_delta("layer1", test_hash()),
            make_add_delta("layer2", test_hash()),
            make_ordered_delta("effect1", 1, test_hash()),
            make_ordered_delta("effect2", 2, test_hash()),
        ];

        let result = strategy.validate(&deltas, &index);
        if let Err(ref e) = result {
            eprintln!("Validation error: {:?}", e);
        }
        assert!(result.is_ok(), "Validation failed: {:?}", result.err());

        let validation = result.unwrap();
        assert_eq!(validation.metadata.batch_count, Some(2));
        assert_eq!(validation.cost_estimate.parallelism_factor, 0.5); // 2/4
    }

    #[test]
    fn hybrid_rejects_duplicate_in_commutative() {
        let strategy = HybridCompositionStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_add_delta("layer1", test_hash()),
            make_add_delta("layer1", test_hash()), // Duplicate
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_err());
    }

    #[test]
    fn hybrid_parallelism_partial() {
        let strategy = HybridCompositionStrategy::new();
        assert!(matches!(strategy.parallelism(), Parallelism::Partial));
    }

    #[test]
    fn hybrid_compute_parallelism_factor() {
        let strategy = HybridCompositionStrategy::new();

        assert_eq!(strategy.compute_parallelism_factor(2, 4), 0.5);
        assert_eq!(strategy.compute_parallelism_factor(0, 4), 0.0);
        assert_eq!(strategy.compute_parallelism_factor(4, 4), 1.0);
        assert_eq!(strategy.compute_parallelism_factor(0, 0), 1.0);
    }

    #[test]
    fn hybrid_custom_classifier() {
        #[derive(Debug)]
        struct AlwaysOrdered;
        impl Classifier for AlwaysOrdered {
            fn classify<T: ArtifactType>(&self, _delta: &StructuralDelta<T>) -> DeltaClass {
                DeltaClass::Ordered(1)
            }
        }

        let strategy = HybridCompositionStrategy::with_classifier(AlwaysOrdered);
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_add_delta("layer1", test_hash()), // Would normally be commutative
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());

        // Should have 0 commutative (all classified as ordered)
        let (commutative, ordered) = strategy.partition_deltas(&deltas);
        assert_eq!(commutative.len(), 0);
        assert_eq!(ordered.len(), 1);
    }
}
