# coa-core

**Creator Orchestrator Agent** - Central intelligence for task decomposition.

## STRUCTURE

```
src/
├── coa.rs            # CreatorOrchestratorAgent
├── decomposition.rs  # TaskDecomposer intent → tasks
├── agent_pool.rs     # Agent lifecycle management
├── types.rs          # COA types (672 lines)
└── error.rs          # Error types and diagnostics
```

## KEY PATTERNS

### Intent Execution
```rust
let coa = CreatorOrchestratorAgent::new(COAConfig::new());
let intent = UserIntent::new("Create a function");
let result = coa.execute_intent(intent).await?;
```

### Task Decomposition
```rust
let decomposer = TaskDecomposer::new(intent, pool);
let tasks = decomposer.decompose().await?;
```

### Agent Pool
```rust
let mut pool = AgentPool::new(config);
let handle = pool.spawn(agent_spec).await?;
pool.send_to(&handle, message).await?;
```

## COMPLEXITY HOTSPOTS

- `types.rs` (672 lines): Dense type definitions
  - AgentSpec, Task, UserIntent, ExecutionResult
  - Constraint, AutonomyLevel, ResourceCaps
- `decomposition.rs` (442 lines): Decomposition logic
- `error.rs` (440 lines): Rich error diagnostics

## ANTI-PATTERNS

- **Skip autonomy checks**: Always respect AutonomyLevel
- **Resource over-allocation**: Respect ResourceCaps
- **Ignore diagnostics**: ConstructionError has SuggestedFix
