//! SymbolRef - Content-addressed symbolic references
//!
//! Provides [`SymbolRef`] for referencing symbols within artifacts with
//! content-hash binding for automatic invalidation detection.

use coa_artifact::ContentHash;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Content-addressed symbolic reference
///
/// Unlike file paths, SymbolRef includes the content hash of the parent artifact,
/// enabling construction-time verification and automatic invalidation detection.
///
/// # Structure
/// - `path`: Hierarchical path (e.g., `["crate", "module", "function"]`)
/// - `parent_hash`: Content hash of containing artifact
/// - `revision`: Optional branch/commit for versioned references
///
/// # Example
/// ```
/// use coa_symbol::SymbolRef;
/// use coa_artifact::ContentHash;
///
/// let hash = ContentHash::compute(b"artifact content");
/// let symbol = SymbolRef::new(
///     vec!["crate".into(), "module".into(), "func".into()],
///     hash
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolRef {
    /// Logical path: `["crate", "module", "symbol"]`
    path: Vec<String>,

    /// Content hash of parent artifact (for hash-binding)
    parent_hash: ContentHash,

    /// Optional: specific version/revision
    revision: Option<Revision>,
}

/// Symbol revision (branch + commit)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision {
    /// Branch name
    branch: String,

    /// Commit hash (content-addressed)
    commit: ContentHash,
}

impl SymbolRef {
    /// Create new symbol reference
    ///
    /// # Arguments
    /// - `path`: Vector of path segments
    /// - `parent_hash`: Content hash of containing artifact
    #[inline]
    #[must_use]
    pub fn new(path: Vec<String>, parent_hash: ContentHash) -> Self {
        Self {
            path,
            parent_hash,
            revision: None,
        }
    }

    /// Create with explicit revision
    #[inline]
    #[must_use]
    pub fn with_revision(
        path: Vec<String>,
        parent_hash: ContentHash,
        revision: Revision,
    ) -> Self {
        Self {
            path,
            parent_hash,
            revision: Some(revision),
        }
    }

    /// Path segments
    #[inline]
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Number of path segments
    #[inline]
    #[must_use]
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// Parent content hash
    #[inline]
    #[must_use]
    pub fn parent_hash(&self) -> &ContentHash {
        &self.parent_hash
    }

    /// Optional revision
    #[inline]
    #[must_use]
    pub fn revision(&self) -> Option<&Revision> {
        self.revision.as_ref()
    }

    /// Last segment (symbol name)
    #[inline]
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.path.last().map(|s| s.as_str())
    }

    /// Full string representation
    ///
    /// Format: `path.to.symbol@hashshort[#branch:commit]`
    #[must_use]
    pub fn to_string(&self) -> String {
        let path_str = self.path.join(".");
        let hash_str = self.parent_hash.short();

        match &self.revision {
            Some(rev) => format!("{}@{}#{}:{}", path_str, hash_str, rev.branch, rev.commit.short()),
            None => format!("{}@{}", path_str, hash_str),
        }
    }

    /// Check if this symbol is an ancestor of another
    ///
    /// An ancestor has a shorter path that matches the prefix.
    #[must_use]
    pub fn is_ancestor_of(&self, other: &SymbolRef) -> bool {
        if self.path.len() >= other.path.len() {
            return false;
        }
        self.path == other.path[..self.path.len()]
    }

    /// Check if this symbol is a descendant of another
    #[inline]
    #[must_use]
    pub fn is_descendant_of(&self, other: &SymbolRef) -> bool {
        other.is_ancestor_of(self)
    }

    /// Check if paths overlap (one is prefix of other)
    #[must_use]
    pub fn overlaps(&self, other: &SymbolRef) -> bool {
        let min_len = self.path.len().min(other.path.len());
        self.path[..min_len] == other.path[..min_len]
    }

    /// Get parent symbol (if not root)
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.path.len() <= 1 {
            None
        } else {
            Some(Self {
                path: self.path[..self.path.len() - 1].to_vec(),
                parent_hash: self.parent_hash,
                revision: self.revision.clone(),
            })
        }
    }

    /// Get child symbol
    #[inline]
    #[must_use]
    pub fn child(&self, segment: impl Into<String>) -> Self {
        let mut path = self.path.clone();
        path.push(segment.into());
        Self {
            path,
            parent_hash: self.parent_hash,
            revision: self.revision.clone(),
        }
    }

    /// Extend path with multiple segments
    #[inline]
    #[must_use]
    pub fn extend(&self, segments: &[impl AsRef<str>]) -> Self {
        let mut path = self.path.clone();
        for seg in segments {
            path.push(seg.as_ref().to_string());
        }
        Self {
            path,
            parent_hash: self.parent_hash,
            revision: self.revision.clone(),
        }
    }

    /// Returns true if this is a root-level symbol (single segment)
    #[inline]
    #[must_use]
    pub fn is_root_level(&self) -> bool {
        self.path.len() == 1
    }

    /// Check if symbol is in a specific namespace (first segment)
    #[inline]
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.path.first().map(|s| s.as_str())
    }

    /// Create a reference to the same path but with different parent hash
    #[inline]
    #[must_use]
    pub fn with_parent_hash(&self, parent_hash: ContentHash) -> Self {
        Self {
            path: self.path.clone(),
            parent_hash,
            revision: self.revision.clone(),
        }
    }

    /// Create a trie-compatible key (slash-separated)
    #[inline]
    #[must_use]
    pub fn to_trie_key(&self) -> String {
        self.path.join("/")
    }
}

