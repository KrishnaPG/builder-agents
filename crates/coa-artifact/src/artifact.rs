//! Artifact type trait and implementations
//!
//! Defines the [`ArtifactType`] trait for content-addressed typed artifacts.
//! This is a sealed trait - only crate-internal types can implement it.

use crate::hash::ContentHash;
use std::fmt::Debug;
use std::marker::PhantomData;

/// Trait for artifact types
///
/// Implement this for each type of work product (Code, Config, Spec, Binary).
/// This trait is **sealed** - only types defined within this crate can implement it.
///
/// # Type Safety
/// - `Content` must be Send + Sync for multi-threaded access
/// - `hash` must be deterministic and collision-resistant
/// - `TYPE_ID` must be globally unique
///
/// # Example
/// ```rust,ignore
/// pub struct CodeArtifact;
///
/// impl ArtifactType for CodeArtifact {
///     type Content = CodeContent;
///
///     fn hash(content: &Self::Content) -> ContentHash {
///         content.compute_hash()
///     }
///
///     const TYPE_ID: &'static str = "code";
/// }
/// ```
pub trait ArtifactType: Send + Sync + 'static + Debug + private::Sealed {
    /// The content type for this artifact
    type Content: Send + Sync + 'static + Debug + Clone + PartialEq;

    /// Compute content hash
    ///
    /// # Contract
    /// - Must be deterministic (same content → same hash)
    /// - Must be collision-resistant
    /// - Should be incremental/Merkle-based for large content
    fn hash(content: &Self::Content) -> ContentHash;

    /// Artifact type identifier
    ///
    /// Must be:
    /// - Globally unique across all artifact types
    /// - Stable (never changes)
    /// - Lowercase alphanumeric with underscores
    const TYPE_ID: &'static str;

    /// Validate content invariants
    ///
    /// Default implementation always succeeds.
    /// Override to enforce type-specific invariants.
    ///
    /// # Errors
    /// Returns error if content violates invariants
    fn validate_content(_content: &Self::Content) -> Result<(), ArtifactError> {
        Ok(())
    }
}

/// Sealed trait - prevents external implementations
///
/// This trait is crate-public for testing but marked `#[doc(hidden)]`
/// to discourage external implementations.
#[doc(hidden)]
pub mod private {
    /// Sealed trait marker
    pub trait Sealed {}
}

/// Errors related to artifact operations
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// Content invariant violation
    #[error("content invariant violated: {0}")]
    InvariantViolation(String),

    /// Hash mismatch (integrity check failed)
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },

    /// Invalid artifact type
    #[error("invalid artifact type: expected {expected}, got {actual}")]
    InvalidType { expected: String, actual: String },
}

/// Content-addressed typed artifact
///
/// # Type Parameters
/// - `T`: The artifact type (Code, Config, Spec, Binary)
///
/// # Invariants
/// - `hash` is always `T::hash(&content)`
/// - Immutable after construction
/// - Cheap to clone (Arc content if needed for large data)
#[derive(Debug, PartialEq, Eq)]
pub struct Artifact<T: ArtifactType> {
    hash: ContentHash,
    content: T::Content,
    _phantom: PhantomData<T>,
}

impl<T: ArtifactType> Clone for Artifact<T> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash,
            content: self.content.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: ArtifactType> Artifact<T> {
    /// Create new artifact (computes hash and validates)
    ///
    /// # Errors
    /// Returns error if content validation fails
    ///
    /// # Performance
    /// O(n) where n = content size (for hash computation)
    pub fn new(content: T::Content) -> Result<Self, ArtifactError> {
        T::validate_content(&content)?;
        let hash = T::hash(&content);
        Ok(Self {
            hash,
            content,
            _phantom: PhantomData,
        })
    }

    /// Create new artifact without validation (unsafe)
    ///
    /// # Safety
    /// Caller must ensure content invariants are satisfied.
    /// Only used internally when content is known valid.
    #[inline]
    #[must_use]
    pub(crate) fn new_unchecked(content: T::Content) -> Self {
        let hash = T::hash(&content);
        Self {
            hash,
            content,
            _phantom: PhantomData,
        }
    }

    /// Content hash (Merkle root)
    #[inline]
    #[must_use]
    pub fn hash(&self) -> &ContentHash {
        &self.hash
    }

    /// Reference to content
    #[inline]
    #[must_use]
    pub fn content(&self) -> &T::Content {
        &self.content
    }

    /// Clone content out of artifact
    #[inline]
    #[must_use]
    pub fn into_content(self) -> T::Content {
        self.content
    }

    /// Verify integrity (useful after deserialization)
    ///
    /// Returns true if hash matches content recomputation
    #[inline]
    #[must_use]
    pub fn verify(&self) -> bool {
        self.hash == T::hash(&self.content)
    }

    /// Map content to new artifact type
    ///
    /// Transforms content type while preserving hash semantics.
    ///
    /// # Type Parameters
    /// - `U`: Target artifact type
    /// - `F`: Transformation function
    ///
    /// # Errors
    /// Returns error if transformation fails or validation fails
    pub fn map<U, F>(self, f: F) -> Result<Artifact<U>, ArtifactError>
    where
        U: ArtifactType,
        F: FnOnce(T::Content) -> U::Content,
    {
        let new_content = f(self.content);
        Artifact::<U>::new(new_content)
    }

    /// Get type identifier
    #[inline]
    #[must_use]
    pub fn type_id() -> &'static str {
        T::TYPE_ID
    }
}

