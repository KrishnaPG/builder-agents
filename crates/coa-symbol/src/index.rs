//! Symbol reference index with radix tree
//!
//! Provides [`SymbolRefIndex`] for O(log n) symbol lookup using radix_trie.

use crate::symbol::SymbolRef;
use crate::symbol::SymbolRefError;
use coa_artifact::ContentHash;
use dashmap::DashMap;
use radix_trie::{Trie, TrieCommon};
use std::sync::RwLock;

/// Symbol index using radix_trie for prefix matching
///
/// We use radix_trie for efficient:
/// - Prefix lookups
/// - Ancestor/descendant checks (overlap detection)
/// - Iteration over subtrees
///
/// The index is thread-safe with concurrent reads via DashMap for the
/// reverse index and RwLock for the trie.
#[derive(Debug)]
pub struct SymbolRefIndex {
    /// Radix trie mapping path -> indexed symbol
    trie: RwLock<Trie<String, IndexedSymbol>>,

    /// Reverse index: parent_hash -> symbols (for invalidation)
    by_parent: DashMap<ContentHash, Vec<SymbolRef>>,
}

/// Indexed symbol with metadata
#[derive(Debug, Clone)]
struct IndexedSymbol {
    symbol: SymbolRef,
    metadata: SymbolMetadata,
}

/// Metadata for indexed symbols
#[derive(Debug, Clone, Default)]
pub struct SymbolMetadata {
    /// Symbol kind (function, type, variable, etc.)
    pub kind: SymbolKind,

    /// Visibility within artifact
    pub visibility: Visibility,

    /// Source location (optional)
    pub source_location: Option<SourceLocation>,

    /// Custom attributes
    pub attributes: Vec<String>,
}

/// Symbol kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolKind {
    /// Unknown/default kind
    #[default]
    Unknown,

    /// Function or method
    Function,

    /// Type (struct, enum, trait, etc.)
    Type,

    /// Variable or constant
    Variable,

    /// Module or namespace
    Module,

    /// Configuration entry
    Config,

    /// Documentation/specification
    Spec,
}

/// Symbol visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    /// Public/exported
    #[default]
    Public,

    /// Internal/private
    Internal,

    /// Restricted visibility
    Restricted,
}

/// Source code location
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub file: String,
}

