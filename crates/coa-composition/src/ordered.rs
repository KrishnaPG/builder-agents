//! Ordered composition strategy
//!
//! Sequential refinement with explicit ordering.

use crate::strategy::{
    CompositionCost, CompositionError, CompositionStrategy, ConflictKind, DeltaClass,
    Granularity, OrderingConstraint, Parallelism, ResolutionSuggestion, TimeComplexity,
    Validation, ValidationDiagnostic, ValidationMetadata,
};
use coa_artifact::{Artifact, ArtifactType, StructuralDelta};
use coa_symbol::SymbolRefIndex;

/// Sequential refinement with explicit ordering
///
/// # Characteristics
/// - Sequential dependency (later deltas see earlier results)
/// - Universal applicability
/// - Deterministic ordering
#[derive(Debug, Clone, Copy, Default)]
pub struct OrderedCompositionStrategy;

impl OrderedCompositionStrategy {
    /// Create new ordered strategy
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract and validate ordering from deltas
    fn extract_ordering<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Vec<Option<u32>>, CompositionError> {
        let orders: Vec<_> = deltas.iter().map(|d| d.order()).collect();

        // Verify all deltas have ordering
        for (i, order) in orders.iter().enumerate() {
            if order.is_none() {
                return Err(CompositionError::validation_failed(
                    ValidationDiagnostic {
                        kind: ConflictKind::MissingOrdering,
                        involved_deltas: vec![i],
                        description: format!("Delta {} missing required order", i),
                        suggestions: vec![ResolutionSuggestion::AddOrdering {
                            suggested_order: vec![(i, i as u32)],
                        }],
                    },
                ));
            }
        }

        Ok(orders)
    }

    /// Build ordering constraints
    fn build_constraints(&self, orders: &[Option<u32>]) -> Vec<OrderingConstraint> {
        let mut constraints = Vec::new();

        for (i, order_i) in orders.iter().enumerate() {
            let order_i = match order_i {
                Some(o) => *o,
                None => continue,
            };

            let mut must_follow = Vec::new();

            for (j, order_j) in orders.iter().enumerate() {
                if i != j {
                    if let Some(order_j) = order_j {
                        if *order_j < order_i {
                            must_follow.push(j);
                        }
                    }
                }
            }

            if !must_follow.is_empty() {
                constraints.push(OrderingConstraint::new(i, must_follow));
            }
        }

        constraints
    }

    /// Sort deltas by order
    fn sort_by_order<'a, T: ArtifactType>(
        &self,
        deltas: &'a [StructuralDelta<T>],
    ) -> Vec<(usize, &'a StructuralDelta<T>)> {
        let mut ordered: Vec<_> = deltas.iter().enumerate().collect();
        ordered.sort_by_key(|(_, d)| d.order().unwrap_or(0));
        ordered
    }

    /// Apply deltas in order
    fn apply_sequential<'a, T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        mut deltas: impl Iterator<Item = &'a StructuralDelta<T>>,
    ) -> Result<Artifact<T>, CompositionError> {
        deltas.try_fold(base.clone(), |_acc, _delta| {
            // Note: Actual application requires ConstitutionalLayer
            Err(CompositionError::CompositionFailed(
                "OrderedCompositionStrategy requires ConstitutionalLayer".to_string(),
            ))
        })
    }
}

impl CompositionStrategy for OrderedCompositionStrategy {
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        _index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        if deltas.len() <= 1 {
            return Ok(Validation::minimal());
        }

        // Extract and validate ordering
        let orders = self.extract_ordering(deltas)?;

        // Build ordering constraints
        let constraints = self.build_constraints(&orders);

        // Build metadata
        let mut metadata = ValidationMetadata::default();
        metadata.ordering = constraints;

        // Check for duplicate orders (warning, not error)
        let mut seen = std::collections::HashSet::new();
        for order in &orders {
            if let Some(o) = order {
                if !seen.insert(*o) {
                    // Duplicate order - still valid but not ideal
                }
            }
        }

        let cost = CompositionCost {
            time: TimeComplexity::ON,
            space: crate::strategy::SpaceComplexity::O1,
            parallelism_factor: 0.0, // Sequential
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

        let ordered = self.sort_by_order(deltas);
        self.apply_sequential(base, ordered.into_iter().map(|(_, d)| d))
    }

