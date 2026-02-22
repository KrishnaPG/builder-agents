//! Structural deltas for artifact transformations
//!
//! Provides [`StructuralDelta`] for semantic (not text-based) transformations
//! on artifacts.

use crate::artifact::{Artifact, ArtifactError, ArtifactType};
use crate::hash::ContentHash;
use crate::path::SymbolPath;
use std::fmt::Debug;

/// Semantic transformation on an artifact
///
/// NOT text patches - these are structural operations with meaning.
/// Each delta targets a specific symbol path within an artifact.
///
/// # Type Parameters
/// - `T`: The artifact type this delta operates on
///
/// # Invariants
/// - `base_hash` must match the artifact being transformed
/// - `target` must be a valid path within the artifact's content
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralDelta<T: ArtifactType> {
    /// Target symbol path within artifact tree
    target: SymbolPath,

    /// The transformation operation
    operation: DeltaOperation<T>,

    /// Expected base hash (optimistic concurrency)
    base_hash: ContentHash,

    /// Human-readable description
    description: String,

    /// Optional ordering hint for composition strategies
    order: Option<u32>,
}

impl<T: ArtifactType> StructuralDelta<T> {
    /// Create new delta
    #[inline]
    #[must_use]
    pub fn new(
        target: SymbolPath,
        operation: DeltaOperation<T>,
        base_hash: ContentHash,
    ) -> Self {
        let description = format!("{:?} at {}", operation, target);
        Self {
            target,
            operation,
            base_hash,
            description,
            order: None,
        }
    }

    /// Create delta with explicit ordering
    #[inline]
    #[must_use]
    pub fn with_order(
        target: SymbolPath,
        operation: DeltaOperation<T>,
        base_hash: ContentHash,
        order: u32,
    ) -> Self {
        let description = format!("{:?} at {}", operation, target);
        Self {
            target,
            operation,
            base_hash,
            description,
            order: Some(order),
        }
    }

    /// Target path
    #[inline]
    #[must_use]
    pub fn target(&self) -> &SymbolPath {
        &self.target
    }

    /// Operation
    #[inline]
    #[must_use]
    pub fn operation(&self) -> &DeltaOperation<T> {
        &self.operation
    }

    /// Base hash (for optimistic concurrency)
    #[inline]
    #[must_use]
    pub fn base_hash(&self) -> &ContentHash {
        &self.base_hash
    }

    /// Description
    #[inline]
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Ordering hint
    #[inline]
    #[must_use]
    pub fn order(&self) -> Option<u32> {
        self.order
    }

    /// Verify delta can apply to artifact
    ///
    /// # Errors
    /// Returns error if base hash doesn't match artifact
    pub fn validate_base(&self, artifact: &Artifact<T>) -> Result<(), DeltaError> {
        let actual_hash = artifact.hash();
        if self.base_hash != *actual_hash {
            return Err(DeltaError::BaseMismatch {
                expected: self.base_hash,
                actual: *actual_hash,
            });
        }
        Ok(())
    }

    /// Map to different artifact type
    ///
    /// # Type Parameters
    /// - `U`: Target artifact type
    /// - `F`: Operation transformation function
    #[inline]
    #[must_use]
    pub fn map_operation<U, F>(self, f: F) -> StructuralDelta<U>
    where
        U: ArtifactType,
        F: FnOnce(DeltaOperation<T>) -> DeltaOperation<U>,
    {
        StructuralDelta {
            target: self.target,
            operation: f(self.operation),
            base_hash: self.base_hash,
            description: self.description,
            order: self.order,
        }
    }
}

/// Delta operations by artifact type
#[derive(Debug)]
pub enum DeltaOperation<T: ArtifactType> {
    /// Add new element at target path
    ///
    /// Fails if element already exists.
    Add(T::Content),

    /// Remove element at target path
    ///
    /// Fails if element doesn't exist.
    Remove,

    /// Replace entire element at target path
    ///
    /// Fails if element doesn't exist.
    Replace(T::Content),

    /// Transform with custom operation
    ///
    /// Uses a transformation trait object for custom logic.
    Transform(Box<dyn Transformation<T>>),
}

impl<T: ArtifactType> Clone for DeltaOperation<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Add(content) => Self::Add(content.clone()),
            Self::Remove => Self::Remove,
            Self::Replace(content) => Self::Replace(content.clone()),
            Self::Transform(_) => panic!("Cannot clone Transform operation"),
        }
    }
}

impl<T: ArtifactType> PartialEq for DeltaOperation<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Add(a), Self::Add(b)) => a == b,
            (Self::Remove, Self::Remove) => true,
            (Self::Replace(a), Self::Replace(b)) => a == b,
            _ => false,
        }
    }
}

