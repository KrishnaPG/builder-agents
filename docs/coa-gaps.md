- The code in `/crates` is **architecturally very close** to the Blueprint v2.2; most of the critical concepts (Artifacts, StructuralDelta, SingleWriter strategies, ValidatedGraph, two‑phase architecture, typed expansion, hash‑chain logs) are present and shaped as specified.
- However, **it is not fully compliant** with the blueprint yet:
  - Many key methods are placeholders (`parse_ingress`, `apply_delta`, `compose`, `execute_task`, dynamic expansion merge, security pipeline completeness).
  - Some blueprint‑critical invariants (mandatory security pipeline, full dynamic expansion, full COA→agent→delta→ConstitutionalLayer integration) are not enforced in working code.

Below is a section‑by‑section compliance review.

---

**Implementation Checklist (Ordered by Invariant Unblocking)**

- **Implement ConstitutionalLayer ingress parsing (No Direct IO + Typed Artifacts):** Wire `parse_ingress` to `ParserRegistry` and type-erased parser dispatch so files become `Artifact<T>` instead of returning a parser error. See [layer.rs](../crates/coa-constitutional/src/layer.rs#L75-L145) and [parsers/mod.rs](../crates/coa-constitutional/src/parsers/mod.rs#L22-L154).
- **Implement artifact transformers + apply_delta (StructuralDelta<T> application):** Add transformer registry and real delta application so StructuralDelta is executable (no placeholder errors). See [layer.rs](../crates/coa-constitutional/src/layer.rs#L147-L173) and [delta.rs](../crates/coa-artifact/src/delta.rs#L11-L172).
- **Implement composition compose for all strategies (Output Integrity by construction):** Replace placeholder errors in SingleWriter/Ordered/Hybrid with real composition using ConstitutionalLayer transforms. See [single_writer.rs](../crates/coa-composition/src/single_writer.rs#L71-L109), [ordered.rs](../crates/coa-composition/src/ordered.rs#L95-L108), and [hybrid.rs](../crates/coa-composition/src/hybrid.rs#L211-L232).
- **Wire COA execution pipeline end-to-end (Agents produce deltas; ConstitutionalLayer applies):** Connect agent task execution to produce `StructuralDelta<T>`, validate via `CompositionStrategy`, and apply via ConstitutionalLayer. The current COA path is stubbed at execution. See [coa.rs](../crates/coa-core/src/coa.rs#L222-L239).
- **Implement serializer egress (Artifact → file):** Add serializer registry and actual serialization to complete ingress/transform/egress loop. See [layer.rs](../crates/coa-constitutional/src/layer.rs#L211-L229).
- **Implement security pipeline completeness validation (Mandatory stages):** Replace placeholder `has_security_pipeline` with directive/graph checks enforcing required stages (static scan, dependency scan, secrets detection, API validation) at construction time. See [validator.rs](../crates/coa-kernel/src/construction/validator.rs#L185-L211).
- **Implement dynamic expansion merge + re-validation (Typed Dynamic Expansion):** Merge validated subgraph into the running validated graph and track expanded nodes; complete `provide_expansion` and `complete_expansion`. See [expansion/mod.rs](../crates/coa-kernel/src/expansion/mod.rs#L127-L215).
- **Ensure symbol index is populated from parsed artifacts (Referential Integrity):** Populate `SymbolRefIndex` from `Artifact<Code/Spec>` so SymbolRef resolution and overlap detection are meaningful beyond empty-index checks. See [index.rs](../crates/coa-symbol/src/index.rs#L101-L175).
- **Enforce output integrity at construction via actual SymbolRef claims:** Integrate symbol claims from agent deltas with `SymbolRefIndex` at GraphBuilder/ConstructionValidator time, not just at composition time. See [validation.rs](../crates/coa-symbol/src/validation.rs#L25-L99).
- **Integrate runtime execution with real node work (Execution invariants on concrete tasks):** Replace DefaultNodeExecutor stub with real task execution, wired to COA task graph, while preserving zero runtime policy validation. See [executor/mod.rs](../crates/coa-kernel/src/executor/mod.rs#L180-L203).
- **Add knowledge-graph governance types and states (Operations spec):** Implement Draft/Verified/Deprecated states, provenance, and audit metadata required by the operations blueprint. No current implementation found in `/crates`.
- **Add metrics collection and immutability (Operations spec):** Track metrics listed in the blueprint (deployment time, autonomy intervention, context leakage, etc.) and persist them immutably. No current implementation found in `/crates`.
- **Add mode directive bundles (Sketchpad/Factory/Multiverse/Renovator):** Implement mode configurations and graph-rebuild enforcement when modes change. No current implementation found in `/crates`.

---

**1. Artifact System & No Direct IO**

Blueprint (01‑intro.md + 04‑agent-model.md):

- Agents never touch files; they operate on `Artifact<T>` and produce `StructuralDelta<T>`.
- Artifact types: Code, Config, Spec, Binary.
- Artifacts are content‑addressed, immutable, symbol‑based (no path‑level addressing).

Implementation:

- **Artifact types implemented as TypedTree‑style containers:**
  - Core trait and container: [artifact.rs](../crates/coa-artifact/src/artifact.rs#L10-L210)
    - `ArtifactType` is sealed, type‑safe, and content‑addressed.
    - `Artifact<T>` enforces `hash == T::hash(content)` and immutability.
  - Concrete types: [types/mod.rs](../crates/coa-artifact/src/types/mod.rs#L1-L18)
    - `CodeArtifact`, `ConfigArtifact`, `SpecArtifact`, `BinaryArtifact` match the blueprint’s table.

- **StructuralDelta<T> implemented as semantic, not text, operations:**
  - [delta.rs](../crates/coa-artifact/src/delta.rs#L11-L148)
    - Carries `target: SymbolPath`, `operation: DeltaOperation<T>`, `base_hash`, `order`.
    - Operations (`Add`, `Remove`, `Replace`, `Transform`) are structural, not line/byte edits.

- **Symbolic addressing & referential integrity:**
  - Symbol paths and symbol index:
    - `SymbolPath` API: [path.rs](../crates/coa-artifact/src/path.rs#L86-L105)
    - `SymbolRefIndex` with overlap detection: [index.rs](../crates/coa-symbol/src/index.rs#L12-L28,L101-L175)
  - Single‑writer validation via `SingleWriterValidator`:  
    [validation.rs](../crates/coa-symbol/src/validation.rs#L10-L72)

- **No Direct IO / Constitutional Layer boundary:**
  - ConstitutionalLayer is the **only** component accessing the filesystem:
    - [layer.rs](../crates/coa-constitutional/src/layer.rs#L37-L54)
    - Comments explicitly mirror the “No Direct IO” blueprint language.
  - Agents, as represented in `coa-core` (`CreatorOrchestratorAgent`, `AgentPool`), do not open files directly; they work with tasks, specs, and deltas.

Status:

- **Architecturally compliant.**  
- **Incomplete in practice:** core methods are stubs:
  - `ConstitutionalLayer::parse_ingress` reads the file but then returns `ParseError::ParserError("type-specific parsing not yet implemented")` instead of actually parsing via the registry.
  - `apply_delta` returns `ApplyError::NoTransformer`, no real structural transforms.
  - `serialize_egress` always returns `SerializeError::NoSerializer`.

So the artifact model matches the blueprint **by type and API**, but the end‑to‑end IO pipeline (file → artifact → delta → artifact → file) is not fully implemented yet.

---

**2. Structural Deltas & Composition Strategies (Output Integrity)**

Blueprint (04‑agent-model.md §8, 06‑composition-strategies.md):

- Single‑writer and pluggable strategies:
  - `SingleWriterStrategy` (default, subtree‑granularity).
  - `CommutativeBatchStrategy`, `OrderedCompositionStrategy`, `HybridCompositionStrategy`.
- Construction‑time enforcement of output integrity using these strategies.

Implementation:

- **CompositionStrategy trait exactly as specified:**
  - [strategy.rs](../crates/coa-composition/src/strategy.rs#L10-L51)
    - `validate`, `compose`, `parallelism`, `granularity`, `name` implemented.

- **SingleWriterStrategy:**
  - [single_writer.rs](../crates/coa-composition/src/single_writer.rs#L1-L12,L23-L55,L71-L111)
  - Uses `SingleWriterValidator` and `SymbolRefIndex` to ensure disjoint targets (subtree overlap checks).
  - `validate` is implemented and returns metadata and cost estimates.

- **Other strategies (Ordered, Hybrid) follow the spec shape:**
  - [ordered.rs](../crates/coa-composition/src/ordered.rs#L1-L11,L95-L108)
  - [hybrid.rs](../crates/coa-composition/src/hybrid.rs#L211-L232)
  - Validation logic exists (ordering constraints, partitioning of commutative vs ordered deltas).

Critical gap:

- All composition strategies have **placeholder `compose` implementations**:
  - `SingleWriterStrategy::apply_single` returns a `CompositionFailed` error complaining it “requires ConstitutionalLayer”.
  - `OrderedCompositionStrategy::apply_sequential` and `HybridCompositionStrategy::compose` similarly return placeholder `CompositionFailed` errors.
- `ConstitutionalLayer::apply_deltas` calls `strategy.validate()` then `strategy.compose()`, but since compose is stubbed, **no real multi‑delta application works** in practice.

Status:

- **Validation side of output‑integrity invariant is implemented and matches the blueprint.**
- **Composition/application side is not implemented**, so the “impossible by construction” guarantee isn’t actually realized end‑to‑end yet.

---

**3. Two‑Phase Architecture & Zero Runtime Policy**

Blueprint (02‑architecture.md):

- Strict separation:
  - Construction Phase: GraphBuilder + ConstructionValidator → ValidatedGraph + ValidationToken.
  - Execution Phase: Executor consumes ValidatedGraph, **no policy validation**, only integrity and primitive enforcement.
- Invariants like DAG integrity, resource bounds, autonomy ceilings are construction‑time only.

Implementation:

- **Construction Phase:**
  - Graph building:
    - [builder.rs](../crates/coa-kernel/src/construction/builder.rs#L32-L49,L96-L146,L207-L229)
      - Enforces DAG structure for `GraphType::ProductionDAG` (cycle detection).
      - Produces `ValidatedGraph` via `validate(signing_key)`.
  - Construction validator:
    - [validator.rs](../crates/coa-kernel/src/construction/validator.rs#L30-L47,L55-L94)
      - `validate_graph`:
        - Validates graph structure, autonomy ceilings, resource bounds (via `ResourceProof::verify_bounds`).
        - Issues capability tokens per node.
        - Creates `ValidationToken` (hash + signature + expiry).
  - ValidatedGraph:
    - [types/v2.rs](../crates/coa-kernel/src/types/v2.rs#L103-L141,L143-L183)
      - Sealed type created only via validator, holds `graph_id`, `validation_token`, `node_tokens`, etc.

- **Execution Phase:**
  - Executor:
    - [executor/mod.rs](../crates/coa-kernel/src/executor/mod.rs#L1-L13,L44-L52,L84-L139)
      - Consumes `ValidatedGraph`.
      - Verifies validation token (expiry, binding) and per‑node capability tokens.
      - Runs nodes with a `NodeExecutor` and aggregates `ExecutionSummary`.
      - Explicitly states and enforces “zero policy validation”; runtime checks are only resource limits and token integrity.
  - Resource enforcement:
    - `ResourceContainer` enforces CPU, memory, token, iteration limits (primitive enforcement, not policy validation).

- **Simulator & tests explicitly check the “zero runtime policy” invariant:**
  - [test_harness/simulator.rs](../crates/coa-kernel/src/test_harness/simulator.rs#L123-L163,L413-L426)
    - Tracks `runtime_policy_validation_count` and asserts it stays at 0.

Status:

- **Architecture very closely matches the blueprint.**
- Minor gaps are mostly in execution behavior (default `NodeExecutor` is a stub that just returns success). For the invariant itself (no runtime policy validation), the implementation is compliant.

---

**4. Dynamic Graph Expansion (Typed Dynamic Expansion)**

Blueprint (02‑architecture.md §3.2, 04‑agent-model.md §8.2, 05‑operations.md minimum requirements):

- Dynamic expansion via staged construction:
  - Expansion stub nodes declare expansion capability.
  - Expansion fragment is validated before attachment.
  - Typed subgraph specification via `ExpansionSchema`.

Implementation:

- **Types match spec exactly:**
  - [types/v2.rs](../crates/coa-kernel/src/types/v2.rs#L14-L31,L64-L77,L185-L217,L233-L256)
    - `NodeSpecV2` has optional `expansion_type: Option<ExpansionType>`.
    - `ExpansionType` holds `schema_type_id`, `max_subgraph_resources`, `max_expansion_depth`.
    - `SubgraphSpec<T: ExpansionSchema>` and trait `ExpansionSchema` match the Blueprint’s typed expansion schema.

- **Expansion mechanics:**
  - [expansion/mod.rs](../crates/coa-kernel/src/expansion/mod.rs#L1-L20,L43-L72,L127-L215,L266-L308)
    - `StagedConstruction` manages expansion stack and current graph.
    - `provide_expansion<T: ExpansionSchema>`:
      - Validates schema (`T::validate_subgraph`) and resource budget.
      - Validates autonomy propagation.
      - Prepares expansion frame.
    - `ExpansionBuilder` extension on `GraphBuilder::add_expansion_node` attaches `ExpansionType` to `NodeSpecV2`.

Critical gaps:

- `provide_expansion` and `complete_expansion` contain **TODOs**:
  - They do not actually merge the subgraph into the main graph or re‑validate the merged graph.
- `is_expanded` is a stub always returning `false`.

Status:

- **Types and validation interface are compliant; behavior is partial.**  
  The “typed dynamic expansion” requirement is structurally in place but not fully implemented.

---

**5. Agent Model, Autonomy, Logging & Test Harness**

Blueprint (04‑agent-model.md, 02‑architecture.md §Context Isolation, 05‑operations.md):

- Runtime agents are ephemeral, policy‑bound, autonomy‑limited.
- Autonomy ceilings embedded in node types; escalation thresholds in execution contracts.
- All state transitions logged in a hash‑chain log; logs are append‑only and integrity‑verifiable.
- Test harness and simulator verify invariants.

Implementation:

- **Agent lifecycle & autonomy:**
  - COA:
    - [coa.rs](../crates/coa-core/src/coa.rs#L23-L49,L67-L93,L167-L201,L213-L239)
      - Defines `CreatorOrchestratorAgent` with config, symbol index, agent pool, decomposer.
      - `execute_intent` → parse spec → decompose → execute tasks through agents.
      - `execute_task` is currently a stub returning `AgentFailed("Task execution not fully implemented")`.
  - AgentPool:
    - [agent_pool.rs](../crates/coa-core/src/agent_pool.rs#L1-L7,L48-L82,L94-L107,L122-L177,L179-L233)
      - Manages acquisition/release/shutdown of agents, with `AgentMessage` and `AgentResponse` enums.
  - Autonomy levels and thresholds:
    - In `coa-core`: [types.rs](../crates/coa-core/src/types.rs#L167-L218) defines `AutonomyLevel` and escalation thresholds.
    - In kernel: [types/mod.rs](../crates/coa-kernel/src/types/mod.rs#L23-L44,125-141) defines `AutonomyLevel` and `AutonomyCeiling`.

- **Logging with hash‑chained events:**
  - [logging/mod.rs](../crates/coa-kernel/src/logging/mod.rs#L7-L24,L25-L53,L56-L70)
    - `EventLog` with `prev_hash` / `hash` and `verify_integrity()`.
  - Tests verify integrity:
    - [tests/log_tests.rs](../crates/coa-kernel/tests/log_tests.rs#L1-L37)

- **Test harness & simulator:**
  - [test_harness/mod.rs](../crates/coa-kernel/src/test_harness/mod.rs#L1-L10)
  - [test_harness/simulator.rs](../crates/coa-kernel/src/test_harness/simulator.rs#L123-L163,L413-L426)
    - Explicitly models construction/execution, tracks violations, and checks zero runtime policy validation.

Status:

- **Conceptually compliant** (autonomy, logging, invariants, testing harness all exist).
- **Execution path from COA → agents → StructuralDelta<T> → ConstitutionalLayer → kernel executor is not wired up.**
  - Agents don’t yet produce real deltas.
  - COA doesn’t call `ConstitutionalLayer`.
  - Kernel executor is not integrated with COA; it runs abstract nodes, not concrete Artifact operations.

---

**6. Security Pipeline**

Blueprint (03‑security.md):

- Security pipeline is encoded as **mandatory stages** in the graph.
- Validation at construction time must ensure:
  - Presence of security analysis stage.
  - Tools and scan depth configured.
  - Secrets detection mandatory even in Sketchpad mode.

Implementation:

- Security is acknowledged at construction validator level, but actual checks are stubbed:
  - [validator.rs](../crates/coa-kernel/src/construction/validator.rs#L55-L73,L185-L211)
    - `validate_node_specs` mentions “security pipeline completeness”.
    - `has_security_pipeline` currently just returns `true` with TODO comment.

Status:

- **Not yet compliant.**
  - There is no concrete graph schema or directives enforcement ensuring a “Security Analysis Stage” node between code generation and tests.
  - No tool selection, scan depth, or secrets‑detection enforcement at the graph level.
- The architecture anticipates this (via directives and node specs) but the critical, “mandatory pipeline stages” invariant is not implemented.

---

**7. Operations, Knowledge Graph, Metrics, Modes & UI**

Blueprint (05‑operations.md, 02‑architecture.md §Neural Graph, 04‑agent-model.md §Modes):

- Requirements:
  - Knowledge graph governance (states: Draft, Verified, Deprecated).
  - Metrics (mean time to safe deployment, autonomy intervention rate, etc.).
  - Modes: Sketchpad, Factory, Multiverse, Renovator, with directive bundles.
  - Neural Graph UI, diff stream, time scrubber, research sandbox, etc.

Implementation in `/crates`:

- These crates are largely **backend infrastructure** (artifacts, composition, kernel, COA).  
- I did not find:
  - Knowledge graph data structures or governance logic.
  - Metrics aggregation code matching the blueprint’s metrics list.
  - Mode enums (`Sketchpad`, `Factory`, `Multiverse`, `Renovator`) or directive bundles wired into kernel behavior.
  - Neural Graph / UI components (expected, as likely out of scope of `/crates`).
  - Research sandbox logic beyond some generic support concepts.

Status:

- For these operational and UX‑oriented parts, **the blueprint defines future system behavior**, but the `/crates` implementation is either out of scope or not yet present. There is no conflicting implementation, but also no realization yet.

---

**8. Summary of Key Gaps vs Blueprint**

To make the `/crates` implementation fully compliant with the Blueprint:

- **Constitutional Layer**
  - Implement `parse_ingress` using `ParserRegistry` and per‑type parsers (Code, Config, Spec, etc.).
  - Implement transformer registry and real `apply_delta`/`apply_deltas` logic for `StructuralDelta<T>`.
  - Implement serializer registry and real `serialize_egress`.

- **Composition Strategies**
  - Implement `compose` for `SingleWriterStrategy`, `OrderedCompositionStrategy`, and `HybridCompositionStrategy` using ConstitutionalLayer transforms.
  - Ensure composition is invoked in actual COA flows, not just tests/helpers.

- **COA Integration**
  - Wire COA task execution so that agents:
    - Receive `Artifact<Spec>` and dependent artifacts.
    - Produce `StructuralDelta<T>` outputs.
    - Send deltas through ConstitutionalLayer + composition strategies to produce new artifacts.

- **Dynamic Expansion**
  - Implement graph merging and re‑validation in `StagedConstruction::provide_expansion` and `complete_expansion`.
  - Track expanded nodes in `is_expanded`.

- **Security Pipeline**
  - Implement `has_security_pipeline` to actually inspect `NodeSpecV2.directives` or graph structure for mandatory security stages.
  - Provide a schema to ensure `Code Generation → Security Analysis → Test Execution → Merge` structure is present for relevant workflows.

- **Operational Features (metrics, modes, knowledge graph)**
  - Add core types and enforcement for modes (Sketchpad / Factory / Multiverse / Renovator) and their directive bundles at kernel/COA level.
  - Implement metrics collection and reporting.
  - Add knowledge graph types and states, if they are expected to live in backend crates.

- **Research Publication Platform Specific Gaps**
  - Implement knowledge graph data structures for papers, authors, citations, concepts (Draft/Verified/Deprecated states).
  - Add academic document parsers (LaTeX, PDF, BibTeX, CSL) to ConstitutionalLayer.
  - Create research workflow agents (literature review, experiment design, analysis, figure/table generation).
  - Integrate scholarly infrastructure (arXiv, PubMed, Semantic Scholar APIs, reference managers).
  - Implement publication-specific composition strategies for combining research findings.
  - Add reproducibility tracking (experiment specification artifacts, result versioning, parameter sweeps).

- **coa-opencode Thin Wrapper Improvements (Completed)**
  - Removed hardcoded model/agent/skill lists from CLI mode
  - Eliminated agent/skill caching that duplicated opencode functionality
  - Made all operations proxy directly to opencode CLI/daemon
  - Removed reload() implementation (handled by opencode itself)
  - Now truly thin wrapper exposing all opencode API dynamically

---

