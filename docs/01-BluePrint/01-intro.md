# COGNITIVE OPERATING SYSTEM

## Foundational Blueprint v2.2

### Safe-by-Construction Architecture with Typed Dynamic Expansion

This document is normative. Engineers and AI agents must treat it as specification, not inspiration.

This document defines:

* System identity
* Constitutional invariants
* Execution model
* Artifact system (TypedTree)
* Output and referential integrity guarantees

For complete specification, see:

* [02-architecture.md](./02-architecture.md) - System Layers, Execution Engine, Cognitive Orchestration, Dynamic Graph Expansion
* [03-security.md](./03-security.md) - Security Pipeline and mandatory stages
* [04-agent-model.md](./04-agent-model.md) - System Primitives, Artifact Model, Agent Model, and Modes
* [05-operations.md](./05-operations.md) - State Flow, Failure Recovery, Knowledge Graph, Metrics, and Release Criteria
* [06-composition-strategies.md](./06-composition-strategies.md) - Pluggable Conflict Resolution (SingleWriter, CommutativeBatch, OrderedComposition)

Any deviation requires formal revision control.

---

# 1. SYSTEM IDENTITY

This system is a **Cognitive Operating System driven by a Single Programmable Orchestrator**.

It combines:

* Deterministic modular execution pipelines
* Visual spatial orchestration
* Adjustable autonomy
* Full temporal traceability
* Safe-by-construction graph assembly
* **Structural artifact model (not text/files)**
* Research sandbox capability
* Policy-bound runtime agent generation

The human role is Architect and Supervisor.

The system contains only one persistent cognitive intelligence:

## Creator-Orchestrator Agent (COA)

All other agents are:

* Dynamically generated
* Policy-bound
* Ephemeral
* Destroyed after task completion

No static multi-agent swarm is shipped.

The COA is programmable but constitutionally constrained.

---

# 2. CORE PRINCIPLES (NON NEGOTIABLE)

1. Determinism precedes autonomy.
2. Every state transition must be logged.
3. All work is decomposed into atomic micro tasks.
4. Autonomy is adjustable per node (encoded in node type).
5. Time is a first class system dimension.
6. Context loading must be minimal and task scoped.
7. All agent reasoning must be inspectable.
8. Escalation thresholds are embedded in execution contracts (not checked at runtime).
9. The system must remain operable under partial failure.
10. Only the COA may instantiate runtime agents.
11. Runtime agents must be ephemeral and policy-bound.
12. Constitutional invariants are immutable and cannot be modified by COA.
13. Graphs must be safe by construction; no runtime governance validation permitted.
14. All policy enforcement occurs at graph construction time; execution follows proven-safe structure.
15. Dynamic graph expansion requires staged construction: expansion output is typed as subgraph specification, validated before execution.
16. Agents produce structural deltas, not text; artifacts are typed trees, not files.
17. Output integrity and referential integrity are guaranteed by construction, not checked at runtime.

If any feature violates these, it is invalid.

---

# 3. ARTIFACT SYSTEM FOUNDATION

## 3.1 First Principle: No Direct IO + A File Is Not a String

**Agents never interact with raw files, text buffers, or byte streams.**

**Agents cannot CRUD files. They can only propose changes.**

See [04-agent-model.md](./04-agent-model.md#51-no-direct-io-model) for the complete No Direct IO Model.

The system operates on **Artifacts** - structured, typed representations:

| External Form                    | Internal Artifact                 | Type               |
| -------------------------------- | --------------------------------- | ------------------ |
| Code files (`.ts`, `.rs`, `.py`) | AST + Symbol table + Module graph | `Artifact<Code>`   |
| Config files (`.yaml`, `.json`)  | Schema-validated tree             | `Artifact<Config>` |
| Spec documents (`.md`)           | Structured document model         | `Artifact<Spec>`   |
| Binary assets                    | Content hash + Metadata           | `Artifact<Binary>` |

**Agents do not write files. They compute StructuralDelta<T>.**

The **Constitutional Application Layer** handles:
- Parsing external files into Artifacts (ingress)
- Validating and applying StructuralDeltas
- Serializing Artifacts to external format (egress)

## 3.2 Output Integrity (Impossible by Construction)

**The Problem**: Multiple tasks writing to the same file → race conditions, data loss.

**The Solution**: Single-writer invariant enforced at graph construction time.

```
Graph Construction REJECTS if:
- Two TaskNodes both target the same Artifact
- Target symbol is not unique in namespace
```

This makes conflicting writes **structurally impossible**, not merely detected.

## 3.3 Referential Integrity (Impossible by Construction)

**The Problem**: Spec changes, tests don't update → false positives.

**The Solution**: Symbolic references with construction-time resolution.

```
Cross-artifact references use SymbolRef (not paths):
  SymbolRef { crate: "auth", module: "login", symbol: "validateToken" }

Graph Construction REJECTS if:
- Delta references SymbolRef that does not exist
- Referenced symbol was removed by another delta
- Spec artifact changed but derived artifacts not regenerated
```

This makes broken references **structurally impossible**.

## 3.4 Semantic Coupling

The system maintains explicit derivation chains:

```
Spec Artifact ──derives──► Code Artifact ──produces──► Output
       │                        │
       └──derives──► Test Artifact ───validates───────┘
```

When Spec changes:
1. Spec Artifact hash changes
2. Derivation edges break (construction-time detection)
3. Graph reconstruction required
4. COA must regenerate Code and Test from new Spec

**Spec-code-test drift is impossible by construction.**

---

# Document Index

| Document                                                       | Contents                                                                            |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [01-intro.md](./01-intro.md)                                   | System Identity, Core Principles, Artifact System (this file)                       |
| [02-architecture.md](./02-architecture.md)                     | System Layers, Execution Engine, Cognitive Orchestration, Dynamic Graph Expansion   |
| [03-security.md](./03-security.md)                             | Security Pipeline (Mandatory Stages)                                                |
| [04-agent-model.md](./04-agent-model.md)                       | System Primitives, Artifact Model, Agent Model, Modes, Output/Referential Integrity |
| [05-operations.md](./05-operations.md)                         | State Flow, Failure/Recovery, Knowledge Graph, Metrics, Minimum Requirements        |
| [06-composition-strategies.md](./06-composition-strategies.md) | Conflict Resolution Strategies for Multi-Agent Composition                          |

---

# END STATE

When implemented correctly, the system is:

* Deterministic
* Spatially observable
* Behaviorally composable
* Autonomy adjustable
* Temporally reversible
* Safe by construction
* Orchestrator driven
* Constitutionally constrained
* Capable of dynamic expansion without runtime governance
* Structurally immune to output conflicts and reference drift

This document set defines the build boundary.

Anything outside it requires formal revision control.
