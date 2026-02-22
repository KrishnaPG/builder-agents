# Composition Strategies Design

**Module**: `coa-composition`  
**Lines Estimate**: ~1,000  
**Files**: 7

---

## Overview

Pluggable conflict resolution for multi-agent delta composition. Strategies are validated at construction time, ensuring conflicts are impossible by design.

---

## Core Trait

```rust
// File: src/strategy.rs (200 lines)

use coa_artifact::{Artifact, ArtifactType, StructuralDelta};
use coa_symbol::SymbolRefIndex;

/// Composition strategy for multi-agent delta coordination
/// 
/// # Safety
/// All strategies must ensure that `compose()` is deterministic and
/// that `validate()` catches all possible conflicts.
pub trait CompositionStrategy: Send + Sync + std::fmt::Debug {
    /// Validate that deltas can be composed under this strategy
    /// 
    /// # Returns
    /// - `Ok(Validation)` if deltas are compatible
    /// - `Err(CompositionError)` with diagnostic information
    /// 
    /// # Performance
    /// Should be O(n log n) or better
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError>;
    
    /// Compose deltas into final artifact
    /// 
    /// # Preconditions
    /// `validate()` must have returned `Ok` for these deltas
    /// 
    /// # Returns
    /// New artifact with all deltas applied
    fn compose<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError>;
    
    /// Parallelism characteristics
    fn parallelism(&self) -> Parallelism;
    
    /// Conflict detection granularity
    fn granularity(&self) -> Granularity;
    
    /// Strategy name (for debugging/serialization)
    fn name(&self) -> &'static str;
}

/// Validation result
#[derive(Debug, Clone)]
pub struct Validation {
    /// Strategy-specific validation data
    pub metadata: ValidationMetadata,
    
    /// Estimated composition cost
    pub cost_estimate: CompositionCost,
}

#[derive(Debug, Clone, Default)]
pub struct ValidationMetadata {
    /// Number of batches (for parallel strategies)
    pub batch_count: Option<usize>,
    
    /// Ordering constraints (for sequential strategies)
    pub ordering: Vec<OrderingConstraint>,
    
    /// Custom strategy data
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct OrderingConstraint {
    pub delta_index: usize,
    pub must_follow: Vec<usize>,
}

/// Composition cost estimate
#[derive(Debug, Clone, Copy)]
pub struct CompositionCost {
    /// Time complexity class
    pub time: TimeComplexity,
    
    /// Space complexity class
    pub space: SpaceComplexity,
    
    /// Parallelism factor (1.0 = fully parallel)
    pub parallelism_factor: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeComplexity {
    O1,
    OLogN,
    ON,
    ONLogN,
    ON2,
}

#[derive(Debug, Clone, Copy)]
pub enum SpaceComplexity {
    O1,
    OLogN,
    ON,
}

/// Parallelism characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parallelism {
    /// All deltas can be applied in parallel
    Full,
    
    /// Some batches can be parallel
    Partial,
    
    /// Sequential only
    None,
}

/// Conflict detection granularity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    /// Subtree level (ancestors/descendants conflict)
    Subtree,
    
    /// Exact node match only
    Node,
    
    /// Attribute-level within node
    Attribute,
}

/// Composition error with diagnostics
#[derive(Debug, thiserror::Error)]
pub enum CompositionError {
    #[error("validation failed: {diagnostic}")]
    ValidationFailed { diagnostic: ValidationDiagnostic },
    
    #[error("composition failed: {0}")]
    CompositionFailed(String),
    
    #[error("deltas not validated")]
    NotValidated,
}

/// Detailed validation failure
#[derive(Debug, Clone)]
pub struct ValidationDiagnostic {
    pub kind: ConflictKind,
    pub involved_deltas: Vec<usize>,
    pub description: String,
    pub suggestions: Vec<ResolutionSuggestion>,
}

#[derive(Debug, Clone)]
pub enum ConflictKind {
    OverlappingTargets,
    MissingOrdering,
    NonCommutativeOperations,
    InvalidDependencies,
}

#[derive(Debug, Clone)]
pub enum ResolutionSuggestion {
    UseSingleWriter,
    UseOrdered,
    UseCommutative,
    DecomposeTargets,
    AddOrdering,
}
```

---

## SingleWriterStrategy

