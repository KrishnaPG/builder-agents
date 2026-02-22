//! Code Artifact Type
//!
//! Structured code artifact with AST, symbol table, and source text.
//! Uses tree-sitter for incremental parsing.

use std::collections::HashMap;

use crate::artifact_type::ArtifactContent;
use crate::hash::ContentHash;
use crate::merkle::ArtifactMerkleTree;

/// Programming language support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    JavaScript,
    C,
    Cpp,
    Java,
}

impl Language {
    /// Get file extensions for this language
    #[inline]
    #[must_use]
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Python => &["py"],
            Language::Go => &["go"],
            Language::JavaScript => &["js", "jsx"],
            Language::C => &["c", "h"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp"],
            Language::Java => &["java"],
        }
    }

    /// Detect language from file extension
    #[inline]
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.trim_start_matches('.');
        match ext {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            "js" | "jsx" => Some(Language::JavaScript),
            "c" | "h" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "hpp" => Some(Language::Cpp),
            "java" => Some(Language::Java),
            _ => None,
        }
    }

    /// Get tree-sitter language (if available)
    #[inline]
    #[must_use]
    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        match self {
            Language::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Language::TypeScript => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            Language::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Language::Go => Some(tree_sitter_go::LANGUAGE.into()),
            _ => None,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Language::Rust => "Rust",
            Language::TypeScript => "TypeScript",
            Language::Python => "Python",
            Language::Go => "Go",
            Language::JavaScript => "JavaScript",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Java => "Java",
        };
        write!(f, "{}", name)
    }
}

/// Code artifact type marker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeArtifact;

impl crate::artifact_type::ArtifactType for CodeArtifact {
    type Content = CodeContent;

    #[inline]
    fn hash(content: &Self::Content) -> ContentHash {
        content.merkle_root()
    }

    const TYPE_ID: &'static str = "code";
}

/// Code content with AST and symbol table
#[derive(Debug, Clone, PartialEq)]
pub struct CodeContent {
    /// Programming language
    language: Language,

    /// Source text
    source: String,

    /// Symbol table
    symbols: SymbolTable,

    /// Merkle tree of AST nodes (for hashing)
    merkle_tree: ArtifactMerkleTree,

    /// Source hash (for change detection)
    source_hash: ContentHash,
}

impl CodeContent {
    /// Parse source code into content
    ///
    /// # Errors
    /// Returns error if parsing fails or language is not supported
    pub fn parse(source: &str, language: Language) -> Result<Self, ParseError> {
        let source_hash = ContentHash::compute(source.as_bytes());

        // Parse with tree-sitter
        let ts_lang = language
            .tree_sitter_language()
            .ok_or(ParseError::UnsupportedLanguage(language))?;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&ts_lang)
            .map_err(|e| ParseError::ParserInit(e.to_string()))?;

        let tree = parser
            .parse(source, None)
            .ok_or(ParseError::ParseFailed)?;

        // Build symbol table
        let symbols = build_symbol_table(&tree, source, language);

        // Build Merkle tree from AST
        let merkle_tree = build_ast_merkle_tree(&tree, source);

        Ok(Self {
            language,
            source: source.to_string(),
            symbols,
            merkle_tree,
            source_hash,
        })
    }

    /// Get programming language
    #[inline]
    #[must_use]
    pub fn language(&self) -> Language {
        self.language
    }

    /// Get source text
    #[inline]
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get symbol table
    #[inline]
    #[must_use]
    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    /// Compute Merkle root hash
    #[inline]
    #[must_use]
    pub fn merkle_root(&self) -> ContentHash {
        self.merkle_tree.root_or_default()
    }

    /// Get source hash
    #[inline]
    #[must_use]
    pub fn source_hash(&self) -> ContentHash {
        self.source_hash
    }

    /// Check if source has changed
    #[inline]
    #[must_use]
    pub fn has_source_changed(&self, new_source: &str) -> bool {
        ContentHash::compute(new_source.as_bytes()) != self.source_hash
    }

    /// Find symbol by name
    #[inline]
    #[must_use]
    pub fn find_symbol(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols.find_by_name(name)
    }

    /// Get all symbols of a kind
    #[inline]
    #[must_use]
    pub fn symbols_of_kind(&self, kind: SymbolKind) -> Vec<&SymbolInfo> {
        self.symbols.by_kind(kind)
    }
}

