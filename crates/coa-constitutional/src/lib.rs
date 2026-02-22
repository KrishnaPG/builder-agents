//! COA Constitutional Layer
//!
//! The trusted boundary between the external world (files) and the COA's
//! internal artifact system.
//!
//! # Core Operations
//!
//! - **Ingress**: Parse external files into typed `Artifact<T>`
//! - **Transform**: Apply `StructuralDelta<T>` to produce new artifacts
//! - **Egress**: Serialize artifacts back to external format
//!
//! # Architecture
//!
//! ```text
//! File System → Parser → Artifact<T> → Transformer → Artifact<T>' → Serializer → File System
//!                  ↑___________↓
//!                    ArtifactCache (content-addressed)
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use coa_constitutional::ConstitutionalLayer;
//! use coa_constitutional::parsers::Language;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let layer = ConstitutionalLayer::new();
//!
//! // Parse file into typed artifact
//! let artifact = layer
//!     .parse_ingress::<CodeArtifact>("src/main.rs")
//!     .await?;
//!
//! // Apply delta
//! let new_artifact = layer.apply_delta(&artifact, &delta)?;
//!
//! // Serialize back
//! layer.serialize_egress(&new_artifact, "src/main.rs").await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(unreachable_pub)]

// Core modules
pub mod cache;
pub mod error;
pub mod layer;
pub mod parsers;

// Re-exports for convenience
pub use cache::{ArtifactCache, CacheStats, TypedCacheKey};
pub use error::{ApplyError, CacheError, ConstitutionalError, ParseError, SerializeError};

/// Version of this crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude module for common imports
pub mod prelude {
    //! Common imports for working with the Constitutional Layer
    pub use crate::cache::{ArtifactCache, CacheStats};
    pub use crate::error::{ApplyError, ConstitutionalError, ParseError, SerializeError};
    pub use crate::parsers::{ArtifactParser, CodeParser, JsonParser, Language, MarkdownParser, YamlParser};
    pub use coa_artifact::{Artifact, ArtifactType, ContentHash, StructuralDelta};
    pub use coa_composition::CompositionStrategy;
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::parsers::{CodeParser, JsonParser, Language, YamlParser};
    use coa_artifact::{ArtifactType, ContentHash};

    #[derive(Debug, Clone)]
    struct TestArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct TestContent {
        data: String,
    }

    impl coa_artifact::__private::Sealed for TestArtifact {}

    impl ArtifactType for TestArtifact {
        type Content = TestContent;

        fn hash(content: &Self::Content) -> ContentHash {
            ContentHash::compute(content.data.as_bytes())
        }

        const TYPE_ID: &'static str = "test";
    }

    #[test]
    fn cache_roundtrip() {
        // Would need async runtime
    }

    #[test]
    fn parser_registry() {
        use crate::parsers::ParserRegistry;

        let mut registry = ParserRegistry::new();
        registry.register(CodeParser::new(Language::Rust));
        registry.register(JsonParser);
        registry.register(YamlParser);

        let extensions = registry.all_extensions();
        assert!(extensions.contains(&"rs"));
        assert!(extensions.contains(&"json"));
        assert!(extensions.contains(&"yaml"));
    }
}