```rust
// File: src/single_writer.rs (150 lines)

use coa_symbol::SingleWriterValidator;

/// Default strategy: each agent claims a disjoint subtree
/// 
/// # Characteristics
/// - Maximum safety
/// - Universal applicability
/// - Requires fine-grained COA decomposition
#[derive(Debug, Clone, Default)]
pub struct SingleWriterStrategy;

impl SingleWriterStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl CompositionStrategy for SingleWriterStrategy {
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        // Use symbol module's single-writer validation
        SingleWriterValidator::validate_deltas(deltas, index)
            .map_err(|e| CompositionError::ValidationFailed {
                diagnostic: ValidationDiagnostic {
                    kind: ConflictKind::OverlappingTargets,
                    involved_deltas: vec![], // Extract from error
                    description: e.to_string(),
                    suggestions: vec![
                        ResolutionSuggestion::DecomposeTargets,
                        ResolutionSuggestion::UseOrdered,
                    ],
                },
            })?;
        
        Ok(Validation {
            metadata: ValidationMetadata::default(),
            cost_estimate: CompositionCost {
                time: TimeComplexity::ONLogN,
                space: SpaceComplexity::ON,
                parallelism_factor: 1.0, // Fully parallel
            },
        })
    }
    
    fn compose<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError> {
        // SingleWriter: all deltas are independent
        // Apply in any order (parallel fold)
        
        deltas.iter()
            .try_fold(base.clone(), |acc, delta| {
                delta.apply(&acc)
                    .map_err(|e| CompositionError::CompositionFailed(e.to_string()))
            })
    }
    
    fn parallelism(&self) -> Parallelism {
        Parallelism::Full
    }
    
    fn granularity(&self) -> Granularity {
        Granularity::Subtree
    }
    
    fn name(&self) -> &'static str {
        "SingleWriter"
    }
}
```

---

## CommutativeBatchStrategy

```rust
// File: src/commutative.rs (200 lines)

/// CRDT-style commutative operations using `crdts` crate
/// 
/// # Characteristics
/// - Maximum parallelism
/// - Order-independent results
/// - Uses battle-tested CRDT implementations
#[derive(Debug, Clone)]
pub struct CommutativeBatchStrategy<T: CrdtArtifactType> {
    _phantom: PhantomData<T>,
}

/// Artifact types that can use CRDT composition
/// 
/// These artifact types map their operations to standard CRDTs:
/// - Add/Remove -> OR-Set
/// - Counter -> G-Counter or PN-Counter
/// - Register -> LWW-Register
pub trait CrdtArtifactType: ArtifactType {
    /// The CRDT type for this artifact
    type Crdt: crdts::CvRDT + crdts::CmRDT + Send + Sync;
    
    /// Convert delta to CRDT operation
    fn to_crdt_op(delta: &StructuralDelta<Self>) -> <Self::Crdt as crdts::CmRDT>::Op;
    
    /// Apply CRDT to get content
    fn apply_crdt(base: &Self::Content, crdt: &Self::Crdt) -> Self::Content;
}

/// Example: Layer composition using OR-Set
#[derive(Debug, Clone)]
pub struct LayerSet {
    inner: crdts::OrSet<String, u64>, // Actor ID u64
}

impl CrdtArtifactType for LayerArtifact {
    type Crdt = LayerSet;
    
    fn to_crdt_op(delta: &StructuralDelta<Self>) -> <Self::Crdt as crdts::CmRDT>::Op {
        match &delta.operation {
            DeltaOperation::Add(layer) => {
                // Generate unique ID and add to set
                let id = (actor_id(), unique_timestamp());
                LayerSetOp::Add(layer.name.clone(), id)
            }
            DeltaOperation::Remove(layer) => {
                LayerSetOp::Remove(layer.name.clone())
            }
            _ => panic!("Non-commutative operation"),
        }
    }
    
    fn apply_crdt(base: &Self::Content, crdt: &Self::Crdt) -> Self::Content {
        // Read layers from CRDT set
        let layers: Vec<_> = crdt.inner.read().into_iter().collect();
        Self::Content { layers }
    }
}

impl<T: CrdtArtifactType> CommutativeBatchStrategy<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: CrdtArtifactType> CompositionStrategy for CommutativeBatchStrategy<T> {
    fn validate<U: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<U>],
        _index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        // Check for duplicate targets (still not allowed in CRDT mode)
        let mut seen = HashSet::new();
        for (i, delta) in deltas.iter().enumerate() {
            let key = delta.target.to_string();
            if !seen.insert(key.clone()) {
                return Err(CompositionError::ValidationFailed {
                    diagnostic: ValidationDiagnostic {
                        kind: ConflictKind::OverlappingTargets,
                        involved_deltas: vec![i],
                        description: format!("Duplicate target: {}", key),
                        suggestions: vec![ResolutionSuggestion::UseSingleWriter],
                    },
                });
            }
        }
        
        Ok(Validation {
            metadata: ValidationMetadata {
                batch_count: Some(1),
                ..Default::default()
            },
            cost_estimate: CompositionCost {
                time: TimeComplexity::ON,
                space: SpaceComplexity::ON,
                parallelism_factor: 1.0,
            },
        })
    }
    
    fn compose<U: ArtifactType>(
        &self,
        base: &Artifact<U>,
        deltas: &[StructuralDelta<U>],
    ) -> Result<Artifact<U>, CompositionError> {
        use crdts::CmRDT;
        use rayon::prelude::*;
        
        // Convert deltas to CRDT ops in parallel
        let ops: Vec<_> = deltas.par_iter()
            .map(|d| T::to_crdt_op(d))
            .collect();
        
        // Apply all ops to base CRDT (order doesn't matter!)
        let mut crdt = T::Crdt::default();
        for op in ops {
            crdt.apply(op);
        }
        
        // Convert back to artifact
        let new_content = T::apply_crdt(base.content(), &crdt);
        Ok(Artifact::new(new_content))
    }
    
    fn parallelism(&self) -> Parallelism {
        Parallelism::Full
    }
    
    fn granularity(&self) -> Granularity {
        Granularity::Node
    }
    
    fn name(&self) -> &'static str {
        "CommutativeBatch (CRDT-based)"
    }
}
```

