# coa-kernel

**DAG-based orchestration engine** with safe-by-construction architecture.

## STRUCTURE

```
src/
├── dag/              # Graph construction and validation
├── executor/         # Async execution engine
├── construction/     # Builder pattern + validation
│   ├── builder.rs    # GraphBuilder
│   └── validator.rs  # Two-phase validation
├── token_integrity/  # Immutable execution tokens
├── validated_graph/  # Runtime graph representation
├── expansion/        # Graph expansion/reduction
├── scheduler/        # Node scheduling
├── state_machine/    # Execution state management
├── test_harness/     # Simulation and stress testing
├── api.rs            # Public API surface
└── main.rs           # CLI binary
```

## KEY PATTERNS

### Two-Phase Design
1. **Construction Phase**: Build and validate graphs
2. **Execution Phase**: Execute pre-validated graphs

```rust
let builder = GraphBuilder::new(GraphType::ProductionDAG);
let validated = builder.validate(&signing_key)?;
let result = Executor::new(verifying_key).run(validated).await?;
```

### Token Integrity
- `Token`: Immutable execution authorization
- `Signable`: Trait for cryptographic signing
- `TokenStore`: Persistence for token state

### Error Handling
- `KernelError`: Centralized error type
- `ConstructionError`: Graph building failures
- `ExecutionError`: Runtime failures

## ANTI-PATTERNS

- **Direct disk I/O**: Use artifact layer, not fs::read/write
- **Mutable graph after validation**: Graphs freeze at validation boundary
- **Skip validation**: Always validate before execution

## TESTING

```bash
# Run with nextest (recommended)
cargo nextest run --package coa-kernel

# Run stress test
cargo run -- stress --nodes 10000

# Run simulator
cargo run -- simulate --ops 10000 --seed 42
```
