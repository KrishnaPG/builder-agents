//! Content-addressed hashing primitives
//!
//! Provides [`ContentHash`], a strongly-typed 32-byte hash used for
//! content addressing throughout the COA system.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// A 32-byte content hash (Blake3)
///
/// Used for content-addressed storage and artifact identification.
/// Immutable and cheap to clone (Copy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Create a new ContentHash from raw bytes
    #[inline]
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get reference to the underlying bytes
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to byte array (consumes self)
    #[inline]
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Create hash from byte slice
    ///
    /// # Errors
    /// Returns error if slice length is not exactly 32 bytes
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, HashError> {
        if bytes.len() != 32 {
            return Err(HashError::InvalidLength {
                expected: 32,
                actual: bytes.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }

    /// Compute Blake3 hash of arbitrary data
    #[inline]
    #[must_use]
    pub fn compute(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self::new(*hash.as_bytes())
    }

    /// Compute hash from serializable value (JSON encoding)
    ///
    /// # Errors
    /// Returns error if serialization fails
    #[inline]
    pub fn compute_serializable<T>(value: &T) -> Result<Self, HashError>
    where
        T: serde::Serialize,
    {
        let json = serde_json::to_vec(value)?;
        Ok(Self::compute(&json))
    }

    /// Short string representation (first 16 hex chars)
    #[inline]
    #[must_use]
    pub fn short(&self) -> String {
        hex::encode(&self.0[..8])
    }

    /// Check if hash is all zeros (placeholder/uninitialized)
    #[inline]
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        let mut i = 0;
        while i < 32 {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

impl Display for ContentHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for ContentHash {
    type Err = HashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        Self::from_slice(&bytes)
    }
}

impl AsRef<[u8; 32]> for ContentHash {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Default for ContentHash {
    fn default() -> Self {
        Self([0; 32])
    }
}

// Serde implementations for compact serialization
impl serde::Serialize for ContentHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> serde::Deserialize<'de> for ContentHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ContentHashVisitor;

        impl<'de> serde::de::Visitor<'de> for ContentHashVisitor {
            type Value = ContentHash;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str("a 32-byte hash as hex string or byte array")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(serde::de::Error::custom)
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                ContentHash::from_slice(value).map_err(serde::de::Error::custom)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 32];
                for (i, byte) in arr.iter_mut().enumerate() {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &"32 bytes"))?;
                }
                Ok(ContentHash::new(arr))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(ContentHashVisitor)
        } else {
            deserializer.deserialize_bytes(ContentHashVisitor)
        }
    }
}

/// Errors that can occur when working with content hashes
#[derive(Debug, thiserror::Error)]
pub enum HashError {
    /// Invalid hash length
    #[error("invalid hash length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },

    /// Hex encoding error
    #[error("hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_new_and_access() {
        let bytes = [1u8; 32];
        let hash = ContentHash::new(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
        assert_eq!(hash.into_bytes(), bytes);
    }

    #[test]
    fn content_hash_from_slice_valid() {
        let bytes = vec![2u8; 32];
        let hash = ContentHash::from_slice(&bytes).unwrap();
        assert_eq!(hash.as_bytes(), &[2u8; 32]);
    }

    #[test]
    fn content_hash_from_slice_invalid_length() {
        let bytes = vec![1u8; 31];
        let result = ContentHash::from_slice(&bytes);
        assert!(matches!(result, Err(HashError::InvalidLength { expected: 32, actual: 31 })));
    }

    #[test]
    fn content_hash_compute_deterministic() {
        let data = b"hello world";
        let h1 = ContentHash::compute(data);
        let h2 = ContentHash::compute(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_compute_different_data() {
        let h1 = ContentHash::compute(b"data1");
        let h2 = ContentHash::compute(b"data2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn content_hash_display_and_parse() {
        let hash = ContentHash::compute(b"test");
        let s = hash.to_string();
        let parsed: ContentHash = s.parse().unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn content_hash_short() {
        let hash = ContentHash::compute(b"test");
        let short = hash.short();
        assert_eq!(short.len(), 16); // 8 bytes = 16 hex chars
        assert!(hash.to_string().starts_with(&short));
    }

    #[test]
    fn content_hash_is_zero() {
        let zero = ContentHash::default();
        assert!(zero.is_zero());

        let non_zero = ContentHash::compute(b"test");
        assert!(!non_zero.is_zero());
    }

    #[test]
    fn content_hash_ordering() {
        let h1 = ContentHash::new([1u8; 32]);
        let h2 = ContentHash::new([2u8; 32]);
        assert!(h1 < h2);
    }

    #[test]
    fn content_hash_serde_json() {
        let hash = ContentHash::compute(b"test");
        let json = serde_json::to_string(&hash).unwrap();
        let decoded: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash, decoded);
    }

    #[test]
    fn content_hash_serde_human_readable() {
        let hash = ContentHash::compute(b"test");
        let json = serde_json::to_string(&hash).unwrap();
        // Should be hex string
        assert!(json.contains('"'));
        assert!(json.len() > 64); // " + 64 hex chars + "
    }
}
