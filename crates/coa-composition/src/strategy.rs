//! Composition strategy trait and core types
//!
//! Provides the [`CompositionStrategy`] trait for pluggable conflict resolution
//! in multi-agent delta composition.

use coa_artifact::{Artifact, ArtifactType, ContentHash, StructuralDelta};
use coa_symbol::SymbolRefIndex;
use std::collections::HashMap;

/// Composition strategy for multi-agent delta coordination
///
/// # Safety
/// All strategies must ensure that `compose()` is deterministic and
/// that `validate()` catches all possible conflicts at construction time.
pub trait CompositionStrategy: Send + Sync + std::fmt::Debug {
    /// Validate that deltas can be composed under this strategy
    ///
    /// # Returns
    /// - `Ok(Validation)` if deltas are compatible
    /// - `Err(CompositionError)` with diagnostic information
    ///
    /// # Performance
    /// Should be O(n log n) or better
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError>;

    /// Compose deltas into final artifact
    ///
    /// # Preconditions
    /// `validate()` must have returned `Ok` for these deltas
    ///
    /// # Returns
    /// New artifact with all deltas applied
    fn compose<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError>;

    /// Parallelism characteristics
    fn parallelism(&self) -> Parallelism;

    /// Conflict detection granularity
    fn granularity(&self) -> Granularity;

    /// Strategy name (for debugging/serialization)
    fn name(&self) -> &'static str;
}

/// Validation result with metadata
#[derive(Debug, Clone)]
pub struct Validation {
    /// Strategy-specific validation data
    pub metadata: ValidationMetadata,

    /// Estimated composition cost
    pub cost_estimate: CompositionCost,
}

impl Validation {
    /// Create minimal validation result
    #[inline]
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            metadata: ValidationMetadata::default(),
            cost_estimate: CompositionCost::default(),
        }
    }

    /// Create validation with metadata
    #[inline]
    #[must_use]
    pub fn with_metadata(metadata: ValidationMetadata) -> Self {
        Self {
            metadata,
            cost_estimate: CompositionCost::default(),
        }
    }

    /// Set cost estimate
    #[inline]
    #[must_use]
    pub fn with_cost(mut self, cost: CompositionCost) -> Self {
        self.cost_estimate = cost;
        self
    }
}

/// Strategy-specific validation metadata
#[derive(Debug, Clone, Default)]
pub struct ValidationMetadata {
    /// Number of batches (for parallel strategies)
    pub batch_count: Option<usize>,

    /// Ordering constraints (for sequential strategies)
    pub ordering: Vec<OrderingConstraint>,

    /// Custom strategy data
    pub custom: HashMap<String, serde_json::Value>,
}

impl ValidationMetadata {
    /// Add ordering constraint
    #[inline]
    pub fn add_ordering(&mut self, constraint: OrderingConstraint) {
        self.ordering.push(constraint);
    }

    /// Set batch count
    #[inline]
    pub fn set_batch_count(&mut self, count: usize) {
        self.batch_count = Some(count);
    }
}

/// Ordering constraint between deltas
#[derive(Debug, Clone)]
pub struct OrderingConstraint {
    /// Index of constrained delta
    pub delta_index: usize,

    /// Indices that must be applied before this delta
    pub must_follow: Vec<usize>,
}

impl OrderingConstraint {
    /// Create new constraint
    #[inline]
    #[must_use]
    pub fn new(delta_index: usize, must_follow: Vec<usize>) -> Self {
        Self {
            delta_index,
            must_follow,
        }
    }
}

/// Composition cost estimate
#[derive(Debug, Clone, Copy)]
pub struct CompositionCost {
    /// Time complexity class
    pub time: TimeComplexity,

    /// Space complexity class
    pub space: SpaceComplexity,

    /// Parallelism factor (1.0 = fully parallel)
    pub parallelism_factor: f64,
}

impl Default for CompositionCost {
    fn default() -> Self {
        Self {
            time: TimeComplexity::ON,
            space: SpaceComplexity::ON,
            parallelism_factor: 0.0,
        }
    }
}

/// Time complexity classes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeComplexity {
    /// O(1) - constant time
    O1,

    /// O(log n) - logarithmic
    OLogN,

    /// O(n) - linear
    ON,

    /// O(n log n) - linearithmic
    ONLogN,

    /// O(n²) - quadratic
    ON2,
}

/// Space complexity classes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceComplexity {
    /// O(1) - constant space
    O1,

    /// O(log n) - logarithmic
    OLogN,

    /// O(n) - linear
    ON,
}

/// Parallelism characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parallelism {
    /// All deltas can be applied in parallel
    Full,

    /// Some batches can be parallel
    Partial,

    /// Sequential only
    None,
}

impl Parallelism {
    /// Check if strategy allows any parallelism
    #[inline]
    #[must_use]
    pub fn allows_parallel(&self) -> bool {
        matches!(self, Self::Full | Self::Partial)
    }
}

