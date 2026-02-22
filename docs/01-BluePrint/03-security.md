# SECURITY PIPELINE

## 4. SECURITY PIPELINE (Mandatory Stages)

Security is enforced as **mandatory execution pipeline stages**, not optional runtime checks.

## 4.1 Policy vs Integrity in Security

| Aspect                     | Policy (Construction Time)                  | Integrity/Enforcement (Runtime)         |
| -------------------------- | ------------------------------------------- | --------------------------------------- |
| **Pipeline Structure**     | Security stages encoded in graph topology   | Stages execute as declared              |
| **Tool Selection**         | Security tools declared in ExecutionProfile | Tools run as specified                  |
| **Scan Depth**             | Scan depth configured at construction       | Scan executes to declared depth         |
| **Secrets Detection**      | Mandatory stage required in graph           | Pattern matching executes               |
| **Signature Verification** | N/A                                         | Cryptographic verification of artifacts |

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

[Back to Index](./01-intro.md) | [Next: Agent Model](./04-agent-model.md)
