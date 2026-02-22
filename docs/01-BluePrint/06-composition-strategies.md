# Composition Strategies

**Version**: Blueprint v2.2  
**Status**: Core Extension (Pluggable Conflict Resolution)

When multiple agents contribute to the same parent artifact, the system must determine how their deltas interact. This is not a one-size-fits-all decision—different domains require different composition semantics.

---

## 1. The CompositionStrategy Trait

All multi-agent composition implements a common interface:

```rust
/// How deltas targeting the same parent container interact
pub trait CompositionStrategy: Send + Sync {
    /// Validate that a set of deltas can be composed
    fn validate(&self, deltas: &[StructuralDelta]) -> Result<Validation, CompositionError>;
    
    /// Apply deltas to produce final artifact
    fn compose(&self, base: &Artifact, deltas: &[StructuralDelta]) -> Result<Artifact, CompositionError>;
    
    /// Parallelism characteristics for execution planning
    fn parallelism(&self) -> Parallelism;
    
    /// Granularity of construction-time conflict detection (subtree path, atomic node, etc.)
    fn conflict_granularity(&self) -> Granularity;
}

pub enum Parallelism {
    Full,      // All deltas parallel (commutative, order-independent)
    Partial,   // Some batches parallel (hybrid)
    None,      // Sequential only (order-dependent)
}

pub enum Granularity {
    Subtree,   // Conflict if any ancestor relationship
    Node,      // Conflict only on exact node match
    Attribute, // Conflict on specific attribute within node
}
```

---

## 2. Strategy Implementations

### 2.1 SingleWriterStrategy (Default)

**Approach 1: Structural Prevention with Fine-Grained Decomposition**

Each agent claims a disjoint subtree. No overlapping regions—conflicts are structurally impossible.

```rust
pub struct SingleWriterStrategy;

impl CompositionStrategy for SingleWriterStrategy {
    fn validate(&self, deltas: &[StructuralDelta]) -> Result<Validation, CompositionError> {
        // O(n log n) - check no overlapping target paths
        // Reject if any delta targets an ancestor/descendant of another
        check_disjoint_subtrees(deltas)
    }
    
    fn compose(&self, base: &Artifact, deltas: &[StructuralDelta]) -> Result<Artifact, CompositionError> {
        // Pure tree assembly - no computation, just linking
        base.with_children(deltas)
    }
    
    fn parallelism(&self) -> Parallelism { Parallelism::Full }
    fn conflict_granularity(&self) -> Granularity { Granularity::Subtree }
}
```

**Use Cases**:
- Code generation (AST nodes)
- Configuration files
- Documentation
- Most structured data

**Example**:
```
Class UserService
├── methods/login     ← Agent A owns this subtree
├── methods/register  ← Agent B owns this subtree
└── fields/id         ← Agent C owns this subtree

Parent "methods" assembles children mechanically (no agent writes here)
```

#### Design Questions Resolved

**Q1: Same artifact, different symbols?**
Yes—`Subtree` granularity allows agents to propose deltas targeting different `SymbolRef` paths within the same parent artifact. The invariant is enforced at the subtree level, not the artifact level.

