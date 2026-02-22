# SYSTEM ARCHITECTURE

## 3. SYSTEM LAYERS

---

## 3.1 Construction-Time Validation Layer (Immutable)

This layer validates graphs **before** they become executable. It does not exist at runtime.

### Policy Validation vs Integrity Verification

| Aspect                     | Construction Time  | Runtime                        |
| -------------------------- | ------------------ | ------------------------------ |
| **Policy Validation**      | "Is this allowed?" | **NOT PERFORMED**              |
| **Integrity Verification** | N/A                | "Has this been tampered with?" |
| **Primitive Enforcement**  | Bounds declared    | Bounds enforced via containers |

**Policy Validation** (Construction only):
- Autonomy ceiling compliance
- Resource bound proving
- Security pipeline completeness
- DAG integrity

**Integrity Verification** (Runtime only):
- Cryptographic token signature verification
- Token expiration checking
- Hash-chain log verification

**Primitive Enforcement** (Runtime):
- Container cgroups enforcing declared memory limits
- CPU time limits via process constraints
- State machine transition enforcement

### Validated at Construction Time

* Policy compliance (node types carry policy tokens)
* DAG integrity (enforced by graph insertion primitives)
* Resource bound provability (caps encoded in node types)
* Security pipeline completeness (mandatory stages, not optional checks)
* Autonomy ceiling compliance (encoded in node type, not checked at runtime)
* Agent contract schema validation

**Once validation passes, the graph is frozen. Execution proceeds with zero policy checks.**

COA cannot bypass this layer—all graphs must pass validation before execution token is granted.

### Two-Phase Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│              CONSTRUCTION PHASE (Policy Validation)              │
├─────────────────────────────────────────────────────────────────┤
│  GraphBuilder → ConstructionValidator → ValidatedGraph          │
│                                                                  │
│  • DAG cycle rejection at edge insertion                        │
│  • Autonomy ceiling encoded in NodeSpec                         │
│  • Resource bounds proven against system limits                 │
│  • Security pipeline verified complete                          │
│  • Capability tokens issued and bound to nodes                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ ValidationToken (cryptographic proof)
┌─────────────────────────────────────────────────────────────────┐
│              EXECUTION PHASE (Integrity Verification)            │
├─────────────────────────────────────────────────────────────────┤
│  Executor ← ValidatedGraph (proof-carrying)                     │
│                                                                  │
│  • Token signature verified (cryptographic integrity)           │
│  • State transitions deterministic (pre-defined contract)       │
│  • Container primitives enforce declared bounds                 │
│                                                                  │
│  NO "validate_action" calls. NO "check_policy" queries.        │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3.2 Layer 1: Execution Engine

Purpose: Deterministic production backbone executing pre-validated graphs.

---

### Meta Layer: Creator-Orchestrator Agent (COA)

The COA is the only persistent cognitive authority.

### Responsibilities

* Parse user intent
* Decompose into DAG micro-task graph
* Generate or select runtime agent templates
* Compose directive bundles dynamically
* Allocate autonomy levels per node (statically encoded)
* **Construct graphs via GraphBuilder that pass validation**
* **Provide subgraph specifications for dynamic expansion**
* Escalate to human when required (via embedded escalation contracts)
* Dissolve runtime agents after task completion
* Update knowledge graph within policy boundaries

### Constraints

COA cannot:

* Modify construction-time validation logic
* Modify constitutional invariants
* Modify hash-chain logic
* Elevate autonomy beyond policy ceilings (enforced by node type system)
* Inject undeclared dependencies (enforced by graph primitives)
* Write verified knowledge without validation
* Bypass construction validation to execute graphs

**COA actions are validated at construction time, not runtime.**

---

### A. Modular Kanban with TDD Loops

All projects are decomposed into atomic micro tasks.

Each micro task contains:

* Input specification
* Acceptance criteria
* Neighbor interface constraints
* **Autonomy ceiling (encoded in task type)**
* **Resource bound declaration (part of task type)**

Every task must:

* Generate tests first
* Implement code
* Pass tests before merge

---

### Context Isolation Contract (Mandatory)

Every micro task executes inside an isolated execution container.

* Container memory boundaries enforced by container primitives (not runtime checks)
* No hidden shared memory between nodes (enforced by graph topology)
* Agents may access only:

  * Task specification
  * Explicit neighbor interface schemas (type-checked at construction)
  * Approved stack dependencies (validated at construction)
  * Read-only verified knowledge

Knowledge Graph access:

* Read-only during execution
* Write permitted only in Research Sandbox

Maximum context window per task is explicitly configured in task type.

Cross-task state leakage:

