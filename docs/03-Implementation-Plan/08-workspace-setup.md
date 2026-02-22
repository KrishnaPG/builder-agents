# Workspace Setup

**Root**: `coa/` workspace  
**Crate Count**: 6 core + 1 adapter + 1 test-utils

---

## Directory Structure

```
coa/
├── Cargo.toml                    # Workspace root
├── Cargo.lock                    # Generated
├── .cargo/
│   └── config.toml              # Build configuration
├── crates/
│   ├── coa-artifact/
│   ├── coa-symbol/
│   ├── coa-composition/
│   ├── coa-constitutional/
│   ├── coa-core/
│   ├── coa-kernel-adapter/
│   └── coa-test-utils/
├── tests/
│   ├── integration/
│   ├── benchmarks/
│   └── fuzz/
├── docs/
│   └── 01-BluePrint/            # Existing
│   └── 03-Implementation-Plan/  # This doc
└── .github/
    └── workflows/
        └── test.yml
```

---

## Root Cargo.toml

```toml
[workspace]
members = [
    "crates/coa-artifact",
    "crates/coa-symbol",
    "crates/coa-composition",
    "crates/coa-constitutional",
    "crates/coa-core",
    "crates/coa-kernel-adapter",
    "crates/coa-test-utils",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["COA Team"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/example/coa"
rust-version = "1.75"

[workspace.dependencies]
# Internal
coa-artifact = { path = "crates/coa-artifact", version = "0.1.0" }
coa-symbol = { path = "crates/coa-symbol", version = "0.1.0" }
coa-composition = { path = "crates/coa-composition", version = "0.1.0" }
coa-constitutional = { path = "crates/coa-constitutional", version = "0.1.0" }
coa-core = { path = "crates/coa-core", version = "0.1.0" }
cog-kernel = { path = "../kernel", version = "0.2.0" }

# Async
tokio = { version = "1.43", features = ["full", "parking_lot"] }
async-trait = "0.1"
futures = "0.3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"

# Cryptography
sha2 = "0.10"
blake3 = "1.6"
ed25519-dalek = { version = "2.1", features = ["rand_core"] }
rand = "0.9"
hex = "0.4"

# === OPEN SOURCE DATA STRUCTURES (Prefer over custom) ===
# Merkle tree - replaces custom implementation
rs_merkle = "1.4"

# Radix/Patricia trie - replaces custom index
radix_trie = "0.2"

# CRDTs - replaces custom commutative operations
crdts = "7.3"

# Immutable collections (HAMT-based)
im = "15.1"

# Caching - replaces custom cache implementation
moka = { version = "0.12", features = ["future"] }

# Collections / Concurrency
dashmap = "6.1"
parking_lot = "0.12"
rayon = "1.10"
crossbeam = "0.8"

# Parsing
tree-sitter = "0.25"
tree-sitter-rust = "0.24"
tree-sitter-typescript = "0.24"
tree-sitter-python = "0.24"
tree-sitter-go = "0.24"
pulldown-cmark = "0.13"

# Schema
schemars = "0.8"
jsonschema = "0.17"

# Error handling
thiserror = "2"
anyhow = "1"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
opentelemetry = "0.28"
opentelemetry-jaeger = "0.22"
metrics = "0.24"
metrics-exporter-prometheus = "0.16"

# Testing
proptest = "1.6"
criterion = { version = "0.5", features = ["async_tokio", "html_reports"] }
mockall = "0.13"
tempfile = "3"
pretty_assertions = "1.4"

# Utilities
uuid = { version = "1.15", features = ["v4", "serde"] }
ulid = { version = "1.2", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
once_cell = "1.20"
regex = "1.11"
indexmap = "2.7"
smallvec = "1.14"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"

[profile.dev]
opt-level = 0
debug = true

[profile.test]
opt-level = 2
debug = true

[profile.bench]
inherits = "release"
debug = true
```

---

## Crate: coa-artifact

### Cargo.toml

```toml
[package]
name = "coa-artifact"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Typed artifact system with Merkle hashing"

[dependencies]
# Hashing
sha2.workspace = true
blake3.workspace = true

# Serialization
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true

# Parsing
tree-sitter.workspace = true
tree-sitter-rust.workspace = true
tree-sitter-typescript.workspace = true
tree-sitter-python.workspace = true
tree-sitter-go.workspace = true

# Schema
schemars.workspace = true

# Error handling
thiserror.workspace = true

# Utilities
uuid.workspace = true

[dev-dependencies]
proptest.workspace = true
criterion.workspace = true

[[bench]]
name = "merkle_bench"
harness = false
```

---

## Crate: coa-symbol

### Cargo.toml