/// Reference to an artifact of unknown type
///
/// Used for type-erased artifact handling.
#[derive(Debug, Clone)]
pub struct DynArtifactRef {
    pub hash: ContentHash,
    pub type_id: String,
}

impl DynArtifactRef {
    /// Create from typed artifact
    #[inline]
    #[must_use]
    pub fn from_typed<T: ArtifactType>(artifact: &Artifact<T>) -> Self {
        Self {
            hash: *artifact.hash(),
            type_id: T::TYPE_ID.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::ContentHash;

    // Test artifact type
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
    fn artifact_creation_succeeds() {
        let content = TestContent {
            data: "hello".to_string(),
        };
        let artifact = Artifact::<TestArtifact>::new(content.clone()).unwrap();
        assert_eq!(artifact.hash(), &TestArtifact::hash(&content));
    }

    #[test]
    fn artifact_hash_deterministic() {
        let content1 = TestContent {
            data: "test data".to_string(),
        };
        let content2 = TestContent {
            data: "test data".to_string(),
        };

        let a1 = Artifact::<TestArtifact>::new(content1).unwrap();
        let a2 = Artifact::<TestArtifact>::new(content2).unwrap();

        assert_eq!(a1.hash(), a2.hash());
    }

    #[test]
    fn artifact_verify_succeeds_for_valid() {
        let artifact = Artifact::<TestArtifact>::new(TestContent {
            data: "valid".to_string(),
        })
        .unwrap();
        assert!(artifact.verify());
    }

    #[test]
    fn artifact_content_access() {
        let content = TestContent {
            data: "access test".to_string(),
        };
        let artifact = Artifact::<TestArtifact>::new(content.clone()).unwrap();
        assert_eq!(artifact.content().data, "access test");
    }

    #[test]
    fn artifact_into_content() {
        let content = TestContent {
            data: "into test".to_string(),
        };
        let artifact = Artifact::<TestArtifact>::new(content).unwrap();
        let extracted = artifact.into_content();
        assert_eq!(extracted.data, "into test");
    }

    #[test]
    fn artifact_clone_preserves_hash() {
        let artifact = Artifact::<TestArtifact>::new(TestContent {
            data: "clone me".to_string(),
        })
        .unwrap();
        let cloned = artifact.clone();
        assert_eq!(artifact.hash(), cloned.hash());
    }

    #[test]
    fn artifact_type_id_static() {
        assert_eq!(Artifact::<TestArtifact>::type_id(), "test");
    }

    #[test]
    fn dyn_artifact_ref_from_typed() {
        let artifact = Artifact::<TestArtifact>::new(TestContent {
            data: "dynamic".to_string(),
        })
        .unwrap();
        let dyn_ref = DynArtifactRef::from_typed(&artifact);
        assert_eq!(dyn_ref.hash, *artifact.hash());
        assert_eq!(dyn_ref.type_id, "test");
    }

    // Test with validation failure
    #[derive(Debug, Clone)]
    struct ValidatedArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct ValidatedContent {
        value: i32,
    }

    impl private::Sealed for ValidatedArtifact {}

    impl ArtifactType for ValidatedArtifact {
        type Content = ValidatedContent;

        fn hash(content: &Self::Content) -> ContentHash {
            ContentHash::compute(&content.value.to_le_bytes())
        }

        const TYPE_ID: &'static str = "validated";

        fn validate_content(content: &Self::Content) -> Result<(), ArtifactError> {
            if content.value < 0 {
                Err(ArtifactError::InvariantViolation(
                    "value must be non-negative".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn artifact_validation_rejects_invalid() {
        let result = Artifact::<ValidatedArtifact>::new(ValidatedContent { value: -1 });
        assert!(matches!(result, Err(ArtifactError::InvariantViolation(_))));
    }

    #[test]
    fn artifact_validation_accepts_valid() {
        let result = Artifact::<ValidatedArtifact>::new(ValidatedContent { value: 42 });
        assert!(result.is_ok());
    }
}
