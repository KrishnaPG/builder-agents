//! COA Symbol System
//!
//! Content-addressed symbolic references with radix tree index.
//!
//! # Overview
//!
//! The symbol system provides:
//! - **SymbolRef**: Content-addressed symbolic references
//! - **SymbolRefIndex**: O(log n) lookup via radix tree
//! - **SingleWriterValidator**: Conflict detection for composition
//!
//! # Example
//!
//! ```rust
//! use coa_symbol::{SymbolRef, SymbolRefIndex, SymbolMetadata};
//! use coa_artifact::ContentHash;
//!
//! // Create index
//! let index = SymbolRefIndex::new();
//!
//! // Insert symbol
//! let symbol = SymbolRef::from_path("crate.module.func", ContentHash::new([1u8; 32]));
//! index.insert(symbol.clone(), SymbolMetadata::new()).unwrap();
//!
//! // Lookup
//! let found = index.get_exact(&symbol);
//! assert!(found.is_some());
//! ```

#![warn(missing_docs)]

pub mod symbol;
pub mod index;
pub mod validation;

// Re-exports
pub use symbol::{IndexEntry, Revision, SourceLocation, SymbolMetadata, SymbolRef};
pub use index::{IndexError, SymbolRefIndex};
pub use validation::{ConflictDiagnostic, ConflictKind, ResolutionSuggestion, SingleWriterValidator, ValidationError};

/// Prelude module for common imports
pub mod prelude {
    //! Common imports for symbol operations
    pub use crate::{
        ConflictDiagnostic, ConflictKind, IndexEntry, ResolutionSuggestion, SingleWriterValidator,
        SymbolMetadata, SymbolRef, SymbolRefIndex, ValidationError,
    };
}

/// Version of this crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
