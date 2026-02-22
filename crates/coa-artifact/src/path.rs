//! Symbol paths for addressing within artifacts
//!
//! Provides [`SymbolPath`] for hierarchical addressing of elements within artifacts.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Path within an artifact tree
///
/// Used to address specific elements for delta operations.
/// Hierarchical structure using string segments.
///
/// # Examples
/// - `["crate", "module", "function"]` → `crate.module.function`
/// - `["config", "database", "host"]` → `config.database.host`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolPath(Vec<String>);

impl SymbolPath {
    /// Create new path from segments
    #[inline]
    #[must_use]
    pub fn new(segments: Vec<String>) -> Self {
        Self(segments)
    }

    /// Create path from a single segment
    #[inline]
    #[must_use]
    pub fn single(segment: impl Into<String>) -> Self {
        Self(vec![segment.into()])
    }

    /// Empty path (root)
    #[inline]
    #[must_use]
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Get path segments
    #[inline]
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    /// Get number of segments
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if path is empty (root)
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get parent path (if not root)
    #[inline]
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.0.is_empty() {
            None
        } else {
            Some(Self(self.0[..self.0.len() - 1].to_vec()))
        }
    }

    /// Get last segment (if not root)
    #[inline]
    #[must_use]
    pub fn last(&self) -> Option<&str> {
        self.0.last().map(|s| s.as_str())
    }

    /// Get first segment (if not root)
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&str> {
        self.0.first().map(|s| s.as_str())
    }

    /// Append a segment, returning new path
    #[inline]
    #[must_use]
    pub fn child(&self, segment: impl Into<String>) -> Self {
        let mut new = self.clone();
        new.0.push(segment.into());
        new
    }

    /// Extend with multiple segments
    #[inline]
    #[must_use]
    pub fn extend(&self, segments: &[impl AsRef<str>]) -> Self {
        let mut new = self.clone();
        for seg in segments {
            new.0.push(seg.as_ref().to_string());
        }
        new
    }

    /// Check if this path is a prefix of another
    ///
    /// # Examples
    /// - `crate.module` is prefix of `crate.module.function`
    /// - `crate.module` is NOT prefix of `crate.other`
    #[inline]
    #[must_use]
    pub fn is_prefix_of(&self, other: &Self) -> bool {
        if self.0.len() > other.0.len() {
            return false;
        }
        self.0 == other.0[..self.0.len()]
    }

    /// Check if this path is an ancestor of another (strict prefix)
    ///
    /// Same as `is_prefix_of` but requires paths to be different.
    #[inline]
    #[must_use]
    pub fn is_ancestor_of(&self, other: &Self) -> bool {
        self.0.len() < other.0.len() && self.is_prefix_of(other)
    }

    /// Check if paths overlap (one is prefix of other)
    #[inline]
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.is_prefix_of(other) || other.is_prefix_of(self)
    }

    /// Get common prefix of two paths
    #[inline]
    #[must_use]
    pub fn common_prefix(&self, other: &Self) -> Self {
        let common: Vec<_> = self
            .0
            .iter()
            .zip(&other.0)
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a.clone())
            .collect();
        Self(common)
    }

    /// Get relative path from ancestor
    ///
    /// # Errors
    /// Returns error if `self` is not a descendant of `ancestor`
    pub fn relative_to(&self, ancestor: &Self) -> Result<Self, PathError> {
        if !ancestor.is_prefix_of(self) {
            return Err(PathError::NotDescendant {
                path: self.to_string(),
                ancestor: ancestor.to_string(),
            });
        }
        Ok(Self(self.0[ancestor.0.len()..].to_vec()))
    }

    /// Iterator over segments from root to leaf
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|s| s.as_str())
    }

    /// Join segments with custom separator
    #[inline]
    #[must_use]
    pub fn join(&self, separator: &str) -> String {
        self.0.join(separator)
    }
}

impl Display for SymbolPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("."))
    }
}

impl FromStr for SymbolPath {
    type Err = PathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::root());
        }

        let segments: Vec<String> = s
            .split('.')
            .map(|seg| {
                // Validate segment
                if seg.is_empty() {
                    Err(PathError::EmptySegment)
                } else if seg.contains(|c: char| !c.is_alphanumeric() && c != '_') {
                    Err(PathError::InvalidSegment(seg.to_string()))
                } else {
                    Ok(seg.to_string())
                }
            })
            .collect::<Result<_, _>>()?;

        Ok(Self(segments))
    }
}

impl From<Vec<String>> for SymbolPath {
    fn from(segments: Vec<String>) -> Self {
        Self(segments)
    }
}

impl From<&[String]> for SymbolPath {
    fn from(segments: &[String]) -> Self {
        Self(segments.to_vec())
    }
}

impl Default for SymbolPath {
    fn default() -> Self {
        Self::root()
    }
}

