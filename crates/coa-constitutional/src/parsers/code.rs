//! Code parser placeholder
//!
//! Full tree-sitter integration will be added once dependency versions align.

use crate::error::ParseError;
use crate::parsers::ArtifactParser;
use coa_artifact::{Artifact, ArtifactType, ContentHash};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// Rust
    Rust,
    /// TypeScript
    TypeScript,
    /// JavaScript
    JavaScript,
    /// Python
    Python,
}

impl Language {
    /// Get file extensions for this language
    #[inline]
    #[must_use]
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::JavaScript => &["js", "jsx"],
            Language::Python => &["py"],
        }
    }

    /// Get human-readable name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
        }
    }
}

/// Parsed code content (simplified)
#[derive(Debug, Clone, PartialEq)]
pub struct CodeContent {
    /// Source language
    pub language: Language,
    /// Source text
    pub source: String,
    /// Extracted symbol names (simplified)
    pub symbols: Vec<String>,
}

/// Code artifact type
#[derive(Debug, Clone)]
pub struct CodeArtifact {
    language: Language,
}

impl CodeArtifact {
    /// Create code artifact for language
    #[inline]
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self { language }
    }
}

impl coa_artifact::__private::Sealed for CodeArtifact {}

impl ArtifactType for CodeArtifact {
    type Content = CodeContent;

    fn hash(content: &Self::Content) -> ContentHash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(content.language.name().as_bytes());
        hasher.update(content.source.as_bytes());
        ContentHash::from_slice(hasher.finalize().as_bytes()).unwrap_or_else(|_| ContentHash::ZERO)
    }

    const TYPE_ID: &'static str = "code";
}

/// Code parser (simplified - full tree-sitter integration pending)
#[derive(Debug, Clone)]
pub struct CodeParser {
    language: Language,
}

impl CodeParser {
    /// Create new parser for language
    #[inline]
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    /// Get parser language
    #[inline]
    #[must_use]
    pub fn language(&self) -> Language {
        self.language
    }

    /// Simple symbol extraction (regex-based placeholder)
    fn extract_symbols(&self, source: &str) -> Vec<String> {
        // Simple extraction of fn, struct, class definitions
        // Full tree-sitter implementation will replace this
        let mut symbols = Vec::new();
        
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(name) = trimmed.strip_prefix("fn ") {
                if let Some(end) = name.find(|c: char| !c.is_alphanumeric() && c != '_') {
                    symbols.push(name[..end].to_string());
                } else {
                    symbols.push(name.to_string());
                }
            } else if let Some(name) = trimmed.strip_prefix("struct ") {
                if let Some(end) = name.find(|c: char| !c.is_alphanumeric() && c != '_') {
                    symbols.push(name[..end].to_string());
                } else {
                    symbols.push(name.to_string());
                }
            } else if let Some(name) = trimmed.strip_prefix("class ") {
                if let Some(end) = name.find(|c: char| !c.is_alphanumeric() && c != '_') {
                    symbols.push(name[..end].to_string());
                } else {
                    symbols.push(name.to_string());
                }
            }
        }
        
        symbols
    }
}

impl ArtifactParser for CodeParser {
    type Output = CodeArtifact;

    fn parse(&self, content: &str) -> Result<Artifact<Self::Output>, ParseError> {
        // Simplified parsing - full AST construction pending tree-sitter integration
        let symbols = self.extract_symbols(content);

        let code_content = CodeContent {
            language: self.language,
            source: content.to_string(),
            symbols,
        };

        Artifact::new(code_content).map_err(|e| {
            ParseError::ValidationError(format!("failed to create artifact: {}", e))
        })
    }

    fn extensions(&self) -> &[&str] {
        self.language.extensions()
    }

    fn priority(&self) -> i32 {
        10 // Higher priority than generic parsers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_extensions() {
        assert_eq!(Language::Rust.extensions(), &["rs"]);
        assert_eq!(Language::TypeScript.extensions(), &["ts", "tsx"]);
        assert_eq!(Language::Python.extensions(), &["py"]);
    }

    #[test]
    fn language_name() {
        assert_eq!(Language::Rust.name(), "rust");
        assert_eq!(Language::Python.name(), "python");
    }

    #[test]
    fn parser_can_parse_rust() {
        let parser = CodeParser::new(Language::Rust);

        assert!(parser.can_parse(std::path::Path::new("main.rs")));
        assert!(parser.can_parse(std::path::Path::new("/path/to/lib.rs")));
        assert!(!parser.can_parse(std::path::Path::new("main.py")));
    }

    #[test]
    fn parser_parse_simple_rust() {
        let parser = CodeParser::new(Language::Rust);

        let source = r#"
fn main() {
    println!("Hello, world!");
}
"#;

        let result = parser.parse(source);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert_eq!(artifact.content().symbols.len(), 1);
        assert!(artifact.content().symbols.contains(&"main".to_string()));
    }

    #[test]
    fn code_artifact_type_id() {
        assert_eq!(CodeArtifact::TYPE_ID, "code");
    }
}
