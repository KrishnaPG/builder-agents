# Constitutional Layer Design

**Module**: `coa-constitutional`  
**Lines Estimate**: ~1,500  
**Files**: 8-10

---

## Overview

The Constitutional Layer handles:
- **Ingress**: Parse external files into `Artifact<T>`
- **Application**: Apply `StructuralDelta<T>` to produce new artifacts
- **Egress**: Serialize artifacts back to external format

This is the trusted boundary between the external world (files) and the COA's internal artifact system.

---

## Core Types

### ConstitutionalLayer

```rust
// File: src/layer.rs (250 lines)

use std::collections::HashMap;
use std::path::Path;

/// Trusted transformation layer
/// 
/// All file I/O goes through this layer. Agents never touch files directly.
pub struct ConstitutionalLayer {
    /// Registered parsers by file extension
    parsers: HashMap<String, Box<dyn ArtifactParser>>,
    
    /// Registered serializers by artifact type
    serializers: HashMap<String, Box<dyn ArtifactSerializer>>,
    
    /// Transformation registry
    transformers: HashMap<String, Box<dyn ArtifactTransformer>>,
    
    /// Cache for parsed artifacts
    cache: ArtifactCache,
}

/// Parse result
#[derive(Debug)]
pub struct ParseResult<T: ArtifactType> {
    pub artifact: Artifact<T>,
    pub metadata: SourceMetadata,
}

/// Source file metadata
#[derive(Debug, Clone)]
pub struct SourceMetadata {
    pub path: std::path::PathBuf,
    pub modified: std::time::SystemTime,
    pub checksum: ContentHash,
}

impl ConstitutionalLayer {
    /// Create new layer with default parsers
    pub fn new() -> Self {
        let mut layer = Self {
            parsers: HashMap::new(),
            serializers: HashMap::new(),
            transformers: HashMap::new(),
            cache: ArtifactCache::new(),
        };
        
        // Register default parsers
        layer.register_default_parsers();
        layer.register_default_serializers();
        
        layer
    }
    
    /// Parse file into typed artifact (Ingress)
    /// 
    /// # Type Safety
    /// Returns `Artifact<T>` - type is known at compile time
    /// 
    /// # Caching
    /// Cached by content hash
    pub async fn parse_ingress<T: ArtifactType>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ParseResult<T>, ParseError> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path).await?;
        
        // Compute content hash
        let checksum = ContentHash::from_bytes(blake3::hash(content.as_bytes()).as_bytes());
        
        // Check cache
        if let Some(cached) = self.cache.get::<T>(&checksum) {
            return Ok(ParseResult {
                artifact: cached,
                metadata: SourceMetadata {
                    path: path.to_path_buf(),
                    modified: std::fs::metadata(path)?.modified()?,
                    checksum,
                },
            });
        }
        
        // Parse
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        let parser = self.parsers.get(extension)
            .ok_or(ParseError::NoParserForExtension(extension.to_string()))?;
        
        let artifact = parser.parse::<T>(&content)?;
        
        // Cache
        self.cache.insert(checksum, artifact.clone());
        
        Ok(ParseResult {
            artifact,
            metadata: SourceMetadata {
                path: path.to_path_buf(),
                modified: std::fs::metadata(path)?.modified()?,
                checksum,
            },
        })
    }
    
    /// Apply delta to artifact (Transformation)
    /// 
    /// # Validation
    /// - Verifies delta.base_hash matches artifact
    /// - Validates operation doesn't break invariants
    pub fn apply_delta<T: ArtifactType>(
        &self,
        artifact: &Artifact<T>,
        delta: &StructuralDelta<T>,
    ) -> Result<Artifact<T>, ApplyError> {
        // Verify base hash
        delta.validate_base(artifact)
            .map_err(|e| ApplyError::InvalidBase(e.to_string()))?;
        
        // Get transformer for this artifact type
        let transformer = self.transformers.get(T::TYPE_ID)
            .ok_or(ApplyError::NoTransformer(T::TYPE_ID.to_string()))?;
        
        // Validate operation
        transformer.validate(artifact, delta)?;
        
        // Apply
        let new_content = transformer.transform(&artifact.content(), &delta.operation)?;
        
        Ok(Artifact::new(new_content))
    }
    
    /// Apply multiple deltas with composition strategy
    pub fn apply_deltas<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
        strategy: &dyn CompositionStrategy,
    ) -> Result<Artifact<T>, ApplyError> {
        // Validate composition
        // (SymbolRefIndex would be passed here in real impl)
        let index = SymbolRefIndex::new();
        strategy.validate(deltas, &index)
            .map_err(|e| ApplyError::CompositionFailed(e.to_string()))?;
        
        // Compose
        strategy.compose(base, deltas)
            .map_err(|e| ApplyError::CompositionFailed(e.to_string()))
    }
    
    /// Serialize artifact to file (Egress)
    pub async fn serialize_egress<T: ArtifactType>(
        &self,
        artifact: &Artifact<T>,
        path: impl AsRef<Path>,
    ) -> Result<(), SerializeError> {
        let path = path.as_ref();
        
        let serializer = self.serializers.get(T::TYPE_ID)
            .ok_or(SerializeError::NoSerializer(T::TYPE_ID.to_string()))?;
        
        let content = serializer.serialize(artifact.content())?;
        
        tokio::fs::write(path, content).await?;
        
        Ok(())
    }
    
    fn register_default_parsers(&mut self) {
        self.parsers.insert("rs".to_string(), Box::new(RustParser));
        self.parsers.insert("ts".to_string(), Box::new(TypeScriptParser));
        self.parsers.insert("json".to_string(), Box::new(JsonParser));
        self.parsers.insert("yaml".to_string(), Box::new(YamlParser));
        self.parsers.insert("md".to_string(), Box::new(MarkdownParser));
    }
    
    fn register_default_serializers(&mut self) {
        self.serializers.insert("code".to_string(), Box::new(CodeSerializer));
        self.serializers.insert("config".to_string(), Box::new(ConfigSerializer));
        self.serializers.insert("spec".to_string(), Box::new(SpecSerializer));
    }
}

/// Parser trait
pub trait ArtifactParser: Send + Sync {
    fn parse<T: ArtifactType>(&self, content: &str) -> Result<Artifact<T>, ParseError>;
}

/// Serializer trait
pub trait ArtifactSerializer: Send + Sync {
    fn serialize<T: ArtifactTypeContent>(&self, content: &T) -> Result<String, SerializeError>;
}

/// Transformer trait (validates and applies operations)
pub trait ArtifactTransformer: Send + Sync {
    fn validate<T: ArtifactType>(
        &self,
        artifact: &Artifact<T>,
        delta: &StructuralDelta<T>,
    ) -> Result<(), ValidationError>;
    
    fn transform<T: ArtifactTypeContent>(
        &self,
        content: &T,
        operation: &DeltaOperation<impl ArtifactType>,
    ) -> Result<T, TransformError>;
}
```