impl SymbolRefIndex {
    /// Create empty index
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            trie: RwLock::new(Trie::new()),
            by_parent: DashMap::new(),
        }
    }

    /// Insert symbol into index
    ///
    /// # Errors
    /// Returns error if symbol already exists or overlaps with existing
    pub fn insert(
        &self,
        symbol: SymbolRef,
        metadata: SymbolMetadata,
    ) -> Result<(), SymbolRefError> {
        let path_key = symbol.to_trie_key();

        // Check for exact duplicate
        let trie = self.trie.read().map_err(|_| SymbolRefError::LockPoisoned)?;
        if trie.get(&path_key).is_some() {
            return Err(SymbolRefError::DuplicateSymbol { path: path_key });
        }

        drop(trie);

        // Check for overlap using trie prefix methods
        if self.has_overlap(&path_key) {
            return Err(SymbolRefError::OverlappingClaims { path: path_key });
        }

        // Insert into trie
        let mut trie = self.trie.write().map_err(|_| SymbolRefError::LockPoisoned)?;
        trie.insert(
            path_key.clone(),
            IndexedSymbol {
                symbol: symbol.clone(),
                metadata,
            },
        );
        drop(trie);

        // Update reverse index
        self.by_parent
            .entry(*symbol.parent_hash())
            .or_default()
            .push(symbol);

        Ok(())
    }

    /// Check if path overlaps with existing symbols
    fn has_overlap(&self, path: &str) -> bool {
        let trie = match self.trie.read() {
            Ok(t) => t,
            Err(_) => return false,
        };

        // Case 1: Existing symbol is prefix of new (ancestor)
        // get_ancestor returns the closest ancestor value
        if trie.get_ancestor(path).is_some() {
            return true;
        }

        // Case 2: New symbol is prefix of existing (descendant)
        if trie.get_raw_descendant(path).is_some() {
            return true;
        }

        false
    }

    /// Lookup exact symbol by reference
    #[must_use]
    pub fn get_exact(&self, symbol: &SymbolRef) -> Option<IndexEntry> {
        let path = symbol.to_trie_key();
        let trie = self.trie.read().ok()?;
        trie.get(&path).map(|idx| IndexEntry {
            symbol: idx.symbol.clone(),
            metadata: idx.metadata.clone(),
        })
    }

    /// Check if symbol exists in index
    #[inline]
    #[must_use]
    pub fn contains(&self, symbol: &SymbolRef) -> bool {
        self.get_exact(symbol).is_some()
    }

    /// Find symbol by exact path
    #[must_use]
    pub fn get_by_path(&self, path: &[String]) -> Option<IndexEntry> {
        let key = path.join("/");
        let trie = self.trie.read().ok()?;
        trie.get(&key).map(|idx| IndexEntry {
            symbol: idx.symbol.clone(),
            metadata: idx.metadata.clone(),
        })
    }

    /// Get all symbols in subtree (descendants of prefix)
    #[must_use]
    pub fn get_descendants(&self, prefix: &[String]) -> Vec<IndexEntry> {
        let key = prefix.join("/");
        let trie = match self.trie.read() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        trie.get_raw_descendant(&key)
            .map(|subtrie| {
                subtrie
                    .values()
                    .map(|idx| IndexEntry {
                        symbol: idx.symbol.clone(),
                        metadata: idx.metadata.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get direct children of a path (non-recursive)
    #[must_use]
    pub fn get_children(&self, parent_path: &[String]) -> Vec<IndexEntry> {
        let parent_depth = parent_path.len();
        let descendants = self.get_descendants(parent_path);

        descendants
            .into_iter()
            .filter(|entry| entry.symbol.depth() == parent_depth + 1)
            .collect()
    }

    /// Find all symbols with matching name (anywhere in tree)
    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Vec<IndexEntry> {
        let trie = match self.trie.read() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        trie.values()
            .filter(|idx| idx.symbol.name() == Some(name))
            .map(|idx| IndexEntry {
                symbol: idx.symbol.clone(),
                metadata: idx.metadata.clone(),
            })
            .collect()
    }

    /// Get all symbols for a parent hash (for invalidation)
    #[inline]
    #[must_use]
    pub fn get_by_parent(&self, parent_hash: &ContentHash) -> Vec<SymbolRef> {
        self.by_parent
            .get(parent_hash)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }

    /// Remove all symbols for a parent hash (when artifact changes)
    ///
    /// Returns number of symbols removed.
    pub fn remove_by_parent(&self, parent_hash: &ContentHash) -> usize {
        let symbols = match self.by_parent.remove(parent_hash) {
            Some((_, symbols)) => symbols,
            None => return 0,
        };

        let mut trie = match self.trie.write() {
            Ok(t) => t,
            Err(_) => return 0,
        };

        let count = symbols.len();
        for symbol in &symbols {
            trie.remove(&symbol.to_trie_key());
        }

        count
    }

    /// Get total symbol count
    #[must_use]
    pub fn len(&self) -> usize {
        match self.trie.read() {
            Ok(t) => t.len(),
            Err(_) => 0,
        }
    }

    /// Check if index is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if any symbol overlaps with given path
    #[must_use]
    pub fn has_any_overlap(&self, path: &[String]) -> bool {
        let key = path.join("/");
        match self.trie.read() {
            Ok(trie) => {
                if trie.get_ancestor(&key).is_some() {
                    return true;
                }
                if trie.get_raw_descendant(&key).is_some() {
                    return true;
                }
                false
            }
            Err(_) => false,
        }
    }

    /// Find symbols that would conflict with given path
    #[must_use]
    pub fn find_conflicts(&self, path: &[String]) -> Vec<IndexEntry> {
        let key = path.join("/");
        let trie = match self.trie.read() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let mut conflicts = Vec::new();

        // Ancestors - walk up the path looking for ancestors
        for i in 1..=key.len() {
            let prefix = &key[..i];
            if let Some(idx) = trie.get(prefix) {
                conflicts.push(IndexEntry {
                    symbol: idx.symbol.clone(),
                    metadata: idx.metadata.clone(),
                });
            }
        }

        // Descendants
        if let Some(subtrie) = trie.get_raw_descendant(&key) {
            for idx in subtrie.values() {
                conflicts.push(IndexEntry {
                    symbol: idx.symbol.clone(),
                    metadata: idx.metadata.clone(),
                });
            }
        }

        conflicts
    }
}

impl Default for SymbolRefIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Entry returned from index lookups
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// The symbol reference
    pub symbol: SymbolRef,

    /// Associated metadata
    pub metadata: SymbolMetadata,
}

// SymbolRefError re-exported from symbol module

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hash() -> ContentHash {
        ContentHash::compute(b"test")
    }

    fn test_hash_n(n: u8) -> ContentHash {
        ContentHash::compute(&[n])
    }

    fn make_symbol(path: &[&str], hash: ContentHash) -> SymbolRef {
        SymbolRef::new(path.iter().map(|s| s.to_string()).collect(), hash)
    }

    #[test]
    fn index_insert_and_lookup() {
        let index = SymbolRefIndex::new();
        let sym = make_symbol(&["auth", "login"], test_hash());

        index.insert(sym.clone(), SymbolMetadata::default()).unwrap();

        let found = index.get_exact(&sym).unwrap();
        assert_eq!(found.symbol, sym);
    }

    #[test]
    fn index_rejects_duplicate() {
        let index = SymbolRefIndex::new();
        let sym = make_symbol(&["auth", "login"], test_hash());

        index.insert(sym.clone(), SymbolMetadata::default()).unwrap();
        let result = index.insert(sym.clone(), SymbolMetadata::default());

        assert!(matches!(result, Err(SymbolRefError::DuplicateSymbol { .. })));
    }

    #[test]
    fn index_detects_ancestor_overlap() {
        let index = SymbolRefIndex::new();
        let parent = make_symbol(&["auth"], test_hash());
        let child = make_symbol(&["auth", "login"], test_hash());

        index.insert(parent, SymbolMetadata::default()).unwrap();
        let result = index.insert(child, SymbolMetadata::default());

        assert!(matches!(result, Err(SymbolRefError::OverlappingClaims { .. })));
    }

    #[test]
    fn index_detects_descendant_overlap() {
        let index = SymbolRefIndex::new();
        let child = make_symbol(&["auth", "login"], test_hash());
        let parent = make_symbol(&["auth"], test_hash());

        index.insert(child, SymbolMetadata::default()).unwrap();
        let result = index.insert(parent, SymbolMetadata::default());

        assert!(matches!(result, Err(SymbolRefError::OverlappingClaims { .. })));
    }

    #[test]
    fn index_get_descendants() {
        let index = SymbolRefIndex::new();
        let h = test_hash();

        index
            .insert(make_symbol(&["a", "b", "c"], h), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["a", "b", "d"], h), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["a", "x"], h), SymbolMetadata::default())
            .unwrap();

        let descendants = index.get_descendants(&["a".to_string(), "b".to_string()]);
        assert_eq!(descendants.len(), 2);
    }

    #[test]
    fn index_get_children() {
        let index = SymbolRefIndex::new();
        let h = test_hash();

        // Insert children of "a" - non-overlapping paths
        index
            .insert(make_symbol(&["a", "b"], h), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["a", "c"], h), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["a", "d"], h), SymbolMetadata::default())
            .unwrap();

        let children = index.get_children(&["a".to_string()]);
        assert_eq!(children.len(), 3); // b, c, and d
    }

    #[test]
    fn index_find_by_name() {
        let index = SymbolRefIndex::new();
        let h = test_hash();

        index
            .insert(make_symbol(&["a", "login"], h), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["b", "login"], h), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["c", "logout"], h), SymbolMetadata::default())
            .unwrap();

        let found = index.find_by_name("login");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn index_get_by_parent() {
        let index = SymbolRefIndex::new();
        let h1 = test_hash_n(1);
        let h2 = test_hash_n(2);

        index
            .insert(make_symbol(&["a"], h1), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["b"], h1), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["c"], h2), SymbolMetadata::default())
            .unwrap();

        let found = index.get_by_parent(&h1);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn index_remove_by_parent() {
        let index = SymbolRefIndex::new();
        let h1 = test_hash_n(1);
        let h2 = test_hash_n(2);

        index
            .insert(make_symbol(&["a"], h1), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["b"], h2), SymbolMetadata::default())
            .unwrap();

        let removed = index.remove_by_parent(&h1);
        assert_eq!(removed, 1);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn index_has_any_overlap() {
        let index = SymbolRefIndex::new();
        let h = test_hash();

        index
            .insert(make_symbol(&["auth", "login"], h), SymbolMetadata::default())
            .unwrap();

        assert!(index.has_any_overlap(&["auth".to_string()]));
        assert!(index.has_any_overlap(&["auth".to_string(), "login".to_string()]));
        assert!(!index.has_any_overlap(&["other".to_string()]));
    }

    #[test]
    fn index_find_conflicts() {
        let index = SymbolRefIndex::new();
        let h = test_hash();

        // Insert non-overlapping symbols
        index
            .insert(make_symbol(&["auth", "login"], h), SymbolMetadata::default())
            .unwrap();
        index
            .insert(make_symbol(&["auth", "logout"], h), SymbolMetadata::default())
            .unwrap();

        // Find conflicts for a path that overlaps with both
        let conflicts = index.find_conflicts(&["auth".to_string()]);
        assert_eq!(conflicts.len(), 2); // Both symbols are under "auth"
    }

    #[test]
    fn symbol_metadata_default() {
        let meta = SymbolMetadata::default();
        assert_eq!(meta.kind, SymbolKind::Unknown);
        assert_eq!(meta.visibility, Visibility::Public);
    }
}
