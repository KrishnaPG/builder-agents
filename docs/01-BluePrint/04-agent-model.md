# AGENT MODEL AND MODES

## 5. SYSTEM PRIMITIVES

1. Node (with encoded autonomy ceiling and resource bounds)
2. Edge (with typed data contract)
3. Directive (structural modifier)
4. Autonomy State (embedded in execution contract)
5. Time State
6. Meta-Agent (COA)
7. **Expansion Stub (for staged construction)**

All features must map to these.

---

## 6. AGENT MODEL (Runtime Agents)

Runtime agents are ephemeral constructs instantiated by COA.

Each agent must define:

1. Role Definition
2. Capability Scope
3. Memory Boundary (enforced by container primitive)
4. Execution Contract (with embedded escalation thresholds)
5. Logging Requirement
6. **Policy Token (construction-time requirement, embedded in ValidatedGraph)**

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
* Must pass schema validation (at construction time)
* Must pass policy validation (at construction time)
* Must register in execution graph (at construction time)
* Must be destroyed after lifecycle completion (contract-enforced)

Unbounded recursion prevented by construction-time recursion depth limits.

---

## 7. MODES (Directive Bundles)

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

[Back to Index](./01-intro.md) | [Previous: Security](./03-security.md) | [Next: Operations](./05-operations.md)
