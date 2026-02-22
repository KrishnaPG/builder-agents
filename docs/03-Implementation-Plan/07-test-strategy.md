# Test Strategy

**Module**: All crates  
**Coverage Target**: >90% line, 100% critical paths

---

## Testing Philosophy

### Safe-by-Construction Requires Rigorous Testing

```
┌─────────────────────────────────────────────────────────────────┐
│  PRINCIPLE: If a bug makes it past construction validation,     │
│             it's a CRITICAL security issue.                     │
└─────────────────────────────────────────────────────────────────┘
```

### Test Categories

| Category | Purpose | Tools |
|----------|---------|-------|
| Unit Tests | Individual functions | `cargo test` |
| Property Tests | Invariants | `proptest` |
| Integration Tests | Cross-module | `tests/` |
| Fuzz Tests | Edge cases | `cargo-fuzz` |
| Benchmarks | Performance | `criterion` |
| Concurrency Tests | Race conditions | `loom`, stress tests |

---

## Unit Tests

### Every Module Requires

```rust
// File: src/module.rs

pub fn critical_function() -> Result<T, E> { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_works() {
        let result = critical_function(valid_input());
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_input_rejected() {
        let result = critical_function(invalid_input());
        assert!(matches!(result, Err(Error::InvalidInput)));
    }

    #[test]
    fn preserves_invariants() {
        let result = critical_function(valid_input()).unwrap();
        assert!(invariant_holds(&result));
    }
}
```

### Artifact System Tests

```rust
// crates/coa-artifact/tests/artifact_tests.rs

mod construction_tests {
    #[test]
    fn artifact_hash_deterministic() {
        let content = create_test_content();
        let a1 = Artifact::new(content.clone());
        let a2 = Artifact::new(content);
        
        assert_eq!(a1.hash(), a2.hash());
    }

    #[test]
    fn tampered_content_detected() {
        let mut artifact = create_test_artifact();
        // Tamper with internal content
        artifact.content = tampered_content();
        
        assert!(!artifact.verify());
    }
}

mod delta_tests {
    use proptest::prelude::*;

    #[test]
    fn delta_application_produces_valid_artifact() {
        let base = create_test_artifact();
        let delta = create_valid_delta(&base);
        
        let result = delta.apply(&base);
        assert!(result.is_ok());
        assert!(result.unwrap().verify());
    }

    #[test]
    fn delta_fails_on_wrong_base() {
        let base = create_test_artifact();
        let other = create_different_artifact();
        
        // Delta expects base's hash
        let delta = create_delta_for(&base);
        
        // Applying to other fails
        assert!(delta.apply(&other).is_err());
    }

    // Property-based test
    proptest! {
        #[test]
        fn add_remove_are_inverse(contents: Vec<u8>) {
            let base = Artifact::new(contents.clone());
            let add = Delta::Add(new_content());
            let remove = Delta::Remove;
            
            let added = add.apply(&base).unwrap();
            let restored = remove.apply(&added).unwrap();
            
            // Content may differ, but structure valid
            assert!(restored.verify());
        }
    }
}
```

---

## Property-Based Testing

### Invariants to Test

```rust
// crates/coa-symbol/tests/property_tests.rs

use proptest::prelude::*;
use coa_symbol::*;

proptest! {
    // Invariant: Index lookup is consistent
    #[test]
    fn index_lookup_finds_inserted(
        path in "[a-z.]{1,100}",
        hash in any::<[u8; 32]>()
    ) {
        let index = SymbolRefIndex::new();
        let symbol = SymbolRef::new(
            path.split('.').map(|s| s.to_string()).collect(),
            ContentHash::new(hash),
        );
        
        index.insert(symbol.clone(), Metadata::default()).unwrap();
        
        let found = index.get_exact(&symbol);
        assert!(found.is_some());
        assert_eq!(found.unwrap().symbol, symbol);
    }

    // Invariant: Overlapping symbols are rejected
    #[test]
    fn overlapping_symbols_rejected(
        parent in "[a-z]{1,20}",
        child in "[a-z]{1,20}",
        hash in any::<[u8; 32]>()
    ) {
        let index = SymbolRefIndex::new();
        
        let parent_sym = SymbolRef::new(
            vec![parent.clone()],
            ContentHash::new(hash),
        );
        let child_sym = SymbolRef::new(
            vec![parent, child],
            ContentHash::new(hash),
        );
        
        index.insert(parent_sym, Metadata::default()).unwrap();
        
        let result = index.insert(child_sym, Metadata::default());
        assert!(result.is_err());
    }

    // Invariant: Single-writer detection
    #[test]
    fn single_writer_detects_overlap(
        paths in prop::collection::vec("[a-z.]{1,50}", 2..10)
    ) {
        let index = SymbolRefIndex::new();
        let deltas: Vec<_> = paths.iter()
            .map(|p| create_delta(p))
            .collect();
        
        let result = SingleWriterValidator::validate_deltas(&deltas, &index);
        
        // Check if any paths overlap
        let has_overlap = paths.iter().enumerate().any(|(i, p1)| {
            paths.iter().enumerate().any(|(j, p2)| {
                i != j && paths_overlap(p1, p2)
            })
        });
        
        if has_overlap {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }
}
```

---

## Integration Tests

### End-to-End Scenarios