/// Conflict detection granularity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    /// Subtree level (ancestors/descendants conflict)
    Subtree,

    /// Exact node match only
    Node,

    /// Attribute-level within node
    Attribute,
}

/// Composition error with diagnostics
#[derive(Debug, thiserror::Error)]
pub enum CompositionError {
    /// Validation failed
    #[error("validation failed: {diagnostic}")]
    ValidationFailed {
        /// Detailed diagnostic
        diagnostic: ValidationDiagnostic,
    },

    /// Composition failed during application
    #[error("composition failed: {0}")]
    CompositionFailed(String),

    /// Deltas not validated
    #[error("deltas not validated")]
    NotValidated,

    /// Invalid delta for strategy
    #[error("invalid delta: {0}")]
    InvalidDelta(String),

    /// Strategy-specific error
    #[error("{0}")]
    Strategy(String),
}

impl CompositionError {
    /// Create validation failed error
    #[inline]
    #[must_use]
    pub fn validation_failed(diagnostic: ValidationDiagnostic) -> Self {
        Self::ValidationFailed { diagnostic }
    }

    /// Create simple validation failed with message
    #[inline]
    #[must_use]
    pub fn validation_failed_simple(kind: ConflictKind, message: impl Into<String>) -> Self {
        Self::ValidationFailed {
            diagnostic: ValidationDiagnostic {
                kind,
                involved_deltas: vec![],
                description: message.into(),
                suggestions: vec![],
            },
        }
    }
}

/// Detailed validation failure diagnostic
#[derive(Debug, Clone)]
pub struct ValidationDiagnostic {
    /// Kind of conflict
    pub kind: ConflictKind,

    /// Involved delta indices
    pub involved_deltas: Vec<usize>,

    /// Human-readable description
    pub description: String,

    /// Suggested resolutions
    pub suggestions: Vec<ResolutionSuggestion>,
}

impl std::fmt::Display for ValidationDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.description)
    }
}

/// Types of conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Overlapping target paths
    OverlappingTargets,

    /// Missing required ordering
    MissingOrdering,

    /// Non-commutative operations in wrong strategy
    NonCommutativeOperations,

    /// Invalid dependencies
    InvalidDependencies,

    /// Strategy capacity exceeded
    CapacityExceeded,
}

/// Resolution suggestions
#[derive(Debug, Clone)]
pub enum ResolutionSuggestion {
    /// Use single writer strategy (disjoint paths)
    UseSingleWriter,

    /// Use ordered composition
    UseOrdered,

    /// Use commutative batch
    UseCommutative,

    /// Decompose targets into non-overlapping paths
    DecomposeTargets {
        /// Common prefix to decompose under
        common_prefix: String,
    },

    /// Add explicit ordering
    AddOrdering {
        /// Suggested order values
        suggested_order: Vec<(usize, u32)>,
    },

    /// Merge into single agent
    MergeAgents,

    /// Use hybrid strategy
    UseHybrid,
}

/// Delta classification for hybrid strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaClass {
    /// Can be applied commutatively
    Commutative,

    /// Requires ordering (value is priority)
    Ordered(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_minimal() {
        let v = Validation::minimal();
        assert!(v.metadata.batch_count.is_none());
        assert!(v.metadata.ordering.is_empty());
    }

    #[test]
    fn validation_with_metadata() {
        let mut meta = ValidationMetadata::default();
        meta.set_batch_count(4);

        let v = Validation::with_metadata(meta);
        assert_eq!(v.metadata.batch_count, Some(4));
    }

    #[test]
    fn validation_with_cost() {
        let cost = CompositionCost {
            time: TimeComplexity::ONLogN,
            space: SpaceComplexity::ON,
            parallelism_factor: 0.5,
        };

        let v = Validation::minimal().with_cost(cost);
        assert_eq!(v.cost_estimate.parallelism_factor, 0.5);
    }

    #[test]
    fn ordering_constraint_new() {
        let c = OrderingConstraint::new(2, vec![0, 1]);
        assert_eq!(c.delta_index, 2);
        assert_eq!(c.must_follow, vec![0, 1]);
    }

    #[test]
    fn parallelism_allows_parallel() {
        assert!(Parallelism::Full.allows_parallel());
        assert!(Parallelism::Partial.allows_parallel());
        assert!(!Parallelism::None.allows_parallel());
    }

    #[test]
    fn composition_error_validation_failed() {
        let diag = ValidationDiagnostic {
            kind: ConflictKind::OverlappingTargets,
            involved_deltas: vec![0, 1],
            description: "test".to_string(),
            suggestions: vec![],
        };

        let err = CompositionError::validation_failed(diag);
        assert!(matches!(err, CompositionError::ValidationFailed { .. }));
    }

    #[test]
    fn composition_error_simple() {
        let err = CompositionError::validation_failed_simple(
            ConflictKind::MissingOrdering,
            "missing order",
        );
        assert!(matches!(err, CompositionError::ValidationFailed { .. }));
    }
}
