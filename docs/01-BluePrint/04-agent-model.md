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
Constitutional Layer: "Is this valid? Does it break invariants? Any conflicts?"
↓
If valid → Applied atomically
If invalid → Rejected, agent notified, human can review
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
// What the agent produces
TransformationProposal<StructuralDelta<T>> {
    delta: StructuralDelta<T>,      // The actual change
    target: SymbolRef,              // What to change (symbolic, not path)
    base_hash: ContentHash,         // Expected state (detects conflicts)
    metadata: ProposalMetadata,     // Agent id, timestamp, reasoning
}

// The Constitutional Layer validates and applies
enum ProposalResult {
    Applied(ContentHash),           // New artifact version
    Rejected(ValidationError),      // Why it failed
    Conflict(Vec<ProposalRef>),     // Concurrent proposals need resolution
}
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
* **Conflict-detectable**: Semantic conflicts identified before application
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

The trusted component that applies agent-produced deltas.

**Responsibilities**:
- Parse external files into TypedTree Artifacts (on ingress)
- Validate StructuralDelta<T> against target Artifact<T>
- Detect conflicts between concurrent deltas
- Apply deltas atomically to produce new Artifact versions
- Serialize Artifacts back to external format (on egress)

**Validation Checks**:
1. **Type Compatibility**: Delta type matches target Artifact type
2. **Invariant Preservation**: AST remains valid, symbols resolve
3. **Conflict Detection**: No concurrent modification of same symbol
4. **Referential Integrity**: All SymbolRefs in delta resolve to existing artifacts

**Output Guarantees**:
- Applied deltas produce content-addressed, immutable artifacts
- Old artifact versions remain accessible via hash
- New artifact automatically updates symbolic references

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

### Output Integrity (Single-Writer Guarantee)

**Construction-time invariant**: Each Artifact node has exactly ONE incoming edge from a delta-producing task.

```
Graph construction REJECTS if:
- Two TaskNodes both produce deltas targeting the same Artifact
- Target symbol is not unique in namespace
```

This makes conflicting writes **impossible by construction**, not detected at runtime.

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
