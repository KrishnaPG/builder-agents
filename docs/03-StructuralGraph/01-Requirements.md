**Requirement**: Given a codebase (one source code file, or set of files, or one project or multiple projects), the AST (or any equivalent structural representation) needs to be captured in a way parallel agent proposals can be applied to it (to achieve users' intents of adding new modules/features/functionality, refactor existing code, fix bugs, optimize performance etc.);

We avoid conflicts by design (e.g. no two agents work in parallel on overlapping subtrees); and we already use Merkle DAG for referential integrities and dependency chain lazy valuations; our system is already designed for Typed StructuralDelta<T> with algebraic merge rules; 

Here are few constraints:
  - large codebase may end up having very large scale structural graphs and 
    1. we can't fit all of that into agent context, 
    2. only a small portion of that graph might need to be worked on at a given point.
  - the very large structural graph (or forest) has to be precomputed, cached, indexed and loaded on-demand (by agents in context or time-scrubbing UI)
  - the structural graphs are for machines (e.g. AST), but humans may need different "views" of the same graph to interact with (e.g. system component view, data flow view, dependency graph view, input/output view etc.); there may be a hierarchy of view layers (at the bottom the machine-friendly AST, and in the middle edit/agent-friendly structures, and on top the human/UI friendly structures for ease of reasoning and control); 
    - e.g. humans may want to see how data in being transformed (ETL pipeline) and decide to add on more "T-junction" in the pipe that writes in parallel to both files and external webhooks; User should not have to deal with file or code level details when specifying their intent for software refactor/update/bug-fix so on.


# Requirement Decomposition
| Dimension                 | What Must Be True                                         | Why It Matters                               |
| ------------------------- | --------------------------------------------------------- | -------------------------------------------- |
| Structural Representation | Code must be represented as typed structural graph (AST+) | Enables semantic edits instead of text diffs |
| Identity Stability        | Nodes must have stable IDs across edits                   | Required for parallel deltas                 |
| Referential Integrity     | Symbol references must be resolvable symbolically         | Prevent spec/code/test drift                 |
| Parallel Proposals        | Multiple agents must propose changes safely               | Enables swarm intelligence                   |
| Conflict Resolution       | Deterministic merge of proposals                          | Avoid race conditions                        |
| Multi-Project Scope       | Cross-repo symbol resolution                              | Real-world monorepos                         |
| Incrementality            | Partial re-parse and partial validation                   | Performance at scale                         |
| Deterministic Application | Same input graph + same deltas = same result              | Reproducibility                              |
| Versioning                | Structural time-travel                                    | Debugging and audit                          |

Below is a **structured requirement breakdown** given the constraints and existing architecture of [BluePrint](../01-BluePrint/01-intro.md).

---

## 1. Scale & Context Constraints

| Concern                  | Requirement                                   | Architectural Implication       |
| ------------------------ | --------------------------------------------- | ------------------------------- |
| Massive structural graph | Must not load entire graph into agent context | Strict subgraph extraction API  |
| Sparse work region       | Operate on minimal induced subgraph           | Dependency-aware slicing engine |
| Context window limits    | Bounded artifact projection                   | View-specific projection layer  |
| Multi-project forest     | Lazy loading per Merkle root                  | Multi-root graph registry       |
| Time scrubbing           | Snapshot-based structural versioning          | Persistent Merkle checkpoints   |

---

## 2. Graph Storage & Indexing Requirements

| Layer                   | Requirement                | Mechanism                             |
| ----------------------- | -------------------------- | ------------------------------------- |
| Persistence             | Immutable Merkle DAG       | Content-addressed storage             |
| Indexing                | Fast symbol lookup         | Global symbol index (trie + hash map) |
| Subtree Retrieval       | O(log n) subtree fetch     | Merkle path traversal                 |
| Cross-artifact lookup   | Reverse reference index    | SymbolRef → Artifact map              |
| Impact analysis         | Dependency adjacency cache | Precomputed edge index                |
| Partial materialization | Load-on-demand nodes       | Lazy node hydration                   |
| Large scale caching     | Hot region LRU cache       | Structural locality heuristic         |
| Cross-version diff      | Efficient Merkle diff      | Hash-based subtree equality           |

---

## 3. Subgraph Extraction Model

Agents must operate on bounded structural slices.

| Extraction Mode       | Description                     | Use Case                     |
| --------------------- | ------------------------------- | ---------------------------- |
| Symbol-Centric        | Extract function + dependencies | Bug fix                      |
| Flow-Centric          | Extract data-flow slice         | Performance tuning           |
| Module-Centric        | Extract module subtree          | Feature addition             |
| Dependency-Centric    | Extract upstream/downstream     | Refactor impact              |
| Test-Coupled          | Spec + Code + Tests slice       | Drift-safe change            |
| Multi-Artifact Bundle | Bundle minimal artifact set     | Complex cross-cutting change |

**Requirement:**
Subgraph extraction must be deterministic and reproducible.

---

## 4. Multi-Layer Graph Model (Critical)

You need **hierarchical graph representations**:

| Layer | Audience       | Structure                | Purpose                         |
| ----- | -------------- | ------------------------ | ------------------------------- |
| L0    | Machine        | AST + Symbol Graph       | Structural truth                |
| L1    | Agent          | Semantic Operation Graph | Editable transformation surface |
| L2    | Architect      | Component Graph          | System reasoning                |
| L3    | Executive/User | Workflow / Dataflow View | Intent-level manipulation       |
| L4    | Time           | Version Graph            | Scrubbing & replay              |

Each layer must:

* Be derivable from lower layer
* Preserve referential traceability
* Be invertible or at least losslessly mapped downward

---

## 5. View Transformation Requirements

| Requirement                   | Meaning                                           |
| ----------------------------- | ------------------------------------------------- |
| Deterministic View Projection | Same structural graph → same view                 |
| Bidirectional Mapping         | UI-level action → StructuralDelta<T>              |
| Partial Projection            | Only show relevant subgraph                       |
| Aggregation                   | Collapse low-level nodes into higher-level blocks |
| Semantic Grouping             | Group by responsibility not file                  |
| Flow Derivation               | Derive dataflow from AST                          |
| Component Derivation          | Derive modules/services graph                     |
| View Caching                  | Precompute heavy views                            |
| View Versioning               | View state tied to graph hash                     |

---

## 6. Human Interaction Abstraction Layer

Users must operate at higher abstraction.

| Intent Type        | Human View          | Downward Translation                           |
| ------------------ | ------------------- | ---------------------------------------------- |
| Add ETL Branch     | Dataflow view       | Add node + edges in flow graph → AST transform |
| Add Feature        | Component view      | New module subtree                             |
| Fix Bug            | Test failure view   | Extract minimal failing slice                  |
| Refactor           | Dependency view     | Move subtree + update refs                     |
| Optimize           | Performance overlay | Body-level AST transform                       |
| Add Webhook Output | IO graph view       | Inject transformation node + effect boundary   |

**Requirement:**
All human actions generate valid StructuralDelta<T> through controlled transformation pipeline.

---

## 7. Caching Strategy Requirements

| Cache Type              | Purpose                               |
| ----------------------- | ------------------------------------- |
| AST Node Cache          | Fast subtree load                     |
| Symbol Index Cache      | Fast lookup                           |
| View Cache              | Pre-rendered component/dataflow views |
| Delta Application Cache | Avoid recomputation                   |
| Impact Cache            | Precomputed dependency closures       |
| Snapshot Cache          | Efficient time scrub                  |
| Hot Region Cache        | Likely-to-edit clusters               |

Cache must be:

* Content-hash keyed
* Invalidation-free due to immutability
* Rebuilt only on new Merkle root

---

## 8. Performance Boundaries

| Problem                   | Requirement                      |
| ------------------------- | -------------------------------- |
| Very large monorepo       | Shard by root namespace          |
| Massive dependency chains | Incremental closure computation  |
| Frequent small edits      | Cheap structural diff            |
| Large refactor            | Parallel subtree rewrite         |
| UI responsiveness         | Async lazy hydration             |
| Concurrent views          | View-specific projection caching |

---

## 9. View Hierarchy Model (Condensed)

```
Human Intent
   ↓
View Layer (Workflow/Dataflow/Component)
   ↓
Semantic Operation Graph
   ↓
Typed StructuralDelta<T>
   ↓
Persistent Merkle AST
   ↓
Content Addressed Storage
```

All downward transitions must be:

* Deterministic
* Type safe
* Referentially valid
* Construction-time validated

---

## 10. Required Core Engines

| Engine                     | Responsibility                  |
| -------------------------- | ------------------------------- |
| Graph Store Engine         | Persistent Merkle DAG storage   |
| Symbol Resolution Engine   | Global namespace + reverse refs |
| Subgraph Extraction Engine | Minimal slice builder           |
| View Projection Engine     | Multi-layer graph derivation    |
| Intent Translation Engine  | UI → delta compiler             |
| Delta Algebra Engine       | Merge & commutativity rules     |
| Snapshot Engine            | Version checkpoints             |
| Time Replay Engine         | Efficient event reconstruction  |

---

## 11. Risk Areas

| Risk                         | Mitigation                          |
| ---------------------------- | ----------------------------------- |
| View drift from AST          | Deterministic derivation only       |
| Symbol explosion             | Namespacing + sharding              |
| Slow large slice extraction  | Precomputed dependency indexes      |
| Memory pressure              | Streaming subtree hydration         |
| Cross-language heterogeneity | Unified intermediate semantic model |
| UI abstraction mismatch      | Typed intent compiler               |

---

## 12. Final Architectural Shape

This is effectively: **A multi-layer, view-driven, content-addressed structural graph OS**.

Key properties:

* Immutable Merkle DAG core
* Deterministic subgraph extraction
* Multi-view projection system
* Intent-to-delta compiler
* Cached hierarchical graph overlays
* Lazy hydration
* Snapshot-time replay
* Zero file-level exposure to user

---