    fn parallelism(&self) -> Parallelism {
        Parallelism::None
    }

    fn granularity(&self) -> Granularity {
        Granularity::Attribute
    }

    fn name(&self) -> &'static str {
        "OrderedComposition"
    }
}

/// Classifier for ordered strategy
pub struct OrderedClassifier;

impl OrderedClassifier {
    /// Check if delta needs ordering
    #[inline]
    #[must_use]
    pub fn needs_order<T: ArtifactType>(delta: &StructuralDelta<T>) -> bool {
        // Transform operations typically need ordering
        matches!(
            delta.operation(),
            coa_artifact::DeltaOperation::Transform(_)
        )
    }

    /// Classify delta for hybrid strategy
    #[inline]
    #[must_use]
    pub fn classify<T: ArtifactType>(delta: &StructuralDelta<T>) -> DeltaClass {
        if Self::needs_order(delta) {
            DeltaClass::Ordered(1)
        } else {
            DeltaClass::Commutative
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

    fn make_delta_with_order(
        target: &str,
        order: u32,
        base_hash: ContentHash,
    ) -> StructuralDelta<TestArtifact> {
        StructuralDelta::with_order(
            SymbolPath::from_str(target).unwrap(),
            DeltaOperation::Remove,
            base_hash,
            order,
        )
    }

    fn test_hash() -> ContentHash {
        ContentHash::compute(b"base")
    }

    #[test]
    fn ordered_strategy_new() {
        let strategy = OrderedCompositionStrategy::new();
        assert_eq!(strategy.name(), "OrderedComposition");
        assert!(matches!(strategy.parallelism(), Parallelism::None));
    }

    #[test]
    fn ordered_validates_with_order() {
        let strategy = OrderedCompositionStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_delta_with_order("step1", 1, test_hash()),
            make_delta_with_order("step2", 2, test_hash()),
        ];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert!(!validation.metadata.ordering.is_empty());
    }

    #[test]
    fn ordered_rejects_missing_order() {
        let strategy = OrderedCompositionStrategy::new();
        let index = SymbolRefIndex::new();

        // Need at least 2 deltas to trigger ordering validation
        let deltas: Vec<StructuralDelta<TestArtifact>> = vec![
            StructuralDelta::new(
                SymbolPath::from_str("step1").unwrap(),
                DeltaOperation::Remove,
                test_hash(),
            ),
            StructuralDelta::new(
                SymbolPath::from_str("step2").unwrap(),
                DeltaOperation::Remove,
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
    fn ordered_single_delta_ok() {
        let strategy = OrderedCompositionStrategy::new();
        let index = SymbolRefIndex::new();

        let deltas: Vec<StructuralDelta<TestArtifact>> = vec![];

        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn ordered_builds_constraints() {
        let strategy = OrderedCompositionStrategy::new();

        let orders = vec![Some(3), Some(1), Some(2)];
        let constraints = strategy.build_constraints(&orders);

        // Delta 0 (order 3) must follow deltas 1 (order 1) and 2 (order 2)
        let c0 = constraints.iter().find(|c| c.delta_index == 0).unwrap();
        assert!(c0.must_follow.contains(&1));
        assert!(c0.must_follow.contains(&2));
    }

    #[test]
    fn ordered_classifier_transform_needs_order() {
        use coa_artifact::Transformation;

        struct DummyTransform;
        impl std::fmt::Debug for DummyTransform {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "DummyTransform")
            }
        }

        impl Transformation<TestArtifact> for DummyTransform {
            fn apply(&self, _content: &TestContent) -> Result<TestContent, coa_artifact::TransformError> {
                Ok(TestContent)
            }

            fn describe(&self) -> String {
                "dummy".to_string()
            }
        }

        let delta = StructuralDelta::new(
            SymbolPath::from_str("test").unwrap(),
            coa_artifact::DeltaOperation::Transform(Box::new(DummyTransform)),
            test_hash(),
        );

        assert!(OrderedClassifier::needs_order(&delta));
        assert!(matches!(
            OrderedClassifier::classify(&delta),
            DeltaClass::Ordered(1)
        ));
    }
}
