# COA (Constitutional Operational Architecture)

**Stack**: Rust workspace | **Architecture**: Kernel + Artifact + Symbol + Composition + Constitutional + Core

## STRUCTURE

```
crates/
├── coa-kernel/         # DAG-based orchestration engine, token integrity, execution
├── coa-artifact/       # Artifact types: delta, hash, merkle, versioning
├── coa-symbol/         # Symbol indexing, validation, lookup
├── coa-composition/    # Commutative, ordered, hybrid composition strategies
├── coa-constitutional/ # Constitutional document parsing (YAML, Markdown, JSON, code)
├── coa-core/           # Task decomposition, agent pool, COA orchestration
├── coa-opencode/       # Switchable Opencode backend (CLI/daemon modes)
└── coa-test-utils/     # Shared test infrastructure
docs/                   # Architecture documentation (BluePrint, Kernel, StructuralGraph)
tests/                  # System tests, integration tests, system plans
3rdParty/opencode/      # External: OpenCode AI agent (separate AGENTS.md)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| DAG construction/execution | `crates/coa-kernel/src/dag/` | Core orchestration primitives |
| Artifact delta/hashing | `crates/coa-artifact/src/delta.rs` | Merkle tree, content addressing |
| Symbol resolution | `crates/coa-symbol/src/index.rs` | Radix trie, validation |
| Composition strategies | `crates/coa-composition/src/` | Ordered, commutative, hybrid |
| Constitutional parsing | `crates/coa-constitutional/src/parsers/` | YAML, MD, JSON, code parsers |
| Task decomposition | `crates/coa-core/src/decomposition.rs` | Intent → subtask splitting |
| Agent backend | `crates/coa-opencode/` | AgentService trait, HTTP client |

## CODE MAP

### Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `Token` | `coa-kernel` | Immutable execution authorization |
| `Artifact<T>` | `coa-artifact` | Typed content with structural delta |
| `Symbol` | `coa-symbol` | Indexed named entity |
| `CompositionStrategy` | `coa-composition` | Strategy pattern for combining agents |
| `AgentService` | `coa-opencode` | Backend abstraction trait |

### Entry Points

- `coa-kernel/src/lib.rs` - Public API exports
- `coa-core/src/lib.rs` - COA orchestration
- `coa-opencode/src/lib.rs` - Agent backend
- `coa-kernel/src/main.rs` - CLI binary

## CONVENTIONS

- **Unsafe**: Forbidden workspace-wide (`unsafe_code = "forbid"`)
- **Documentation**: All public items must be documented (`missing_docs = "warn"`)
- **Lints**: Strict clippy (`all = "warn"`, `pedantic = "warn"`)
- **Error handling**: Use `thiserror` for custom errors, `anyhow` for propagation
- **Async**: `tokio` runtime with `parking_lot` features
- **IDs**: `ulid` for tokens, `uuid` for entities

## ANTI-PATTERNS

- **Direct disk I/O**: Kernel forbids direct I/O; use artifact layer
- **String errors**: Use typed errors, not `Box<dyn Error>` strings
- **Unbounded channels**: All channels must have capacity limits
- **Synchronous blocking**: Use async throughout, never block threads

## WORKSPACE DEPENDENCIES

Shared crates defined in root `Cargo.toml`:
- Serialization: `serde`, `serde_json`, `serde_yaml`
- Async: `tokio`, `async-trait`, `futures`
- Crypto: `ed25519-dalek`, `blake3`, `sha2`
- Data structures: `dashmap`, `radix_trie`, `im`, `crdts`
- Observability: `tracing`, `opentelemetry`

## COMMANDS

```bash
# Build workspace
cargo build --workspace

# Run kernel
cargo run -p coa-kernel

# Run server example
cargo run -p coa-opencode --example server

# Test
cargo test --workspace

# Check lints
cargo clippy --workspace -- -D warnings
```
