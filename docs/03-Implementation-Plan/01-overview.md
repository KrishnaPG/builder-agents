# COA Implementation Plan Overview

**Version**: 1.0  
**Status**: Design Phase  
**Target**: High-performance, extensible, testable COA layer

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    COA LAYER (New Implementation)               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Artifact  │  │  SymbolRef  │  │   CompositionStrategy   │  │
│  │   System    │  │   System    │  │        Trait            │  │
│  │   (01-)     │  │   (02-)     │  │        (03-)            │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
│         │                │                      │               │
│         └────────────────┼──────────────────────┘               │
│                          ▼                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Constitutional Layer (04-)                  │   │
│  │         (Parse / Apply Deltas / Serialize)              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                          │                                      │
│                          ▼                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │               COA Orchestrator Core (05-)               │   │
│  │     (Task Decomposition / Agent Management / Reasoning) │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Uses
┌──────────────────────────▼──────────────────────────────────────┐
│                    KERNEL LAYER (v2.0) - Existing               │
│              (GraphBuilder / Executor / Token Integrity)        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Breakdown

| Module | File | Responsibility | Lines Est. |
|--------|------|----------------|------------|
| Artifact System | `01-artifact-system.md` | Typed work products, content hashing | - |
| SymbolRef System | `02-symbolref-system.md` | Content-addressed references, index | - |
| Composition Strategies | `03-composition-strategies.md` | Conflict resolution traits | - |
| Constitutional Layer | `04-constitutional-layer.md` | Parse/apply/serialize | - |
| COA Core | `05-coa-core.md` | Orchestration, task decomposition | - |
| Test Strategy | `06-test-strategy.md` | Testing approach, coverage | - |
| Workspace Setup | `07-workspace-setup.md` | Cargo.toml, dependencies | - |

---

## Design Principles

### 1. Safe by Construction
- All invalid states unrepresentable at type level
- Validation at construction time, never at runtime
- Use Rust's type system for static guarantees

### 2. Zero-Cost Abstractions
- Trait objects only at orchestration boundaries
- Monomorphized generic code for hot paths
- `#[inline]` for small functions
- `const fn` where possible

### 3. Extensibility via Traits
```rust
// Plugin architecture - users implement traits
pub trait ArtifactType: Send + Sync + 'static { ... }
pub trait CompositionStrategy: Send + Sync { ... }
pub trait Transformation<T: ArtifactType>: Send + Sync { ... }
```

### 4. Async-First
- All I/O operations async
- `tokio` runtime
- Streaming for large artifacts
- Backpressure support

### 5. Observability
- OpenTelemetry tracing throughout
- Structured logging (JSON)
- Metrics export (Prometheus)
- Jaeger-compatible spans

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| SymbolRef lookup | O(log n) | Radix tree index |
| Delta validation | O(k log n) | k = deltas, n = symbols |
| Artifact hash | Incremental | Merkle tree updates |
| Memory overhead | <50 bytes/symbol | Including index |
| Throughput | 10k deltas/sec | Single threaded |
| Latency (p99) | <10ms | Construction validation |

---

## Crate Structure

```
coa/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── coa-artifact/            # Artifact system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── artifact.rs
│   │       ├── delta.rs
│   │       ├── hash.rs
│   │       └── types/
│   │           ├── code.rs
│   │           ├── config.rs
│   │           └── spec.rs
│   │
│   ├── coa-symbol/              # SymbolRef system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── symbol.rs
│   │       ├── index.rs
│   │       ├── merkle.rs
│   │       └── validation.rs
│   │
│   ├── coa-composition/         # Composition strategies
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── strategy.rs
│   │       ├── single_writer.rs
│   │       ├── commutative.rs
│   │       ├── ordered.rs
│   │       └── hybrid.rs
│   │
│   ├── coa-constitutional/      # Application layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── layer.rs
│   │       ├── parser.rs
│   │       ├── serializer.rs
│   │       └── transformers/
│   │           ├── code.rs
│   │           └── config.rs
│   │
│   ├── coa-core/                # Orchestrator
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── coa.rs
│   │       ├── decomposition.rs
│   │       ├── agent_pool.rs
│   │       ├── reasoning.rs
│   │       └── diagnostics.rs
│   │
│   └── coa-kernel-adapter/      # Kernel integration
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── graph_builder.rs
│           └── executor.rs
│
└── tests/
    ├── integration/             # Integration tests
    ├── benches/                 # Criterion benchmarks
    └── fuzz/                    # Fuzzing targets
```

---

## Dependencies - Maximizing Open Source

### Philosophy: Build Glue, Not Infrastructure

| Custom Component | Replaced By | Rationale |
|-----------------|-------------|-----------|
| Custom Merkle tree | `rs_merkle` | Battle-tested, incremental updates |
| Custom radix tree | `qp-trie` or `radix_trie` | Memory-efficient prefix matching |
| Custom CRDTs | `crdts` | Research-backed implementations |
| Custom caching | `moka` | High-performance concurrent cache |
| Custom immutable collections | `im` | HAMT-based, copy-on-write |
| Custom ID generation | `ulid` or `uuid` | Standard, sortable IDs |
| Content addressing | `multihash` | Future-proof hashing |

