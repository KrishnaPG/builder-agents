//! COA Composition System
//!
//! Pluggable conflict resolution for multi-agent delta composition.
//!
//! # Core Concepts
//!
//! - [`CompositionStrategy`]: Core trait for conflict resolution strategies
//! - [`SingleWriterStrategy`]: Disjoint subtree claims (maximum safety)
//! - [`OrderedCompositionStrategy`]: Explicit ordering (sequential refinement)
//! - [`CommutativeBatchStrategy`]: Order-independent operations (maximum parallelism)
//! - [`HybridCompositionStrategy`]: Best of both worlds
//! - [`StrategyRegistry`]: Registry for strategy selection
//!
//! # Example
//!
//! ```rust,ignore
//! use coa_composition::{StrategyRegistry, SingleWriterStrategy};
//! use coa_symbol::SymbolRefIndex;
//!
//! // Create registry with defaults
//! let registry = StrategyRegistry::with_defaults();
//!
//! // Get strategy
//! let strategy = registry.select("code", "modify").unwrap();
//!
//! // Validate and compose
//! let validation = strategy.validate(&deltas, &index)?;
//! let result = strategy.compose(&base, &deltas)?;
//! ```

#![warn(missing_docs)]
#![warn(unreachable_pub)]

// Strategy implementations
mod commutative;
mod hybrid;
mod ordered;
mod registry;
mod single_writer;
mod strategy;

// Re-exports
pub use commutative::{CommutativeBatchStrategy, CommutativeClassifier};
pub use hybrid::HybridCompositionStrategy;
pub use ordered::{OrderedClassifier, OrderedCompositionStrategy};
pub use registry::{StrategyHint, StrategyRegistry, StrategySelector};
pub use single_writer::{SingleWriterClassifier, SingleWriterStrategy};
pub use strategy::{
    CompositionCost, CompositionError, CompositionStrategy, ConflictKind, DeltaClass,
    Granularity, OrderingConstraint, Parallelism, ResolutionSuggestion, SpaceComplexity,
    TimeComplexity, Validation, ValidationDiagnostic, ValidationMetadata,
};

/// Version of this crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;
    use coa_artifact::{ArtifactType, ContentHash, DeltaOperation, StructuralDelta, SymbolPath};
    use coa_symbol::SymbolRefIndex;
    use std::str::FromStr;

    // Test artifact
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
    fn registry_integration() {
        let registry = StrategyRegistry::with_defaults();
        let index = SymbolRefIndex::new();

        // Verify registry contains expected strategies
        assert!(registry.contains("single_writer"));
        assert!(registry.contains("ordered"));
        assert!(registry.contains("commutative"));
        assert!(registry.contains("hybrid"));

        // Test single writer directly
        let sw = SingleWriterStrategy::new();
        let deltas = vec![
            make_delta("a.b", test_hash()),
            make_delta("a.c", test_hash()),
        ];
        assert!(sw.validate(&deltas, &index).is_ok());
    }

    #[test]
    fn selector_integration() {
        let selector = StrategySelector::new();

        // Select based on hint
        let name = selector.select_name("code", "modify");
        assert_eq!(name, "hybrid"); // Default hint is Balanced
    }

    #[test]
    fn single_writer_vs_ordered() {
        let index = SymbolRefIndex::new();

        // Single writer accepts disjoint paths
        let sw = SingleWriterStrategy::new();
        let sw_deltas = vec![
            make_delta("auth.login", test_hash()),
            make_delta("auth.register", test_hash()),
        ];
        assert!(sw.validate(&sw_deltas, &index).is_ok());

        // Ordered needs explicit order - with at least 2 deltas
        let ordered = OrderedCompositionStrategy::new();
        let ordered_deltas = vec![
            make_delta("auth.login", test_hash()),
            make_delta("auth.register", test_hash()),
        ];
        // Will fail - no order set
        assert!(ordered.validate(&ordered_deltas, &index).is_err());
    }

    #[test]
    fn commutative_vs_hybrid() {
        let index = SymbolRefIndex::new();

        // Commutative accepts Add/Remove
        let comm = CommutativeBatchStrategy::new();
        let comm_deltas: Vec<StructuralDelta<TestArtifact>> = vec![
            StructuralDelta::new(
                SymbolPath::from_str("layer1").unwrap(),
                DeltaOperation::Add(TestContent),
                test_hash(),
            ),
        ];
        assert!(comm.validate(&comm_deltas, &index).is_ok());

        // Hybrid partitions based on operation type
        let hybrid = HybridCompositionStrategy::new();
        assert!(hybrid.validate(&comm_deltas, &index).is_ok());
    }
}
