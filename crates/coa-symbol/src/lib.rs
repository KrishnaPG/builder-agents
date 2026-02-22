//! COA Symbol System
//!
//! Content-addressed symbolic references with radix tree indexing.
//!
//! # Core Concepts
//!
//! - [`SymbolRef`]: Content-addressed reference to symbols within artifacts
//! - [`Revision`]: Branch + commit for versioned references
//! - [`SymbolRefIndex`]: O(log n) lookup using radix_trie
//! - [`SingleWriterValidator`]: Ensures non-overlapping delta claims
//!
//! # Example
//!
//! ```rust,ignore
//! use coa_symbol::{SymbolRef, SymbolRefIndex, SymbolMetadata};
//! use coa_artifact::ContentHash;
//!
//! // Create index
//! let index = SymbolRefIndex::new();
//!
//! // Create symbol reference
//! let hash = ContentHash::compute(b"artifact content");
//! let symbol = SymbolRef::new(
//!     vec!["crate".into(), "module".into(), "func".into()],
//!     hash
//! );
//!
//! // Index it
//! index.insert(symbol.clone(), SymbolMetadata::default()).unwrap();
//!
//! // Lookup
//! let found = index.get_exact(&symbol);
//! ```

#![warn(unreachable_pub)]
#![allow(missing_docs)]

// Core modules
mod index;
mod symbol;
mod validation;

// Re-exports
pub use index::{
    IndexEntry, SourceLocation, SymbolKind, SymbolMetadata, SymbolRefIndex, Visibility,
};
pub use symbol::{Revision, SymbolRef, SymbolRefError};
pub use validation::{
    ConflictAnalyzer, ConflictKind, ResolutionSuggestion, SingleWriterValidator, ValidationDiagnostic,
    ValidationError,
};

/// Version of this crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;
    use coa_artifact::ContentHash;

    #[test]
    fn symbol_index_lifecycle() {
        let index = SymbolRefIndex::new();
        let hash = ContentHash::compute(b"test artifact");

        // Create and insert symbols
        let sym1 = SymbolRef::new(vec!["auth".into(), "login".into()], hash);
        let sym2 = SymbolRef::new(vec!["auth".into(), "register".into()], hash);

        index.insert(sym1.clone(), SymbolMetadata::default()).unwrap();
        index.insert(sym2.clone(), SymbolMetadata::default()).unwrap();

        // Verify lookups
        assert!(index.contains(&sym1));
        assert!(index.contains(&sym2));

        // Test descendants
        let descendants = index.get_descendants(&["auth".to_string()]);
        assert_eq!(descendants.len(), 2);

        // Remove by parent
        let removed = index.remove_by_parent(&hash);
        assert_eq!(removed, 2);
        assert!(index.is_empty());
    }

    #[test]
    fn single_writer_with_index() {
        let validator = SingleWriterValidator::new();
        let index = SymbolRefIndex::new();
        let hash = ContentHash::compute(b"base");

        // Insert existing
        let existing = SymbolRef::new(vec!["api".into()], hash);
        index.insert(existing, SymbolMetadata::default()).unwrap();

        // Verify we can look it up
        assert!(index.contains(&SymbolRef::new(vec!["api".into()], hash)));
    }
}