```toml
[package]
name = "coa-symbol"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Content-addressed symbolic references with radix tree index"

[dependencies]
coa-artifact.workspace = true

# Collections
dashmap.workspace = true
parking_lot.workspace = true

# Serialization
serde = { workspace = true, features = ["derive"] }
hex.workspace = true

# Error handling
thiserror.workspace = true

[dev-dependencies]
proptest.workspace = true
tokio = { workspace = true, features = ["test-util"] }
```

---

## Crate: coa-composition

### Cargo.toml

```toml
[package]
name = "coa-composition"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Pluggable conflict resolution strategies for multi-agent composition"

[dependencies]
coa-artifact.workspace = true
coa-symbol.workspace = true

# Parallelism
rayon.workspace = true

# Serialization
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true

# Error handling
thiserror.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["full"] }
proptest.workspace = true
```

---

## Crate: coa-constitutional

### Cargo.toml

```toml
[package]
name = "coa-constitutional"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Trusted transformation layer (parse/apply/serialize)"

[dependencies]
coa-artifact.workspace = true
coa-symbol.workspace = true
coa-composition.workspace = true

# Async
tokio = { workspace = true, features = ["fs", "io-util"] }

# Parsing
tree-sitter.workspace = true
tree-sitter-rust.workspace = true
serde_json.workspace = true
serde_yaml.workspace = true
pulldown-cmark.workspace = true

# Caching
dashmap.workspace = true

# Hashing
blake3.workspace = true

# Error handling
thiserror.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["full", "test-util"] }
tempfile.workspace = true
```

---

## Crate: coa-core

### Cargo.toml

```toml
[package]
name = "coa-core"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Creator-Orchestrator Agent - central intelligence and coordination"

[dependencies]
coa-artifact.workspace = true
coa-symbol.workspace = true
coa-composition.workspace = true
coa-constitutional.workspace = true
cog-kernel.workspace = true

# Async
tokio = { workspace = true, features = ["full"] }
async-trait.workspace = true
futures.workspace = true

# Collections
dashmap.workspace = true

# Serialization
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true

# Cryptography
ed25519-dalek.workspace = true
rand.workspace = true

# Error handling
thiserror.workspace = true
anyhow.workspace = true

# Observability
tracing.workspace = true

[dev-dependencies]
tokio-test.workspace = true
mockall.workspace = true
```

---

## Crate: coa-kernel-adapter

### Cargo.toml

```toml
[package]
name = "coa-kernel-adapter"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Adapter between COA layer and kernel"

[dependencies]
cog-kernel.workspace = true

# Async
tokio.workspace = true

# Cryptography
ed25519-dalek.workspace = true
rand.workspace = true

# Error handling
thiserror.workspace = true
```

---

## Crate: coa-test-utils

### Cargo.toml

```toml
[package]
name = "coa-test-utils"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Shared testing utilities"
publish = false  # Internal only

[dependencies]
coa-artifact.workspace = true
coa-symbol.workspace = true
coa-core.workspace = true

# Testing
proptest.workspace = true
tempfile.workspace = true
```

---

## Build Configuration

### .cargo/config.toml

```toml
[build]
target-dir = "target"
rustflags = ["-C", "target-cpu=native"]

[test]
timeout = 300  # 5 minutes per test

[env]
RUST_LOG = "info"
RUST_BACKTRACE = "1"

[registries.crates-io]
protocol = "sparse"
```

---

## Development Scripts

### scripts/test.sh

```bash
#!/bin/bash
set -e

echo "=== Running COA Tests ==="

echo "1. Unit tests..."
cargo test --workspace --lib

echo "2. Property tests..."
cargo test --workspace proptest

echo "3. Integration tests..."
cargo test --workspace --test '*'

echo "4. Doc tests..."
cargo test --workspace --doc

echo "5. Clippy..."
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "=== All tests passed ==="
```

### scripts/bench.sh

```bash
#!/bin/bash
set -e

echo "=== Running Benchmarks ==="
cargo bench --workspace "$@"
```

---

## IDE Configuration

### .vscode/settings.json

```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.check.extraArgs": ["--all-targets", "--all-features"],
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.procMacro.enable": true
}
```

### rustfmt.toml

```toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
edition = "2021"
```

---

## Makefile (Optional)

```makefile
.PHONY: all test bench clean fmt lint doc

all: fmt lint test

test:
	cargo test --workspace --all-features

bench:
	cargo bench --workspace

clean:
	cargo clean
	find . -name "*.profraw" -delete

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

doc:
	cargo doc --workspace --all-features --no-deps

install-hooks:
	cp scripts/pre-commit.sh .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
```

---

## Getting Started

```bash
# Clone repository
git clone https://github.com/example/coa
cd coa

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run specific crate tests
cargo test -p coa-artifact

# Run with logging
RUST_LOG=debug cargo test -p coa-core

# Build documentation
cargo doc --workspace --open

# Run benchmarks
cargo bench --workspace
```
