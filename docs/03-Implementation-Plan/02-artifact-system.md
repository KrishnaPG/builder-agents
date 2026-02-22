# Artifact System Design

**Module**: `coa-artifact`  
**Lines Estimate**: ~1,500  
**Files**: 8-10

---

## Core Types

### Artifact<T>

```rust
// File: src/artifact.rs (120 lines)
use std::marker::PhantomData;
use sha2::{Sha256, Digest};

/// Content-addressed typed artifact
/// 
/// # Type Parameters
/// - `T`: The artifact type (Code, Config, Spec, etc.)
/// 
/// # Invariants
/// - `hash` is always `T::hash(&content)`
/// - Immutable after construction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact<T: ArtifactType> {
    hash: ContentHash,
    content: T::Content,
    _phantom: PhantomData<T>,
}

impl<T: ArtifactType> Artifact<T> {
    /// Create new artifact (computes hash)
    /// 
    /// # Performance
    /// O(n) where n = content size
    pub fn new(content: T::Content) -> Self {
        let hash = T::hash(&content);
        Self {
            hash,
            content,
            _phantom: PhantomData,
        }
    }
    
    /// Content hash (Merkle root)
    #[inline]
    pub fn hash(&self) -> &ContentHash {
        &self.hash
    }
    
    /// Reference to content
    #[inline]
    pub fn content(&self) -> &T::Content {
        &self.content
    }
    
    /// Verify integrity (useful after deserialization)
    pub fn verify(&self) -> bool {
        self.hash == T::hash(&self.content)
    }
}

/// Content hash (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
```

### ArtifactType Trait

```rust
// File: src/lib.rs (80 lines)

/// Trait for artifact types
/// 
/// Implement this for each type of work product:
/// - `CodeArtifact` (AST + symbol table)
/// - `ConfigArtifact` (schema-validated tree)
/// - `SpecArtifact` (structured document)
/// - `BinaryArtifact` (content hash + metadata)
pub trait ArtifactType: Send + Sync + 'static + private::Sealed {
    /// The content type for this artifact
    type Content: Send + Sync + 'static;
    
    /// Compute content hash
    /// 
    /// # Contract
    /// Must be deterministic and collision-resistant
    fn hash(content: &Self::Content) -> ContentHash;
    
    /// Artifact type identifier
    const TYPE_ID: &'static str;
}

// Prevent external implementations for now
mod private {
    pub trait Sealed {}
}
```

---

## StructuralDelta<T>

```rust
// File: src/delta.rs (200 lines)

/// Semantic transformation on an artifact
/// 
/// NOT text patches - these are structural operations with meaning
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralDelta<T: ArtifactType> {
    /// Target symbol within artifact tree
    target: SymbolPath,
    
    /// The transformation operation
    operation: DeltaOperation<T>,
    
    /// Expected base hash (optimistic concurrency)
    base_hash: ContentHash,
    
    /// Human-readable description
    description: String,
}

/// Delta operations by artifact type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaOperation<T: ArtifactType> {
    /// Add new element
    Add(T::Content),
    
    /// Remove element
    Remove,
    
    /// Replace entire element
    Replace(T::Content),
    
    /// Transform with custom operation
    Transform(Box<dyn Transformation<T>>),
}

/// Transformation trait for custom operations
pub trait Transformation<T: ArtifactType>: Send + Sync {
    /// Apply transformation to content
    fn apply(&self, target: &T::Content) -> Result<T::Content, TransformError>;
    
    /// Inverse operation (for time scrubber)
    fn inverse(&self) -> Option<Box<dyn Transformation<T>>>;
    
    /// Describe the transformation
    fn description(&self) -> String;
}

impl<T: ArtifactType> StructuralDelta<T> {
    /// Create new delta
    pub fn new(
        target: SymbolPath,
        operation: DeltaOperation<T>,
        base_hash: ContentHash,
    ) -> Self {
        let description = format!("{:?} at {:?}", operation, target);
        Self {
            target,
            operation,
            base_hash,
            description,
        }
    }
    
    /// Verify delta can apply to artifact
    pub fn validate_base(&self, artifact: &Artifact<T>) -> Result<(), DeltaError> {
        if self.base_hash != *artifact.hash() {
            return Err(DeltaError::BaseMismatch {
                expected: self.base_hash,
                actual: *artifact.hash(),
            });
        }
        Ok(())
    }
    
    /// Apply delta to artifact, producing new artifact
    pub fn apply(&self, artifact: &Artifact<T>) -> Result<Artifact<T>, DeltaError> {
        self.validate_base(artifact)?;
        
        let new_content = match &self.operation {
            DeltaOperation::Add(content) => T::merge(&artifact.content, &self.target, content)?,
            DeltaOperation::Remove => T::remove(&artifact.content, &self.target)?,
            DeltaOperation::Replace(content) => T::replace(&artifact.content, &self.target, content)?,
            DeltaOperation::Transform(t) => t.apply(&artifact.content)?,
        };
        
        Ok(Artifact::new(new_content))
    }
}

/// Path within artifact tree
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolPath(Vec<String>);

impl SymbolPath {
    pub fn new(segments: Vec<String>) -> Self {
        Self(segments)
    }
    
    pub fn from_str(path: &str) -> Self {
        Self(path.split('.').map(|s| s.to_string()).collect())
    }
    
    pub fn segments(&self) -> &[String] {
        &self.0
    }
    
    pub fn parent(&self) -> Option<SymbolPath> {
        if self.0.len() <= 1 {
            None
        } else {
            Some(Self(self.0[..self.0.len()-1].to_vec()))
        }
    }
}
```