---

## Code Parser (Tree-sitter)

```rust
// File: src/parsers/code.rs (300 lines)

use tree_sitter::{Parser, Tree, Node};

/// Tree-sitter based code parser
pub struct CodeParser {
    language: Language,
}

impl CodeParser {
    pub fn new(language: Language) -> Self {
        Self { language }
    }
    
    fn parse_to_ast(&self, source: &str) -> Result<Tree, ParseError> {
        let mut parser = Parser::new();
        parser.set_language(self.language.into_tree_sitter())?;
        
        parser.parse(source, None)
            .ok_or(ParseError::SyntaxError)
    }
    
    fn build_symbol_table(&self, tree: &Tree, source: &str) -> SymbolTable {
        let mut table = SymbolTable::new();
        let root = tree.root_node();
        
        self.visit_node(&root, source, vec![], &mut table);
        
        table
    }
    
    fn visit_node(
        &self,
        node: &Node,
        source: &str,
        path: Vec<String>,
        table: &mut SymbolTable,
    ) {
        // Extract symbols based on node type
        match node.kind() {
            "function_item" | "function_declaration" => {
                if let Some(name) = self.extract_name(node, source) {
                    let mut symbol_path = path.clone();
                    symbol_path.push(name);
                    
                    table.insert(SymbolPath::new(symbol_path.clone()), Symbol {
                        name,
                        kind: SymbolKind::Function,
                        span: node.byte_range(),
                        signature: self.extract_signature(node, source),
                    });
                    
                    // Continue into function body
                    if let Some(body) = node.child_by_field_name("body") {
                        self.visit_node(&body, source, symbol_path, table);
                    }
                }
            }
            "struct_item" | "class_declaration" => {
                if let Some(name) = self.extract_name(node, source) {
                    let mut symbol_path = path.clone();
                    symbol_path.push(name);
                    
                    table.insert(SymbolPath::new(symbol_path.clone()), Symbol {
                        name,
                        kind: SymbolKind::Type,
                        span: node.byte_range(),
                        signature: None,
                    });
                    
                    // Visit members
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            self.visit_node(&child, source, symbol_path.clone(), table);
                        }
                    }
                }
            }
            _ => {
                // Continue traversal
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.visit_node(&child, source, path.clone(), table);
                    }
                }
            }
        }
    }
    
    fn extract_name(&self, node: &Node, source: &str) -> Option<String> {
        node.child_by_field_name("name")
            .map(|n| n.utf8_text(source.as_bytes()).unwrap().to_string())
    }
    
    fn extract_signature(&self, node: &Node, source: &str) -> Option<String> {
        // Extract function signature
        node.child_by_field_name("parameters")
            .map(|p| p.utf8_text(source.as_bytes()).unwrap().to_string())
    }
}

impl ArtifactParser for CodeParser {
    fn parse<T: ArtifactType>(&self, content: &str) -> Result<Artifact<T>, ParseError> {
        // Only handles CodeArtifact
        if T::TYPE_ID != "code" {
            return Err(ParseError::WrongType);
        }
        
        let tree = self.parse_to_ast(content)?;
        let symbol_table = self.build_symbol_table(&tree, content);
        
        let code_content = CodeContent {
            language: self.language,
            ast: tree,
            source: content.to_string(),
            symbol_table,
            // ...
        };
        
        Ok(Artifact::new(code_content))
    }
}
```

