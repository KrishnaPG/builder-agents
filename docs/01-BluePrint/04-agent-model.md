# AGENT MODEL AND MODES

## 5. SYSTEM PRIMITIVES

1. **Artifact<T>** - TypedTree representing a structured document (AST, config tree, spec model)
2. **StructuralDelta<T>** - Semantic transformation on Artifact<T> (not text patches)
3. **Edge** - Typed dependency between Artifacts (symbolic reference, not path)
4. **Directive** - Structural modifier on subgraphs
5. **Autonomy State** - Embedded in execution contract
6. **Time State**
7. **Meta-Agent (COA)**
8. **Expansion Stub (for staged construction)**

All features must map to these.

---

## 5.1 No Direct IO Model

**Core Principle**: Runtime agents have zero filesystem access. They cannot create, read, update, or delete files. They can only *propose* changes.

### The Model (Human Readable)

**Traditional AI Coding (Unsafe)**:
```
Agent: "I'll write the code to auth.ts"
Agent opens file → writes bytes → saves
Problem: Agent overwrote your changes, added a backdoor, or corrupted the file
```

**No Direct IO Model (Safe by Construction)**:
```
Agent: "I propose adding a login method to the AuthService class"
↓
Graph Construction: "Is this valid? Does it break invariants? Any conflicts?"
↓
If valid → Delta embedded in ValidatedGraph
If invalid → Graph construction REJECTS, COA notified
↓
Runtime → Constitutional Layer applies atomically (zero validation)
```

**Key Insight**: The agent is like a consultant giving advice, not a contractor with keys to your house.

### What This Makes Impossible

| Traditional Risk                      | (No Direct-IO) Prevention                  |
| ------------------------------------- | ------------------------------------------ |
| Agent overwrites human changes        | Agent literally cannot access filesystem   |
| Two agents race on same file          | Constitutional layer serializes proposals  |
| Agent writes to wrong path            | Paths don't exist at agent interface       |
| Agent exfiltrates data via file       | No outbound file access                    |
| Agent leaves debug code in production | All proposals validated before application |

### Technical Representation

```rust
// What the agent produces (during task execution)
TransformationProposal<StructuralDelta<T>> {
    delta: StructuralDelta<T>,      // The actual change
    target: SymbolRef,              // What to change (symbolic, not path)
    base_hash: ContentHash,         // Expected state (for optimistic concurrency check)
    metadata: ProposalMetadata,     // Agent id, timestamp, reasoning
}

// Construction-time validation result (before runtime execution)
enum ConstructionValidation {
    Validated,                      // Delta embedded in graph, will execute
    Rejected(ValidationError),      // Graph construction fails, COA must handle
}

// Note: Conflicts are impossible by construction—SingleWriterStrategy and
// composition validation reject overlapping claims at graph construction time.
// No "conflict resolution" happens at runtime.
```

**The agent sees**: Symbols, types, structure  
**The Constitutional Layer handles**: Files, paths, serialization, validation

---

## 5.2 Artifact Model (TypedTree)

**Core Principle**: A file is not a string. Agents never see bytes or text buffers.

Artifacts are structured, typed objects:

| Artifact Type  | Internal Representation           |
| -------------- | --------------------------------- |
| Code files     | AST + Module graph + Symbol table |
| Config files   | Schema-validated tree             |
| Spec documents | Structured document model         |
| Binary assets  | Content hash + Metadata (opaque)  |

### Artifact Properties

* **Immutable**: Once created, an artifact's content hash is fixed
* **Content-addressed**: Stored and referenced by hash, not path
* **Symbolically linked**: Cross-references use SymbolRef, not file paths
* **Type-enforced**: All operations on Artifact<T> must respect type T

---

## 5.2 StructuralDelta<T>

Agents do not write files. They compute semantic transformations.

**NOT**: Text patches (`insert at line 42`, `replace "foo" with "bar"`)
**BUT**: Structural deltas with semantic meaning:

```
AddMethod {
    target: SymbolRef("AuthService"),
    signature: MethodSignature { name: "login", params: [...] },
    body: Block { ... }
}

UpdateConfigValue {
    path: KeyPath(["server", "timeout"]),
    value: Duration(60, Seconds),
    schema_check: PassesValidation
}
```

### Delta Properties

* **Type-safe**: Delta<T> can only apply to Artifact<T>
* **Invariant-preserving**: Constitutional layer verifies AST remains valid
* **Conflict-free by construction**: Composition strategy validates no overlapping claims at graph construction time
* **Reversible**: Each delta has inverse operation for time scrubber

### Multi-Agent Composition

