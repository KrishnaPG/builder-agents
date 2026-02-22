//! COA Artifact System
//!
//! Typed, content-addressed artifacts with structural delta support.
//!
//! # Core Concepts
//!
//! - [`Artifact<T>`]: Content-addressed container for typed content
//! - [`ArtifactType`]: Trait for defining artifact types (Code, Config, Spec, etc.)
//! - [`ContentHash`]: 32-byte Blake3 hash for content addressing
//! - [`StructuralDelta<T>`]: Semantic transformation operations
//! - [`SymbolPath`]: Hierarchical addressing within artifacts
//!
//! # Example
//!
//! ```rust,ignore
//! use coa_artifact::{Artifact, ContentHash, ArtifactType};
//!
//! // Create a code artifact
//! let content = CodeContent::parse(source, Language::Rust)?;
//! let artifact = Artifact::<CodeArtifact>::new(content)?;
//!
//! // Content hash is computed automatically
//! println!("Hash: {}", artifact.hash());
//! ```

#![warn(unreachable_pub)]
#![allow(missing_docs)]

// Core modules
mod artifact;
mod delta;
mod hash;
mod path;

// Re-exports
pub use artifact::{Artifact, ArtifactError, ArtifactType, DynArtifactRef};

/// Sealed trait support - for implementing custom artifact types.
/// **Note:** This is only for internal/testing use and may change.
#[doc(hidden)]
pub mod __private {
    pub use super::artifact::private::Sealed;
}
pub use delta::{
    DeltaBuilder, DeltaError, DeltaOperation, StructuralDelta, TransformError, Transformation,
};
pub use hash::{ContentHash, HashError};
pub use path::{PathError, SymbolPath};

/// Artifact type implementations
pub mod types {
    //! Concrete artifact types

    // Will be implemented next
}

/// Merkle tree support
pub mod merkle;

pub use merkle::{ArtifactMerkleTree, Blake3Hasher, MerkleProof};

/// Version of this crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::artifact::private;
    use crate::hash::ContentHash;
    use std::str::FromStr;

    // Test artifact for integration
    #[derive(Debug, Clone)]
    struct TextArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct TextContent {
        lines: Vec<String>,
    }

    impl private::Sealed for TextArtifact {}

    impl ArtifactType for TextArtifact {
        type Content = TextContent;

        fn hash(content: &Self::Content) -> ContentHash {
            let data = content.lines.join("\n");
            ContentHash::compute(data.as_bytes())
        }

        const TYPE_ID: &'static str = "text";
    }

    #[test]
    fn full_artifact_lifecycle() {
        // Create artifact
        let content = TextContent {
            lines: vec!["line 1".to_string(), "line 2".to_string()],
        };
        let artifact = Artifact::<TextArtifact>::new(content).unwrap();

        // Verify hash
        assert!(artifact.verify());

        // Create delta
        let new_content = TextContent {
            lines: vec!["line 1".to_string(), "modified".to_string()],
        };
        let delta = StructuralDelta::<TextArtifact>::new(
            SymbolPath::single("lines"),
            DeltaOperation::Replace(new_content),
            *artifact.hash(),
        );

        // Verify delta can validate base
        assert!(delta.validate_base(&artifact).is_ok());
    }

    #[test]
    fn hash_and_path_integration() {
        let path = SymbolPath::from_str("module.submodule.function").unwrap();
        assert_eq!(path.len(), 3);

        let data = b"test data for hashing";
        let hash = ContentHash::compute(data);
        assert!(!hash.is_zero());

        let short = hash.short();
        assert_eq!(short.len(), 16);
    }
}
