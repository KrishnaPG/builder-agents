//! Scenario Plan: Factory Mode Code Change with Strict TDD
//!
//! Blueprint anchors:
//! - Mode B: Factory (strict TDD, 100% coverage) – 04-agent-model.md §7 / Mode B
//! - No Direct IO + Artifact system – 01-intro.md §3.1
//! - Output & referential integrity – 04-agent-model.md §8
//! - State flow contract – 05-operations.md §8
//!
//! High-level story:
//! - A developer wants to add or modify a feature in an existing service
//!   (e.g., add structured logging to HTTP handlers, or extend an endpoint).
//! - They operate in Factory mode, which enforces test-first behavior, full
//!   coverage, and mandatory security pipeline stages.
//! - COA orchestrates the full flow: intent → spec → tasks → kernel graph →
//!   construction-time validation → execution → artifact updates.
//!
//! Representative user input:
//! - Natural language intent:
//!   "Add structured request logging to all public HTTP handlers in `src/api`
//!    so we can trace requests end-to-end. Do not change response semantics."
//! - Context:
//!   - Existing codebase snapshot as Artifacts (Code + Config).
//!   - Mode: Factory (strict TDD).
//!   - Autonomy ceiling: L3.
//!   - Resource caps and timeouts suitable for a small feature.
//!
//! Expected system behavior (end-to-end):
//! 1. Intent parsing:
//!    - Goal classified as ModifyExisting Code.
//!    - Target region: src/api/* (symbol-based, not path-only).
//!    - Acceptance criteria: all public handlers call a logging primitive at
//!      entry, with no behavior change.
//! 2. Decomposition:
//!    - Task A: Analyze symbols in src/api to find public HTTP handlers.
//!    - Task B: For each handler, synthesize a StructuralDelta<Code> that
//!      injects logging without changing semantics.
//!    - Task C: Ensure logging utilities/imports exist or are added safely.
//!    - Task D: Generate or update tests to cover the new behavior.
//!    - Task E: Run tests + security checks; collect diagnostics.
//! 3. Graph construction:
//!    - Build a task graph with nodes for analysis, delta generation,
//!      composition, application, test generation, test execution, and
//!      security pipeline.
//!    - Enforce SingleWriterStrategy (no conflicting deltas per symbol).
//!    - Enforce autonomy/resource caps and mode-specific directives (Factory).
//!    - Reject graphs that skip required steps (e.g., tests or security).
//! 4. Execution (kernel):
//!    - Execute validated graph: analysis → delta creation → composition →
//!      delta application → tests → security.
//!    - No runtime policy validation; only integrity checks and container
//!      primitives are allowed at runtime.
//! 5. Output:
//!    - New code artifacts where each handler has logging injected.
//!    - Updated test artifacts achieving target coverage.
//!    - Diagnostics summarizing:
//!      - Number of handlers found/updated.
//!      - Test counts and pass/fail.
//!      - Security pipeline status.
//!    - Structural diff representation (AST/TypedTree), not raw text diff.
//!
//! Test plan dimensions:
//! - Correctness:
//!   - All targeted handlers updated; non-target code untouched.
//!   - Behavior preserved (responses unchanged for existing tests).
//!   - Graph rejects if required tests/security steps are missing.
//! - Coverage:
//!   - Ensure new/updated tests cover all injected logging paths.
//!   - Verify Factory mode refuses to complete if coverage is insufficient.
//! - Integrity:
//!   - No output integrity violations (single writer).
//!   - No referential integrity violations (SymbolRefs still resolve).
//! - Mode behavior:
//!   - Same intent run in non-Factory mode should allow looser behavior; this
//!     scenario specifically asserts the stricter Factory constraints.
//! - Regression hooks:
//!   - Stable assertions over:
//!     - Count of updated handlers.
//!     - Presence of logging constructs in specific locations.
//!     - Summary metrics (tests run, security stages executed).

