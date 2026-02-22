//! Spec Artifact Type
//!
//! Structured specification documents (Markdown-based).
//! Supports requirements extraction and cross-references.

use std::collections::HashMap;

use crate::artifact_type::{ArtifactContent, ArtifactType};
use crate::hash::ContentHash;
use crate::merkle::ArtifactMerkleTree;

/// Spec artifact type marker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecArtifact;

impl ArtifactType for SpecArtifact {
    type Content = SpecContent;

    #[inline]
    fn hash(content: &Self::Content) -> ContentHash {
        content.merkle_root()
    }

    const TYPE_ID: &'static str = "spec";
}

/// Specification document content
#[derive(Debug, Clone, PartialEq)]
pub struct SpecContent {
    /// Document sections
    sections: Vec<Section>,

    /// Extracted requirements
    requirements: Vec<Requirement>,

    /// Cross-references
    references: Vec<CrossRef>,

    /// Raw markdown source
    source: String,

    /// Merkle tree for hashing
    merkle_tree: ArtifactMerkleTree,
}

impl SpecContent {
    /// Parse from markdown source
    ///
    /// # Errors
    /// Returns error if parsing fails
    pub fn parse(source: &str) -> Result<Self, SpecParseError> {
        let sections = parse_sections(source)?;
        let requirements = extract_requirements(&sections);
        let references = extract_references(&sections);

        // Build Merkle tree from sections
        let leaves: Vec<ContentHash> = sections
            .iter()
            .map(|s| ContentHash::compute(s.text.as_bytes()))
            .collect();
        let merkle_tree = ArtifactMerkleTree::from_leaves(&leaves);

        Ok(Self {
            sections,
            requirements,
            references,
            source: source.to_string(),
            merkle_tree,
        })
    }

    /// Get sections
    #[inline]
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// Get requirements
    #[inline]
    #[must_use]
    pub fn requirements(&self) -> &[Requirement] {
        &self.requirements
    }

    /// Get references
    #[inline]
    #[must_use]
    pub fn references(&self) -> &[CrossRef] {
        &self.references
    }

    /// Get source
    #[inline]
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Compute Merkle root
    #[inline]
    #[must_use]
    pub fn merkle_root(&self) -> ContentHash {
        self.merkle_tree.root_or_default()
    }

    /// Find section by ID
    #[inline]
    #[must_use]
    pub fn find_section(&self, id: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.id.as_deref() == Some(id))
    }

    /// Find requirement by ID
    #[inline]
    #[must_use]
    pub fn find_requirement(&self, id: &str) -> Option<&Requirement> {
        self.requirements.iter().find(|r| r.id == id)
    }

    /// Get section hierarchy
    #[inline]
    #[must_use]
    pub fn section_tree(&self) -> Vec<&Section> {
        // Return top-level sections (level 1 and 2)
        self.sections
            .iter()
            .filter(|s| s.level <= 2)
            .collect()
    }
}

impl ArtifactContent for SpecContent {
    #[inline]
    fn approximate_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.source.len()
            + self.sections.iter().map(|s| s.text.len()).sum::<usize>()
    }
}

/// Document section
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// Section level (1 = H1, 2 = H2, etc.)
    pub level: u8,

    /// Section title
    pub title: String,

    /// Optional anchor ID
    pub id: Option<String>,

    /// Section text content
    pub text: String,

    /// Line range in source
    pub line_range: std::ops::Range<usize>,
}

/// Requirement extracted from spec
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// Requirement ID (e.g., "REQ-001")
    pub id: String,

    /// Requirement text
    pub text: String,

    /// Priority level
    pub priority: Priority,

    /// Status
    pub status: ReqStatus,

    /// Section reference
    pub section_title: String,
}

/// Requirement priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Priority {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Requirement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReqStatus {
    #[default]
    Draft,
    Proposed,
    Accepted,
    Implemented,
    Verified,
    Deprecated,
}

/// Cross-reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRef {
    /// Source section
    pub from: String,

    /// Target section/ID
    pub to: String,

    /// Reference type
    pub kind: RefKind,
}

/// Reference kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    /// Links to
    LinksTo,
    /// Depends on
    DependsOn,
    /// See also
    SeeAlso,
    /// Implements
    Implements,
}

