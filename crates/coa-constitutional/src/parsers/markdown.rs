//! Markdown specification parser
//!
//! Uses pulldown-cmark for parsing Markdown into structured spec artifacts.

use crate::error::ParseError;
use crate::parsers::ArtifactParser;
use coa_artifact::{Artifact, ArtifactType, ContentHash};
use pulldown_cmark::{Event, Parser as MdParser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

/// Markdown specification content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkdownContent {
    /// Original source
    pub source: String,
    /// Document title (first H1)
    pub title: Option<String>,
    /// Section hierarchy
    pub sections: Vec<Section>,
    /// Code blocks extracted from document
    pub code_blocks: Vec<CodeBlock>,
    /// Frontmatter metadata (if any)
    pub metadata: Option<serde_yaml::Value>,
}

/// Document section
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Heading level (1-6)
    pub level: u8,
    /// Section title
    pub title: String,
    /// Content text (markdown)
    pub content: String,
    /// Nested subsections
    pub children: Vec<Section>,
}

/// Code block extracted from document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeBlock {
    /// Language identifier (e.g., "rust", "json")
    pub language: Option<String>,
    /// Code content
    pub code: String,
    /// Associated heading (nearest parent)
    pub context: Option<String>,
}

/// Markdown spec artifact type
#[derive(Debug, Clone)]
pub struct MarkdownArtifact;

impl coa_artifact::__private::Sealed for MarkdownArtifact {}

impl ArtifactType for MarkdownArtifact {
    type Content = MarkdownContent;

    fn hash(content: &Self::Content) -> ContentHash {
        ContentHash::compute(content.source.as_bytes())
    }

    const TYPE_ID: &'static str = "markdown";
}

/// Markdown parser
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkdownParser;

impl MarkdownParser {
    /// Create new markdown parser
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract frontmatter from content
    fn extract_frontmatter(&self, content: &str) -> (Option<serde_yaml::Value>, String) {
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let frontmatter = &content[3..end + 3];
                let rest = content[end + 6..].to_string();
                match serde_yaml::from_str(frontmatter) {
                    Ok(value) => return (Some(value), rest),
                    Err(_) => return (None, content.to_string()),
                }
            }
        }
        (None, content.to_string())
    }

    /// Parse markdown into structured content
    fn parse_structure(&self, content: &str) -> MarkdownContent {
        let parser = MdParser::new(content);

        let mut sections: Vec<Section> = Vec::new();
        let mut code_blocks: Vec<CodeBlock> = Vec::new();
        let mut current_section: Option<Section> = None;
        let mut in_code_block = false;
        let mut current_code: Option<(Option<String>, String)> = None;
        let mut current_heading: Option<String> = None;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    // Finish previous section if any
                    if let Some(section) = current_section.take() {
                        Self::push_section(&mut sections, section);
                    }

                    // Start new section
                    current_section = Some(Section {
                        level: level as u8,
                        title: String::new(),
                        content: String::new(),
                        children: Vec::new(),
                    });
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some(ref mut section) = current_section {
                        current_heading = Some(section.title.clone());
                    }
                }
                Event::Text(text) => {
                    if let Some(ref mut section) = current_section {
                        if section.title.is_empty() {
                            section.title = text.to_string();
                        } else {
                            section.content.push_str(&text);
                        }
                    } else if in_code_block {
                        if let Some((_, ref mut code)) = current_code {
                            code.push_str(&text);
                        }
                    }
                }
                Event::Code(code) => {
                    if in_code_block {
                        if let Some((_, ref mut c)) = current_code {
                            c.push_str(&code);
                        }
                    }
                }
                Event::Start(Tag::CodeBlock(lang)) => {
                    in_code_block = true;
                    let lang_str = match lang {
                        pulldown_cmark::CodeBlockKind::Fenced(lang_str) => {
                            if lang_str.is_empty() {
                                None
                            } else {
                                Some(lang_str.to_string())
                            }
                        }
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                    current_code = Some((lang_str, String::new()));
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                    if let Some((lang, code)) = current_code.take() {
                        code_blocks.push(CodeBlock {
                            language: lang,
                            code,
                            context: current_heading.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        // Finish last section
        if let Some(section) = current_section.take() {
            Self::push_section(&mut sections, section);
        }

        // Find title (first H1)
        let title = sections
            .iter()
            .find(|s| s.level == 1)
            .map(|s| s.title.clone());

        MarkdownContent {
            source: content.to_string(),
            title,
            sections,
            code_blocks,
            metadata: None, // Set by caller after frontmatter extraction
        }
    }

    /// Push section to appropriate parent
    fn push_section(sections: &mut Vec<Section>, section: Section) {
        // Find parent based on level
        if let Some(parent) = sections.iter_mut().rev().find(|s| s.level < section.level) {
            parent.children.push(section);
        } else {
            sections.push(section);
        }
    }
}

impl ArtifactParser for MarkdownParser {
    type Output = MarkdownArtifact;

    fn parse(&self, content: &str) -> Result<Artifact<Self::Output>, ParseError> {
        // Extract frontmatter
        let (metadata, body) = self.extract_frontmatter(content);

        // Parse structure
        let mut md_content = self.parse_structure(&body);
        md_content.metadata = metadata;

        // Create artifact
        Artifact::new(md_content).map_err(|e| {
            ParseError::ValidationError(format!("artifact creation failed: {}", e))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["md", "markdown"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_parser_basic() {
        let parser = MarkdownParser;
        let content = r#"# Title

Some content here.

## Section 1

More content.
"#;

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert_eq!(artifact.content().title, Some("Title".to_string()));
        assert!(!artifact.content().sections.is_empty());
    }

    #[test]
    fn markdown_parser_with_code() {
        let parser = MarkdownParser;
        let content = r#"# Spec

```rust
fn main() {}
```

```json
{"key": "value"}
```
"#;

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert_eq!(artifact.content().code_blocks.len(), 2);

        let first = &artifact.content().code_blocks[0];
        assert_eq!(first.language, Some("rust".to_string()));
    }

    #[test]
    fn markdown_parser_with_frontmatter() {
        let parser = MarkdownParser;
        let content = r#"---
title: My Spec
author: Test
---

# Actual Title

Content.
"#;

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert!(artifact.content().metadata.is_some());
        let metadata = artifact.content().metadata.as_ref().unwrap();
        assert_eq!(metadata["title"], "My Spec");
    }

    #[test]
    fn markdown_parser_empty() {
        let parser = MarkdownParser;
        let content = "";

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert!(artifact.content().title.is_none());
    }

    #[test]
    fn markdown_artifact_type_id() {
        assert_eq!(MarkdownArtifact::TYPE_ID, "markdown");
    }

    #[test]
    fn markdown_parser_extensions() {
        let parser = MarkdownParser;
        assert!(parser.extensions().contains(&"md"));
        assert!(parser.extensions().contains(&"markdown"));
    }
}
