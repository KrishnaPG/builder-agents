//! Error types for the Constitutional Layer
//!
//! Provides comprehensive error handling for:
//! - Parse operations (file → Artifact)
//! - Apply operations (delta transformation)
//! - Serialize operations (Artifact → file)

use coa_artifact::{ArtifactError, DeltaError, SymbolPath};
use coa_composition::CompositionError;
use std::path::PathBuf;

/// Errors during file parsing (ingress)
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// No parser registered for file extension
    #[error("no parser registered for extension: '{0}'")]
    NoParserForExtension(String),

    /// Syntax error in source file
    #[error("syntax error in {path}: {message}")]
    SyntaxError { path: PathBuf, message: String },

    /// IO error during file read
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Invalid artifact type for parser
    #[error("invalid artifact type: expected {expected}, got {actual}")]
    InvalidType { expected: String, actual: String },

    /// Parser-specific error
    #[error("parser error: {0}")]
    ParserError(String),

    /// Content validation failed
    #[error("content validation failed: {0}")]
    ValidationError(String),
}

impl ParseError {
    /// Create syntax error for path
    pub fn syntax_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::SyntaxError {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create IO error for path
    pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

/// Errors during delta application
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    /// Base hash mismatch (optimistic concurrency failure)
    #[error("base hash mismatch: expected {expected}, got {actual}")]
    InvalidBase {
        expected: String,
        actual: String,
    },

    /// Target symbol not found in artifact
    #[error("target not found: {0}")]
    TargetNotFound(SymbolPath),

    /// Target already exists (for Add operations)
    #[error("target already exists: {0}")]
    TargetAlreadyExists(SymbolPath),

    /// Validation failed before application
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// Composition strategy validation failed
    #[error("composition failed: {0}")]
    CompositionFailed(#[from] CompositionError),

    /// No transformer registered for artifact type
    #[error("no transformer for artifact type: {0}")]
    NoTransformer(String),

    /// Transformation operation failed
    #[error("transformation failed: {0}")]
    TransformFailed(String),

    /// Delta application error from artifact system
    #[error("delta error: {0}")]
    DeltaError(#[from] DeltaError),

    /// Artifact system error
    #[error("artifact error: {0}")]
    ArtifactError(#[from] ArtifactError),
}

impl ApplyError {
    /// Create invalid base error
    pub fn invalid_base(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::InvalidBase {
            expected: expected.into(),
            actual: actual.into(),
        }
    }
}

/// Errors during artifact serialization (egress)
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    /// No serializer registered for artifact type
    #[error("no serializer for artifact type: {0}")]
    NoSerializer(String),

    /// Serialization logic failed
    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    /// IO error during file write
    #[error("io error writing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Format-specific error
    #[error("format error: {0}")]
    FormatError(String),
}

impl SerializeError {
    /// Create IO error for path
    pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

/// Errors during cache operations
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Type mismatch in cache retrieval
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// Cache capacity exceeded
    #[error("cache capacity exceeded")]
    CapacityExceeded,

    /// Internal cache error
    #[error("cache error: {0}")]
    Internal(String),
}

/// Combined constitutional layer error
#[derive(Debug, thiserror::Error)]
pub enum ConstitutionalError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("apply error: {0}")]
    Apply(#[from] ApplyError),

    #[error("serialize error: {0}")]
    Serialize(#[from] SerializeError),

    #[error("cache error: {0}")]
    Cache(#[from] CacheError),
}

/// Result type alias for constitutional operations
pub type ConstitutionalResult<T> = Result<T, ConstitutionalError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let err = ParseError::NoParserForExtension("rs".to_string());
        assert_eq!(err.to_string(), "no parser registered for extension: 'rs'");
    }

    #[test]
    fn apply_error_display() {
        let err = ApplyError::invalid_base("abc123", "def456");
        assert!(err.to_string().contains("base hash mismatch"));
    }

    #[test]
    fn serialize_error_display() {
        let err = SerializeError::NoSerializer("custom".to_string());
        assert_eq!(err.to_string(), "no serializer for artifact type: custom");
    }

    #[test]
    fn error_conversions() {
        let parse_err = ParseError::NoParserForExtension("rs".to_string());
        let constitutional_err: ConstitutionalError = parse_err.into();
        assert!(matches!(constitutional_err, ConstitutionalError::Parse(_)));
    }
}