/// Parse error
#[derive(Debug, Clone, thiserror::Error)]
pub enum SpecParseError {
    #[error("parse failed: {0}")]
    ParseFailed(String),
}

/// Parse sections from markdown
fn parse_sections(source: &str) -> Result<Vec<Section>, SpecParseError> {
    let mut sections = Vec::new();
    let mut current_section: Option<Section> = None;
    let mut line_num = 0;

    for line in source.lines() {
        line_num += 1;

        // Check for heading
        if let Some((level, title)) = parse_heading(line) {
            // Save previous section
            if let Some(section) = current_section.take() {
                sections.push(section);
            }

            // Extract ID from heading if present (# Title {#id})
            let (title, id) = extract_heading_id(&title);

            current_section = Some(Section {
                level,
                title,
                id,
                text: String::new(),
                line_range: line_num..line_num,
            });
        } else if let Some(ref mut section) = current_section {
            section.text.push_str(line);
            section.text.push('\n');
        }
    }

    // Don't forget the last section
    if let Some(section) = current_section {
        sections.push(section);
    }

    // Update line ranges
    for section in &mut sections {
        section.line_range.end = line_num;
    }

    Ok(sections)
}

/// Parse heading line, returns (level, title)
fn parse_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();

    // Count leading #
    let level = trimmed.chars().take_while(|&c| c == '#').count();

    if level == 0 || level > 6 {
        return None;
    }

    let title = trimmed[level..].trim().to_string();
    Some((level as u8, title))
}

/// Extract {#id} from heading
fn extract_heading_id(title: &str) -> (String, Option<String>) {
    if let Some(start) = title.rfind("{#") {
        if let Some(end) = title[start..].find('}') {
            let id = title[start + 2..start + end].to_string();
            let clean_title = title[..start].trim().to_string();
            return (clean_title, Some(id));
        }
    }
    (title.to_string(), None)
}

/// Extract requirements from sections
fn extract_requirements(sections: &[Section]) -> Vec<Requirement> {
    let mut requirements = Vec::new();
    let mut req_counter = 0;

    for section in sections {
        // Look for requirement patterns:
        // - [REQ-XXX] or REQ-XXX:
        // - **Requirement:** or *Requirement:*
        for line in section.text.lines() {
            if let Some(req) = parse_requirement_line(line, &section.title, &mut req_counter) {
                requirements.push(req);
            }
        }
    }

    requirements
}

/// Parse a single requirement line
fn parse_requirement_line(
    line: &str,
    section_title: &str,
    counter: &mut u32,
) -> Option<Requirement> {
    let trimmed = line.trim();

    // Check for explicit ID
    let id = if let Some(start) = trimmed.find("[REQ-") {
        let end = trimmed[start..].find(']')?;
        trimmed[start + 1..start + end].to_string()
    } else if trimmed.starts_with("REQ-") {
        let end = trimmed.find(':').unwrap_or(trimmed.len());
        trimmed[..end].to_string()
    } else if trimmed.to_lowercase().contains("requirement") {
        // Auto-generate ID
        *counter += 1;
        format!("REQ-{:03}", counter)
    } else {
        return None;
    };

    // Extract text
    let text = trimmed
        .splitn(2, ':')
        .nth(1)
        .unwrap_or(trimmed)
        .trim()
        .to_string();

    // Parse priority
    let priority = if trimmed.to_lowercase().contains("critical") {
        Priority::Critical
    } else if trimmed.to_lowercase().contains("high") {
        Priority::High
    } else if trimmed.to_lowercase().contains("low") {
        Priority::Low
    } else {
        Priority::Medium
    };

    Some(Requirement {
        id,
        text,
        priority,
        status: ReqStatus::Draft,
        section_title: section_title.to_string(),
    })
}

/// Extract cross-references
fn extract_references(sections: &[Section]) -> Vec<CrossRef> {
    let mut references = Vec::new();

    for section in sections {
        for line in section.text.lines() {
            // Look for reference patterns:
            // - See [Section Name](#anchor)
            // - Depends on: REQ-XXX
            // - Implements: REQ-XXX

            if let Some(refs) = parse_reference_line(line, &section.title) {
                references.extend(refs);
            }
        }
    }

    references
}