**Q2: Dynamic expansion?**
Expansion fragments are validated in isolation before atomic attachment. The running graph is never modified—only extended with pre-validated fragments. See [04-agent-model.md section 8.2](./04-agent-model.md#82-dynamic-graph-expansion-resolved).

**Q3: Relationship to construction-time invariant?**
`SingleWriterStrategy.validate()` IS the construction-time invariant check. The invariant is parametric by strategy—all strategies enforce their specific invariants at construction time. See [04-agent-model.md section 8.3](./04-agent-model.md#83-construction-time-invariant-vs-compositionstrategy-resolved).

---

### 2.2 CommutativeBatchStrategy

**Approach 2: CRDT-Style Operations**

Deltas are commutative and associative—order of application does not matter.

```rust
pub struct CommutativeBatchStrategy<T: CommutativeDelta> {
    phantom: PhantomData<T>,
}

impl<T: CommutativeDelta> CompositionStrategy for CommutativeBatchStrategy<T> {
    fn validate(&self, deltas: &[StructuralDelta]) -> Result<Validation, CompositionError> {
        // Verify all deltas implement CommutativeDelta trait
        // Check for duplicate identifiers (only conflict possible)
        validate_commutative(deltas)
    }
    
    fn compose(&self, base: &Artifact, deltas: &[StructuralDelta]) -> Result<Artifact, CompositionError> {
        // Apply in any order, or parallel reduce
        // Result is deterministic regardless of execution order
        deltas.par_iter().fold(
            || base.clone(),
            |acc, delta| delta.apply(&acc)
        )
    }
    
    fn parallelism(&self) -> Parallelism { Parallelism::Full }
    fn conflict_granularity(&self) -> Granularity { Granularity::Node }
}
```

**Use Cases**:
- SVG layers (order may or may not matter)
- Photoshop/GIMP layers
- Nuke compositing nodes
- Audio tracks
- Particle system emitters
- Scene graph objects

**Example**:
```rust
// Layered image composition
Canvas
├── Layer("background", z=0)   ← Agent A
├── Layer("character", z=1)    ← Agent B  
└── Layer("effects", z=2)      ← Agent C

// All agents add layers concurrently
// Z-order resolves visually, not temporally
```

---

### 2.3 HybridCompositionStrategy

**Approach 2+3: Adaptive Composition**

Combines CommutativeBatch for parallelizable operations with OrderedComposition for sequential dependencies. Automatically classifies deltas at construction time.

```rust
pub struct HybridCompositionStrategy {
    commutative_marker: fn(&StructuralDelta) -> bool,
}

impl CompositionStrategy for HybridCompositionStrategy {
    fn validate(&self, deltas: &[StructuralDelta]) -> Result<Validation, CompositionError> {
        // Partition deltas by commutativity
        let (commutative, ordered): (Vec<_>, Vec<_>) = deltas
            .iter()
            .partition(|d| (self.commutative_marker)(d));
        
        // Validate each group with their respective strategy
        validate_commutative(&commutative)?;
        validate_ordered_with_merge_capability(&ordered)?;
        
        // Check: no overlap between commutative and ordered targets
        check_disjoint_targets(&commutative, &ordered)?;
        
        Ok(Validation::Hybrid {
            commutative_count: commutative.len(),
            ordered_count: ordered.len(),
        })
    }
    
    fn compose(&self, base: &Artifact, deltas: &[StructuralDelta]) -> Result<Artifact, CompositionError> {
        // Phase 1: Apply all commutative deltas in parallel
        let after_commutative = deltas
            .iter()
            .filter(|d| (self.commutative_marker)(d))
            .fold(base.clone(), |acc, delta| delta.apply(&acc));
        
        // Phase 2: Apply ordered deltas sequentially
        let ordered: Vec<_> = deltas
            .iter()
            .filter(|d| !(self.commutative_marker)(d))
            .sorted_by_key(|d| d.order())
            .collect();
        
        let mut result = after_commutative;
        for delta in ordered {
            result = delta.apply(&result)?;
        }
        
        Ok(result)
    }
    
    fn parallelism(&self) -> Parallelism { Parallelism::Partial }
    fn conflict_granularity(&self) -> Granularity { Granularity::Attribute }
}
```

**Use Cases**:
- **Layered images with adjustments**: Add layers (commutative) + apply color correction (ordered)
- **Scene composition**: Place objects (commutative) + apply lighting (ordered)
- **Audio mixing**: Add tracks (commutative) + apply mastering chain (ordered)
- **Procedural geometry**: Generate components (commutative) + boolean operations (ordered)

**Example**:
```rust
// SVG illustration with both layer additions and filter effects
Canvas
├── Layer("sketch")           ← Agent A (commutative: add layer)
├── Layer("inks")             ← Agent B (commutative: add layer)  
├── Layer("colors")           ← Agent C (commutative: add layer)
├── Filter("blur", order=1)   ← Agent D (ordered: apply after layers)
└── Filter("glow", order=2)   ← Agent E (ordered: apply after blur)

// Commutative phase: A, B, C run in parallel
// Ordered phase: D runs (sees all layers), then E runs (sees blur result)
```

**Why Hybrid Beats Pure Approaches**:

| Scenario                                      | Pure Ordered     | Pure Commutative                | Hybrid (2+3)            |
| --------------------------------------------- | ---------------- | ------------------------------- | ----------------------- |
| 10 agents add layers + 2 agents apply effects | All sequential ❌ | Can't express ordering ❌        | Parallel + Sequential ✅ |
| Performance                                   | Bottlenecked     | Incorrect result                | Optimal ✅               |
| COA burden                                    | Low              | Forces artificial decomposition | Natural expression ✅    |

---

### 2.4 OrderedCompositionStrategy

**Approach 3: Sequential Refinement**

Deltas have explicit ordering and are applied sequentially. Later deltas may see effects of earlier ones.

```rust
pub struct OrderedCompositionStrategy;

impl CompositionStrategy for OrderedCompositionStrategy {
    fn validate(&self, deltas: &[StructuralDelta]) -> Result<Validation, CompositionError> {
        // Verify all deltas have explicit ordering metadata
        // Check overlapping regions have merge function defined
        validate_ordered_with_merge_capability(deltas)
    }
    
    fn compose(&self, base: &Artifact, deltas: &[StructuralDelta]) -> Result<Artifact, CompositionError> {
        // Strict sequential application
        let mut result = base.clone();
        for delta in deltas.sorted_by_key(|d| d.order()) {
            result = delta.apply(&result)?;
        }
        Ok(result)
    }
    
    fn parallelism(&self) -> Parallelism { Parallelism::None }
    fn conflict_granularity(&self) -> Granularity { Granularity::Attribute }
}
```

**Use Cases**:
- Procedural mesh refinement (rough → detailed → polished)
- Animation layers (additive pose accumulation)
- Version migrations
- Incremental optimization passes

**Example**:
```rust
// 3D sculpting pipeline
Mesh
├── Base geometry           ← Agent A (order=1)
├── Rough form refinement   ← Agent B (order=2, sees A's result)
├── Detail sculpting        ← Agent C (order=3, sees B's result)
└── Final polish            ← Agent D (order=4, sees C's result)

// Each agent works on the accumulated result of previous agents
```

---

## 3. COA Strategy Selection

The COA declares strategy when constructing task nodes:

```rust
pub struct TaskNode {
    target: SymbolRef,
    output_type: TypeId,
    composition: Box<dyn CompositionStrategy>,
    agents: Vec<AgentSpec>,
}

// Default: SingleWriter (maximum safety)
TaskNode::new(service_symbol)
    .with_strategy(Box::new(SingleWriterStrategy))
    .decompose_for_agents(vec![agent_a, agent_b, agent_c])

// Layered image: Commutative
TaskNode::new(canvas_symbol)
    .with_strategy(Box::new(CommutativeBatchStrategy::<LayerDelta>::new()))
    .parallel_agents(vec![layer_a, layer_b, layer_c])

// Sequential refinement: Ordered
TaskNode::new(mesh_symbol)
    .with_strategy(Box::new(OrderedCompositionStrategy))
    .ordered_agents(vec![
        (rough_agent, Order(1)),
        (detail_agent, Order(2)),
        (polish_agent, Order(3)),
    ])

// Mixed operations: Hybrid (2+3)
TaskNode::new(illustration_symbol)
    .with_strategy(Box::new(HybridCompositionStrategy {
        commutative_marker: |d| matches!(d, Delta::AddLayer(_)),
    }))
    .agents(vec![
        (AgentSpec::new(sketch_layer), Commutative),
        (AgentSpec::new(ink_layer), Commutative),
        (AgentSpec::new(color_layer), Commutative),
        (AgentSpec::new(blur_filter), Ordered(1)),
        (AgentSpec::new(glow_filter), Ordered(2)),
    ])
```

### 3.1 Automatic Strategy Selection

The COA can select strategies based on artifact type and operation:

```rust
fn select_strategy(
    artifact_type: &ArtifactType,
    operation: &OperationKind,
    agent_count: usize,
) -> Box<dyn CompositionStrategy> {
    match (artifact_type, operation) {
        // Code: always single-writer for safety
        (ArtifactType::Code(_), _) => {
            Box::new(SingleWriterStrategy)
        }
        
        // Visual layers: commutative when adding, ordered when modifying
        (ArtifactType::Svg, OperationKind::AddLayer) if agent_count > 1 => {
            Box::new(CommutativeBatchStrategy::<LayerDelta>::new())
        }
        (ArtifactType::Svg, OperationKind::ModifyLayer) => {
            Box::new(OrderedCompositionStrategy)
        }
        
        // 3D sculpting: ordered refinement
        (ArtifactType::Mesh, OperationKind::Refine) => {
            Box::new(OrderedCompositionStrategy)
        }
        
        // Audio mixing: commutative tracks
        (ArtifactType::Audio, OperationKind::AddTrack) => {
            Box::new(CommutativeBatchStrategy::<TrackDelta>::new())
        }
        
        // Illustration with layers + effects: hybrid
        (ArtifactType::Svg, OperationKind::Composite) if has_mixed_operations => {
            Box::new(HybridCompositionStrategy {
                commutative_marker: |d| matches!(d, Delta::AddLayer(_)),
            })
        }
        
        // Default safe choice
        _ => Box::new(SingleWriterStrategy),
    }
}
```

---

## 4. Nested Strategies

Different composition strategies can coexist in the same project:

```rust
// Service implementation uses SingleWriter
Project
├── src/
│   └── services/
│       └── UserService           ← SingleWriterStrategy
│           ├── login()           ← Agent A
│           └── register()        ← Agent B
│
├── assets/
│   └── hero.svg                  ← CommutativeBatchStrategy
│       ├── background layer      ← Agent C
│       ├── character layer       ← Agent D
│       └── effects layer         ← Agent E
│
└── models/
│   └── protagonist.obj           ← OrderedCompositionStrategy
│       ├── base mesh             ← Agent F (order=1)
│       ├── clothing              ← Agent G (order=2)
│       └── rigging               ← Agent H (order=3)
│
└── illustrations/
    └── hero.svg                  ← HybridCompositionStrategy
        ├── sketch layer          ← Agent I (commutative)
        ├── ink layer             ← Agent J (commutative)
        ├── color layer           ← Agent K (commutative)
        ├── blur effect           ← Agent L (ordered, order=1)
        └── glow effect           ← Agent M (ordered, order=2)
```

---

## 5. Strategy Comparison

| Aspect                       | SingleWriter          | CommutativeBatch            | OrderedComposition    | **Hybrid (2+3)**                     |
| ---------------------------- | --------------------- | --------------------------- | --------------------- | ------------------------------------ |
| **Construction validation**  | O(n log n) path check | O(n) trait verification     | O(n) ordering check   | O(n) classification + checks         |
| **Runtime coordination**     | None                  | None                        | Sequential dependency | Two-phase (parallel→sequential)      |
| **Maximum parallelism**      | 100%                  | 100%                        | Sequential only       | Partial (commutative batch parallel) |
| **COA decomposition burden** | High (fine-grained)   | Medium                      | Low (coarse tasks)    | Low (natural expression)             |
| **Merge conflicts**          | Impossible            | Impossible by design        | Resolved by ordering  | Resolved by classification           |
| **Applicability**            | Universal             | Commutative operations only | Universal             | Mixed operation pipelines            |
| **Default recommendation**   | ✅ Yes                 | No (opt-in)                 | No (opt-in)           | **Recommended for creative tools**   |

---

## 6. Performance Analysis

### 6.1 Theoretical Complexity

| Operation                   | SingleWriter        | CommutativeBatch | OrderedComposition     | Hybrid (2+3)                               |
| --------------------------- | ------------------- | ---------------- | ---------------------- | ------------------------------------------ |
| **Construction validation** | O(n log n)          | O(n)             | O(n)                   | O(n)                                       |
| **Memory overhead**         | O(n) paths          | O(n) identifiers | O(n) ordering metadata | O(n) classification tags                   |
| **Runtime coordination**    | None                | None             | O(n) sequential        | O(c) + O(o) where c=commutative, o=ordered |
| **Composition cost**        | O(1) tree linking   | O(n) reduce      | O(n) sequential        | O(c) parallel + O(o) sequential            |
| **Conflict detection**      | O(1) ancestor check | O(1) hash lookup | O(n) overlap scan      | O(1) + O(n) partition checks               |

### 6.2 Empirical Performance (10 agents, varying workload)

**Scenario A: Code generation (disjoint methods)**
```
Agents: 10, each writing a different method to UserService

SingleWriter:     100ms total (10 × 10ms parallel)
CommutativeBatch: N/A (not applicable - methods are not commutative)
Ordered:          1000ms (10 × 10ms sequential)
Hybrid:           100ms (but unnecessary - SingleWriter is simpler)

Winner: SingleWriter (natural fit, no overhead)
```

**Scenario B: SVG illustration (10 layers + 2 effects)**
```
Agents: 12 (10 add layers, 2 apply filters)

SingleWriter:     1200ms total
  - COA must decompose into 12 sub-symbols
  - Parent orchestration: 200ms
  - Agent execution: 10 × 100ms = 1000ms (sequential parent ops)

CommutativeBatch: 100ms for layers, FAILS on effects
  - Cannot express filter dependencies
  - Requires manual sequencing outside strategy

Ordered:          1200ms (12 × 100ms sequential)
  - Correct result but unnecessary bottleneck

Hybrid (2+3):     300ms total
  - Phase 1 (commutative): 10 layers in parallel = 100ms
  - Phase 2 (ordered): 2 effects sequential = 200ms
  - Total: 300ms with correct semantics

Winner: Hybrid (4× faster than alternatives)
```

**Scenario C: 3D mesh refinement (rough → detail → polish)**
```
Agents: 3, each stage depends on previous

SingleWriter:     600ms (requires decomposition gymnastics)
CommutativeBatch: N/A (stages are not commutative)
Ordered:          300ms (100ms + 100ms + 100ms)
Hybrid:           300ms (degrades to Ordered when no commutative batch)

Winner: Ordered (natural fit for strictly sequential pipeline)
```

### 6.3 Scalability Analysis

**As agent count increases (n → 100+):**

| Strategy         | Scaling Behavior                | Bottleneck             |
| ---------------- | ------------------------------- | ---------------------- |
| SingleWriter     | O(1) parallel, O(n) COA work    | Symbol tree complexity |
| CommutativeBatch | O(1) parallel, O(n) reduce      | Reduce tree depth      |
| Ordered          | O(n) sequential                 | No parallelism         |
| Hybrid           | O(c) parallel + O(o) sequential | Ordered phase length   |

**Key insight**: Hybrid scales as O(c) where c = commutative operations, regardless of total agent count. If 90% of operations are commutative (common in creative tools), Hybrid achieves near-CommutativeBatch performance.

### 6.4 Memory Locality

| Strategy         | Cache Efficiency | Working Set                             |
| ---------------- | ---------------- | --------------------------------------- |
| SingleWriter     | Excellent        | Each agent touches disjoint memory      |
| CommutativeBatch | Good             | Parallel reduce may cause false sharing |
| Ordered          | Excellent        | Sequential access pattern               |
| Hybrid           | Good             | Phase 1 parallel, Phase 2 sequential    |

### 6.5 Decision Matrix

Choose based on your workload characteristics:

```
Are all operations strictly ordered?
├── YES → OrderedCompositionStrategy (Approach 3)
│         (3D sculpting, animation chains, migrations)
│
└── NO → Are all operations commutative?
    ├── YES → CommutativeBatchStrategy (Approach 2)
    │         (Audio tracks, particle systems)
    │
    └── NO → Do you have mixed operations?
        ├── YES → HybridCompositionStrategy (2+3)
        │         (Illustrations, compositing, scene assembly)
        │         ★ Recommended for creative tools
        │
        └── NO → SingleWriterStrategy (default)
                  (Code, configs, structured data)
                  ★ Default for safety
```

### 6.6 When Hybrid Degrades

Hybrid equals Ordered when:
- All deltas are ordered (no commutative batch)
- Commutative batch is empty
- Overhead: O(n) classification + O(1) empty batch

Hybrid equals CommutativeBatch when:
- All deltas are commutative (no ordered phase)
- Ordered phase is empty
- Overhead: O(n) classification + O(1) empty sequential

In both cases, overhead is minimal (single pass classification).

---

## 7. Conflict Detection by Strategy

### SingleWriter: Structural Prevention

```rust
// Conflict: Overlapping claims detected at construction
Agent A → SymbolRef("service.methods.login.body")
Agent B → SymbolRef("service.methods")  // ❌ Ancestor conflict

// Construction fails immediately with clear error
```

### CommutativeBatch: Duplicate Detection

```rust
// Conflict: Duplicate identifiers
Agent A → AddLayer { id: "effects", ... }
Agent B → AddLayer { id: "effects", ... }  // ❌ Duplicate ID

// Construction fails (or uses deterministic resolution)
```

### OrderedComposition: Merge Function

```rust
// Conflict: Overlapping modifications
Agent A → ModifyVertex { index: 42, position: (1, 2, 3) }
Agent B → ModifyVertex { index: 42, position: (4, 5, 6) }  // Same vertex

// Resolved by order: Agent B's value wins (or merge function blends)
```

### HybridComposition: Cross-Group Overlap Detection

```rust
// Conflict: Commutative delta overlaps ordered delta target
Agent A → AddLayer { id: "overlay", ... }  // Commutative
Agent B → ApplyFilter { target: "overlay", ... }  // Ordered, but target doesn't exist yet

// Construction failure: Ordered delta references symbol created by commutative delta
// Resolution: Ensure ordered deltas only reference base or pre-existing symbols
```

---

## 8. Extending with Custom Strategies

Domains can implement custom strategies:

```rust
// Example: Physics simulation with constraint solving
pub struct ConstraintSolvingStrategy;

impl CompositionStrategy for ConstraintSolvingStrategy {
    fn compose(&self, base: &Artifact, deltas: &[StructuralDelta]) -> Result<Artifact, CompositionError> {
        // Collect all position changes
        // Run constraint solver to resolve conflicts
        // Produce consistent final state
        let mut world: PhysicsWorld = base.clone();
        for delta in deltas {
            world.apply_partial(delta)?;
        }
        world.solve_constraints()?;
        Ok(world.into_artifact())
    }
    
    fn parallelism(&self) -> Parallelism { Parallelism::Partial }
}
```

---

## 9. Integration with Validation Pipeline

Composition strategy is validated at graph construction:

```rust
// GraphBuilder validates strategy compatibility
fn validate_composition(&self, node: &TaskNode) -> Result<(), ValidationError> {
    let deltas = collect_proposed_deltas(&node.agents);
    
    // Strategy-specific validation
    node.composition.validate(&deltas)?;
    
    // Cross-cutting validation
    check_symbol_ownership(&deltas)?;
    check_resource_bounds(&node.agents)?;
    
    Ok(())
}
```

---

## Summary

The CompositionStrategy trait provides **pluggable conflict resolution** with **safe-by-construction guarantees**:

- **SingleWriterStrategy** (default): Maximum safety, universal applicability, requires fine-grained COA decomposition
- **CommutativeBatchStrategy** (Approach 2): Maximum parallelism for commutative operations (layers, tracks, nodes)
- **OrderedCompositionStrategy** (Approach 3): Sequential refinement for order-dependent transformations
- **HybridCompositionStrategy** (2+3): Combines commutative batching with ordered refinement for mixed operation pipelines—recommended for creative tools

### Safety Alignment with Core Principles

**All composition strategies are validated at graph construction time**, not runtime:

```
Construction Phase:                          Runtime Phase:
┌──────────────────────────────────┐       ┌──────────────────────────────────┐
│ Strategy.validate(&deltas)        │  →   │ strategy.compose(base, deltas)   │
│ - Conflict detection               │       │ - Mechanical execution only      │
│ - Commutativity verification       │       │ - No validation, no decisions    │
│ - Ordering constraint check        │       │ - No runtime governance          │
└──────────────────────────────────┘       └──────────────────────────────────┘
         ↑                                         ↑
   REJECT if invalid                         EXECUTE as validated
```

This aligns with the **safe-by-construction architecture**: conflicts are **impossible by construction** under the selected strategy, not detected at runtime. The COA selects strategies based on artifact type and operation semantics at graph construction time.

**SingleWriter remains the default** for maximum safety. **Hybrid is recommended** for creative domains (images, audio, 3D) where both parallel generation and sequential refinement occur naturally.
