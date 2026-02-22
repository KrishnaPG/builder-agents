//! Binary Artifact Type
//!
//! Simplest artifact type - raw byte content.
//! Used for files that don't need structured parsing.

use crate::artifact_type::{ArtifactContent, ArtifactType};
use crate::hash::ContentHash;

/// Binary artifact marker type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryArtifact;

impl ArtifactType for BinaryArtifact {
    type Content = BinaryContent;

    #[inline]
    fn hash(content: &Self::Content) -> ContentHash {
        ContentHash::compute(&content.0)
    }

    const TYPE_ID: &'static str = "binary";
}

/// Binary content - raw bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryContent(Vec<u8>);

impl BinaryContent {
    /// Create from byte vector
    #[inline]
    #[must_use]
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    /// Create from string
    #[inline]
    #[must_use]
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into().into_bytes())
    }

    /// Get reference to bytes
    #[inline]
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.0
    }

    /// Get mutable reference to bytes
    #[inline]
    #[must_use]
    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    /// Convert to bytes (consumes self)
    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Try convert to string
    ///
    /// # Errors
    /// Returns error if bytes are not valid UTF-8
    #[inline]
    pub fn to_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.0.clone())
    }

    /// Check if content is valid UTF-8
    #[inline]
    #[must_use]
    pub fn is_utf8(&self) -> bool {
        std::str::from_utf8(&self.0).is_ok()
    }

    /// Get content length
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl ArtifactContent for BinaryContent {
    #[inline]
    fn approximate_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.0.capacity()
    }
}

impl Default for BinaryContent {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl From<Vec<u8>> for BinaryContent {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl From<&[u8]> for BinaryContent {
    fn from(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl From<String> for BinaryContent {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl From<&str> for BinaryContent {
    fn from(s: &str) -> Self {
        Self::from_string(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_content_new() {
        let content = BinaryContent::new(vec![1, 2, 3]);
        assert_eq!(content.data(), &[1, 2, 3]);
    }

    #[test]
    fn binary_content_from_string() {
        let content = BinaryContent::from_string("hello");
        assert_eq!(content.data(), b"hello");
        assert!(content.is_utf8());
    }

    #[test]
    fn binary_content_size() {
        let content = BinaryContent::new(vec![0u8; 100]);
        assert!(content.approximate_size() >= 100);
    }

    #[test]
    fn binary_content_empty() {
        let content = BinaryContent::default();
        assert!(content.is_empty());
        assert_eq!(content.len(), 0);
    }

    #[test]
    fn binary_artifact_hash() {
        let content = BinaryContent::new(b"test".to_vec());
        let hash = BinaryArtifact::hash(&content);
        assert!(!hash.is_null());

        // Same content -> same hash
        let content2 = BinaryContent::new(b"test".to_vec());
        let hash2 = BinaryArtifact::hash(&content2);
        assert_eq!(hash, hash2);

        // Different content -> different hash
        let content3 = BinaryContent::new(b"other".to_vec());
        let hash3 = BinaryArtifact::hash(&content3);
        assert_ne!(hash, hash3);
    }
}
