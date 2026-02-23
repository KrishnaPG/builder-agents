# OPERATIONS AND GOVERNANCE

## 8. STATE FLOW CONTRACT

For each micro task:

1. Task created
2. Context isolation (enforced by primitives)
3. Test generation
4. Code generation
5. Test execution
6. Diff creation
7. Structural validation (construction-time)
8. Merge decision (autonomy level encoded in node type)
9. Deployment or sandbox run
10. Knowledge graph update

All policy validation occurs at step 7 (construction time).

Steps 8-10 execute with zero runtime governance checks.

Production branches may write metadata only.

Verified knowledge requires sandbox validation.

---

## 9. FAILURE AND RECOVERY MODEL

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

## 10. KNOWLEDGE GRAPH GOVERNANCE

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

## 11. METRICS

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

## 12. MINIMUM IMPLEMENTATION REQUIREMENTS

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
* Typed dynamic expansion (staged construction)

Partial implementations are prototypes.

---

[Back to Index](./01-intro.md) | [Previous: Agent Model](./04-agent-model.md)