/// Errors related to symbol paths
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    /// Empty segment in path
    #[error("path contains empty segment")]
    EmptySegment,

    /// Invalid segment characters
    #[error("invalid segment: {0} (must be alphanumeric or underscore)")]
    InvalidSegment(String),

    /// Not a descendant path
    #[error("path '{path}' is not a descendant of '{ancestor}'")]
    NotDescendant { path: String, ancestor: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_new_and_segments() {
        let path = SymbolPath::new(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(path.segments(), &["a", "b"]);
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn path_single() {
        let path = SymbolPath::single("only");
        assert_eq!(path.segments(), &["only"]);
    }

    #[test]
    fn path_root() {
        let path = SymbolPath::root();
        assert!(path.is_empty());
        assert_eq!(path.len(), 0);
    }

    #[test]
    fn path_parent() {
        let path = SymbolPath::new(vec!["a".into(), "b".into(), "c".into()]);
        let parent = path.parent().unwrap();
        assert_eq!(parent.segments(), &["a", "b"]);
    }

    #[test]
    fn path_root_parent_is_none() {
        let path = SymbolPath::root();
        assert!(path.parent().is_none());
    }

    #[test]
    fn path_last_and_first() {
        let path = SymbolPath::new(vec!["first".into(), "middle".into(), "last".into()]);
        assert_eq!(path.first(), Some("first"));
        assert_eq!(path.last(), Some("last"));
    }

    #[test]
    fn path_child() {
        let parent = SymbolPath::new(vec!["parent".into()]);
        let child = parent.child("child");
        assert_eq!(child.segments(), &["parent", "child"]);
    }

    #[test]
    fn path_extend() {
        let base = SymbolPath::new(vec!["base".into()]);
        let extended = base.extend(&["a", "b"]);
        assert_eq!(extended.segments(), &["base", "a", "b"]);
    }

    #[test]
    fn path_is_prefix_of() {
        let a = SymbolPath::from_str("a.b").unwrap();
        let b = SymbolPath::from_str("a.b.c").unwrap();
        assert!(a.is_prefix_of(&b));
        assert!(!b.is_prefix_of(&a));
    }

    #[test]
    fn path_is_ancestor_of() {
        let parent = SymbolPath::from_str("a").unwrap();
        let child = SymbolPath::from_str("a.b").unwrap();
        assert!(parent.is_ancestor_of(&child));
        assert!(!child.is_ancestor_of(&parent));

        // Same path is not ancestor
        let same = SymbolPath::from_str("a").unwrap();
        assert!(!parent.is_ancestor_of(&same));
    }

    #[test]
    fn path_overlaps() {
        let a = SymbolPath::from_str("a.b").unwrap();
        let b = SymbolPath::from_str("a.b.c").unwrap();
        let c = SymbolPath::from_str("a.x").unwrap();

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn path_common_prefix() {
        let a = SymbolPath::from_str("a.b.c").unwrap();
        let b = SymbolPath::from_str("a.b.d").unwrap();
        let common = a.common_prefix(&b);
        assert_eq!(common.segments(), &["a", "b"]);
    }

    #[test]
    fn path_relative_to() {
        let full = SymbolPath::from_str("a.b.c.d").unwrap();
        let ancestor = SymbolPath::from_str("a.b").unwrap();
        let relative = full.relative_to(&ancestor).unwrap();
        assert_eq!(relative.segments(), &["c", "d"]);
    }

    #[test]
    fn path_relative_to_fails() {
        let path = SymbolPath::from_str("a.b").unwrap();
        let not_ancestor = SymbolPath::from_str("x.y").unwrap();
        let result = path.relative_to(&not_ancestor);
        assert!(matches!(result, Err(PathError::NotDescendant { .. })));
    }

    #[test]
    fn path_display() {
        let path = SymbolPath::new(vec!["a".into(), "b".into()]);
        assert_eq!(path.to_string(), "a.b");
    }

    #[test]
    fn path_from_str_valid() {
        let path: SymbolPath = "a.b.c".parse().unwrap();
        assert_eq!(path.segments(), &["a", "b", "c"]);
    }

    #[test]
    fn path_from_str_empty() {
        let path: SymbolPath = "".parse().unwrap();
        assert!(path.is_empty());
    }

    #[test]
    fn path_from_str_empty_segment() {
        let result: Result<SymbolPath, _> = "a..b".parse();
        assert!(matches!(result, Err(PathError::EmptySegment)));
    }

    #[test]
    fn path_from_str_invalid_chars() {
        let result: Result<SymbolPath, _> = "a.b-c".parse();
        assert!(matches!(result, Err(PathError::InvalidSegment(_))));
    }

    #[test]
    fn path_iter() {
        let path = SymbolPath::new(vec!["a".into(), "b".into()]);
        let collected: Vec<_> = path.iter().collect();
        assert_eq!(collected, vec!["a", "b"]);
    }

    #[test]
    fn path_join() {
        let path = SymbolPath::new(vec!["a".into(), "b".into()]);
        assert_eq!(path.join("/"), "a/b");
        assert_eq!(path.join("::"), "a::b");
    }
}