```rust
// tests/integration/multi_agent_workflow.rs

#[tokio::test]
async fn three_agents_composition() {
    // Setup
    let coa = setup_coa().await;
    let base = coa.parse_ingress::<CodeArtifact>("test.rs").await.unwrap();
    
    // Agent 1: Add function A
    let delta1 = create_delta("service.fn_a", add_function("fn_a"));
    
    // Agent 2: Add function B
    let delta2 = create_delta("service.fn_b", add_function("fn_b"));
    
    // Agent 3: Add function C (independent)
    let delta3 = create_delta("utils.helper", add_function("helper"));
    
    // Compose with SingleWriter
    let strategy = SingleWriterStrategy::new();
    let result = coa.compose(&base, &[delta1, delta2, delta3], &strategy).await;
    
    assert!(result.is_ok());
    
    // Verify all functions exist
    let artifact = result.unwrap();
    assert!(artifact.has_symbol("service.fn_a"));
    assert!(artifact.has_symbol("service.fn_b"));
    assert!(artifact.has_symbol("utils.helper"));
}

#[tokio::test]
async fn overlapping_claims_rejected() {
    let coa = setup_coa().await;
    let base = create_test_artifact();
    
    // Two agents claiming overlapping paths
    let delta1 = create_delta("service", add_module("service"));
    let delta2 = create_delta("service.login", add_function("login"));
    
    let strategy = SingleWriterStrategy::new();
    let result = coa.compose(&base, &[delta1, delta2], &strategy).await;
    
    assert!(result.is_err());
    
    // Verify diagnostic is helpful
    let err = result.unwrap_err();
    assert!(err.diagnostic().is_some());
    assert!(!err.suggested_fixes().is_empty());
}
```

---

## Concurrency Tests

### Loom for State Machine Verification

```rust
// crates/coa-symbol/tests/concurrent_tests.rs

#[test]
fn concurrent_insertions_safe() {
    use std::sync::Arc;
    use std::thread;
    
    let index = Arc::new(SymbolRefIndex::new());
    
    let handles: Vec<_> = (0..10).map(|i| {
        let idx = index.clone();
        thread::spawn(move || {
            let symbol = SymbolRef::new(
                vec![format!("symbol{}", i)],
                ContentHash::new([i as u8; 32]),
            );
            idx.insert(symbol, Metadata::default())
        })
    }).collect();
    
    for h in handles {
        h.join().unwrap().unwrap();
    }
    
    // All 10 should be present
    assert_eq!(index.len(), 10);
}
```

---

## Fuzz Testing

### Targets

```rust
// fuzz/fuzz_targets/parse.rs

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to parse any input
        let _ = CodeContent::parse(s, Language::Rust);
    }
});

// fuzz/fuzz_targets/delta_apply.rs

fuzz_target!(|data: &[u8]| {
    // Fuzz delta application
    if let Ok((base, delta)) = deserialize_delta_application(data) {
        let _ = delta.apply(&base);
    }
});
```

---

## Benchmarks

### Performance Targets

```rust
// benches/symbol_lookup.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use coa_symbol::*;

fn symbol_lookup_benchmark(c: &mut Criterion) {
    // Setup: 100k symbols
    let index = SymbolRefIndex::new();
    for i in 0..100_000 {
        let symbol = SymbolRef::new(
            vec!["crate".to_string(), format!("module{}", i), format!("symbol{}", i)],
            ContentHash::new([0; 32]),
        );
        index.insert(symbol, Metadata::default()).unwrap();
    }
    
    let target = SymbolRef::new(
        vec!["crate".to_string(), "module50000".to_string(), "symbol50000".to_string()],
        ContentHash::new([0; 32]),
    );
    
    c.bench_function("symbol_lookup_100k", |b| {
        b.iter(|| {
            black_box(index.get_exact(&target));
        });
    });
}

criterion_group!(benches, symbol_lookup_benchmark);
criterion_main!(benches);
```

### Benchmark Results Format

```markdown
| Operation | N=1k | N=10k | N=100k | N=1M |
|-----------|------|-------|--------|------|
| Insert | 100ns | 120ns | 150ns | 200ns |
| Lookup | 80ns | 90ns | 110ns | 150ns |
| Validation | 1ms | 12ms | 150ms | 2s |
```

---

## Test Utilities

### Shared Test Helpers

```rust
// crates/coa-test-utils/src/lib.rs

pub fn create_test_code_artifact() -> Artifact<CodeArtifact> {
    let content = CodeContent::parse(
        "fn main() {}",
        Language::Rust,
    ).unwrap();
    Artifact::new(content)
}

pub fn create_delta(target: &str, op: DeltaOperation) -> StructuralDelta<CodeArtifact> {
    StructuralDelta::new(
        SymbolPath::from_str(target),
        op,
        ContentHash::new([0; 32]),
    )
}

pub fn setup_test_coa() -> CreatorOrchestratorAgent {
    let config = COAConfig {
        max_concurrent_agents: 4,
        default_autonomy: AutonomyLevel::L3,
        system_limits: SystemLimits::default(),
        auto_apply_fixes: false,
        escalation_threshold: EscalationThreshold::Always,
    };
    
    CreatorOrchestratorAgent::new(config)
}
```

---

## CI/CD Testing

### GitHub Actions

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Run tests
        run: cargo test --all-features --workspace
      
      - name: Run property tests
        run: cargo test --all-features proptest --workspace
      
      - name: Run benchmarks (smoke test)
        run: cargo bench --no-run
      
      - name: Check coverage
        run: |
          cargo tarpaulin --all-features --workspace --out Xml
          codecov
      
      - name: Fuzz (quick check)
        run: |
          cargo install cargo-fuzz
          cargo fuzz run parse --max-total-time=60
```

---

## Test Checklist

### Before Merge

- [ ] All unit tests pass
- [ ] Property tests pass (1000+ iterations)
- [ ] No clippy warnings
- [ ] Code coverage >90%
- [ ] New code has tests
- [ ] Documentation examples tested

### Before Release

- [ ] Integration tests pass
- [ ] Benchmarks show no regression
- [ ] Fuzz tests run for 1 hour without crash
- [ ] Concurrency tests pass (loom)
- [ ] Manual end-to-end test