impl Revision {
    /// Create new revision
    #[inline]
    #[must_use]
    pub fn new(branch: impl Into<String>, commit: ContentHash) -> Self {
        Self {
            branch: branch.into(),
            commit,
        }
    }

    /// Branch name
    #[inline]
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.branch
    }

    /// Commit hash
    #[inline]
    #[must_use]
    pub fn commit(&self) -> &ContentHash {
        &self.commit
    }
}

impl Display for SymbolRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Errors for SymbolRef operations
#[derive(Debug, thiserror::Error)]
pub enum SymbolRefError {
    /// Invalid format
    #[error("invalid symbol reference format: {0}")]
    InvalidFormat(String),

    /// Invalid hash
    #[error("invalid parent hash: {0}")]
    InvalidHash(String),

    /// Empty path
    #[error("symbol path cannot be empty")]
    EmptyPath,

    /// Duplicate symbol
    #[error("duplicate symbol at path: {path}")]
    DuplicateSymbol { path: String },

    /// Overlapping symbol claims
    #[error("symbol path overlaps with existing: {path}")]
    OverlappingClaims { path: String },

    /// Lock poisoned
    #[error("index lock poisoned")]
    LockPoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hash() -> ContentHash {
        ContentHash::compute(b"test artifact")
    }

    #[test]
    fn symbol_ref_new() {
        let hash = test_hash();
        let sym = SymbolRef::new(vec!["a".into(), "b".into()], hash);

        assert_eq!(sym.path(), &["a", "b"]);
        assert_eq!(sym.parent_hash(), &hash);
        assert!(sym.revision().is_none());
    }

    #[test]
    fn symbol_ref_with_revision() {
        let hash = test_hash();
        let commit = ContentHash::compute(b"commit");
        let revision = Revision::new("main", commit);

        let sym = SymbolRef::with_revision(vec!["a".into()], hash, revision.clone());

        assert_eq!(sym.revision(), Some(&revision));
    }

    #[test]
    fn symbol_ref_depth() {
        let hash = test_hash();
        let sym1 = SymbolRef::new(vec!["a".into()], hash);
        let sym2 = SymbolRef::new(vec!["a".into(), "b".into(), "c".into()], hash);

        assert_eq!(sym1.depth(), 1);
        assert_eq!(sym2.depth(), 3);
    }

    #[test]
    fn symbol_ref_name() {
        let hash = test_hash();
        let sym = SymbolRef::new(vec!["crate".into(), "module".into(), "func".into()], hash);

        assert_eq!(sym.name(), Some("func"));
    }

    #[test]
    fn symbol_ref_ancestor_detection() {
        let hash = test_hash();
        let parent = SymbolRef::new(vec!["a".into(), "b".into()], hash);
        let child = SymbolRef::new(vec!["a".into(), "b".into(), "c".into()], hash);
        let unrelated = SymbolRef::new(vec!["a".into(), "x".into()], hash);

        assert!(parent.is_ancestor_of(&child));
        assert!(!child.is_ancestor_of(&parent));
        assert!(!parent.is_ancestor_of(&unrelated));
    }