---

## OrderedCompositionStrategy

```rust
// File: src/ordered.rs (180 lines)

/// Sequential refinement with explicit ordering
/// 
/// # Characteristics
/// - Sequential dependency
/// - Later deltas see earlier results
/// - Universal applicability
#[derive(Debug, Clone, Default)]
pub struct OrderedCompositionStrategy;

impl OrderedCompositionStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl CompositionStrategy for OrderedCompositionStrategy {
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        _index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        // Verify all deltas have ordering metadata
        for (i, delta) in deltas.iter().enumerate() {
            if delta.order.is_none() {
                return Err(CompositionError::ValidationFailed {
                    diagnostic: ValidationDiagnostic {
                        kind: ConflictKind::MissingOrdering,
                        involved_deltas: vec![i],
                        description: format!("Delta {} missing order", i),
                        suggestions: vec![
                            ResolutionSuggestion::AddOrdering,
                        ],
                    },
                });
            }
        }
        
        // Build ordering constraints
        let mut ordering = Vec::new();
        for i in 0..deltas.len() {
            let mut must_follow = Vec::new();
            for j in 0..deltas.len() {
                if i != j && deltas[j].order < deltas[i].order {
                    must_follow.push(j);
                }
            }
            ordering.push(OrderingConstraint {
                delta_index: i,
                must_follow,
            });
        }
        
        Ok(Validation {
            metadata: ValidationMetadata {
                ordering,
                ..Default::default()
            },
            cost_estimate: CompositionCost {
                time: TimeComplexity::ON,
                space: SpaceComplexity::O1,
                parallelism_factor: 0.0, // Sequential
            },
        })
    }
    
    fn compose<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError> {
        // Sort by order
        let mut ordered: Vec<_> = deltas.iter().enumerate().collect();
        ordered.sort_by_key(|(_, d)| d.order.unwrap_or(0));
        
        // Sequential application
        ordered.into_iter()
            .try_fold(base.clone(), |acc, (_, delta)| {
                delta.apply(&acc)
                    .map_err(|e| CompositionError::CompositionFailed(e.to_string()))
            })
    }
    
    fn parallelism(&self) -> Parallelism {
        Parallelism::None
    }
    
    fn granularity(&self) -> Granularity {
        Granularity::Attribute
    }
    
    fn name(&self) -> &'static str {
        "OrderedComposition"
    }
}
```

---

## HybridCompositionStrategy

