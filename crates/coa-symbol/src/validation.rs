//! Validation for single-writer and composition constraints
//!
//! Provides validators for ensuring non-overlapping symbol claims
//! and detecting conflicts in delta composition.

use crate::index::{IndexEntry, SymbolRefIndex};
use crate::symbol::SymbolRef;
use coa_artifact::{ArtifactType, StructuralDelta, SymbolPath};

/// Single-writer invariant validation
///
/// Ensures no overlapping SymbolRef claims across agents.
/// This is the foundation of safe parallel composition.
#[derive(Debug, Clone, Copy)]
pub struct SingleWriterValidator;

impl SingleWriterValidator {
    /// Create new validator instance
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validate that deltas have disjoint target symbols
    ///
    /// # Returns
    /// - `Ok(())` if no overlaps
    /// - `Err(ValidationError::OverlappingClaims)` if conflict
    ///
    /// # Performance
    /// O(n²) in worst case where n = number of deltas
    /// Can be optimized to O(n log n) with interval tree
    pub fn validate_deltas<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        _index: &SymbolRefIndex,
    ) -> Result<(), ValidationError> {
        if deltas.len() <= 1 {
            return Ok(());
        }

        // Build set of claimed paths with their indices for error reporting
        let claims: Vec<(usize, &SymbolPath)> = deltas
            .iter()
            .enumerate()
            .map(|(i, d)| (i, d.target()))
            .collect();

        // Check each pair for overlap
        for i in 0..claims.len() {
            for j in (i + 1)..claims.len() {
                let (idx_a, path_a) = claims[i];
                let (idx_b, path_b) = claims[j];

                if Self::paths_overlap(path_a, path_b) {
                    return Err(ValidationError::OverlappingClaims {
                        claim1: path_a.to_string(),
                        claim2: path_b.to_string(),
                        delta1_index: idx_a,
                        delta2_index: idx_b,
                        suggestion: ResolutionSuggestion::DecomposeTargets {
                            common_prefix: Self::find_common_prefix(path_a, path_b),
                        },
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate against existing index
    ///
    /// Checks if any delta targets would overlap with existing symbols.
    pub fn validate_against_index<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<(), ValidationError> {
        for (i, delta) in deltas.iter().enumerate() {
            let path: Vec<String> = delta.target().segments().to_vec();

            if index.has_any_overlap(&path) {
                let conflicts = index.find_conflicts(&path);
                return Err(ValidationError::ClaimOverlapsExisting {
                    new_claim: delta.target().to_string(),
                    existing: conflicts
                        .into_iter()
                        .map(|e| e.symbol.to_string())
                        .collect(),
                    delta_index: i,
                    suggestion: ResolutionSuggestion::UseDifferentTarget,
                });
            }
        }

        Ok(())
    }

    /// Check if two paths overlap (one is prefix of other)
    #[inline]
    #[must_use]
    pub fn paths_overlap(a: &SymbolPath, b: &SymbolPath) -> bool {
        a.overlaps(b)
    }

    /// Find common prefix of two paths
    #[inline]
    #[must_use]
    pub fn find_common_prefix(a: &SymbolPath, b: &SymbolPath) -> String {
        a.common_prefix(b).to_string()
    }

    /// Check if path is valid for writing (not empty, valid characters)
    pub fn validate_path_format(path: &SymbolPath) -> Result<(), ValidationError> {
        if path.is_empty() {
            return Err(ValidationError::InvalidPath {
                reason: "empty path".to_string(),
            });
        }

        for segment in path.segments() {
            if segment.is_empty() {
                return Err(ValidationError::InvalidPath {
                    reason: "empty segment".to_string(),
                });
            }
            if !segment
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
            {
                return Err(ValidationError::InvalidPath {
                    reason: format!("invalid characters in segment: {}", segment),
                });
            }
        }

        Ok(())
    }
}

impl Default for SingleWriterValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation errors with diagnostic information
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// Two deltas claim overlapping paths
    #[error("overlapping claims: '{claim1}' and '{claim2}'")]
    OverlappingClaims {
        claim1: String,
        claim2: String,
        delta1_index: usize,
        delta2_index: usize,
        suggestion: ResolutionSuggestion,
    },

    /// Delta claims path that overlaps existing symbol
    #[error("claim '{new_claim}' overlaps existing symbols: {existing:?}")]
    ClaimOverlapsExisting {
        new_claim: String,
        existing: Vec<String>,
        delta_index: usize,
        suggestion: ResolutionSuggestion,
    },

    /// Invalid path format
    #[error("invalid path: {reason}")]
    InvalidPath { reason: String },

    /// Validation infrastructure error
    #[error("validation error: {0}")]
    Internal(String),
}

/// Suggested resolution for validation failures
#[derive(Debug, Clone)]
pub enum ResolutionSuggestion {
    /// Decompose targets into non-overlapping paths
    DecomposeTargets { common_prefix: String },

    /// Use a different target
    UseDifferentTarget,

    /// Use sequential composition
    UseSequential,

    /// Merge agents
    MergeAgents,
}

/// Diagnostic information for validation failures
#[derive(Debug, Clone)]
pub struct ValidationDiagnostic {
    /// Kind of conflict
    pub kind: ConflictKind,

    /// Involved delta indices
    pub involved_deltas: Vec<usize>,

    /// Human-readable description
    pub description: String,

    /// Suggested resolution
    pub suggestion: ResolutionSuggestion,
}

/// Types of conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Two deltas claim same/overlapping paths
    OverlappingDeltaClaims,

    /// Delta claims path inside existing symbol
    InsideExistingSymbol,

    /// Delta claims path that contains existing symbol
    ContainsExistingSymbol,

    /// Invalid path format
    InvalidPathFormat,
}

/// Conflict analyzer for producing detailed diagnostics
pub struct ConflictAnalyzer;

impl ConflictAnalyzer {
    /// Analyze overlapping claims
    pub fn analyze_overlap<T: ArtifactType>(
        delta_a: &StructuralDelta<T>,
        delta_b: &StructuralDelta<T>,
    ) -> ValidationDiagnostic {
        let common_prefix = Self::find_common_prefix(delta_a.target(), delta_b.target());

        ValidationDiagnostic {
            kind: ConflictKind::OverlappingDeltaClaims,
            involved_deltas: vec![],
            description: format!(
                "Both deltas target paths under common prefix: {}",
                common_prefix
            ),
            suggestion: ResolutionSuggestion::DecomposeTargets { common_prefix },
        }
    }

    /// Find common prefix of two paths
    fn find_common_prefix(a: &SymbolPath, b: &SymbolPath) -> String {
        a.common_prefix(b).to_string()
    }

    /// Suggest decomposition strategy
    pub fn suggest_decomposition(
        conflicts: &[IndexEntry],
        target_path: &SymbolPath,
    ) -> Vec<String> {
        // Find available sub-paths that don't conflict
        let mut suggestions = Vec::new();

        for i in 0..10 {
            // Limit search
            let candidate = format!("{}_variant_{}", target_path, i);
            if !conflicts.iter().any(|c| c.symbol.to_string() == candidate) {
                suggestions.push(candidate);
                if suggestions.len() >= 3 {
                    break;
                }
            }
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coa_artifact::{ArtifactType, ContentHash};
    use std::str::FromStr;

    // Test artifact type for tests
    #[derive(Debug, Clone)]
    struct TestArtifact;

    #[derive(Debug, Clone, PartialEq)]
    struct TestContent;

    impl coa_artifact::__private::Sealed for TestArtifact {}

    impl ArtifactType for TestArtifact {
        type Content = TestContent;

        fn hash(_content: &Self::Content) -> ContentHash {
            ContentHash::compute(b"test")
        }

        const TYPE_ID: &'static str = "test";
    }

    fn make_delta(target: &str, base_hash: ContentHash) -> StructuralDelta<TestArtifact> {
        StructuralDelta::new(
            SymbolPath::from_str(target).unwrap(),
            coa_artifact::DeltaOperation::Remove,
            base_hash,
        )
    }

    fn test_hash() -> ContentHash {
        ContentHash::compute(b"base")
    }

    #[test]
    fn single_writer_validates_disjoint_paths() {
        let validator = SingleWriterValidator::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_delta("auth.login", test_hash()),
            make_delta("auth.register", test_hash()),
        ];

        let result = validator.validate_deltas(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn single_writer_rejects_overlapping() {
        let validator = SingleWriterValidator::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_delta("auth", test_hash()),
            make_delta("auth.login", test_hash()),
        ];

        let result = validator.validate_deltas(&deltas, &index);
        assert!(matches!(result, Err(ValidationError::OverlappingClaims { .. })));
    }

    #[test]
    fn single_writer_rejects_same_path() {
        let validator = SingleWriterValidator::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![
            make_delta("auth.login", test_hash()),
            make_delta("auth.login", test_hash()),
        ];

        let result = validator.validate_deltas(&deltas, &index);
        assert!(matches!(result, Err(ValidationError::OverlappingClaims { .. })));
    }

    #[test]
    fn single_writer_allows_single_delta() {
        let validator = SingleWriterValidator::new();
        let index = SymbolRefIndex::new();

        let deltas = vec![make_delta("auth.login", test_hash())];

        let result = validator.validate_deltas(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn single_writer_allows_empty() {
        let validator = SingleWriterValidator::new();
        let index = SymbolRefIndex::new();

        let deltas: Vec<StructuralDelta<TestArtifact>> = vec![];

        let result = validator.validate_deltas(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn paths_overlap_detection() {
        let a = SymbolPath::from_str("a.b").unwrap();
        let b = SymbolPath::from_str("a.b.c").unwrap();
        let c = SymbolPath::from_str("a.x").unwrap();

        assert!(SingleWriterValidator::paths_overlap(&a, &b));
        assert!(SingleWriterValidator::paths_overlap(&b, &a));
        assert!(!SingleWriterValidator::paths_overlap(&a, &c));
    }

    #[test]
    fn validate_against_index_detects_conflict() {
        let validator = SingleWriterValidator::new();
        let index = SymbolRefIndex::new();

        // Insert existing symbol
        let existing = SymbolRef::new(
            vec!["auth".to_string()],
            ContentHash::compute(b"existing"),
        );
        index.insert(existing, Default::default()).unwrap();

        // Try to validate delta targeting descendant
        let deltas = vec![make_delta("auth.login", test_hash())];

        let result = validator.validate_against_index(&deltas, &index);
        assert!(matches!(
            result,
            Err(ValidationError::ClaimOverlapsExisting { .. })
        ));
    }

    #[test]
    fn validate_path_format_valid() {
        let path = SymbolPath::from_str("valid.path.here").unwrap();
        assert!(SingleWriterValidator::validate_path_format(&path).is_ok());
    }

    #[test]
    fn validate_path_format_rejects_empty() {
        let path = SymbolPath::root();
        let result = SingleWriterValidator::validate_path_format(&path);
        assert!(matches!(result, Err(ValidationError::InvalidPath { .. })));
    }
}