### Core Dependencies

```toml
# Async runtime
tokio = { version = "1.35", features = ["full", "parking_lot"] }
futures = "0.3"
async-trait = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"

# Content addressing & hashing
blake3 = "1.5"
multihash = "0.19"
hex = "0.4"

# Data structures - USE EXISTING CRATES
rs_merkle = "1.4"           # Merkle tree - NOT custom
radix_trie = "0.2"          # Radix tree - NOT custom  
crdts = "7.3"               # CRDT implementations
im = "15.1"                 # Immutable collections

# Caching
moka = { version = "0.12", features = ["future"] }

# Concurrency
dashmap = "5.5"
parking_lot = "0.12"
rayon = "1.8"
crossbeam = "0.8"

# Parsing (all open source)
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-typescript = "0.20"
tree-sitter-python = "0.20"
pulldown-cmark = "0.9"

# Cryptography (tokens)
ed25519-dalek = "2"
rand = "0.8"

# Schema
schemars = "0.8"
jsonschema = "0.17"

# Error handling
thiserror = "1"
anyhow = "1"

# Observability
tracing = "0.1"
opentelemetry = "0.21"
metrics = "0.22"

# ID generation
ulid = { version = "1", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }

# Testing
proptest = "1.4"
criterion = { version = "0.5", features = ["async_tokio"] }
mockall = "0.12"
```

### What We DON'T Build

| Don't Build | Use Instead | Savings (Lines) |
|-------------|-------------|-----------------|
| Merkle tree | `rs_merkle::MerkleTree` | ~200 |
| Radix tree | `radix_trie::Trie` | ~300 |
| GCounter/PNCounter | `crdts::GCounter` | ~150 |
| LWWRegister | `crdts::LWWReg` | ~100 |
| Immutable HashMap | `im::HashMap` | ~500 |
| Concurrent cache | `moka::future::Cache` | ~400 |
| ULID generation | `ulid::Ulid` | ~100 |

### What We DO Build (Glue Code Only)

| Build | Lines | Why Custom |
|-------|-------|------------|
| `Artifact<T>` trait | ~50 | Domain-specific content model |
| `SymbolRef` struct | ~100 | Hash-binding requirement |
| `CompositionStrategy` trait | ~80 | COA-specific semantics |
| `ConstitutionalLayer` | ~200 | Integration/orchestration |
| `COA` orchestrator | ~300 | Business logic |
| **Total Custom** | **~730** | vs ~3000+ without crates |

---

## Implementation Phases

### Phase 1: Foundation (Weeks 1-2)
- Artifact system core types
- SymbolRef + basic index
- Merkle hashing
- Unit tests

### Phase 2: Composition (Weeks 3-4)
- Strategy trait
- SingleWriter + CommutativeBatch
- Validation pipeline
- Property-based tests

### Phase 3: Constitutional (Weeks 5-6)
- Parse/apply/serialize
- Code artifact support (Tree-sitter)
- Config artifact support
- Integration tests

### Phase 4: COA Core (Weeks 7-8)
- Orchestrator
- Task decomposition
- Agent pool management
- Diagnostics system

### Phase 5: Integration (Weeks 9-10)
- Kernel adapter
- End-to-end tests
- Benchmarks
- Documentation

---

## Success Criteria

1. **Correctness**: All blueprint invariants enforced at construction time
2. **Performance**: Meets targets above
3. **Test Coverage**: >90% line coverage, 100% critical paths
4. **Documentation**: All public APIs documented with examples
5. **Ergonomics**: Clean, composable API

---

## Open Source Dependencies Summary

### Data Structures (Don't Build)

| Crate | Purpose | Replaces ~Lines |
|-------|---------|-----------------|
| `rs_merkle` | Merkle tree with proofs | 200 |
| `radix_trie` | Prefix tree for SymbolRef index | 300 |
| `crdts` | CRDT implementations (G-Set, LWW-Reg, etc.) | 400 |
| `im` | Immutable HAMT collections | 500 |
| `moka` | Concurrent cache with TTL | 300 |

### Parsing (Don't Build)

| Crate | Purpose |
|-------|---------|
| `tree-sitter` | Incremental parsing for code |
| `serde_json` | JSON config parsing |
| `serde_yaml` | YAML config parsing |
| `pulldown-cmark` | Markdown spec parsing |
| `schemars` | JSON Schema validation |

### Async & Concurrency (Don't Build)

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `rayon` | Data parallelism |
| `dashmap` | Concurrent HashMap |
| `parking_lot` | Efficient mutexes |

### Custom Code Budget

| Component | Estimated Lines | Why Custom |
|-----------|-----------------|------------|
| `Artifact<T>` trait + struct | ~50 | Domain-specific content model |
| `SymbolRef` + hash-binding | ~100 | Blueprint requirement |
| `CompositionStrategy` trait | ~80 | COA-specific semantics |
| `ConstitutionalLayer` | ~150 | Integration glue |
| `COA` orchestrator | ~250 | Business logic |
| **Total Custom** | **~630** | vs ~4000+ without crates |

**Custom code is <20% of total implementation.**
