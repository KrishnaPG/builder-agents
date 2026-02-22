# COGNITIVE OPERATING SYSTEM

## Foundational Blueprint v2.1

### Safe-by-Construction Architecture with Typed Dynamic Expansion

This document is normative. Engineers and AI agents must treat it as specification, not inspiration.

This document defines:

* System identity
* Constitutional invariants
* Execution model
* Construction-time validation
* Orchestrator model
* Agent lifecycle
* Compliance rules (static)
* UX model
* Metrics
* Failure handling
* Knowledge governance
* Minimum release criteria

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
13. **Graphs must be safe by construction; no runtime governance validation permitted.**
14. **All policy enforcement occurs at graph construction time; execution follows proven-safe structure.**
15. **Dynamic graph expansion requires staged construction: expansion output is typed as subgraph specification, validated before execution.**

If any feature violates these, it is invalid.

---

# 3. SYSTEM LAYERS

---

## 3.1 Construction-Time Validation Layer (Immutable)

This layer validates graphs **before** they become executable. It does not exist at runtime.

### Policy Validation vs Integrity Verification

| Aspect | Construction Time | Runtime |
|--------|-------------------|---------|
| **Policy Validation** | "Is this allowed?" | **NOT PERFORMED** |
| **Integrity Verification** | N/A | "Has this been tampered with?" |
| **Primitive Enforcement** | Bounds declared | Bounds enforced via containers |

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

# 4. SECURITY PIPELINE (Mandatory Stages)

Security is enforced as **mandatory execution pipeline stages**, not optional runtime checks.

## 4.1 Policy vs Integrity in Security

| Aspect | Policy (Construction Time) | Integrity/Enforcement (Runtime) |
|--------|---------------------------|--------------------------------|
| **Pipeline Structure** | Security stages encoded in graph topology | Stages execute as declared |
| **Tool Selection** | Security tools declared in ExecutionProfile | Tools run as specified |
| **Scan Depth** | Scan depth configured at construction | Scan executes to declared depth |
| **Secrets Detection** | Mandatory stage required in graph | Pattern matching executes |
| **Signature Verification** | N/A | Cryptographic verification of artifacts |

## 4.2 Pipeline Structure

Pipeline structure (encoded in graph at construction time):

```
Code Generation Node
  ↓ [output - type-checked]
Security Analysis Stage (mandatory pipeline stage)
  ├── Static code analysis (tool execution)
  ├── Dependency scanning (tool execution)
  ├── License compliance (manifest validation)
  ├── Secrets detection (pattern matching)
  └── API contract validation (schema check)
  ↓ [output - type-checked]
Test Execution Stage
  ↓ [output - type-checked]
Merge Stage (structural, not decision-based)
```

**Key Distinction**: The pipeline structure is **validated at construction** (policy). The stages **execute as declared** at runtime (no "should we run security checks?" decision).

## 4.3 Sketchpad Mode

**Sketchpad mode** may configure lighter tool chains at construction time but cannot skip mandatory stages (enforced by pipeline schema validation).

---

# 5. SYSTEM PRIMITIVES

1. Node (with encoded autonomy ceiling and resource bounds)
2. Edge (with typed data contract)
3. Directive (structural modifier)
4. Autonomy State (embedded in execution contract)
5. Time State
6. Meta-Agent (COA)
7. **Expansion Stub (for staged construction)**

All features must map to these.

---

# 6. AGENT MODEL (Runtime Agents)

Runtime agents are ephemeral constructs instantiated by COA.

Each agent must define:

1. Role Definition
2. Capability Scope
3. Memory Boundary (enforced by container primitive)
4. Execution Contract (with embedded escalation thresholds)
5. Logging Requirement
6. **Policy Token (construction-time requirement, embedded in ValidatedGraph)**

## 6.1 Policy Token vs Runtime Integrity

| Aspect | Policy Token (Construction) | Runtime Verification |
|--------|----------------------------|---------------------|
| **Issuance** | Bound to node during GraphBuilder::validate() | N/A |
| **Contents** | Autonomy level, resource bounds, directive hash | N/A |
| **Verification** | N/A | Cryptographic signature check |
| **Expiration** | Declared at issuance | Checked at runtime |
| **Decision** | "What is this agent allowed?" (construction) | "Is this authentic?" (runtime) |

