//! Constitutional Layer - Main entry point
//!
//! Provides the trusted boundary for:
//! - File → Artifact parsing (ingress)
//! - Delta application (transformation)
//! - Artifact → File serialization (egress)

use crate::cache::ArtifactCache;
use crate::error::{ApplyError, ParseError, SerializeError};
use crate::parsers::ParserRegistry;
use coa_artifact::{Artifact, ArtifactType, ContentHash, StructuralDelta};
use coa_composition::CompositionStrategy;
use coa_symbol::SymbolRefIndex;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Result of parsing a file
#[derive(Debug, Clone)]
pub struct ParseResult<T: ArtifactType> {
    /// The parsed artifact
    pub artifact: Artifact<T>,
    /// Source file metadata
    pub metadata: SourceMetadata,
}

/// Source file metadata
#[derive(Debug, Clone)]
pub struct SourceMetadata {
    /// File path
    pub path: PathBuf,
    /// Last modified time
    pub modified: SystemTime,
    /// Content checksum
    pub checksum: ContentHash,
}

/// Constitutional Layer - Trusted transformation boundary
///
/// This is the only component that interacts with the external filesystem.
/// All agent operations go through this layer.
///
/// # Security
/// - Agents never touch files directly
/// - All parsing/serialization is centralized
/// - Content-addressed caching prevents duplication
#[derive(Debug, Clone)]
pub struct ConstitutionalLayer {
    /// Registered parsers by file extension
    parsers: ParserRegistry,
    /// Content-addressed cache
    cache: ArtifactCache,
    /// Maximum file size to parse (bytes)
    max_file_size: usize,
}

impl ConstitutionalLayer {
    /// Create new layer with default parsers
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    /// Create layer with specific cache capacity
    #[inline]
    #[must_use]
    pub fn with_capacity(cache_capacity: u64) -> Self {
        Self {
            parsers: crate::parsers::default_parsers(),
            cache: ArtifactCache::new(cache_capacity),
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }

    /// Parse file into typed artifact (Ingress)
    ///
    /// # Type Parameters
    /// * `T` - Expected artifact type
    ///
    /// # Arguments
    /// * `path` - File path to parse
    ///
    /// # Returns
    /// Parsed artifact with metadata
    ///
    /// # Errors
    /// - `ParseError::NoParserForExtension` if no parser registered
    /// - `ParseError::SyntaxError` if file has invalid syntax
    /// - `ParseError::Io` if file read fails
    pub async fn parse_ingress<T: ArtifactType>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ParseResult<T>, ParseError> {
        let path = path.as_ref();

        // Read file
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ParseError::io_error(path, e))?;

        // Check file size
        if content.len() > self.max_file_size {
            return Err(ParseError::ValidationError(format!(
                "file too large: {} bytes (max: {})",
                content.len(),
                self.max_file_size
            )));
        }

        // Compute checksum
        let checksum = ContentHash::compute(content.as_bytes());

        // Check cache
        if let Some(cached) = self.cache.get::<T>(&checksum).await {
            let metadata = SourceMetadata {
                path: path.to_path_buf(),
                modified: std::fs::metadata(path)
                    .and_then(|m| m.modified())
                    .unwrap_or_else(|_| SystemTime::now()),
                checksum,
            };
            return Ok(ParseResult {
                artifact: cached,
                metadata,
            });
        }

        // Find parser
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let _parser = self
            .parsers
            .find_for_path(path)
            .ok_or_else(|| ParseError::NoParserForExtension(extension.to_string()))?;

        // Parse (Note: This is simplified - actual implementation would need
        // type-erased parsers or per-type registration)
        // For now, return error indicating we need proper implementation
        Err(ParseError::ParserError(
            "type-specific parsing not yet implemented - use parser directly".to_string(),
        ))
    }

    /// Apply single delta to artifact
    ///
    /// # Arguments
    /// * `artifact` - Base artifact
    /// * `delta` - Delta to apply
    ///
    /// # Returns
    /// New artifact with delta applied
    ///
    /// # Errors
    /// - `ApplyError::InvalidBase` if delta.base_hash doesn't match
    /// - `ApplyError::TargetNotFound` if delta target doesn't exist
    /// - `ApplyError::ValidationFailed` if operation invalid
    pub fn apply_delta<T: ArtifactType>(
        &self,
        artifact: &Artifact<T>,
        delta: &StructuralDelta<T>,
    ) -> Result<Artifact<T>, ApplyError> {
        // Verify base hash
        delta
            .validate_base(artifact)
            .map_err(ApplyError::DeltaError)?;

        // The actual transformation would require a transformer registry
        // similar to parsers. For now, this is a placeholder.
        Err(ApplyError::NoTransformer(T::TYPE_ID.to_string()))
    }

    /// Apply multiple deltas with composition strategy
    ///
    /// # Type Parameters
    /// * `T` - Artifact type
    /// * `S` - Composition strategy type
    ///
    /// # Arguments
    /// * `base` - Base artifact
    /// * `deltas` - Deltas to apply
    /// * `strategy` - Composition strategy
    /// * `index` - Symbol index for validation
    ///
    /// # Returns
    /// New artifact with all deltas composed and applied
    pub fn apply_deltas<T, S>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
        strategy: &S,
        index: &SymbolRefIndex,
    ) -> Result<Artifact<T>, ApplyError>
    where
        T: ArtifactType,
        S: CompositionStrategy,
    {
        // Validate composition
        strategy
            .validate(deltas, index)
            .map_err(ApplyError::CompositionFailed)?;

        // Compose
        strategy
            .compose(base, deltas)
            .map_err(ApplyError::CompositionFailed)
    }

    /// Serialize artifact to file (Egress)
    ///
    /// # Arguments
    /// * `artifact` - Artifact to serialize
    /// * `path` - Output file path
    ///
    /// # Errors
    /// - `SerializeError::NoSerializer` if type not supported
    /// - `SerializeError::Io` if file write fails
    pub async fn serialize_egress<T: ArtifactType>(
        &self,
        _artifact: &Artifact<T>,
        _path: impl AsRef<Path>,
    ) -> Result<(), SerializeError> {
        // Serializers would be registered similar to parsers
        // For now, placeholder implementation
        Err(SerializeError::NoSerializer(
            "serialization not yet implemented".to_string(),
        ))
    }

    /// Get cache reference
    #[inline]
    #[must_use]
    pub fn cache(&self) -> &ArtifactCache {
        &self.cache
    }

    /// Get mutable cache reference
    #[inline]
    pub fn cache_mut(&mut self) -> &mut ArtifactCache {
        &mut self.cache
    }
}

impl Default for ConstitutionalLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_creation() {
        let layer = ConstitutionalLayer::new();
        assert_eq!(layer.max_file_size, 10 * 1024 * 1024);
    }

    #[test]
    fn layer_with_capacity() {
        let layer = ConstitutionalLayer::with_capacity(1000);
        // Cache is created with specified capacity
        let _ = layer.cache();
    }

    #[test]
    fn layer_default() {
        let layer: ConstitutionalLayer = Default::default();
        let _ = layer.cache();
    }
}