When multiple agents contribute deltas to the same parent container, the system uses **Composition Strategies** to determine how they interact. See [06-composition-strategies.md](./06-composition-strategies.md) for the complete specification of:

- `SingleWriterStrategy` (default) - Disjoint subtree claims, structurally conflict-free
- `CommutativeBatchStrategy` - CRDT-style for layers, tracks, nodes
- `OrderedCompositionStrategy` - Sequential refinement for order-dependent transformations

The COA selects the appropriate strategy based on artifact type and operation semantics.

---

## 6. AGENT MODEL (Runtime Agents)

Runtime agents are ephemeral constructs instantiated by COA.

Each agent must define:

1. Role Definition
2. Capability Scope (read-only Artifact access)
3. Memory Boundary (enforced by container primitive)
4. Execution Contract (with embedded escalation thresholds)
5. Logging Requirement
6. **Policy Token (construction-time requirement, embedded in ValidatedGraph)**

### Agent I/O Contract

**Input**: Read-only view of:
- Task specification (Artifact<Spec>)
- Explicit dependency Artifacts (via symbolic references)
- Approved knowledge graph nodes

**Output**: 
- **Single**: StructuralDelta<T> (where T matches task output type)
- **Multiple**: Vec<StructuralDelta<T>> (for batch transformations)

**Prohibited**:
- Filesystem access
- Network access (unless declared in capability scope)
- Direct mutation of any Artifact
- Generation of text/bytes (must be structured)

## 6.1 Policy Token vs Runtime Integrity

| Aspect           | Policy Token (Construction)                     | Runtime Verification           |
| ---------------- | ----------------------------------------------- | ------------------------------ |
| **Issuance**     | Bound to node during GraphBuilder::validate()   | N/A                            |
| **Contents**     | Autonomy level, resource bounds, directive hash | N/A                            |
| **Verification** | N/A                                             | Cryptographic signature check  |
| **Expiration**   | Declared at issuance                            | Checked at runtime             |
| **Decision**     | "What is this agent allowed?" (construction)    | "Is this authentic?" (runtime) |

## 6.2 Agent Constraints

* Cannot self-elevate autonomy (state machine prevents this)
* Cannot persist memory outside container (enforced by primitives)
* Cannot instantiate other agents directly (must request through COA)
* Cannot access filesystem or network (container-enforced)
* Must return StructuralDelta<T> (type-enforced at construction)
* Must pass schema validation (at construction time)
* Must pass policy validation (at construction time)
* Must register in execution graph (at construction time)
* Must be destroyed after lifecycle completion (contract-enforced)

Unbounded recursion prevented by construction-time recursion depth limits.

---

## 6.3 Constitutional Application Layer

The trusted component with TWO distinct phases: **Construction-Time Validation** and **Runtime Application**.

### Phase 1: Construction-Time Validation

**When**: Graph construction (before any runtime execution)

**Responsibilities**:
- Parse external files into TypedTree Artifacts (on ingress)
- Validate StructuralDelta<T> against target Artifact<T> type
- Verify composition strategy (no overlapping claims under selected strategy)
- Resolve all SymbolRefs to existing artifacts (referential integrity)
- Enforce output integrity (single-writer invariant)

**Result**: ValidatedGraph (immutable, proof-carrying)

### Phase 2: Runtime Application

**When**: Task execution (zero validation, mechanical only)

**Responsibilities**:
- Receive agent's TransformationProposal
- Verify base_hash matches expected state (optimistic concurrency, not conflict detection)
- Apply deltas atomically to produce new Artifact versions
- Update Merkle DAG with new content hashes
- Serialize Artifacts back to external format (on egress)

**Key Invariant**: Runtime application performs **NO validation**—all validation happened at construction time. The layer mechanically applies pre-validated deltas.

### Output Guarantees
- Applied deltas produce content-addressed, immutable artifacts
- Old artifact versions remain accessible via hash
- New artifact hash propagates through dependent SymbolRefs

---

## 7. MODES (Directive Bundles)

Modes are directive bundles. Switching modes requires graph reconstruction and revalidation.

COA may synthesize mode composition dynamically.

---

## Mode A: Sketchpad

High speed, reduced coverage, minimal UI.

Security pipeline: Light configuration (but secrets detection still mandatory).

Artifact validation: Schema-only (minimal semantic checks).

---

## Mode B: Factory

Strict TDD. 100 percent test coverage required.

Coverage includes:

* Line coverage
* Branch coverage
* Mutation coverage optional

Pipeline view:

Spec → Code Generation → Security → Test Generation → QA → Deploy.