/// Parse reference patterns from a line
fn parse_reference_line(line: &str, from_section: &str) -> Option<Vec<CrossRef>> {
    let mut refs = Vec::new();
    let lower = line.to_lowercase();

    // Depends on
    if lower.contains("depends on") || lower.contains("depends upon") {
        for word in line.split_whitespace() {
            if word.starts_with("REQ-") || word.starts_with("[#") {
                let to = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-').to_string();
                refs.push(CrossRef {
                    from: from_section.to_string(),
                    to,
                    kind: RefKind::DependsOn,
                });
            }
        }
    }

    // Implements
    if lower.contains("implements") {
        for word in line.split_whitespace() {
            if word.starts_with("REQ-") {
                let to = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-').to_string();
                refs.push(CrossRef {
                    from: from_section.to_string(),
                    to,
                    kind: RefKind::Implements,
                });
            }
        }
    }

    // See also / Links to (markdown links)
    if line.contains("](") {
        // Extract link targets
        for (text, target) in extract_markdown_links(line) {
            refs.push(CrossRef {
                from: from_section.to_string(),
                to: target,
                kind: RefKind::LinksTo,
            });
        }
    }

    if refs.is_empty() {
        None
    } else {
        Some(refs)
    }
}

/// Extract markdown links: [text](target)
fn extract_markdown_links(text: &str) -> Vec<(String, String)> {
    let mut links = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '[' {
            // Find closing ]
            if let Some(text_end) = chars[i..].iter().position(|&c| c == ']') {
                let link_text: String = chars[i + 1..i + text_end].iter().collect();

                // Check for (
                if i + text_end + 1 < chars.len() && chars[i + text_end + 1] == '(' {
                    if let Some(target_end) = chars[i + text_end + 1..].iter().position(|&c| c == ')')
                    {
                        let target: String = chars[i + text_end + 2..i + text_end + 1 + target_end]
                            .iter()
                            .collect();
                        links.push((link_text, target));
                        i += text_end + target_end + 2;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_parse_sections() {
        let markdown = r#"# Heading 1

Content 1

## Heading 2

Content 2
"#;

        let content = SpecContent::parse(markdown).unwrap();
        assert_eq!(content.sections().len(), 2);
        assert_eq!(content.sections()[0].title, "Heading 1");
        assert_eq!(content.sections()[1].title, "Heading 2");
    }

    #[test]
    fn spec_extract_heading_id() {
        let (title, id) = extract_heading_id("Title {#my-id}");
        assert_eq!(title, "Title");
        assert_eq!(id, Some("my-id".to_string()));

        let (title, id) = extract_heading_id("No ID");
        assert_eq!(title, "No ID");
        assert_eq!(id, None);
    }

    #[test]
    fn spec_parse_heading() {
        assert_eq!(parse_heading("# H1"), Some((1, "H1".to_string())));
        assert_eq!(parse_heading("## H2"), Some((2, "H2".to_string())));
        assert_eq!(parse_heading("No heading"), None);
        assert_eq!(parse_heading("#No space"), Some((1, "No space".to_string())));
    }

    #[test]
    fn spec_extract_requirements() {
        let markdown = r#"# Section

REQ-001: System must be fast.
**Requirement:** [REQ-002] Must be reliable.
This is a requirement with high priority.
"#;

        let content = SpecContent::parse(markdown).unwrap();
        assert!(!content.requirements().is_empty());

        let req1 = content.find_requirement("REQ-001");
        assert!(req1.is_some());
        assert!(req1.unwrap().text.contains("fast"));
    }

    #[test]
    fn spec_extract_references() {
        let markdown = r#"# Design

Depends on REQ-001.
See [Overview](#overview).
"#;

        let content = SpecContent::parse(markdown).unwrap();
        assert!(!content.references().is_empty());
    }

    #[test]
    fn spec_merkle_root() {
        let content1 = SpecContent::parse("# Title\n\nContent").unwrap();
        let content2 = SpecContent::parse("# Title\n\nContent").unwrap();
        let content3 = SpecContent::parse("# Different\n\nContent").unwrap();

        // Same content -> same hash
        assert_eq!(content1.merkle_root(), content2.merkle_root());

        // Different content -> different hash
        assert_ne!(content1.merkle_root(), content3.merkle_root());
    }
}
