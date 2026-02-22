# Open Source Strategy

**Principle**: Build glue, not infrastructure. Stand on the shoulders of giants.

---

## Why Use Open Source?

### 1. Battle-Tested Correctness
- `rs_merkle`: Used in blockchain projects, audited
- `radix_trie`: Used in Rust compiler, battle-tested
- `crdts`: Research-backed implementations from academia
- `moka`: Production cache at high-scale companies

### 2. Maintenance Burden
- **Custom code**: We maintain forever
- **Open source**: Community maintains, security patches automatic

### 3. Time to Market
- Building Merkle tree: 1 week + testing
- Using `rs_merkle`: 1 day integration

### 4. Performance
- These crates are optimized by domain experts
- Often faster than naive custom implementation

---

## Crate Selection Criteria

| Criterion | Weight | How We Evaluate |
|-----------|--------|-----------------|
| Maintenance | High | Recent commits, responsive maintainers |
| Usage | High | >100k downloads, used by major projects |
| Documentation | High | Good docs, examples, API stability |
| Safety | Critical | `cargo audit` clean, no unsafe (or audited) |
| Performance | Medium | Benchmarks vs alternatives |
| License | Critical | MIT/Apache-2.0 compatible |

### Veto Conditions
- No commits in 6 months (unmaintained)
- `cargo audit` shows vulnerabilities
- GPL/Copyleft license
- No documentation

---

## Selected Crates

### Tier 1: Critical Infrastructure (Proven, Stable)

| Crate | Version | Downloads | Used By |
|-------|---------|-----------|---------|
| `tokio` | 1.35+ | 150M+ | AWS, Discord, Microsoft |
| `serde` | 1.0+ | 200M+ | Nearly every Rust project |
| `blake3` | 1.5+ | 10M+ | Zcash, IPFS |
| `dashmap` | 5.5+ | 50M+ | Vector, Meilisearch |
| `rayon` | 1.8+ | 80M+ | Rust compiler, ripgrep |

### Tier 2: Domain-Specific (Well-Vetted)

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| `rs_merkle` | 1.4+ | Merkle tree | High - focused, tested |
| `radix_trie` | 0.2+ | Prefix matching | High - used in rustc |
| `crdts` | 7.3+ | CRDTs | Medium - research-based |
| `moka` | 0.12+ | Caching | High - production use |
| `im` | 15.1+ | Immutable collections | High - established |

### Tier 3: Utilities (Standard)

| Crate | Version | Purpose |
|-------|---------|---------|
| `tree-sitter` | 0.20+ | Parsing |
| `thiserror` | 1.0+ | Error handling |
| `ulid` | 1.0+ | ID generation |
| `proptest` | 1.4+ | Property testing |

---

## What We Build vs What We Use

### We Use (Open Source)

```
┌─────────────────────────────────────────────────────────┐
│  Merkle Tree        │  rs_merkle                        │
│  Radix Tree         │  radix_trie                       │
│  CRDTs              │  crdts                            │
│  Cache              │  moka                             │
│  Immutable Maps     │  im::HashMap                      │
│  Async Runtime      │  tokio                            │
│  Concurrent HashMap │  dashmap                          │
│  Parser Generator   │  tree-sitter                      │
│  Serialization      │  serde                            │
│  Hashing            │  blake3                           │
└─────────────────────────────────────────────────────────┘
```

### We Build (Glue Code Only)

```
┌─────────────────────────────────────────────────────────┐
│  Artifact<T>        │  ~50 lines - trait + struct       │
│  SymbolRef          │  ~100 lines - hash-bound paths    │
│  CompositionStrategy│  ~80 lines - validation glue      │
│  ConstitutionalLayer│  ~150 lines - parse/apply/serialize│
│  COA Orchestrator   │  ~250 lines - decomposition       │
└─────────────────────────────────────────────────────────┘
  Total Custom: ~630 lines
```

---

## Risk Mitigation

### Risk: Crate Abandonment

**Mitigation**:
- Pin to specific versions in `Cargo.lock`
- Fork critical crates to organization
- Monitor with `cargo outdated`
- Maintain list of alternatives

### Risk: Breaking Changes

**Mitigation**:
- Use semver-compatible versions
- Pin major versions in `Cargo.toml`
- CI tests catch breakage
- Abstract behind our traits (swappable)

### Risk: Security Vulnerabilities

**Mitigation**:
- `cargo audit` in CI
- Dependabot alerts
- Regular dependency updates
- Prefer crates with `cargo vet` certifications

---

## Integration Examples

### rs_merkle Integration

```rust
// Instead of 200 lines custom Merkle tree:
pub struct ArtifactMerkleTree {
    inner: rs_merkle::MerkleTree<Blake3Hasher>,
}

// 50 lines of glue code
```

### radix_trie Integration

```rust
// Instead of 300 lines custom radix tree:
pub struct SymbolIndex {
    inner: radix_trie::Trie<String, SymbolRef>,
}

// 100 lines of glue code
```

### crdts Integration

```rust
// Instead of 400 lines custom CRDTs:
pub struct LayerSet {
    inner: crdts::OrSet<String, u64>,
}

// 80 lines of glue code
```

### moka Integration

```rust
// Instead of 300 lines custom cache:
pub struct ArtifactCache {
    inner: moka::future::Cache<ContentHash, Arc<dyn Any>>,
}

// 60 lines of glue code
```

---

## Audit Trail

| Decision Date | Crate | Version | Rationale |
|--------------|-------|---------|-----------|
| 2024-02-22 | rs_merkle | 1.4 | Best maintained Merkle crate |
| 2024-02-22 | radix_trie | 0.2 | Used in rustc, NIF-safe |
| 2024-02-22 | crdts | 7.3 | Research-backed, clean API |
| 2024-02-22 | moka | 0.12 | Async support, high performance |
| 2024-02-22 | im | 15.1 | Standard immutable collections |

---

## Success Metrics

### Before (Custom Everything)
- Estimated code: ~4,000 lines
- Testing burden: High
- Maintenance: Ongoing forever
- Bug surface: Large

### After (Open Source First)
- Custom code: ~630 lines
- Testing burden: Focus on glue only
- Maintenance: Updates via `cargo update`
- Bug surface: Minimal (trust ecosystem)

**Code Reduction: ~84%**

---

## When to Build Custom

Despite open-source-first, we build custom when:

1. **Blueprint requirements are unique**
   - Hash-binding in SymbolRef (no crate does this)
   - Construction-time validation semantics

2. **No suitable crate exists**
   - CompositionStrategy orchestration
   - COA-specific decomposition logic

3. **Performance requirements exceed crates**
   - After benchmarking shows gap
   - After attempting optimization of existing crate

4. **Security critical with no audit**
   - Token integrity (use ed25519-dalek, custom glue)
   - Cryptographic binding

---

## Summary

**Build glue, not infrastructure.**

We focus our engineering effort on:
- COA business logic (decomposition, orchestration)
- Blueprint-specific invariants (hash-binding, validation)
- Integration between crates

We let the ecosystem handle:
- Data structures (Merkle trees, tries, CRDTs)
- Async runtime
- Parsing
- Caching
- Concurrency primitives

**Result**: Faster delivery, higher quality, less maintenance.