---

## Delta Application

```rust
// File: src/transformers/code_transformer.rs (400 lines)

/// Code transformation engine
pub struct CodeTransformer;

impl ArtifactTransformer for CodeTransformer {
    fn validate<T: ArtifactType>(
        &self,
        artifact: &Artifact<T>,
        delta: &StructuralDelta<T>,
    ) -> Result<(), ValidationError> {
        let code = artifact.content();
        let target = &delta.target;
        
        // Verify target exists (for replace/remove)
        match &delta.operation {
            DeltaOperation::Replace(_) | DeltaOperation::Remove => {
                if code.get_symbol(target).is_none() {
                    return Err(ValidationError::TargetNotFound(target.to_string()));
                }
            }
            DeltaOperation::Add(_) => {
                if code.get_symbol(target).is_some() {
                    return Err(ValidationError::TargetAlreadyExists(target.to_string()));
                }
            }
            DeltaOperation::Transform(_) => {
                if code.get_symbol(target).is_none() {
                    return Err(ValidationError::TargetNotFound(target.to_string()));
                }
            }
        }
        
        Ok(())
    }
    
    fn transform<T: ArtifactTypeContent>(
        &self,
        content: &T,
        operation: &DeltaOperation<impl ArtifactType>,
    ) -> Result<T, TransformError> {
        // This would use tree-sitter's edit API
        match operation {
            DeltaOperation::Add(new_content) => {
                self.apply_add(content, new_content)
            }
            DeltaOperation::Remove => {
                self.apply_remove(content)
            }
            DeltaOperation::Replace(new_content) => {
                self.apply_replace(content, new_content)
            }
            DeltaOperation::Transform(t) => {
                t.apply(content)
                    .map_err(|e| TransformError::TransformationFailed(e.to_string()))
            }
        }
    }
}

impl CodeTransformer {
    fn apply_add<T>(
        &self,
        content: &T,
        new_content: &T,
    ) -> Result<T, TransformError> {
        // Insert new content at appropriate location
        // Use tree-sitter's edit API
        todo!()
    }
    
    fn apply_remove<T>(&self, content: &T) -> Result<T, TransformError> {
        // Remove content
        todo!()
    }
    
    fn apply_replace<T>(
        &self,
        content: &T,
        new_content: &T,
    ) -> Result<T, TransformError> {
        // Replace content
        todo!()
    }
}
```

---

## Caching - Using `moka`

```rust
// File: src/cache.rs (80 lines) - GLUE CODE ONLY

use moka::future::Cache;
use std::sync::Arc;

/// Content-addressed artifact cache using moka
/// 
/// Moka provides:
/// - LRU eviction
/// - Time-based expiration
/// - Async support
/// - High concurrency
pub struct ArtifactCache {
    inner: Cache<ContentHash, Arc<dyn Any + Send + Sync>>,
}

impl ArtifactCache {
    /// Create cache with capacity
    pub fn new(max_capacity: u64) -> Self {
        Self {
            inner: Cache::new(max_capacity),
        }
    }
    
    /// Create with time-based expiration
    pub fn with_ttl(max_capacity: u64, ttl: Duration) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
        }
    }
    
    /// Insert artifact
    pub async fn insert<T: ArtifactType>(&self, hash: ContentHash, artifact: Artifact<T>) {
        self.inner.insert(hash, Arc::new(artifact)).await;
    }
    
    /// Get artifact
    pub async fn get<T: ArtifactType>(&self, hash: &ContentHash) -> Option<Artifact<T>> {
        self.inner.get(hash).await
            .and_then(|arc| {
                arc.downcast_ref::<Artifact<T>>()
                    .cloned()
            })
    }
    
    /// Get or compute
    pub async fn get_or_insert_with<T: ArtifactType, F, Fut>(
        &self,
        hash: ContentHash,
        f: F,
    ) -> Artifact<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Artifact<T>>,
    {
        let arc = self.inner
            .get_or_insert_with(hash, async move || {
                Arc::new(f().await)
            })
            .await;
        
        arc.downcast_ref::<Artifact<T>>()
            .cloned()
            .expect("type matches")
    }
    
    /// Invalidate entry
    pub async fn invalidate(&self, hash: &ContentHash) {
        self.inner.invalidate(hash).await;
    }
    
    /// Cache stats
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hit_rate: self.inner.hit_rate(),
            entries: self.inner.entry_count(),
        }
    }
}

---

## Tests

```rust
// File: tests/constitutional_tests.rs (400 lines)

