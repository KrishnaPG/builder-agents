//! Scenario Plan: Sketchpad Mode Rapid Prototype
//!
//! Blueprint anchors:
//! - Mode A: Sketchpad – 04-agent-model.md §7 / Mode A.
//! - No Direct IO and artifact model – 01-intro.md §3.1; 04-agent-model.md §5.1.
//!
//! High-level story:
//! - A developer or product engineer wants to quickly prototype a new
//!   feature or idea (e.g., a new API endpoint or UI flow) without the full
//!   rigor of Factory mode, but still within safe-by-construction bounds.
//! - Sketchpad favors speed and lighter validation while preserving core
//!   invariants: no direct IO, artifact system, and basic security checks.
//!
//! Representative user input:
//! - Intent:
//!   "Prototype a new `/beta-recommendations` endpoint that returns a fixed
//!    JSON payload so we can test the UX. Prioritize speed over tests."
//! - Context:
//!   - Existing service skeleton.
//!   - Mode: Sketchpad.
//!   - Autonomy level tuned for fast iteration.
//!
//! Expected system behavior:
//! 1. Intent parsing:
//!    - Goal: CreateNew Code (experimental).
//!    - Targets: API module(s) and router.
//!    - Acceptance: endpoint exists and returns specified payload; tests may
//!      be light but basic safety still maintained.
//! 2. Decomposition:
//!    - Task A: Add new route and handler via StructuralDelta<Code>.
//!    - Task B: Generate minimal smoke tests.
//!    - Task C: Wire up basic logging/metrics (optional).
//! 3. Graph construction:
//!    - Mode directives relax coverage requirements but keep:
//!      - No direct IO.
//!      - Single-writer.
//!      - Referential integrity.
//!    - Security pipeline configured in light mode (still scanning secrets).
//! 4. Execution:
//!    - Apply deltas to add endpoint and tests.
//!    - Run minimal tests and security checks.
//! 5. Output:
//!    - New endpoint artifacts and test artifacts.
//!    - Diagnostics showing that this is Sketchpad output (not production).
//!
//! Test plan dimensions:
//! - Mode behavior:
//!   - Same scenario in Factory mode would require much stronger test
//!     guarantees; here we assert that Sketchpad permits lighter tests
//!     while still enforcing invariants.
//! - Safety:
//!   - No direct IO violations.
//! - Regression hooks:
//!   - Minimal synthetic API where running the same intent under Sketchpad
//!     vs Factory yields different graph shapes / test requirements, which
//!     we can assert in tests.