impl<T: ArtifactType> DeltaOperation<T> {
    /// Check if operation is commutative
    ///
    /// Add/Remove are generally commutative when targeting different paths.
    /// Transform may or may not be commutative depending on implementation.
    #[inline]
    #[must_use]
    pub fn is_commutative(&self) -> bool {
        matches!(self, Self::Add(_) | Self::Remove)
    }

    /// Check if operation reads existing state
    #[inline]
    #[must_use]
    pub fn reads_state(&self) -> bool {
        matches!(self, Self::Replace(_) | Self::Transform(_))
    }

    /// Check if operation writes state
    #[inline]
    #[must_use]
    pub fn writes_state(&self) -> bool {
        true // All operations write
    }
}

/// Transformation trait for custom operations
///
/// Implement this for domain-specific transformations.
pub trait Transformation<T: ArtifactType>: Send + Sync + Debug {
    /// Apply transformation to content
    ///
    /// # Errors
    /// Returns error if transformation cannot be applied
    fn apply(&self, content: &T::Content) -> Result<T::Content, TransformError>;

    /// Describe the transformation
    fn describe(&self) -> String;

    /// Check if transformation is reversible
    #[inline]
    #[must_use]
    fn is_reversible(&self) -> bool {
        false
    }

    /// Get inverse transformation (if available)
    #[inline]
    #[must_use]
    fn inverse(&self) -> Option<Box<dyn Transformation<T>>> {
        None
    }
}

/// Errors specific to delta operations
#[derive(Debug, thiserror::Error)]
pub enum DeltaError {
    /// Base hash mismatch (optimistic concurrency failure)
    #[error("base hash mismatch: expected {expected}, got {actual}")]
    BaseMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },

    /// Target not found
    #[error("target not found: {0}")]
    TargetNotFound(SymbolPath),

    /// Target already exists
    #[error("target already exists: {0}")]
    TargetAlreadyExists(SymbolPath),

    /// Invalid operation for target
    #[error("invalid operation '{operation}' for target '{target}'")]
    InvalidOperation {
        operation: String,
        target: SymbolPath,
    },

    /// Transformation failed
    #[error("transformation failed: {0}")]
    TransformationFailed(#[from] TransformError),

    /// Artifact error
    #[error("artifact error: {0}")]
    Artifact(#[from] ArtifactError),
}

/// Errors during transformation
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    /// Generic failure
    #[error("{0}")]
    Failed(String),

    /// Invalid input
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// State conflict
    #[error("state conflict: {0}")]
    StateConflict(String),

    /// Not implemented
    #[error("transformation not implemented")]
    NotImplemented,
}

/// Builder for constructing deltas
#[derive(Debug)]
pub struct DeltaBuilder<T: ArtifactType> {
    target: Option<SymbolPath>,
    operation: Option<DeltaOperation<T>>,
    base_hash: Option<ContentHash>,
    order: Option<u32>,
}

impl<T: ArtifactType> DeltaBuilder<T> {
    /// Create new builder
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            target: None,
            operation: None,
            base_hash: None,
            order: None,
        }
    }

    /// Set target path
    #[inline]
    #[must_use]
    pub fn target(mut self, path: SymbolPath) -> Self {
        self.target = Some(path);
        self
    }

    /// Set operation
    #[inline]
    #[must_use]
    pub fn operation(mut self, op: DeltaOperation<T>) -> Self {
        self.operation = Some(op);
        self
    }

    /// Set base hash
    #[inline]
    #[must_use]
    pub fn base_hash(mut self, hash: ContentHash) -> Self {
        self.base_hash = Some(hash);
        self
    }

    /// Set base hash from artifact
    #[inline]
    #[must_use]
    pub fn for_artifact(self, artifact: &Artifact<T>) -> Self {
        self.base_hash(*artifact.hash())
    }

    /// Set ordering
    #[inline]
    #[must_use]
    pub fn order(mut self, order: u32) -> Self {
        self.order = Some(order);
        self
    }

    /// Build delta
    ///
    /// # Errors
    /// Returns error if any required field is missing
    pub fn build(self) -> Result<StructuralDelta<T>, DeltaError> {
        let target = self.target.ok_or_else(|| {
            DeltaError::InvalidOperation {
                operation: "missing target".to_string(),
                target: SymbolPath::root(),
            }
        })?;

        let operation = self.operation.ok_or_else(|| {
            DeltaError::InvalidOperation {
                operation: "missing operation".to_string(),
                target: target.clone(),
            }
        })?;

        let base_hash = self.base_hash.ok_or_else(|| {
            DeltaError::InvalidOperation {
                operation: "missing base_hash".to_string(),
                target: target.clone(),
            }
        })?;

        let mut delta = StructuralDelta::new(target, operation, base_hash);
        delta.order = self.order;
        Ok(delta)
    }
}