---

## Concrete Artifact Types

### Code Artifact (Tree-sitter based)

```rust
// File: src/types/code.rs (300 lines)

use tree_sitter::{Parser, Tree, Node};

/// Code artifact type
pub struct CodeArtifact;

impl ArtifactType for CodeArtifact {
    type Content = CodeContent;
    
    fn hash(content: &Self::Content) -> ContentHash {
        // Incremental Merkle hash of AST
        content.merkle_root()
    }
    
    const TYPE_ID: &'static str = "code";
}

/// Code artifact content
#[derive(Debug, Clone)]
pub struct CodeContent {
    language: Language,
    ast: Tree,
    source: String,
    symbol_table: SymbolTable,
    merkle_tree: MerkleTree,
}

impl CodeContent {
    /// Parse source code into artifact
    pub fn parse(source: &str, language: Language) -> Result<Self, ParseError> {
        let mut parser = Parser::new();
        parser.set_language(language.into())?;
        
        let tree = parser.parse(source, None)
            .ok_or(ParseError::ParseFailed)?;
        
        let symbol_table = Self::build_symbol_table(&tree, source);
        let merkle_tree = Self::build_merkle_tree(&tree, source);
        
        Ok(Self {
            language,
            ast: tree,
            source: source.to_string(),
            symbol_table,
            merkle_tree,
        })
    }
    
    /// Incremental hash computation
    fn merkle_root(&self) -> ContentHash {
        self.merkle_tree.root()
    }
    
    /// Get symbol at path
    pub fn get_symbol(&self, path: &SymbolPath) -> Option<&Symbol> {
        self.symbol_table.get(path)
    }
    
    /// Apply structural edit
    pub fn apply_edit(&mut self, edit: &TreeEdit) -> Result<(), EditError> {
        // Tree-sitter incremental parsing
        let new_tree = self.ast.edit(&edit.into());
        self.ast = new_tree;
        self.source = edit.apply_to_source(&self.source)?;
        self.rebuild_indices();
        Ok(())
    }
}

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    // ... more
}
```

### Config Artifact

```rust
// File: src/types/config.rs (150 lines)

use serde::{Serialize, de::DeserializeOwned};
use schemars::schema::RootSchema;

/// Config artifact type
pub struct ConfigArtifact<T: ConfigSchema> {
    _phantom: PhantomData<T>,
}

impl<T: ConfigSchema> ArtifactType for ConfigArtifact<T> {
    type Content = T;
    
    fn hash(content: &Self::Content) -> ContentHash {
        let json = serde_json::to_string(content).expect("serializable");
        ContentHash::from_bytes(blake3::hash(json.as_bytes()).as_bytes())
    }
    
    const TYPE_ID: &'static str = T::TYPE_ID;
}

/// Schema-trait for config types
pub trait ConfigSchema: Serialize + DeserializeOwned + Send + Sync + 'static {
    const TYPE_ID: &'static str;
    fn schema() -> RootSchema;
}
```

### Spec Artifact

```rust
// File: src/types/spec.rs (100 lines)

/// Spec document artifact (Markdown-based structured)
pub struct SpecArtifact;

impl ArtifactType for SpecArtifact {
    type Content = SpecContent;
    
    fn hash(content: &Self::Content) -> ContentHash {
        content.merkle_root()
    }
    
    const TYPE_ID: &'static str = "spec";
}

/// Structured specification content
#[derive(Debug, Clone)]
pub struct SpecContent {
    /// Document sections (heading + content)
    sections: Vec<Section>,
    
    /// Extracted requirements
    requirements: Vec<Requirement>,
    
    /// Cross-references
    references: Vec<CrossRef>,
}
```

---

## Merkle Tree - Using `rs_merkle`

