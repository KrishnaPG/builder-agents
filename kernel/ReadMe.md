# Cognitive OS Kernel

The immutable enforcement layer.

## Build

```bash
cargo build --release
```

## Test

### Quick Test (Shows Full Report)

```bash
# Shows the complete test report with all results
cargo test --package cog_kernel -- --nocapture

# or
cargo nextest run
```

### Using cargo-nextest (Recommended)

Beautiful, structured test output with parallel execution:

```bash
# Install nextest (one-time)
cargo install cargo-nextest

# Run all tests with nice formatting
cargo nextest run --package cog_kernel
```

### Standard cargo test

```bash
# Minimal output (test names only)
cargo test --package cog_kernel
```

## Run CLI

```bash
# Run stress test
cargo run -- stress --nodes 10000 --iterations 5000

# Run simulator with report
cargo run -- simulate --ops 10000 --seed 42

# Generate integrity report
cargo run -- report
```

## Project Structure

```
kernel/
├── src/           # Library source code
├── tests/         # Integration tests
├── .config/       # Nextest configuration
└── Cargo.toml     # Package manifest
```

## Features

- `default` - Standard build
- `perf` - Performance optimizations (dashmap, smallvec)
- `strict-debug` - Panic on illegal state transitions (debug only)