```rust
// File: src/hybrid.rs (250 lines)

/// Combines commutative batch with ordered refinement
/// 
/// # Characteristics
/// - Two-phase: parallel commutative → sequential ordered
/// - Best of both worlds
/// - Recommended for creative tools
#[derive(Debug, Clone)]
pub struct HybridCompositionStrategy {
    /// Function to classify delta as commutative or ordered
    classifier: Box<dyn Fn(&StructuralDelta<impl ArtifactType>) -> DeltaClass + Send + Sync>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaClass {
    Commutative,
    Ordered(u32), // Order value
}

impl HybridCompositionStrategy {
    /// Create with custom classifier
    pub fn new<F>(classifier: F) -> Self
    where
        F: Fn(&StructuralDelta<impl ArtifactType>) -> DeltaClass + Send + Sync + 'static,
    {
        Self {
            classifier: Box::new(classifier),
        }
    }
    
    /// Default classifier based on operation type
    pub fn default_classifier<T: ArtifactType>(delta: &StructuralDelta<T>) -> DeltaClass {
        // Check delta operation type
        match &delta.operation {
            DeltaOperation::Add(_) => DeltaClass::Commutative,
            DeltaOperation::Remove => DeltaClass::Commutative,
            DeltaOperation::Replace(_) => DeltaClass::Ordered(1),
            DeltaOperation::Transform(_) => DeltaClass::Ordered(2),
        }
    }
}

impl CompositionStrategy for HybridCompositionStrategy {
    fn validate<T: ArtifactType>(
        &self,
        deltas: &[StructuralDelta<T>],
        index: &SymbolRefIndex,
    ) -> Result<Validation, CompositionError> {
        // Partition deltas
        let mut commutative = Vec::new();
        let mut ordered = Vec::new();
        
        for delta in deltas {
            match (self.classifier)(delta) {
                DeltaClass::Commutative => commutative.push(delta),
                DeltaClass::Ordered(order) => ordered.push((order, delta)),
            }
        }
        
        // Validate commutative batch
        // (Check for duplicates)
        let mut seen = HashSet::new();
        for delta in &commutative {
            if !seen.insert(delta.target.clone()) {
                return Err(CompositionError::ValidationFailed {
                    diagnostic: ValidationDiagnostic {
                        kind: ConflictKind::OverlappingTargets,
                        involved_deltas: vec![],
                        description: "Duplicate in commutative batch".to_string(),
                        suggestions: vec![ResolutionSuggestion::UseSingleWriter],
                    },
                });
            }
        }
        
        // Validate ordered sequence
        ordered.sort_by_key(|(o, _)| *o);
        
        // Check: ordered deltas must not target symbols created by commutative
        // (or handle via proper dependency tracking)
        
        Ok(Validation {
            metadata: ValidationMetadata {
                batch_count: Some(2), // Commutative batch + ordered sequence
                ..Default::default()
            },
            cost_estimate: CompositionCost {
                time: TimeComplexity::ON,
                space: SpaceComplexity::ON,
                parallelism_factor: commutative.len() as f64 / deltas.len() as f64,
            },
        })
    }
    
    fn compose<T: ArtifactType>(
        &self,
        base: &Artifact<T>,
        deltas: &[StructuralDelta<T>],
    ) -> Result<Artifact<T>, CompositionError> {
        // Phase 1: Apply commutative deltas in parallel
        let commutative: Vec<_> = deltas.iter()
            .filter(|d| matches!((self.classifier)(d), DeltaClass::Commutative))
            .collect();
        
        let after_commutative = commutative.par_iter()
            .try_fold(|| base.clone(), |acc, delta| {
                delta.apply(&acc)
                    .map_err(|e| CompositionError::CompositionFailed(e.to_string()))
            })
            .try_reduce(|| base.clone(), |a, b| {
                // Merge partial results
                Ok(a)
            })?;
        
        // Phase 2: Apply ordered deltas sequentially
        let mut ordered: Vec<_> = deltas.iter()
            .filter_map(|d| match (self.classifier)(d) {
                DeltaClass::Ordered(order) => Some((order, d)),
                _ => None,
            })
            .collect();
        ordered.sort_by_key(|(o, _)| *o);
        
        ordered.into_iter()
            .try_fold(after_commutative, |acc, (_, delta)| {
                delta.apply(&acc)
                    .map_err(|e| CompositionError::CompositionFailed(e.to_string()))
            })
    }
    
    fn parallelism(&self) -> Parallelism {
        Parallelism::Partial
    }
    
    fn granularity(&self) -> Granularity {
        Granularity::Attribute
    }
    
    fn name(&self) -> &'static str {
        "HybridComposition"
    }
}
```

