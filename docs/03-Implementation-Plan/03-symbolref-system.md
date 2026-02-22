# SymbolRef System Design

**Module**: `coa-symbol`  
**Lines Estimate**: ~1,200  
**Files**: 6-8

---

## Overview

The SymbolRef system provides **content-addressed symbolic references** with O(log n) lookup via a radix tree index. This is the foundation for referential integrity and composition validation.

---

## Core Types

### SymbolRef

```rust
// File: src/symbol.rs (150 lines)

use coa_artifact::ContentHash;

/// Content-addressed symbolic reference
/// 
/// Unlike file paths, SymbolRef includes the content hash of the parent,
/// enabling construction-time verification and automatic invalidation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolRef {
    /// Logical path: ["crate", "module", "symbol"]
    path: Vec<String>,
    
    /// Content hash of parent artifact (for hash-binding)
    parent_hash: ContentHash,
    
    /// Optional: specific version/revision
    revision: Option<Revision>,
}

/// Symbol revision (branch + commit)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision {
    branch: String,
    commit: ContentHash,
}

impl SymbolRef {
    /// Create new symbol reference
    pub fn new(path: Vec<String>, parent_hash: ContentHash) -> Self {
        Self {
            path,
            parent_hash,
            revision: None,
        }
    }
    
    /// Path segments
    #[inline]
    pub fn path(&self) -> &[String] {
        &self.path
    }
    
    /// Parent content hash
    #[inline]
    pub fn parent_hash(&self) -> &ContentHash {
        &self.parent_hash
    }
    
    /// Full string representation
    pub fn to_string(&self) -> String {
        let path_str = self.path.join(".");
        let hash_str = hex::encode(self.parent_hash.as_bytes());
        format!("{}@{}", path_str, &hash_str[..8])
    }
    
    /// Check if this symbol is an ancestor of another
    pub fn is_ancestor_of(&self, other: &SymbolRef) -> bool {
        if self.path.len() >= other.path.len() {
            return false;
        }
        self.path == other.path[..self.path.len()]
    }
    
    /// Get parent symbol (if not root)
    pub fn parent(&self) -> Option<SymbolRef> {
        if self.path.len() <= 1 {
            None
        } else {
            Some(Self {
                path: self.path[..self.path.len()-1].to_vec(),
                parent_hash: self.parent_hash, // Same parent
                revision: self.revision.clone(),
            })
        }
    }
    
    /// Get child symbol
    pub fn child(&self, segment: impl Into<String>) -> SymbolRef {
        let mut path = self.path.clone();
        path.push(segment.into());
        Self {
            path,
            parent_hash: self.parent_hash,
            revision: self.revision.clone(),
        }
    }
}
```

### SymbolRefIndex - Using `radix_trie`