**Test Generation**: Derived from Spec Artifact via structured transformation (not manual text).

---

## Mode C: Multiverse

Parallel branches.

Rules:

1. Branch autonomy isolated (enforced by graph topology)
2. Knowledge graph shared read-only
3. Writes require branch labeling
4. Feature drag creates patch artifact (StructuralDelta diff)
5. Final merge requires:

   * Cross-branch structural merge (semantic, not textual)
   * Symbolic reference reconciliation
   * Directive reconciliation (graph construction)
   * Autonomy compliance (encoded in node types, no runtime check)

---

## Mode D: Renovator

Incremental rewrite with Live system continuity .

Must:

1. Maintain compatibility adapter (as Artifact<InterfaceContract>)
2. Route traffic through adapter
3. Replace modules incrementally (via StructuralDelta chains)
4. Prevent downtime

Heatmap overlay required.

---

## 8. OUTPUT AND REFERENTIAL INTEGRITY

### Output Integrity (Single-Writer by Default)

**Default Strategy**: `SingleWriterStrategy` enforces single-writer semantics at the granularity specified by the `conflict_granularity()` of the selected strategy.

```
Graph construction REJECTS under SingleWriterStrategy if:
- Two TaskNodes both claim ownership of the same SymbolRef path (subtree granularity)
- Target symbol is not unique in namespace
```

**Alternative Strategies** (see [06-composition-strategies.md](./06-composition-strategies.md)):
- `CommutativeBatchStrategy` - Allows parallel writes to disjoint subtrees when operations commute
- `OrderedCompositionStrategy` - Sequential refinement for order-dependent transformations  
- `HybridCompositionStrategy` - Combines parallel batching with sequential refinement

This makes conflicting writes **impossible by construction** under the selected strategy, not merely detected at runtime.

#### 8.1 Sub-Artifact Granularity (Resolved)

**Question**: Can two agents propose deltas targeting different `SymbolRef`s within the same parent artifact?

**Resolution**: **Yes**, with `Subtree` granularity (the default for `SingleWriterStrategy`).

```rust
// Two agents CAN propose deltas to the same parent artifact if their target SymbolRef paths are disjoint:
// Agent A: StructuralDelta { target: SymbolRef { file: "utils.ts", path: ["helpers", "formatDate"] }, ... }
// Agent B: StructuralDelta { target: SymbolRef { file: "utils.ts", path: ["helpers", "parseDate"] }, ... }
// 
// These are VALID - different subtrees under "helpers"

// But this is INVALID:
// Agent A: StructuralDelta { target: SymbolRef { file: "utils.ts", path: ["config", "apiUrl"] }, ... }
// Agent B: StructuralDelta { target: SymbolRef { file: "utils.ts", path: ["config", "apiUrl"] }, ... }
// 
// Same exact path = conflict under SingleWriterStrategy
```

The granularity is determined by `CompositionStrategy::conflict_granularity()`:
- `Subtree`: Conflict if paths share ancestor relationship (default for SingleWriter)
- `Node`: Conflict only on exact node match
- `Attribute`: Conflict on specific attribute within node

#### 8.2 Dynamic Graph Expansion (Resolved)

**Question**: When COA adds TaskNodes mid-execution, how is single-writer re-validated?

**Resolution**: **Expansion fragments are validated before attachment**, not by modifying the running graph.

```rust
// 1. COA creates EXPANSION FRAGMENT (isolated subgraph)
let expansion = graph_builder.create_fragment(|builder| {
    builder.add_task(write_config_api)?;
    builder.add_task(write_config_db)?;
    Ok(())
})?; // Validation runs HERE, before any attachment

// 2. Fragment validated against CURRENT graph state
//    - Collects all SymbolRef claims in expansion
//    - Checks against existing claims in running graph
//    - REJECTS if overlap detected

// 3. Atomic attachment to running graph
graph.attach_fragment(expansion)?; // No re-validation needed
```

**Key insight**: The "running graph" is append-only. We never modify existing nodes—we only attach pre-validated fragments. The validation happens at fragment construction time, not at attachment time.

#### 8.3 Construction-Time Invariant vs CompositionStrategy (Resolved)

**Question**: How does the construction-time invariant map to `SingleWriterStrategy`? Are they the same or layered?

**Resolution**: **They are the SAME check**—the construction-time invariant is implemented BY the selected `CompositionStrategy`.

```rust
// GraphBuilder delegates to the strategy:
fn validate_output_integrity(&self, node: &TaskNode) -> Result<(), ValidationError> {
    let deltas = collect_deltas(&node.agents);
    
    // This IS the construction-time invariant check:
    node.composition_strategy.validate(&deltas)?;
    
    Ok(())
}
```