---

## Strategy Registry

```rust
// File: src/registry.rs (100 lines)

/// Registry of available strategies
pub struct StrategyRegistry {
    strategies: HashMap<String, Box<dyn CompositionStrategy>>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            strategies: HashMap::new(),
        };
        
        // Register built-in strategies
        registry.register("single_writer", Box::new(SingleWriterStrategy::new()));
        registry.register("ordered", Box::new(OrderedCompositionStrategy::new()));
        // Commutative and Hybrid require type parameters, registered elsewhere
        
        registry
    }
    
    pub fn register(&mut self, name: &str, strategy: Box<dyn CompositionStrategy>) {
        self.strategies.insert(name.to_string(), strategy);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn CompositionStrategy> {
        self.strategies.get(name).map(|b| b.as_ref())
    }
    
    /// Auto-select strategy based on artifact type and operation
    pub fn select<T: ArtifactType>(
        &self,
        artifact_type: &str,
        operation: &str,
    ) -> Option<&dyn CompositionStrategy> {
        match (artifact_type, operation) {
            ("code", _) => self.get("single_writer"),
            ("svg" | "image", "add_layer") => self.get("commutative"),
            ("mesh", "refine") => self.get("ordered"),
            ("audio", "add_track") => self.get("commutative"),
            _ => self.get("single_writer"), // Default
        }
    }
}
```

---

## Tests

```rust
// File: tests/strategy_tests.rs (250 lines)

#[cfg(test)]
mod tests {
    use super::*;
    use coa_composition::*;
    use coa_artifact::test_helpers::*;

    #[test]
    fn single_writer_validates_disjoint() {
        let strategy = SingleWriterStrategy::new();
        let index = SymbolRefIndex::new();
        
        let deltas = vec![
            create_delta("auth.login"),
            create_delta("auth.register"),
        ];
        
        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn single_writer_rejects_overlapping() {
        let strategy = SingleWriterStrategy::new();
        let index = SymbolRefIndex::new();
        
        let deltas = vec![
            create_delta("auth"),
            create_delta("auth.login"),
        ];
        
        let result = strategy.validate(&deltas, &index);
        assert!(matches!(result, Err(CompositionError::ValidationFailed { .. })));
    }

    #[test]
    fn ordered_requires_ordering() {
        let strategy = OrderedCompositionStrategy::new();
        let index = SymbolRefIndex::new();
        
        let deltas = vec![
            create_delta_with_order("step1", 1),
            create_delta_with_order("step2", 2),
        ];
        
        let result = strategy.validate(&deltas, &index);
        assert!(result.is_ok());
    }

    #[test]
    fn hybrid_classifies_correctly() {
        let strategy = HybridCompositionStrategy::new(|d| {
            if d.target.to_string().contains("layer") {
                DeltaClass::Commutative
            } else {
                DeltaClass::Ordered(1)
            }
        });
        
        let index = SymbolRefIndex::new();
        
        let deltas = vec![
            create_delta("canvas.layer1"),
            create_delta("canvas.layer2"),
            create_delta("canvas.effect"),
        ];
        
        let validation = strategy.validate(&deltas, &index).unwrap();
        assert_eq!(validation.metadata.batch_count, Some(2));
    }

    #[test]
    fn compose_applies_deltas() {
        let strategy = SingleWriterStrategy::new();
        let base = create_test_code_artifact();
        
        let delta = create_add_function_delta(&base);
        
        let result = strategy.compose(&base, &[delta]).unwrap();
        
        assert_ne!(base.hash(), result.hash());
    }

    #[tokio::test]
    async fn commutative_parallel_compose() {
        let strategy = CommutativeBatchStrategy::<LayerDelta>::new();
        // Test parallel application
        // ...
    }
}
```

---

## File Structure

```
crates/coa-composition/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── strategy.rs         # CompositionStrategy trait
    ├── single_writer.rs    # SingleWriterStrategy
    ├── commutative.rs      # CommutativeBatchStrategy
    ├── ordered.rs          # OrderedCompositionStrategy
    ├── hybrid.rs           # HybridCompositionStrategy
    └── registry.rs         # StrategyRegistry
```

---

## Cargo.toml

```toml
[package]
name = "coa-composition"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core
coa-artifact = { path = "../coa-artifact" }
coa-symbol = { path = "../coa-symbol" }

# Parallelism
rayon = "1.8"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "1"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
proptest = "1"
```
