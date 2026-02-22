//! Strategy registry for composition strategies
//!
//! Provides [`StrategyRegistry`] for managing and selecting composition strategies.

use std::collections::HashSet;

/// Registry of available composition strategy names
///
/// This is a lightweight registry that maps names to strategy types.
/// Since strategies are type-parameterized, they're used directly, not as trait objects.
#[derive(Debug, Default, Clone)]
pub struct StrategyRegistry {
    strategies: HashSet<String>,
}

impl StrategyRegistry {
    /// Create new empty registry
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            strategies: HashSet::new(),
        }
    }

    /// Create registry with built-in strategies
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register("single_writer");
        registry.register("ordered");
        registry.register("commutative");
        registry.register("hybrid");
        registry
    }

    /// Register a strategy name
    pub fn register(&mut self, name: &str) {
        self.strategies.insert(name.to_string());
    }

    /// Check if strategy exists
    #[inline]
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.strategies.contains(name)
    }

    /// Remove strategy
    #[inline]
    pub fn remove(&mut self, name: &str) -> bool {
        self.strategies.remove(name)
    }

    /// List all registered strategy names
    #[inline]
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.strategies.iter().map(|s| s.as_str()).collect()
    }

    /// Get number of registered strategies
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    /// Check if registry is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }

    /// Auto-select strategy name based on context
    ///
    /// # Selection Logic
    /// - `code` → `single_writer` (safety-critical)
    /// - `svg`/`image` with `add_layer` → `commutative`
    /// - `audio`/`video` with `add_track` → `commutative`
    /// - `mesh` with `refine` → `ordered`
    /// - default → `single_writer`
    #[must_use]
    pub fn select_name(&self, artifact_type: &str, operation: &str) -> &'static str {
        match (artifact_type, operation) {
            ("code", _) => "single_writer",
            ("svg" | "image", "add_layer" | "remove_layer") => "commutative",
            ("audio" | "video", "add_track" | "remove_track") => "commutative",
            ("mesh", "refine" | "subdivide") => "ordered",
            ("config" | "spec", _) => "hybrid",
            _ => "single_writer",
        }
    }

    /// Iterate over all strategy names
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.strategies.iter()
    }
}

/// Strategy selection hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrategyHint {
    /// Prioritize safety (use SingleWriter)
    Safety,

    /// Prioritize parallelism (use Commutative)
    Parallelism,

    /// Balance (use Hybrid) - default
    #[default]
    Balanced,

    /// Explicit ordering required
    Ordered,
}

/// Builder for strategy selection
#[derive(Debug, Default)]
pub struct StrategySelector {
    hint: StrategyHint,
}

impl StrategySelector {
    /// Create new selector
    #[must_use]
    pub fn new() -> Self {
        Self {
            hint: StrategyHint::Balanced,
        }
    }

    /// Set selection hint
    #[inline]
    #[must_use]
    pub fn with_hint(mut self, hint: StrategyHint) -> Self {
        self.hint = hint;
        self
    }

    /// Select strategy name based on hint
    #[must_use]
    pub fn select_name(&self, _artifact_type: &str, _operation: &str) -> &'static str {
        match self.hint {
            StrategyHint::Safety => "single_writer",
            StrategyHint::Parallelism => "commutative",
            StrategyHint::Ordered => "ordered",
            StrategyHint::Balanced => "hybrid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_new_empty() {
        let registry = StrategyRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn registry_with_defaults() {
        let registry = StrategyRegistry::with_defaults();
        assert_eq!(registry.len(), 4);
        assert!(registry.contains("single_writer"));
        assert!(registry.contains("ordered"));
        assert!(registry.contains("commutative"));
        assert!(registry.contains("hybrid"));
    }

    #[test]
    fn registry_register() {
        let mut registry = StrategyRegistry::new();
        registry.register("custom");
        assert!(registry.contains("custom"));
    }

    #[test]
    fn registry_remove() {
        let mut registry = StrategyRegistry::with_defaults();
        let removed = registry.remove("commutative");
        assert!(removed);
        assert!(!registry.contains("commutative"));
    }

    #[test]
    fn registry_names() {
        let registry = StrategyRegistry::with_defaults();
        let names = registry.names();

        assert!(names.contains(&"single_writer"));
        assert!(names.contains(&"hybrid"));
    }

    #[test]
    fn registry_select_name_code() {
        let registry = StrategyRegistry::with_defaults();
        let name = registry.select_name("code", "modify");
        assert_eq!(name, "single_writer");
    }

    #[test]
    fn registry_select_name_svg_layer() {
        let registry = StrategyRegistry::with_defaults();
        let name = registry.select_name("svg", "add_layer");
        assert_eq!(name, "commutative");
    }

    #[test]
    fn registry_select_name_mesh_refine() {
        let registry = StrategyRegistry::with_defaults();
        let name = registry.select_name("mesh", "refine");
        assert_eq!(name, "ordered");
    }

    #[test]
    fn registry_select_name_default() {
        let registry = StrategyRegistry::with_defaults();
        let name = registry.select_name("unknown", "unknown");
        assert_eq!(name, "single_writer");
    }

    #[test]
    fn selector_new() {
        let selector = StrategySelector::new();
        let name = selector.select_name("code", "modify");
        assert_eq!(name, "hybrid"); // Default hint is Balanced
    }

    #[test]
    fn selector_with_hint_safety() {
        let selector = StrategySelector::new().with_hint(StrategyHint::Safety);
        let name = selector.select_name("image", "add_layer");
        assert_eq!(name, "single_writer");
    }

    #[test]
    fn selector_with_hint_parallelism() {
        let selector = StrategySelector::new().with_hint(StrategyHint::Parallelism);
        let name = selector.select_name("code", "modify");
        assert_eq!(name, "commutative");
    }

    #[test]
    fn strategy_hint_variants() {
        assert!(StrategyHint::Safety != StrategyHint::Parallelism);
        assert!(StrategyHint::Balanced != StrategyHint::Ordered);
    }
}