```rust
// File: src/index.rs (150 lines) - GLUE CODE ONLY

use radix_trie::{Trie, TrieCommon};
use dashmap::DashMap;

/// Symbol index using radix_trie for prefix matching
/// 
/// We use radix_trie for efficient:
/// - Prefix lookups
/// - Ancestor/descendant checks (overlap detection)
/// - Iteration over subtrees
pub struct SymbolRefIndex {
    /// Radix trie mapping path -> symbol + metadata
    trie: RwLock<Trie<String, IndexedSymbol>>,
    
    /// Reverse index: parent_hash -> symbols
    by_parent: DashMap<ContentHash, Vec<SymbolRef>>,
}

#[derive(Debug, Clone)]
struct IndexedSymbol {
    symbol: SymbolRef,
    metadata: SymbolMetadata,
}

impl SymbolRefIndex {
    /// Create empty index
    pub fn new() -> Self {
        Self {
            trie: RwLock::new(Trie::new()),
            by_parent: DashMap::new(),
        }
    }
    
    /// Insert symbol into index
    pub fn insert(&self, symbol: SymbolRef, metadata: SymbolMetadata) -> Result<(), IndexError> {
        let path = symbol.path().join("/"); // Trie key
        
        // Check for exact duplicate
        let trie = self.trie.read();
        if trie.get(&path).is_some() {
            return Err(IndexError::DuplicateSymbol { path });
        }
        
        // Check for overlap using trie prefix methods
        if self.has_overlap(&path, &trie) {
            return Err(IndexError::OverlappingClaims { path });
        }
        drop(trie);
        
        // Insert
        let mut trie = self.trie.write();
        trie.insert(path, IndexedSymbol { symbol: symbol.clone(), metadata });
        
        // Update reverse index
        self.by_parent
            .entry(symbol.parent_hash().clone())
            .or_default()
            .push(symbol);
        
        Ok(())
    }
    
    /// Check if path overlaps with existing symbols
    fn has_overlap(&self, path: &str, trie: &Trie<String, IndexedSymbol>) -> bool {
        // Case 1: Existing symbol is prefix of new (ancestor)
        // get_ancestor_values returns all values for prefixes
        if trie.get_ancestor_values(path).next().is_some() {
            return true;
        }
        
        // Case 2: New symbol is prefix of existing (descendant)
        // get_raw_descendant returns subtree
        if trie.get_raw_descendant(path).is_some() {
            return true;
        }
        
        false
    }
    
    /// Lookup exact symbol
    pub fn get_exact(&self, symbol: &SymbolRef) -> Option<IndexEntry> {
        let path = symbol.path().join("/");
        self.trie.read()
            .get(&path)
            .map(|idx| IndexEntry {
                symbol: idx.symbol.clone(),
                metadata: idx.metadata.clone(),
            })
    }
    
    /// Check if symbol exists
    pub fn contains(&self, path: &[String]) -> bool {
        let key = path.join("/");
        self.trie.read().get(&key).is_some()
    }
    
    /// Find all symbols in subtree
    pub fn get_descendants(&self, prefix: &[String]) -> Vec<IndexEntry> {
        let key = prefix.join("/");
        let trie = self.trie.read();
        
        trie.get_raw_descendant(&key)
            .map(|subtrie| {
                subtrie.values()
                    .map(|idx| IndexEntry {
                        symbol: idx.symbol.clone(),
                        metadata: idx.metadata.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Validate references resolve
    pub fn validate_references(
        &self,
        references: &[SymbolRef],
    ) -> Result<(), ValidationError> {
        for (i, reference) in references.iter().enumerate() {
            if self.get_exact(reference).is_none() {
                let similar = self.find_similar(reference);
                return Err(ValidationError::UnresolvedReference {
                    index: i,
                    reference: reference.clone(),
                    similar_suggestions: similar,
                });
            }
        }
        Ok(())
    }
    
    /// Find similar symbols (same last segment)
    fn find_similar(&self, target: &SymbolRef) -> Vec<SymbolRef> {
        let target_name = target.path().last()?;
        
        self.trie.read()
            .values()
            .filter(|idx| idx.symbol.path().last() == Some(target_name))
            .take(5)
            .map(|idx| idx.symbol.clone())
            .collect()
    }
}
```

---

## Single-Writer Validation