**Relationship**:
- `SingleWriterStrategy.validate()` implements the single-writer construction-time invariant
- `CommutativeBatchStrategy.validate()` implements the commutative-batch construction-time invariant
- etc.

The invariant is **parametric**—different strategies enforce different invariants, but ALL are enforced at construction time, making violations impossible by construction for the selected strategy.

### Referential Integrity (Symbolic Coupling)

**Cross-artifact references use SymbolRef, not paths**:

```
// NOT: import { foo } from "../utils/helpers.ts"
// BUT:  SymbolRef { crate: "utils", module: "helpers", symbol: "foo" }
```

**Construction-time invariant**: All SymbolRefs in a StructuralDelta must resolve to existing symbols in the Artifact graph.

```
Graph construction REJECTS if:
- Delta references SymbolRef that does not exist
- Referenced symbol was removed by another delta in same graph
- Symbol type does not match expected interface
```

This makes broken references **impossible by construction**.

#### 8.4 Forward References (Not a Problem)

**Question**: How to handle mutually recursive structures (A references B, B references A)?

**Resolution**: **Not a SymbolRef problem. Internal structure is opaque to the graph.**

SymbolRef points to **nodes in the Merkle DAG**, not to internal AST relationships. If an artifact's internal structure (AST, document tree, etc.) contains mutual references, that's the artifact's internal concern:

```
Artifact<UserModule> (hash: abc123...)
├── SymbolRef: "helpers/is_even" → points to subtree node
│   └── AST internally: calls "is_odd" (internal reference, not SymbolRef)
└── SymbolRef: "helpers/is_odd" → points to subtree node
    └── AST internally: calls "is_even" (internal reference, not SymbolRef)
```

The graph only validates that SymbolRefs resolve. Internal AST structure is validated by the **Constitutional Application Layer** when parsing/applying deltas, not by graph construction.

**Key insight**: SymbolRef is for **cross-artifact** linking. Intra-artifact structure is the artifact type's responsibility.

#### 8.5 Staged Creation (Hash-Bound)

**Question**: If artifact A references artifact B, how is construction order enforced?

**Resolution**: **Hash binding in Merkle DAG makes this automatic.**

```rust
// SymbolRef includes parent's content hash:
SymbolRef {
    crate: "utils",
    module: "helpers",
    symbol: "formatDate",
    parent_hash: "sha256:abc123...",  // Hash of parent artifact
}

// To create this SymbolRef, the hash must be known.
// If B's hash isn't known, the SymbolRef cannot be constructed.
```

**Construction-time invariant**: A delta cannot propose a SymbolRef with an unknown `parent_hash`. This is enforced by the **Merkle structure**, not by explicit dependency edges.

The graph is **append-only and hash-linked**:
- Every artifact's hash depends on its content (including SymbolRefs it contains)
- Every SymbolRef embeds the parent's hash
- Invalidation propagates forward through hash mismatch

#### 8.6 Cross-Boundary References (External Artifacts)

**Question**: How are external references (npm, std lib) handled?

**Resolution**: **External Artifacts are graph roots with stable hashes.**

```rust
// External dependency as root Artifact:
Artifact<ExternalModule> {
    id: "system:std:collections:HashMap",
    content_hash: "sha256:def456...",  // Computed from manifest lock
    source: ExternalManifest,           // Cargo.lock, package-lock.json, etc.
    symbol_tree: [...],                 // Exported symbols as tree
}

// SymbolRef to external:
SymbolRef {
    crate: "system:std",
    module: "collections",
    symbol: "HashMap",
    parent_hash: "sha256:def456...",  // From lock file
}
```

External Artifacts are:
- Generated from lock files (`Cargo.lock`, `package-lock.json`) at workspace init
- Read-only roots in the graph
- Version-pinned via hash (changing version = different hash = different graph)

### Semantic Coupling (Spec-Code-Test Chain)

The system maintains derived relationships:

```
Spec Artifact ──derives──► Code Artifact ──produces──► Output
       │                        │
       └──derives──► Test Artifact ───validates───────┘
```

**When Spec changes**:
1. Spec Artifact hash changes
2. Derivation edges break (construction-time detection)
3. Graph reconstruction required
4. COA must regenerate Code and Test Artifacts from new Spec
5. New graph validated before execution

This makes spec-code-test drift **impossible by construction**.

---

[Back to Index](./01-intro.md) | [Previous: Security](./03-security.md) | [Next: Operations](./05-operations.md)