#[cfg(test)]
mod tests {
    use super::*;
    use coa_constitutional::*;

    #[tokio::test]
    async fn parse_ingress_reads_file() {
        let layer = ConstitutionalLayer::new();
        
        // Create temp file
        let temp = tempfile::NamedTempFile::new().unwrap();
        tokio::fs::write(temp.path(), "fn main() {}")
            .await
            .unwrap();
        
        let result = layer.parse_ingress::<CodeArtifact>(temp.path()).await;
        assert!(result.is_ok());
        
        let parsed = result.unwrap();
        assert_eq!(parsed.artifact.content().language, Language::Rust);
    }

    #[test]
    fn apply_delta_changes_content() {
        let layer = ConstitutionalLayer::new();
        let base = create_test_code_artifact();
        
        let delta = create_add_function_delta(&base);
        
        let result = layer.apply_delta(&base, &delta);
        assert!(result.is_ok());
        
        let new_artifact = result.unwrap();
        assert_ne!(base.hash(), new_artifact.hash());
    }

    #[test]
    fn apply_delta_rejects_wrong_base() {
        let layer = ConstitutionalLayer::new();
        
        let base = create_test_code_artifact();
        let mut other = base.clone();
        // Modify other
        other.content.source = "different".to_string();
        
        // Delta expects base hash, but we apply to other
        let delta = StructuralDelta::new(
            SymbolPath::from_str("main"),
            DeltaOperation::Remove,
            *base.hash(),
        );
        
        let result = layer.apply_delta(&other, &delta);
        assert!(matches!(result, Err(ApplyError::InvalidBase(_))));
    }

    #[test]
    fn cache_returns_same_artifact() {
        let layer = ConstitutionalLayer::new();
        let content = "fn main() {}";
        
        // Compute hash
        let hash = ContentHash::from_bytes(blake3::hash(content.as_bytes()).as_bytes());
        
        let artifact = create_test_code_artifact_with_source(content);
        layer.cache.insert(hash, artifact.clone());
        
        let cached = layer.cache.get::<CodeArtifact>(&hash);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().hash(), artifact.hash());
    }

    #[tokio::test]
    async fn serialize_egress_writes_file() {
        let layer = ConstitutionalLayer::new();
        let artifact = create_test_code_artifact();
        
        let temp = tempfile::NamedTempFile::new().unwrap();
        
        let result = layer.serialize_egress(&artifact, temp.path()).await;
        assert!(result.is_ok());
        
        let written = tokio::fs::read_to_string(temp.path()).await.unwrap();
        assert!(!written.is_empty());
    }
}
```

---

## File Structure

```
crates/coa-constitutional/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── layer.rs            # ConstitutionalLayer
    ├── cache.rs            # ArtifactCache
    ├── error.rs            # Error types
    ├── parsers/
    │   ├── mod.rs          # Parser exports
    │   ├── code.rs         # Tree-sitter parsers
    │   ├── json.rs         # JSON parser
    │   ├── yaml.rs         # YAML parser
    │   └── markdown.rs     # Markdown parser
    └── transformers/
        ├── mod.rs          # Transformer exports
        ├── code.rs         # Code transformation
        ├── config.rs       # Config transformation
        └── spec.rs         # Spec transformation
```

---

## Cargo.toml

```toml
[package]
name = "coa-constitutional"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core
coa-artifact = { path = "../coa-artifact" }
coa-symbol = { path = "../coa-symbol" }
coa-composition = { path = "../coa-composition" }

# Async
tokio = { version = "1", features = ["fs", "io-util"] }

# Parsing
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
serde_json = "1"
serde_yaml = "0.9"
pulldown-cmark = "0.9"  # Markdown

# Caching
dashmap = "5"

# Hashing
blake3 = "1"

# Error handling
thiserror = "1"

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3"
```