* Critical violation
* Triggers escalation (via embedded escalation contract)
* Node freeze enforced by execution contract

This contract overrides convenience optimizations.

---

### B. Neural Graph

The UX displays software as a directed network graph.

Nodes:

* Module
* Task container
* Agent group
* **Expansion stub (declares dynamic expansion capability)**

Edges:

* Dependency
* API flow
* Data flow
* **Expansion dependency (parent-child relationship)**

Node states:

* Green: stable
* Red: failing tests
* Yellow: building
* Purple: escalation triggered (contract-driven, not runtime check)

Nodes pulse during active execution.

Graph Consistency Contract (enforced by construction primitives):

* Production branches must remain DAG (enforced at insertion)
* Cycles allowed only in Research Sandbox
* Edge modification requires graph reconstruction and revalidation
* Node deletion prohibited (nodes may be marked deprecated)
* Node deactivation preserves historical trace

---

### C. Stack Selector (Vending Machine Model)

Two modes:

1. Standard OSS templates;  e.g. User selects "Standard Web App" -> AI auto-selects proven OSS (React, Node, Postgres etc.).
2. Custom proprietary ingestion; e.g. User selects "Custom" and uploads/selects their proprietary libraries (from same or other projects/codebase). The AI ingest the docs and creates "Custom Agents" that specialize in those packages.

Custom ingestion must:

* Generate dependency manifest
* Validate license compliance (at construction time)
* Generate specialized runtime agent schema

Stack selection locks dependency boundaries per branch (enforced by type system).

Stack Boundary Enforcement:

* Stack version immutable per branch (enforced by graph primitives)
* Dependency change requires new branch (enforced by construction rules)
* Unauthorized runtime injection prevented by container isolation primitives

---

### D. Management & Debugging: "Black Box Recorder"

Since AI agents can get stuck in loops, the UX needs an **"Agent Tracer"**:
*   **Autonomous Debugging:** When an agent fails a test, it creates a "Bug Ticket" visible on the Node Graph. It attempts to fix it autonomously.
*   **Human Intervention:** If it fails 3 times, the node flashes Purple. The user clicks it and sees a "Context Diff", exactly what the agent tried to change. The user can manually approve a fix or type a hint ("Check the API key").

All agent actions must log:

* Prompt inputs
* Internal reasoning traces
* Code diffs
* Test outputs
* Stack versions
* Autonomy level (as encoded, not as checked)
* Directive state

Logs must be:

* Append-only
* Hash-chain verifiable

Log tampering invalidates deployment eligibility.

---

### E. Diff Stream

Live stream of:

* Code changes
* Config changes
* Test updates

Pause, inspect, rewind supported.

---

### F. Research Sandbox

Separate isolated compute environment.

Supports:

* Hypothesis input: 
* Jupyter-style execution
* Benchmark generation
* Chart generation
* Paper drafting

Knowledge Graph integration:

* Extract findings
* Persist with provenance
* Link concepts visually

Sandbox permits knowledge write operations.

---

## 3.3 Layer 2: Cognitive Orchestration (Living Spatial Canvas)

---

### Living Spatial Canvas

The interface is an infinite, zoomable topological map of the project. At a macro level, one sees high-level workflows (e.g., "Architecture Design" or "Commercialization Research"). Zoom in, and the nodes expand to reveal individual agents actively debating logic, mapping data schemas, or writing code in real-time.

Zoom levels:

1. Macro: workflows and system clusters
2. Meso: modules and pipelines
3. Micro: reasoning threads

Graph state must synchronize in real time with execution engine.

---

### Drag and Drop Directive Blocks

Users control the outcomes via context blocks rather than typing prompts. Need a fast proof-of-concept? Drag a "Speed/Prototype" block onto the swarm. Shifting to production? Drop a "Strict TDD & Security" block onto them, and watch the swarm immediately reconfigure its behavior.

Directives are behavioral modifiers applied to:

* Project
* Cluster
* Node

Examples:

* Speed Prototype
* Strict TDD
* Security Hardened
* Multiverse Compare
* Refactor Mode

Directives modify:

* Test coverage thresholds
* Merge gating rules
* Agent debate length
* Security scan depth (pipeline stage configuration)
* Documentation requirements

---

### Directive Precedence Model

Resolution order:

1. Node
2. Cluster
3. Project
4. Mode preset

If conflict remains:
   * Restrictive directive dominates permissive directive.
   * Security and governance directives encoded in graph structure.
   * Autonomy ceiling encoded in node type (cannot exceed organization policy).

All directive changes require:
  * Graph reconstruction
  * Re-validation
  * New execution token

---