```rust
// File: src/validation.rs (200 lines)

/// Single-writer invariant validation
/// 
/// Ensures no overlapping SymbolRef claims across agents.
pub struct SingleWriterValidator;

impl SingleWriterValidator {
    /// Validate that deltas have disjoint target symbols
    /// 
    /// # Returns
    /// - `Ok(())` if no overlaps
    /// - `Err(ValidationError::OverlappingClaims)` if conflict
    pub fn validate_deltas<T: ArtifactType>(
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<(), ValidationError> {
        // Build set of claimed paths
        let mut claims: Vec<(&SymbolPath, &StructuralDelta<T>)> = deltas
            .iter()
            .map(|d| (&d.target, d))
            .collect();
        
        // Sort by path for deterministic error messages
        claims.sort_by(|a, b| a.0.cmp(b.0));
        
        // Check each pair for overlap
        for i in 0..claims.len() {
            for j in (i + 1)..claims.len() {
                let (path_i, delta_i) = claims[i];
                let (path_j, delta_j) = claims[j];
                
                if Self::paths_overlap(path_i, path_j) {
                    return Err(ValidationError::OverlappingClaims {
                        claim1: path_i.to_string(),
                        claim2: path_j.to_string(),
                        delta1_description: delta_i.description.clone(),
                        delta2_description: delta_j.description.clone(),
                    });
                }
            }
        }
        
        // Also check against existing symbols in index
        for (path, _) in &claims {
            if let Some(existing) = index.find_containing(path) {
                return Err(ValidationError::ClaimOverlapsExisting {
                    new_claim: path.to_string(),
                    existing: existing.symbol.to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Check if two paths overlap (one is prefix of other)
    fn paths_overlap(a: &SymbolPath, b: &SymbolPath) -> bool {
        let a_segments = a.segments();
        let b_segments = b.segments();
        
        let min_len = a_segments.len().min(b_segments.len());
        a_segments[..min_len] == b_segments[..min_len]
    }
}

/// Diagnostic information for validation failures
#[derive(Debug, Clone)]
pub struct ConflictDiagnostic {
    pub kind: ConflictKind,
    pub involved_symbols: Vec<SymbolRef>,
    pub suggested_resolution: ResolutionSuggestion,
}

#[derive(Debug, Clone)]
pub enum ConflictKind {
    /// Two deltas claim same/overlapping paths
    OverlappingDeltaClaims,
    
    /// Delta claims path inside existing symbol
    InsideExistingSymbol,
    
    /// Delta claims path that contains existing symbol
    ContainsExistingSymbol,
}

#[derive(Debug, Clone)]
pub enum ResolutionSuggestion {
    /// Decompose into non-overlapping paths
    Decompose { suggested_paths: Vec<SymbolPath> },
    
    /// Use sequential composition instead
    UseSequential,
    
    /// Merge into single agent
    MergeAgents,
}
```

---

## Incremental Validation

```rust
// File: src/incremental.rs (150 lines)

/// Incremental validation for dynamic expansion
/// 
/// Only validates changed subtrees, not entire graph.
pub struct IncrementalValidator {
    base_index: SymbolRefIndex,
    changes: Vec<Change>,
}

#[derive(Debug, Clone)]
pub enum Change {
    Added(SymbolRef),
    Removed(SymbolRef),
    Modified(SymbolRef, ContentHash /* old */, ContentHash /* new */),
}

impl IncrementalValidator {
    pub fn new(base_index: SymbolRefIndex) -> Self {
        Self {
            base_index,
            changes: Vec::new(),
        }
    }
    
    /// Record a change
    pub fn record(&mut self, change: Change) {
        self.changes.push(change);
    }
    
    /// Validate only affected references
    /// 
    /// # Performance
    /// O(k log n) where k = affected references, not total references
    pub fn validate_affected(
        &self,
        new_deltas: &[StructuralDelta<impl ArtifactType>],
    ) -> Result<(), ValidationError> {
        let affected_paths: HashSet<_> = self.changes.iter()
            .map(|c| match c {
                Change::Added(s) => s.path().to_vec(),
                Change::Removed(s) => s.path().to_vec(),
                Change::Modified(s, _, _) => s.path().to_vec(),
            })
            .collect();
        
        for delta in new_deltas {
            let target = &delta.target;
            
            // Check if delta touches affected subtree
            if affected_paths.iter().any(|p| {
                Self::paths_overlap(p, target.segments())
            }) {
                // Full validation needed for this delta
                self.validate_delta_full(delta)?;
            }
            // Else: skip validation (dependencies unchanged)
        }
        
        Ok(())
    }
    
    fn paths_overlap(a: &[String], b: &[String]) -> bool {
        let min_len = a.len().min(b.len());
        a[..min_len] == b[..min_len]
    }
    
    fn validate_delta_full<T: ArtifactType>(
        &self,
        delta: &StructuralDelta<T>,
    ) -> Result<(), ValidationError> {
        // ... full validation logic
        todo!()
    }
}
```

---

## Tests

