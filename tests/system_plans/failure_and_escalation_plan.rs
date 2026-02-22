//! Scenario Plan: Failure, Escalation, and Recovery
//!
//! Blueprint anchors:
//! - Failure and recovery model – 05-operations.md §9.
//! - Metrics and governance – 05-operations.md §§10–11.
//! - Autonomy and escalation – 01-intro.md (system identity, invariants),
//!   04-agent-model.md (autonomy state).
//!
//! High-level story:
//! - A change attempts to violate security policy or fails tests repeatedly.
//! - The system must respect retry limits, trigger escalation, preserve
//!   history, and maintain a clean graph with no broken invariants.
//!
//! Representative user input:
//! - Intent:
//!   "Add a new admin-only endpoint for force-resetting user passwords."
//! - Context:
//!   - Security policy forbids this class of operation without special
//!     safeguards.
//!   - Mode: Factory.
//!   - Autonomy level initially high but bounded by policy.
//!
//! Expected system behavior:
//! 1. Intent parsing:
//!    - Goal: CreateNew security-sensitive Code.
//!    - Targets: auth / user management modules.
//!    - Acceptance: must pass strict security pipeline stages.
//! 2. Decomposition:
//!    - Task A: Propose design and security model.
//!    - Task B: Implement code with required checks.
//!    - Task C: Generate targeted security tests.
//!    - Task D: Run security pipeline stages (e.g., secret scanning,
//!      policy enforcement).
//! 3. Failure path:
//!    - Security pipeline rejects the change (e.g., violates policy).
//!    - Retry loop:
//!      - Up to 3 attempts (per execution contract) to revise proposal.
//!      - After 3 failures, escalation contract triggers.
//!    - Escalation:
//!      - Human notified.
//!      - Graph state preserved.
//!      - No unsafe deltas applied to production graph.
//! 4. Recovery and history:
//!    - Revert generates a new branch instead of mutating history.
//!    - Knowledge graph records failed attempts and reasons.
//! 5. Output:
//!    - Final state may be:
//!      - Rejected change with escalation record, or
//!      - Safe alternative implementation that passes security.
//!    - Metrics updated: autonomy intervention rate, escalation frequency,
//!      policy override attempts.
//!
//! Test plan dimensions:
//! - Failure handling:
//!   - Synthetic scenario where security pipeline is guaranteed to reject
//!     a naive implementation.
//!   - Verify retry limit honored and escalation triggered.
//! - Safety:
//!   - No unsafe code or deltas applied to production artifacts after
//!     repeated failures.
//! - Observability:
//!   - Verify metrics and knowledge graph records for failed attempts and
//!     escalation events.
//! - Regression hooks:
//!   - Stable scenario where changing the policy or retry behavior should
//!     surface as failing tests, giving a long-term safety regression
//!     signal.