    #[test]
    fn symbol_ref_descendant_detection() {
        let hash = test_hash();
        let parent = SymbolRef::new(vec!["a".into()], hash);
        let child = SymbolRef::new(vec!["a".into(), "b".into()], hash);

        assert!(child.is_descendant_of(&parent));
        assert!(!parent.is_descendant_of(&child));
    }

    #[test]
    fn symbol_ref_overlaps() {
        let hash = test_hash();
        let a = SymbolRef::new(vec!["a".into(), "b".into()], hash);
        let b = SymbolRef::new(vec!["a".into(), "b".into(), "c".into()], hash);
        let c = SymbolRef::new(vec!["a".into(), "x".into()], hash);

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn symbol_ref_parent() {
        let hash = test_hash();
        let child = SymbolRef::new(vec!["a".into(), "b".into(), "c".into()], hash);
        let parent = child.parent().unwrap();

        assert_eq!(parent.path(), &["a", "b"]);
        assert_eq!(parent.parent_hash(), &hash);
    }

    #[test]
    fn symbol_ref_root_parent_is_none() {
        let hash = test_hash();
        let root = SymbolRef::new(vec!["a".into()], hash);
        assert!(root.parent().is_none());
    }

    #[test]
    fn symbol_ref_child() {
        let hash = test_hash();
        let parent = SymbolRef::new(vec!["a".into(), "b".into()], hash);
        let child = parent.child("c");

        assert_eq!(child.path(), &["a", "b", "c"]);
    }

    #[test]
    fn symbol_ref_extend() {
        let hash = test_hash();
        let base = SymbolRef::new(vec!["a".into()], hash);
        let extended = base.extend(&["b", "c"]);

        assert_eq!(extended.path(), &["a", "b", "c"]);
    }

    #[test]
    fn symbol_ref_is_root_level() {
        let hash = test_hash();
        let root = SymbolRef::new(vec!["a".into()], hash);
        let nested = SymbolRef::new(vec!["a".into(), "b".into()], hash);

        assert!(root.is_root_level());
        assert!(!nested.is_root_level());
    }

    #[test]
    fn symbol_ref_namespace() {
        let hash = test_hash();
        let sym = SymbolRef::new(vec!["crate".into(), "module".into()], hash);

        assert_eq!(sym.namespace(), Some("crate"));
    }

    #[test]
    fn symbol_ref_with_parent_hash() {
        let hash1 = ContentHash::compute(b"artifact1");
        let hash2 = ContentHash::compute(b"artifact2");
        let sym1 = SymbolRef::new(vec!["a".into(), "b".into()], hash1);
        let sym2 = sym1.with_parent_hash(hash2);

        assert_eq!(sym1.path(), sym2.path());
        assert_eq!(sym2.parent_hash(), &hash2);
    }

    #[test]
    fn symbol_ref_to_trie_key() {
        let hash = test_hash();
        let sym = SymbolRef::new(vec!["a".into(), "b".into(), "c".into()], hash);

        assert_eq!(sym.to_trie_key(), "a/b/c");
    }

    #[test]
    fn symbol_ref_display() {
        let hash = test_hash();
        let sym = SymbolRef::new(vec!["crate".into(), "func".into()], hash);
        let s = sym.to_string();

        assert!(s.contains("crate.func"));
        assert!(s.contains('@'));
    }

    #[test]
    fn symbol_ref_with_revision_display() {
        let hash = test_hash();
        let commit = ContentHash::compute(b"commit");
        let revision = Revision::new("feature", commit);
        let sym = SymbolRef::with_revision(vec!["a".into()], hash, revision);

        let s = sym.to_string();
        assert!(s.contains('#'));
        assert!(s.contains("feature"));
    }

    #[test]
    fn revision_new() {
        let commit = ContentHash::compute(b"commit");
        let rev = Revision::new("main", commit);

        assert_eq!(rev.branch(), "main");
        assert_eq!(rev.commit(), &commit);
    }
}