```rust
// File: tests/index_tests.rs (300 lines)

#[cfg(test)]
mod tests {
    use super::*;
    use coa_symbol::*;

    #[test]
    fn index_insert_and_lookup() {
        let index = SymbolRefIndex::new();
        
        let symbol = SymbolRef::new(
            vec!["auth".to_string(), "login".to_string()],
            ContentHash::new([1; 32]),
        );
        
        index.insert(symbol.clone(), SymbolMetadata::default()).unwrap();
        
        let found = index.get_exact(&symbol).unwrap();
        assert_eq!(found.symbol, symbol);
    }

    #[test]
    fn index_rejects_duplicate() {
        let index = SymbolRefIndex::new();
        
        let symbol = SymbolRef::new(
            vec!["auth".to_string(), "login".to_string()],
            ContentHash::new([1; 32]),
        );
        
        index.insert(symbol.clone(), SymbolMetadata::default()).unwrap();
        
        let result = index.insert(symbol, SymbolMetadata::default());
        assert!(matches!(result, Err(IndexError::DuplicateSymbol { .. })));
    }

    #[test]
    fn index_detects_ancestor_overlap() {
        let index = SymbolRefIndex::new();
        
        // Insert parent
        let parent = SymbolRef::new(
            vec!["auth".to_string()],
            ContentHash::new([1; 32]),
        );
        index.insert(parent, SymbolMetadata::default()).unwrap();
        
        // Try to insert child (should fail - overlap)
        let child = SymbolRef::new(
            vec!["auth".to_string(), "login".to_string()],
            ContentHash::new([1; 32]),
        );
        
        let result = index.insert(child, SymbolMetadata::default());
        assert!(matches!(result, Err(IndexError::OverlappingClaims { .. })));
    }

    #[test]
    fn index_detects_descendant_overlap() {
        let index = SymbolRefIndex::new();
        
        // Insert child first
        let child = SymbolRef::new(
            vec!["auth".to_string(), "login".to_string()],
            ContentHash::new([1; 32]),
        );
        index.insert(child, SymbolMetadata::default()).unwrap();
        
        // Try to insert parent (should fail - overlap)
        let parent = SymbolRef::new(
            vec!["auth".to_string()],
            ContentHash::new([1; 32]),
        );
        
        let result = index.insert(parent, SymbolMetadata::default());
        assert!(matches!(result, Err(IndexError::OverlappingClaims { .. })));
    }

    #[test]
    fn validate_reports_unresolved() {
        let index = SymbolRefIndex::new();
        
        let existing = SymbolRef::new(
            vec!["auth".to_string(), "login".to_string()],
            ContentHash::new([1; 32]),
        );
        index.insert(existing, SymbolMetadata::default()).unwrap();
        
        // Reference to non-existent symbol
        let missing = SymbolRef::new(
            vec!["auth".to_string(), "logout".to_string()],
            ContentHash::new([1; 32]),
        );
        
        let result = index.validate_references(&[missing]);
        assert!(matches!(result, Err(ValidationError::UnresolvedReference { .. })));
    }

    #[test]
    fn single_writer_validates_disjoint_paths() {
        let index = SymbolRefIndex::new();
        
        let delta1 = create_delta("auth.login");
        let delta2 = create_delta("auth.register");
        
        // Should pass - disjoint paths
        let result = SingleWriterValidator::validate_deltas(
            &[delta1, delta2],
            &index,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn single_writer_rejects_overlapping() {
        let index = SymbolRefIndex::new();
        
        let delta1 = create_delta("auth");
        let delta2 = create_delta("auth.login");
        
        // Should fail - overlapping
        let result = SingleWriterValidator::validate_deltas(
            &[delta1, delta2],
            &index,
        );
        assert!(matches!(result, Err(ValidationError::OverlappingClaims { .. })));
    }

    #[test]
    fn symbol_ref_parent_child() {
        let parent = SymbolRef::new(
            vec!["auth".to_string()],
            ContentHash::new([1; 32]),
        );
        
        let child = parent.child("login");
        
        assert_eq!(child.path(), &["auth", "login"]);
        assert_eq!(child.parent_hash(), parent.parent_hash());
    }
}
```

---

## File Structure

```
crates/coa-symbol/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── symbol.rs           # SymbolRef, Revision
    ├── index.rs            # SymbolRefIndex, IndexNode
    ├── validation.rs       # SingleWriterValidator
    ├── incremental.rs      # IncrementalValidator
    └── error.rs            # Error types
```

---

## Cargo.toml

```toml
[package]
name = "coa-symbol"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core
coa-artifact = { path = "../coa-artifact" }

# Collections
dashmap = "5"
parking_lot = "0.12"

# Serialization
serde = { version = "1", features = ["derive"] }
hex = "0.4"

# Error handling
thiserror = "1"

[dev-dependencies]
proptest = "1"
criterion = "0.5"
```