impl<T: ArtifactType> Default for DeltaBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::private;
    use crate::artifact::ArtifactType;
    use crate::hash::ContentHash;
    use std::str::FromStr;

    #[derive(Debug, Clone)]
    struct TestArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct TestContent {
        data: String,
    }

    impl private::Sealed for TestArtifact {}

    impl ArtifactType for TestArtifact {
        type Content = TestContent;

        fn hash(content: &Self::Content) -> ContentHash {
            ContentHash::compute(content.data.as_bytes())
        }

        const TYPE_ID: &'static str = "test";
    }

    #[test]
    fn delta_new() {
        let path = SymbolPath::from_str("test.path").unwrap();
        let hash = ContentHash::compute(b"base");
        let delta = StructuralDelta::<TestArtifact>::new(
            path.clone(),
            DeltaOperation::Remove,
            hash,
        );

        assert_eq!(delta.target(), &path);
        assert_eq!(delta.base_hash(), &hash);
        assert!(matches!(delta.operation(), DeltaOperation::Remove));
    }

    #[test]
    fn delta_with_order() {
        let path = SymbolPath::from_str("test").unwrap();
        let hash = ContentHash::compute(b"base");
        let delta =
            StructuralDelta::<TestArtifact>::with_order(path, DeltaOperation::Remove, hash, 42);

        assert_eq!(delta.order(), Some(42));
    }

    #[test]
    fn delta_validate_base_success() {
        let content = TestContent {
            data: "test".to_string(),
        };
        let artifact = Artifact::<TestArtifact>::new(content).unwrap();

        let delta = StructuralDelta::<TestArtifact>::new(
            SymbolPath::from_str("test").unwrap(),
            DeltaOperation::Remove,
            *artifact.hash(),
        );

        assert!(delta.validate_base(&artifact).is_ok());
    }

    #[test]
    fn delta_validate_base_fails() {
        let content = TestContent {
            data: "test".to_string(),
        };
        let artifact = Artifact::<TestArtifact>::new(content).unwrap();

        let wrong_hash = ContentHash::compute(b"wrong");
        let delta = StructuralDelta::<TestArtifact>::new(
            SymbolPath::from_str("test").unwrap(),
            DeltaOperation::Remove,
            wrong_hash,
        );

        let result = delta.validate_base(&artifact);
        assert!(matches!(result, Err(DeltaError::BaseMismatch { .. })));
    }

    #[test]
    fn delta_operation_is_commutative() {
        let content = TestContent {
            data: "x".to_string(),
        };
        assert!(DeltaOperation::<TestArtifact>::Add(content.clone()).is_commutative());
        assert!(DeltaOperation::<TestArtifact>::Remove.is_commutative());
        assert!(!DeltaOperation::<TestArtifact>::Replace(content.clone()).is_commutative());
    }

    #[test]
    fn delta_operation_reads_state() {
        let content = TestContent {
            data: "x".to_string(),
        };
        assert!(!DeltaOperation::<TestArtifact>::Add(content.clone()).reads_state());
        assert!(!DeltaOperation::<TestArtifact>::Remove.reads_state());
        assert!(DeltaOperation::<TestArtifact>::Replace(content.clone()).reads_state());
    }

    #[test]
    fn delta_builder_success() {
        let content = TestContent {
            data: "test".to_string(),
        };
        let artifact = Artifact::<TestArtifact>::new(content).unwrap();

        let delta = DeltaBuilder::<TestArtifact>::new()
            .target(SymbolPath::from_str("path").unwrap())
            .operation(DeltaOperation::Remove)
            .for_artifact(&artifact)
            .order(1)
            .build()
            .unwrap();

        assert_eq!(delta.target().to_string(), "path");
        assert_eq!(delta.order(), Some(1));
    }

    #[test]
    fn delta_builder_missing_target() {
        let result = DeltaBuilder::<TestArtifact>::new()
            .operation(DeltaOperation::Remove)
            .base_hash(ContentHash::compute(b"test"))
            .build();
        assert!(matches!(result, Err(DeltaError::InvalidOperation { .. })));
    }

    #[test]
    fn delta_builder_missing_operation() {
        let result = DeltaBuilder::<TestArtifact>::new()
            .target(SymbolPath::from_str("test").unwrap())
            .base_hash(ContentHash::compute(b"test"))
            .build();
        assert!(matches!(result, Err(DeltaError::InvalidOperation { .. })));
    }
}