### Dial of Autonomy

Every task node features a simple slider (Level 0 to Level 5). Set it low for strict Human-in-the-Loop (HITL) where agents pause for your approval on every major decision. Push it to max, and the swarm autonomously researches, codes, tests, and self-corrects in the background.

Levels:

0: Full HITL
1: Approval before merge
2: Auto code, human merge
3: Auto merge in sandbox
4: Auto merge + test deploy
5: Full autonomous within boundary

Level >3 requires:

* Organization policy token (compile-time requirement)
* Multi-factor human approval (recorded in construction log)

**Autonomy level is encoded in node type; no runtime check required.**

---

### Autonomy Escalation Rules (Embedded in Execution Contract)

Escalation conditions declared in node contract:

* After 3 test failures
* After 1 security violation
* After 1 autonomy violation

Escalation behavior (contract-driven):
* Autonomy reduces by 1 (state transition encoded in contract)
* Human notification triggered

Autonomy cannot auto-increase (enforced by state machine design).

---

### Time Lapse Scrubber

A global timeline slider at the bottom of the screen (like a video editor). If a build fails or a research hypothesis goes off track, scrub backward to watch the exact moment the agents' logic or architectural choices diverged, making debugging intuitive and visual.

Supports:

* Scrubbing backward
* Visual replay
* Autonomy inspection
* Directive comparison
* Code diff comparison

Scrubbing does not mutate history.

Revert creates new branch.

Must support minimum 10,000 event replay, with sub-second latency.

---

## 3.4 Dynamic Graph Expansion (Staged Construction)

Research and exploration tasks require dynamic planning where the graph expands based on intermediate outputs. This is supported through **typed staged construction**.

### Expansion Node Pattern

**Expansion Nodes** are statically typed nodes whose output is a **Subgraph Specification**:

```
Meta-Graph (statically validated at t=0)
├── Expansion Node (type: ExpandsTo<SubgraphSchema>)
│   └── Output: Subgraph Specification (produced at t=1)
├── Validation Gate (construction-time validation at t=1)
│   └── Produces: Validated Subgraph
└── Execution of Expanded Graph (at t=2)
```

**Critical Invariant**: Each validation gate is a **new construction phase**. The expansion subgraph must pass full policy validation before execution resumes.

### Expansion Type Safety

1. **Expansion Output Type**: Every expansion node declares `ExpandsTo<T>` where T is the subgraph schema type
2. **Resource Bound Propagation**: Parent graph declares maximum resources for all possible expansions
3. **Autonomy Inheritance**: Expanded subgraph nodes inherit autonomy ceiling from parent declaration
4. **DAG Preservation**: Expansion output must form valid DAG when spliced into parent graph
5. **Re-validation Required**: Expanded subgraph must pass construction validation before execution resumes

### Staged Construction Protocol

For dynamic graph expansion:

1. **Stage 1 - Meta-Graph Execution**: Execute up to expansion point
2. **Stage 2 - Subgraph Generation**: Expansion node produces subgraph specification
3. **Stage 3 - Construction-Time Validation**: 
   * Validate subgraph schema against `ExpandsTo<T>` type
   * Verify resource bounds
   * Verify autonomy ceilings
   * Verify security pipeline completeness
4. **Stage 4 - Graph Splicing**: Insert validated subgraph into execution graph
5. **Stage 5 - Continuation**: Resume execution with expanded graph

**Critical invariant**: No expansion executes without passing Stage 3 validation.

### Example: Scientific Hypothesis Exploration

```
Initial Graph (validated at t=0):
├── Benchmark Node (type: BenchmarkRun)
├── Analysis Node (type: ConditionalExpansion<ExpandsTo<ArchitectureBranch>>)
└── Merge Point (declared, awaits expansion)

At t=1 (benchmark fails):
├── Analysis Node outputs: SubgraphSpecification
│   ├── Alternative Architecture A
│   ├── Alternative Architecture B
│   └── Comparative Testing Framework

Stage 3 Validation (at t=1):
├── Validate all branches satisfy BenchmarkRun schema
├── Verify total resources (A + B + Framework) < parent declared max
├── Verify autonomy ceiling inherited correctly
└── Generate execution token

At t=2 (post-validation):
├── Expanded Graph executes (zero runtime checks)
├── Branch A and Branch B run in parallel (if resources permit)
└── Results feed into Merge Point
```

### Recursive Expansion

Expansion nodes may themselves contain expansion nodes (research branching). Each level:
* Must declare resource bounds for its subtree
* Must satisfy construction-time validation
* Cannot exceed recursion depth declared in root graph type

---

[Back to Index](./01-intro.md)