## 6.2 Agent Constraints

* Cannot self-elevate autonomy (state machine prevents this)
* Cannot persist memory outside container (enforced by primitives)
* Cannot instantiate other agents directly (must request through COA)
* Must pass schema validation (at construction time)
* Must pass policy validation (at construction time)
* Must register in execution graph (at construction time)
* Must be destroyed after lifecycle completion (contract-enforced)

Unbounded recursion prevented by construction-time recursion depth limits.

---

# 7. MODES (Directive Bundles)

Modes are directive bundles. Switching modes requires graph reconstruction and revalidation.

COA may synthesize mode composition dynamically.

---

## Mode A: Sketchpad

High speed, reduced coverage, minimal UI.

Security pipeline: Light configuration (but secrets detection still mandatory).

---

## Mode B: Factory

Strict TDD. 100 percent test coverage required.

Coverage includes:

* Line coverage
* Branch coverage
* Mutation coverage optional

Pipeline view:

Code → Security → QA → Deploy.

---

## Mode C: Multiverse

Parallel branches.

Rules:

1. Branch autonomy isolated (enforced by graph topology)
2. Knowledge graph shared read-only
3. Writes require branch labeling
4. Feature drag creates patch artifact
5. Final merge requires:

   * Cross-branch test validation (pipeline stage)
   * Directive reconciliation (graph construction)
   * Autonomy compliance (encoded in node types, no runtime check)

---

## Mode D: Renovator

Incremental rewrite with Live system continuity .

Must:

1. Maintain compatibility adapter
2. Route traffic through adapter
3. Replace modules incrementally
4. Prevent downtime

Heatmap overlay required.

---

# 8. STATE FLOW CONTRACT

For each micro task:

1. Task created
2. Context isolation (enforced by primitives)
3. Test generation
4. Code generation
5. Test execution
6. Diff creation
7. **Structural validation (construction-time)**
8. Merge decision (autonomy level encoded in node type)
9. Deployment or sandbox run
10. Knowledge graph update

**All policy validation occurs at step 7 (construction time).**

Steps 8-10 execute with zero runtime governance checks.

Production branches may write metadata only.

Verified knowledge requires sandbox validation.

---

# 9. FAILURE AND RECOVERY MODEL

Failure types:

* Test failure
* Security violation (detected at mandatory pipeline stage)
* Dependency conflict (detected at construction time)
* Infinite reasoning loop (prevented by construction-time iteration caps)
* Autonomy violation (escalation contract triggered)

Retry limit: 3 (encoded in execution contract).

Post limit: escalation contract triggers human notification.

Revert generates new branch.

History cannot be deleted.

---

# 10. KNOWLEDGE GRAPH GOVERNANCE

Knowledge nodes must contain:

* Source branch
* Timestamp
* Validation status
* Authoring agent

Validation states:

* Draft
* Verified
* Deprecated

Only Verified knowledge influences production.

Rollback preserves lineage.

Snapshots required.

Cross-project sharing requires approval.

---

# 11. METRICS

Must track:

* Mean time to safe deployment
* Test coverage ratio
* Autonomy intervention rate
* Escalation frequency
* Bug escape rate
* Recovery time using time scrub
* Knowledge graph growth rate
* Context load size per task
* Context leakage incidents
* Directive conflict frequency
* Deadlock occurrences
* Autonomy reduction events
* Policy override attempts
* Resource cap breaches

Metrics must be immutable and auditable.

---

# 12. MINIMUM IMPLEMENTATION REQUIREMENTS

v1 release must include:

* Neural Graph with live updates
* Micro task isolation engine
* TDD enforcement
* Autonomy dial (encoded in node types)
* Directive system
* Time scrubber
* Immutable logging
* Escalation contracts (embedded, not runtime checks)
* COA orchestration
* Construction-time validation layer
* **Typed dynamic expansion (staged construction)**

Partial implementations are prototypes.

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
* **Capable of dynamic expansion without runtime governance**

This document defines the build boundary.

Anything outside it requires formal revision control.
