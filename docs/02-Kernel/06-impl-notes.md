# IMPLEMENTATION NOTES

## 26.1 Test Organization

```
kernel/
 ├── src/
 │    └── ...
 ├── tests/
 │    ├── integration/           # Workflow tests
 │    │    ├── node_lifecycle.rs
 │    │    ├── escalation_flow.rs
 │    │    └── concurrent_ops.rs
 │    ├── property/              # Property-based tests
 │    │    ├── dag_properties.rs
 │    │    ├── state_properties.rs
 │    │    └── token_properties.rs
 │    ├── negative/              # Failure mode tests
 │    │    ├── graph_violations.rs
 │    │    ├── autonomy_violations.rs
 │    │    └── compliance_violations.rs
 │    └── common/                # Test utilities
 │         ├── mod.rs
 │         ├── generators.rs     # Test data generators
 │         └── assertions.rs     # Invariant assertions
```

## 26.2 CI/CD Integration

```yaml
# .github/workflows/kernel-certification.yml
name: Kernel Certification

on: [push, pull_request]

jobs:
	test:
		runs-on: ubuntu-latest
		steps:
			- uses: actions/checkout@v3
			
			- name: Run Unit Tests
				run: cargo test --lib
			
			- name: Run Integration Tests
				run: cargo test --test '*'
			
			- name: Run Property Tests
				run: cargo test --test property -- --nocapture
			
			- name: Generate Coverage
				run: cargo tarpaulin --out Xml
			
			- name: Run COA Simulator (Quick)
				run: cargo run --release -- simulate --ops 100000 --seed 42
			
			- name: Run COA Simulator (Certification)
				if: github.ref == 'refs/heads/main'
				run: cargo run --release -- simulate --ops 10000000 --seeds 100
			
			- name: Run Stress Test
				run: cargo run --release -- stress --nodes 10000 --iterations 5000
			
			- name: Generate Report
				run: cargo run --release -- report
			
			- name: Upload Certification Report
				uses: actions/upload-artifact@v3
				with:
					name: certification-report
					path: kernel-certification-report.txt
```

---
