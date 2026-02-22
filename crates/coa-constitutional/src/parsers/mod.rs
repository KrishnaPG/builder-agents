//! Artifact parsers for different file formats
//!
//! Provides parsing from external file formats into typed Artifacts:
//! - Code files (Rust, TypeScript, Python)
//! - Config files (JSON, YAML) via serde
//! - Spec files (Markdown) via pulldown-cmark

use crate::error::ParseError;
use coa_artifact::{Artifact, ArtifactType};
use std::path::Path;

mod code;
mod json;
mod markdown;
mod yaml;

pub use code::{CodeParser, CodeArtifact, CodeContent, Language};
pub use json::{JsonParser, JsonArtifact, JsonContent};
pub use markdown::{MarkdownParser, MarkdownArtifact, MarkdownContent};
pub use yaml::{YamlParser, YamlArtifact, YamlContent};

/// Parser trait for converting file content into typed artifacts
///
/// Implement this trait to add support for new file formats.
pub trait ArtifactParser: Send + Sync + 'static {
    /// The artifact type this parser produces
    type Output: ArtifactType;

    /// Parse content string into artifact
    fn parse(&self, content: &str) -> Result<Artifact<Self::Output>, ParseError>;

    /// Check if this parser can handle the given path
    fn can_parse(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.extensions().contains(&ext))
            .unwrap_or(false)
    }

    /// Supported file extensions (without dot)
    fn extensions(&self) -> &[&str];

    /// Parser priority (higher = tried first when multiple parsers match)
    fn priority(&self) -> i32 {
        0
    }
}

/// Parser registration for dynamic parser management
pub struct ParserRegistry {
    parsers: Vec<Box<dyn DynArtifactParser>>,
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ParserRegistry {
    fn clone(&self) -> Self {
        // Create fresh registry - parsers are stateless
        default_parsers()
    }
}

impl std::fmt::Debug for ParserRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserRegistry")
            .field("parser_count", &self.parsers.len())
            .field("extensions", &self.all_extensions())
            .finish()
    }
}

/// Type-erased parser for storage in registry
pub trait DynArtifactParser: Send + Sync {
    fn can_parse(&self, path: &Path) -> bool;
    fn priority(&self) -> i32;
    fn extensions(&self) -> &[&str];
}

impl<P> DynArtifactParser for P
where
    P: ArtifactParser,
{
    fn can_parse(&self, path: &Path) -> bool {
        ArtifactParser::can_parse(self, path)
    }

    fn priority(&self) -> i32 {
        ArtifactParser::priority(self)
    }

    fn extensions(&self) -> &[&str] {
        ArtifactParser::extensions(self)
    }
}

impl ParserRegistry {
    /// Create empty registry
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            parsers: Vec::new(),
        }
    }

    /// Register a parser
    pub fn register<P: ArtifactParser>(&mut self, parser: P) {
        self.parsers.push(Box::new(parser));
        // Sort by priority (higher first)
        self.parsers
            .sort_by_key(|p| std::cmp::Reverse(p.priority()));
    }

    /// Find parser for path
    #[must_use]
    pub fn find_for_path(&self, path: &Path) -> Option<&dyn DynArtifactParser> {
        self.parsers.iter().find(|p| p.can_parse(path)).map(|p| &**p)
    }

    /// Get all registered extensions
    #[must_use]
    pub fn all_extensions(&self) -> Vec<&str> {
        self.parsers
            .iter()
            .flat_map(|p| p.extensions())
            .copied()
            .collect()
    }
}

/// Create default parser registry with built-in parsers
#[inline]
#[must_use]
pub fn default_parsers() -> ParserRegistry {
    let mut registry = ParserRegistry::new();

    // Code parsers
    registry.register(CodeParser::new(Language::Rust));
    registry.register(CodeParser::new(Language::TypeScript));
    registry.register(CodeParser::new(Language::Python));

    // Config parsers
    registry.register(JsonParser);
    registry.register(YamlParser);

    // Spec parsers
    registry.register(MarkdownParser);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestParser;

    #[derive(Debug, Clone)]
    struct TestArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct TestContent;

    impl coa_artifact::__private::Sealed for TestArtifact {}

    impl ArtifactType for TestArtifact {
        type Content = TestContent;

        fn hash(_content: &Self::Content) -> coa_artifact::ContentHash {
            coa_artifact::ContentHash::compute(b"test")
        }

        const TYPE_ID: &'static str = "test";
    }

    impl ArtifactParser for TestParser {
        type Output = TestArtifact;

        fn parse(&self, _content: &str) -> Result<Artifact<Self::Output>, ParseError> {
            Artifact::new(TestContent).map_err(|e| {
                ParseError::ValidationError(format!("artifact error: {}", e))
            })
        }

        fn extensions(&self) -> &[&str] {
            &["test"]
        }
    }

    #[test]
    fn parser_can_parse_by_extension() {
        let parser = TestParser;

        assert!(parser.can_parse(Path::new("file.test")));
        assert!(parser.can_parse(Path::new("/path/to/file.test")));
        assert!(!parser.can_parse(Path::new("file.txt")));
        assert!(!parser.can_parse(Path::new("file")));
    }

    #[test]
    fn registry_find_parser() {
        let mut registry = ParserRegistry::new();
        registry.register(TestParser);

        let found = registry.find_for_path(Path::new("file.test"));
        assert!(found.is_some());

        let not_found = registry.find_for_path(Path::new("file.txt"));
        assert!(not_found.is_none());
    }

    #[test]
    fn registry_all_extensions() {
        let mut registry = ParserRegistry::new();
        registry.register(TestParser);

        let exts = registry.all_extensions();
        assert!(exts.contains(&"test"));
    }

    #[test]
    fn registry_clone() {
        let registry = default_parsers();
        let cloned = registry.clone();
        
        // Both should have the same extensions
        assert_eq!(registry.all_extensions(), cloned.all_extensions());
    }

    #[test]
    fn registry_debug() {
        let registry = default_parsers();
        let debug_str = format!("{:?}", registry);
        assert!(debug_str.contains("ParserRegistry"));
    }
}