impl ArtifactContent for CodeContent {
    #[inline]
    fn approximate_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.source.len()
            + self.symbols.approximate_size()
    }
}

/// Symbol table for code navigation
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SymbolTable {
    /// Symbol name -> info mapping
    symbols: HashMap<String, SymbolInfo>,
}

impl SymbolTable {
    /// Create empty symbol table
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    /// Add symbol
    #[inline]
    pub fn add(&mut self, info: SymbolInfo) {
        self.symbols.insert(info.name.clone(), info);
    }

    /// Find symbol by name
    #[inline]
    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols.get(name)
    }

    /// Get all symbols of a kind
    #[inline]
    #[must_use]
    pub fn by_kind(&self, kind: SymbolKind) -> Vec<&SymbolInfo> {
        self.symbols
            .values()
            .filter(|s| s.kind == kind)
            .collect()
    }

    /// Get all symbols
    #[inline]
    #[must_use]
    pub fn all(&self) -> Vec<&SymbolInfo> {
        self.symbols.values().collect()
    }

    /// Get symbol count
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Approximate memory size
    #[inline]
    fn approximate_size(&self) -> usize {
        self.symbols
            .values()
            .map(|s| s.name.len() + std::mem::size_of::<SymbolInfo>())
            .sum()
    }
}

/// Symbol information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    /// Symbol name
    pub name: String,

    /// Symbol kind
    pub kind: SymbolKind,

    /// Byte range in source
    pub span: std::ops::Range<usize>,

    /// Parent symbol (if nested)
    pub parent: Option<String>,

    /// Visibility
    pub visibility: Visibility,
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Type,
    Variable,
    Constant,
    Module,
    Import,
    Field,
    Variant,
}

/// Symbol visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Parse error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(Language),

    #[error("parser initialization failed: {0}")]
    ParserInit(String),

    #[error("parse failed")]
    ParseFailed,

    #[error("syntax error at {line}:{column}: {message}")]
    SyntaxError {
        line: usize,
        column: usize,
        message: String,
    },
}

/// Build symbol table from AST
fn build_symbol_table(
    tree: &tree_sitter::Tree,
    source: &str,
    language: Language,
) -> SymbolTable {
    let mut table = SymbolTable::new();
    let root = tree.root_node();

    match language {
        Language::Rust => build_rust_symbols(&root, source, &mut table, None),
        Language::Python => build_python_symbols(&root, source, &mut table, None),
        _ => {
            // Generic symbol extraction for other languages
            build_generic_symbols(&root, source, &mut table, None);
        }
    }

    table
}

/// Build Merkle tree from AST nodes
fn build_ast_merkle_tree(tree: &tree_sitter::Tree, source: &str) -> ArtifactMerkleTree {
    let mut leaves = Vec::new();
    collect_node_hashes(tree.root_node(), source, &mut leaves);
    ArtifactMerkleTree::from_leaves(&leaves)
}

/// Collect hashes from AST nodes
fn collect_node_hashes(node: tree_sitter::Node, source: &str, leaves: &mut Vec<ContentHash>) {
    // Hash node type and text
    let node_text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let hash_input = format!("{}:{}", node.kind(), node_text);
    leaves.push(ContentHash::compute(hash_input.as_bytes()));

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_node_hashes(child, source, leaves);
        }
    }
}

