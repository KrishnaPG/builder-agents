//! Artifact Type Implementations
//!
//! Provides concrete artifact types for common use cases:
//! - Binary: Raw byte content
//! - Code: Parsed AST with symbol table
//! - Config: Schema-validated configuration
//! - Spec: Structured specification documents

pub mod binary;
pub mod code;
pub mod config;
pub mod spec;

// Re-export common types
pub use binary::{BinaryArtifact, BinaryContent};
pub use code::{CodeArtifact, CodeContent, Language};
pub use config::{ConfigArtifact, ConfigContent};
pub use spec::{SpecArtifact, SpecContent};