```rust
// File: src/hash.rs (50 lines) - GLUE CODE ONLY

use rs_merkle::{MerkleTree as RsMerkleTree, Hasher};

/// Wrapper around rs_merkle with our ContentHash type
#[derive(Debug, Clone)]
pub struct ArtifactMerkleTree {
    inner: RsMerkleTree<Blake3Hasher>,
}

/// Hasher adapter for rs_merkle
pub struct Blake3Hasher;

impl Hasher for Blake3Hasher {
    type Hash = [u8; 32];
    
    fn hash(data: &[u8]) -> Self::Hash {
        *blake3::hash(data).as_bytes()
    }
}

impl ArtifactMerkleTree {
    /// Build from leaf hashes
    pub fn from_leaves(leaves: Vec<ContentHash>) -> Self {
        let leaves: Vec<_> = leaves.into_iter()
            .map(|h| *h.as_bytes())
            .collect();
        
        Self {
            inner: RsMerkleTree::from_leaves(&leaves),
        }
    }
    
    /// Root hash
    pub fn root(&self) -> ContentHash {
        ContentHash::new(self.inner.root().unwrap_or([0; 32]))
    }
    
    /// Get proof for leaf
    pub fn proof(&self, leaf_index: usize) -> MerkleProof {
        self.inner.proof(&[leaf_index])
    }
    
    /// Verify proof
    pub fn verify_proof(
        &self,
        leaf: ContentHash,
        leaf_index: usize,
        proof: &MerkleProof,
    ) -> bool {
        proof.verify(
            self.root().as_bytes(),
            &[leaf_index],
            &[*leaf.as_bytes()],
            self.inner.leaves().len(),
        )
    }
}

/// Incremental Merkle tree for streaming updates
/// 
/// For cases where we need to update individual leaves,
/// we use rs_merkle's update method or rebuild selectively.
```
```

---

## Tests

```rust
// File: tests/artifact_tests.rs (300 lines)

#[cfg(test)]
mod tests {
    use super::*;
    use coa_artifact::*;

    #[test]
    fn artifact_hash_deterministic() {
        let content = CodeContent::parse("fn main() {}", Language::Rust).unwrap();
        let a1 = Artifact::<CodeArtifact>::new(content.clone());
        let a2 = Artifact::<CodeArtifact>::new(content);
        
        assert_eq!(a1.hash(), a2.hash());
    }

    #[test]
    fn artifact_verify_succeeds_for_valid() {
        let artifact = create_test_code_artifact();
        assert!(artifact.verify());
    }

    #[test]
    fn artifact_verify_fails_for_tampered() {
        let mut artifact = create_test_code_artifact();
        // Tamper with content
        artifact.content.source = "tampered".to_string();
        assert!(!artifact.verify());
    }

    #[test]
    fn delta_application_produces_new_hash() {
        let base = create_test_code_artifact();
        let delta = create_add_function_delta(&base);
        
        let new_artifact = delta.apply(&base).unwrap();
        
        assert_ne!(base.hash(), new_artifact.hash());
    }

    #[test]
    fn delta_base_mismatch_detected() {
        let base = create_test_code_artifact();
        let mut other = base.clone();
        // Modify other
        other.content.source = "different".to_string();
        
        let delta = StructuralDelta::new(
            SymbolPath::from_str("main"),
            DeltaOperation::Remove,
            *other.hash(), // Wrong base hash
        );
        
        assert!(delta.apply(&base).is_err());
    }

    #[test]
    fn merkle_tree_update_changes_root() {
        let leaves: Vec<_> = (0..8).map(|i| {
            ContentHash::new([i as u8; 32])
        }).collect();
        
        let mut tree = MerkleTree::from_leaves(leaves);
        let old_root = tree.root();
        
        tree.update_leaf(3, ContentHash::new([255; 32]));
        let new_root = tree.root();
        
        assert_ne!(old_root, new_root);
    }

    #[test]
    fn code_parse_produces_valid_ast() {
        let code = r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;
        
        let content = CodeContent::parse(code, Language::Rust).unwrap();
        
        assert!(content.symbol_table.contains("add"));
        assert_eq!(content.language, Language::Rust);
    }
}
```

---

## File Structure

```
crates/coa-artifact/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API, ArtifactType trait
    ├── artifact.rs         # Artifact<T> struct
    ├── delta.rs            # StructuralDelta<T>
    ├── hash.rs             # MerkleTree, ContentHash
    ├── error.rs            # Error types
    └── types/
        ├── mod.rs          # Type exports
        ├── code.rs         # CodeArtifact (Tree-sitter)
        ├── config.rs       # ConfigArtifact (JSON/YAML)
        └── spec.rs         # SpecArtifact (Markdown)
```

---

## Cargo.toml

```toml
[package]
name = "coa-artifact"
version = "0.1.0"
edition = "2021"

[dependencies]
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Hashing
sha2 = "0.10"
blake3 = "1"

# Parsing
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-typescript = "0.20"
# ... more languages

# Schema validation
schemars = "0.8"

# Error handling
thiserror = "1"

[dev-dependencies]
proptest = "1"
criterion = "0.5"
```