/// Build Rust-specific symbols
fn build_rust_symbols(
    node: &tree_sitter::Node,
    source: &str,
    table: &mut SymbolTable,
    parent: Option<String>,
) {
    use SymbolKind::*;

    let kind = match node.kind() {
        "function_item" => Some(Function),
        "struct_item" => Some(Struct),
        "enum_item" => Some(Enum),
        "trait_item" => Some(Trait),
        "impl_item" => None, // Handle separately
        "mod_item" => Some(Module),
        "const_item" => Some(Constant),
        "static_item" => Some(Constant),
        "use_declaration" => Some(Import),
        _ => None,
    };

    if let Some(symbol_kind) = kind {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = name_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .to_string();

            if !name.is_empty() {
                let info = SymbolInfo {
                    name: name.clone(),
                    kind: symbol_kind,
                    span: node.byte_range(),
                    parent: parent.clone(),
                    visibility: extract_visibility(node, source),
                };
                table.add(info);

                // Recurse with new parent
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        build_rust_symbols(&child, source, table, Some(name.clone()));
                    }
                }
                return;
            }
        }
    }

    // Continue recursion without changing parent
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            build_rust_symbols(&child, source, table, parent.clone());
        }
    }
}

/// Build Python-specific symbols
fn build_python_symbols(
    node: &tree_sitter::Node,
    source: &str,
    table: &mut SymbolTable,
    parent: Option<String>,
) {
    use SymbolKind::*;

    let kind = match node.kind() {
        "function_definition" => Some(Function),
        "class_definition" => Some(Type),
        "import_statement" | "import_from_statement" => Some(Import),
        _ => None,
    };

    if let Some(symbol_kind) = kind {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = name_node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .to_string();

            if !name.is_empty() {
                let info = SymbolInfo {
                    name: name.clone(),
                    kind: symbol_kind,
                    span: node.byte_range(),
                    parent: parent.clone(),
                    visibility: Visibility::Public, // Python is dynamic
                };
                table.add(info);

                // Recurse with new parent for classes
                if node.kind() == "class_definition" {
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            build_python_symbols(&child, source, table, Some(name.clone()));
                        }
                    }
                    return;
                }
            }
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            build_python_symbols(&child, source, table, parent.clone());
        }
    }
}

/// Build generic symbols for other languages
fn build_generic_symbols(
    node: &tree_sitter::Node,
    source: &str,
    table: &mut SymbolTable,
    parent: Option<String>,
) {
    // Simple heuristic: look for identifier nodes
    if node.kind().contains("identifier") || node.kind().contains("name") {
        let name = node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
        if !name.is_empty() && name.len() < 100 {
            let info = SymbolInfo {
                name,
                kind: SymbolKind::Variable,
                span: node.byte_range(),
                parent,
                visibility: Visibility::Public,
            };
            table.add(info);
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            build_generic_symbols(&child, source, table, None);
        }
    }
}

/// Extract visibility from node
fn extract_visibility(node: &tree_sitter::Node, source: &str) -> Visibility {
    // Check first child for visibility modifiers
    if let Some(first) = node.child(0) {
        let text = first.utf8_text(source.as_bytes()).unwrap_or("");
        match text {
            "pub" => Visibility::Public,
            "priv" => Visibility::Private,
            _ => Visibility::Private, // Default for Rust
        }
    } else {
        Visibility::Private
    }
}

// Import serde traits for Language
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("unknown"), None);
    }

    #[test]
    fn language_extensions() {
        assert!(Language::Rust.extensions().contains(&"rs"));
        assert!(Language::TypeScript.extensions().contains(&"ts"));
    }

    #[test]
    fn symbol_table_add_and_find() {
        let mut table = SymbolTable::new();

        let info = SymbolInfo {
            name: "test_fn".to_string(),
            kind: SymbolKind::Function,
            span: 0..10,
            parent: None,
            visibility: Visibility::Public,
        };

        table.add(info);
        assert!(table.find_by_name("test_fn").is_some());
        assert!(table.find_by_name("missing").is_none());
    }

    #[test]
    fn code_content_size() {
        let content = CodeContent {
            language: Language::Rust,
            source: "fn main() {}".to_string(),
            symbols: SymbolTable::new(),
            merkle_tree: ArtifactMerkleTree::new(),
            source_hash: ContentHash::default(),
        };

        assert!(content.approximate_size() > 0);
    }
}
