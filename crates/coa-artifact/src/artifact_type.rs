//! Artifact Type Trait
//!
//! Defines the interface for types that can be stored as artifacts.
//! Each artifact type specifies its content type and hashing strategy.

use crate::hash::ContentHash;

/// Sealed trait to prevent external implementations
mod private {
    pub trait Sealed {}
}

/// Trait for artifact types
///
/// Implement this trait for each type of work product (code, config, spec, etc.).
/// The trait is sealed to ensure only approved implementations exist.
///
/// # Type Safety
/// - `Content` is the actual data type stored
/// - `hash()` must be deterministic and collision-resistant
/// - `TYPE_ID` provides a unique identifier for serialization
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
pub trait ArtifactType: Send + Sync + 'static + private::Sealed {
    /// The content type for this artifact
    ///
    /// This is the actual data stored within the artifact.
    type Content: ArtifactContent;

    /// Compute deterministic content hash
    ///
    /// # Contract
    /// - Must be deterministic: same content → same hash
    /// - Must be collision-resistant
    /// - Should be fast (called on every artifact creation)
    fn hash(content: &Self::Content) -> ContentHash;

    /// Unique type identifier for serialization
    ///
    /// Used to distinguish artifact types in storage and wire formats.
    const TYPE_ID: &'static str;

    /// Get schema for this artifact type (optional)
    ///
    /// Returns JSON schema if available for validation.
    fn schema() -> Option<schemars::schema::RootSchema> {
        None
    }
}

/// Trait for artifact content types
///
/// This is a marker trait for types that can be stored in artifacts.
/// It requires Send + Sync for thread safety, Clone for duplication,
/// and Debug/PartialEq for testing.
pub trait ArtifactContent: Send + Sync + Clone + std::fmt::Debug + PartialEq {
    /// Get approximate memory size in bytes
    ///
    /// Used for cache accounting and memory management.
    fn approximate_size(&self) -> usize;
}

/// Trait for versioned artifact content
///
/// Allows artifacts to track their schema version for migrations.
pub trait VersionedContent: ArtifactContent {
    /// Schema version number
    const VERSION: u32;

    /// Migrate from previous version
    ///
    /// # Errors
    /// Returns error if migration fails
    fn migrate_from(previous: &Self) -> Result<Self, MigrationError>
    where
        Self: Sized;
}

/// Errors during content migration
#[derive(Debug, Clone, thiserror::Error)]
pub enum MigrationError {
    #[error("unsupported version: {from} -> {to}")]
    UnsupportedVersion { from: u32, to: u32 },

    #[error("migration failed: {0}")]
    Failed(String),
}

/// Marker trait for immutable artifact content
///
/// Types implementing this trait guarantee that their internal state
/// cannot be modified after creation, enabling safe sharing.
pub trait ImmutableContent: ArtifactContent {}

// Sealed trait implementations for internal types
impl private::Sealed for crate::types::code::CodeArtifact {}
impl private::Sealed for crate::types::config::ConfigArtifact {}
impl private::Sealed for crate::types::spec::SpecArtifact {}
impl private::Sealed for crate::types::binary::BinaryArtifact {}

#[cfg(test)]
mod tests {
    use super::*;

    // Test type for verifying trait bounds
    #[derive(Debug, Clone, PartialEq)]
    struct TestContent {
        data: String,
    }

    impl ArtifactContent for TestContent {
        fn approximate_size(&self) -> usize {
            self.data.len()
        }
    }

    #[test]
    fn artifact_content_size() {
        let content = TestContent {
            data: "hello".to_string(),
        };
        assert_eq!(content.approximate_size(), 5);
    }

    #[test]
    fn artifact_content_equality() {
        let c1 = TestContent {
            data: "test".to_string(),
        };
        let c2 = TestContent {
            data: "test".to_string(),
        };
        let c3 = TestContent {
            data: "other".to_string(),
        };

        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }
}
