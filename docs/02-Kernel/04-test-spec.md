# TEST COVERAGE REQUIREMENTS

## v2.0: Safe-by-Construction Testing

**Critical Principle:** Tests must verify the **two-phase architecture**:
1. **Construction Phase**: Policy validation occurs here
2. **Execution Phase**: Only integrity verification (zero policy checks)

---

## 24.1 Coverage Targets

| Coverage Type        | Minimum | Target |
| -------------------- | ------- | ------ |
| Line Coverage        | 95%     | 98%    |
| Branch Coverage      | 90%     | 95%    |
| Mutation Coverage    | 80%     | 90%    |
| API Surface Coverage | 100%    | 100%   |
| Invariant Coverage   | 100%    | 100%   |
| Construction Phase   | 100%    | 100%   |
| Execution Phase      | 100%    | 100%   |

---

## 24.2 Test Categories

### 24.2.1 Construction Phase Tests

Tests for `GraphBuilder`, `ConstructionValidator`, and `TokenIssuer`.

#### DAG Construction Tests

```rust
#[test]
fn test_dag_accepts_valid_edge() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    let n1 = builder.add_node(valid_node_spec());
    let n2 = builder.add_node(valid_node_spec());
    
    // Should succeed - no cycle
    assert!(builder.add_edge(n1, n2).is_ok());
}

#[test]
fn test_dag_rejects_self_loop() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    let n1 = builder.add_node(valid_node_spec());
    
    // Should fail - self-loop
    assert!(builder.add_edge(n1, n1).is_err());
}

#[test]
fn test_dag_rejects_cycle_in_production() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    let n1 = builder.add_node(valid_node_spec());
    let n2 = builder.add_node(valid_node_spec());
    let n3 = builder.add_node(valid_node_spec());
    
    builder.add_edge(n1, n2).unwrap();
    builder.add_edge(n2, n3).unwrap();
    
    // Should fail - would create cycle n1→n2→n3→n1
    assert!(builder.add_edge(n3, n1).is_err());
}

#[test]
fn test_sandbox_allows_cycle() {
    let mut builder = GraphBuilder::new(GraphType::SandboxGraph);
    let n1 = builder.add_node(valid_node_spec());
    let n2 = builder.add_node(valid_node_spec());
    
    builder.add_edge(n1, n2).unwrap();
    
    // Should succeed - cycles allowed in sandbox
    assert!(builder.add_edge(n2, n1).is_ok());
}
```

#### NodeSpec Validation Tests

```rust
#[test]
fn test_rejects_autonomy_above_ceiling() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    let system_ceiling = AutonomyLevel::L3;
    
    let spec = NodeSpec {
        autonomy_ceiling: AutonomyLevel::L5,  // Above system ceiling
        ..valid_node_spec()
    };
    
    builder.add_node(spec);
    
    // Validation should fail
    let result = builder.validate();
    assert!(matches!(result, Err(ValidationError::AutonomyCeilingExceeded)));
}

#[test]
fn test_rejects_unprovable_resource_bounds() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = NodeSpec {
        resource_bounds: ResourceCaps {
            cpu_time_ms: u64::MAX,  // Impossible
            ..Default::default()
        },
        ..valid_node_spec()
    };
    
    builder.add_node(spec);
    
    // Validation should fail - cannot prove these bounds
    let result = builder.validate();
    assert!(matches!(result, Err(ValidationError::ResourceBoundsNotProvable)));
}

#[test]
fn test_accepts_node_at_exact_ceiling() {
    let spec = NodeSpec {
        autonomy_ceiling: AutonomyLevel::L5,
        resource_bounds: ResourceCaps {
            cpu_time_ms: 10000,
            memory_bytes: 1024 * 1024 * 1024,
            token_limit: 100000,
            iteration_cap: 1000,
        },
        ..valid_node_spec()
    };
    
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    builder.add_node(spec);
    
    // Should validate successfully
    assert!(builder.validate().is_ok());
}
```

#### ValidatedGraph Tests

```rust
#[test]
fn test_validated_graph_is_sealed() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    builder.add_node(valid_node_spec());
    
    let validated = builder.validate().unwrap();
    
    // ValidatedGraph fields are private - cannot construct directly
    // This is a compile-time guarantee
}

#[test]
fn test_validation_produces_token() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    builder.add_node(valid_node_spec());
    
    let validated = builder.validate().unwrap();
    
    // ValidatedGraph contains ValidationToken
    assert!(validated.validation_token().is_some());
}

#[test]
fn test_executor_rejects_unvalidated_graph() {
    // Compile-time guarantee: Executor::run() only accepts ValidatedGraph
    // Cannot pass Dag or GraphBuilder
    
    // This test documents the type system guarantee
    let executor = Executor::new(verifying_key);
    
    // The following would NOT compile:
    // let builder = GraphBuilder::new(GraphType::ProductionDAG);
    // executor.run(builder);  // ERROR: expected ValidatedGraph
}
```

#### Security Pipeline Tests

```rust
#[test]
fn test_rejects_incomplete_security_pipeline() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    // Add node without security stage
    let spec = NodeSpec {
        directives: DirectiveSet::without_security(),
        ..valid_node_spec()
    };
    
    builder.add_node(spec);
    
    // Should fail - mandatory security stage missing
    let result = builder.validate();
    assert!(matches!(result, Err(ValidationError::SecurityPipelineIncomplete)));
}

#[test]
fn test_accepts_complete_security_pipeline() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let spec = NodeSpec {
        directives: DirectiveSet::with_full_security(),
        ..valid_node_spec()
    };
    
    builder.add_node(spec);
    
    // Should validate - all mandatory stages present
    assert!(builder.validate().is_ok());
}
```

---

### 24.2.2 Execution Phase Tests

Tests for `Executor` - verify **zero policy checks** at runtime.

#### Token Integrity Tests (NOT Policy Validation)

```rust
#[test]
fn test_executor_verifies_token_signature() {
    let validated = create_validated_graph();
    let executor = Executor::new(verifying_key);
    
    // If token signature is invalid, execution fails
    // This is INTEGRITY verification, not policy validation
    let result = executor.run(validated).await;
    assert!(result.is_ok());
}

#[test]
fn test_executor_rejects_expired_token() {
    // Token with past expiration
    let validated = create_validated_graph_with_expired_token();
    let executor = Executor::new(verifying_key);
    
    let result = executor.run(validated).await;
    assert!(matches!(result, Err(ExecutionError::TokenExpired)));
}

#[test]
fn test_executor_checks_token_node_binding() {
    // Token bound to different node
    let validated = create_validated_graph_with_mismatched_token();
    let executor = Executor::new(verifying_key);
    
    let result = executor.run(validated).await;
    assert!(matches!(result, Err(ExecutionError::TokenBindingFailure)));
}

#[test]
fn test_zero_runtime_policy_validation() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    static POLICY_CHECK_COUNT: AtomicUsize = AtomicUsize::new(0);
    
    // Inject counter into ConstructionValidator
    // Executor should NOT increment this counter
    
    let validated = create_validated_graph();
    let executor = Executor::new(verifying_key);
    
    executor.run(validated).await.unwrap();
    
    // CRITICAL: Zero policy checks during execution
    assert_eq!(POLICY_CHECK_COUNT.load(Ordering::SeqCst), 0);
}
```

#### State Machine Tests

```rust
#[test]
fn test_state_transition_enforced() {
    let validated = create_validated_graph_with_states(vec![
        NodeState::Created,
        NodeState::Isolated,
        NodeState::Testing,
    ]);
    
    let executor = Executor::new(verifying_key);
    let result = executor.run(validated).await;
    
    assert!(result.is_ok());
}

#[test]
fn test_illegal_state_transition_fails() {
    // Graph with illegal transition: Created → Merged
    let validated = create_validated_graph_with_transition(
        NodeState::Created,
        NodeState::Merged,  // Illegal
    );
    
    let executor = Executor::new(verifying_key);
    let result = executor.run(validated).await;
    
    assert!(matches!(result, Err(ExecutionError::IllegalStateTransition)));
}
```

#### Resource Enforcement Tests (NOT Validation)

```rust
#[test]
fn test_container_enforces_memory_limit() {
    let spec = NodeSpec {
        resource_bounds: ResourceCaps {
            memory_bytes: 1024 * 1024,  // 1 MB
            ..valid_resource_caps()
        },
        ..valid_node_spec()
    };
    
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    builder.add_node(spec);
    let validated = builder.validate().unwrap();
    
    let executor = Executor::new(verifying_key);
    
    // Work that tries to exceed 1MB
    let result = executor.run_with_work(validated, memory_intensive_work()).await;
    
    // Container ENFORCES the pre-declared limit
    assert!(matches!(result, Err(ExecutionError::ResourceEnforcementTriggered)));
}

#[test]
fn test_container_enforces_cpu_time_limit() {
    let spec = NodeSpec {
        resource_bounds: ResourceCaps {
            cpu_time_ms: 100,  // 100ms
            ..valid_resource_caps()
        },
        ..valid_node_spec()
    };
    
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    builder.add_node(spec);
    let validated = builder.validate().unwrap();
    
    let executor = Executor::new(verifying_key);
    
    // Work that exceeds 100ms
    let result = executor.run_with_work(validated, cpu_intensive_work()).await;
    
    assert!(matches!(result, Err(ExecutionError::ResourceEnforcementTriggered)));
}
```

---

### 24.2.3 Dynamic Expansion Tests

Tests for `StagedConstruction` and expansion validation.

#### Expansion Validation Tests

```rust
#[test]
fn test_expansion_requires_validation() {
    let mut staged = create_staged_construction_with_expansion_point();
    
    // Provide expansion subgraph
    let subgraph = SubgraphSpec::<TestSchema> {
        nodes: vec![valid_node_spec(), valid_node_spec()],
        edges: vec![(NodeId::new(), NodeId::new())],
        _phantom: PhantomData,
    };
    
    // Must validate before completing
    staged.provide_expansion(subgraph).unwrap();
    
    let expanded = staged.complete_expansion();
    assert!(expanded.is_ok());
}

#[test]
fn test_rejects_invalid_expansion_subgraph() {
    let mut staged = create_staged_construction_with_expansion_point();
    
    // Invalid subgraph - exceeds resource budget
    let subgraph = SubgraphSpec::<TestSchema> {
        nodes: vec![NodeSpec {
            resource_bounds: ResourceCaps {
                cpu_time_ms: u64::MAX,  // Exceeds parent budget
                ..valid_resource_caps()
            },
            ..valid_node_spec()
        }],
        edges: vec![],
        _phantom: PhantomData,
    };
    
    // Should fail validation
    let result = staged.provide_expansion(subgraph);
    assert!(matches!(result, Err(ValidationError::ExpansionBudgetExceeded)));
}

#[test]
fn test_expansion_inherits_autonomy_ceiling() {
    let parent_ceiling = AutonomyLevel::L3;
    
    let mut staged = create_staged_construction_with_expansion_point_and_ceiling(parent_ceiling);
    
    let subgraph = SubgraphSpec::<TestSchema> {
        nodes: vec![NodeSpec {
            autonomy_ceiling: AutonomyLevel::L5,  // Exceeds parent
            ..valid_node_spec()
        }],
        edges: vec![],
        _phantom: PhantomData,
    };
    
    // Should fail - expansion cannot exceed parent ceiling
    let result = staged.provide_expansion(subgraph);
    assert!(matches!(result, Err(ValidationError::AutonomyCeilingExceeded)));
}
```

#### Recursive Expansion Tests

```rust
#[test]
fn test_recursive_expansion_depth_limit() {
    let max_depth = 2;
    let mut staged = create_staged_construction_with_recursive_expansion(max_depth);
    
    // First expansion
    let subgraph1 = create_expansion_with_expansion_node(max_depth - 1);
    staged.provide_expansion(subgraph1).unwrap();
    let expanded1 = staged.complete_expansion().unwrap();
    
    // Execute to next expansion point
    let mut staged2 = StagedConstruction::new(expanded1);
    staged2.execute_until_expansion().await.unwrap();
    
    // Second expansion
    let subgraph2 = create_simple_subgraph();
    staged2.provide_expansion(subgraph2).unwrap();
    let expanded2 = staged2.complete_expansion().unwrap();
    
    // Third expansion should fail (depth limit)
    let mut staged3 = StagedConstruction::new(expanded2);
    // ...
}
```

---

### 24.2.4 Property-Based Tests

```rust
proptest! {
    // DAG operations maintain acyclicity (production)
    #[test]
    fn prop_dag_remains_acyclic(ops: Vec<DagOperation>) {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        for op in ops {
            match op {
                DagOperation::AddNode => { builder.add_node(valid_node_spec()); }
                DagOperation::AddEdge(from, to) => {
                    // May fail if would create cycle
                    let _ = builder.add_edge(from, to);
                }
            }
        }
        
        // If validation succeeds, graph is acyclic
        if let Ok(validated) = builder.validate() {
            assert!(is_acyclic(&validated));
        }
    }
    
    // Construction-time resource proving
    #[test]
    fn prop_resource_bounds_provable(nodes: Vec<NodeSpec>) {
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        
        for node in nodes {
            builder.add_node(node);
        }
        
        // Validation proves or disproves resource bounds
        match builder.validate() {
            Ok(validated) => {
                // Bounds were provable
                assert!(resource_bounds_provable(&validated));
            }
            Err(ValidationError::ResourceBoundsNotProvable) => {
                // Bounds not provable - correct rejection
            }
            _ => panic!("Unexpected validation error"),
        }
    }
    
    // Token integrity verification
    #[test]
    fn prop_token_forgery_detected(mut token: CapabilityToken) {
        // Tamper with token
        token.signature = forge_signature(&token);
        
        // Integrity verification should fail
        let result = TokenIntegrity::verify_integrity(&token, &verifying_key);
        assert!(result.is_err());
    }
    
    // Hash chain integrity
    #[test]
    fn prop_hash_chain_unbroken(events: Vec<Event>) {
        let log = EventLog::new();
        
        for event in events {
            log.append(event).unwrap();
        }
        
        assert!(log.verify_integrity().is_ok());
    }
    
    // Expansion type safety
    #[test]
    fn prop_expansion_type_safe(schema: TestSchema, subgraph: SubgraphSpec<TestSchema>) {
        // Subgraph must conform to schema or validation fails
        match schema.validate_subgraph(&subgraph) {
            Ok(()) => {
                // Schema-conforming
            }
            Err(_) => {
                // Non-conforming - correctly rejected
            }
        }
    }
}
```

---

### 24.2.5 Integration Tests

```rust
// Test complete two-phase workflow
#[test]
fn test_complete_construction_execution_workflow() {
    // === CONSTRUCTION PHASE ===
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let n1 = builder.add_node(NodeSpec {
        autonomy_ceiling: AutonomyLevel::L3,
        resource_bounds: ResourceCaps { cpu_time_ms: 5000, memory_bytes: 512*1024*1024, token_limit: 50000, iteration_cap: 500 },
        directives: DirectiveSet::with_full_security(),
        expansion_type: None,
    });
    
    let n2 = builder.add_node(NodeSpec {
        autonomy_ceiling: AutonomyLevel::L3,
        resource_bounds: ResourceCaps { cpu_time_ms: 5000, memory_bytes: 512*1024*1024, token_limit: 50000, iteration_cap: 500 },
        directives: DirectiveSet::with_full_security(),
        expansion_type: None,
    });
    
    builder.add_edge(n1, n2).unwrap();
    
    // Validate - produces proof-carrying graph
    let validated = builder.validate().expect("Validation should succeed");
    
    // === EXECUTION PHASE ===
    let executor = Executor::new(verifying_key);
    let result = executor.run(validated).await;
    
    assert!(result.is_ok());
}

// Test expansion workflow
#[tokio::test]
async fn test_complete_expansion_workflow() {
    // Create base graph with expansion node
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let base_node = builder.add_node(valid_node_spec());
    let expansion_node = builder.add_expansion_node::<TestSchema>(
        valid_node_spec(),
        ResourceCaps { cpu_time_ms: 10000, memory_bytes: 1024*1024*1024, token_limit: 100000, iteration_cap: 1000 },
        2, // max depth
    );
    
    builder.add_edge(base_node, expansion_node).unwrap();
    
    let validated = builder.validate().unwrap();
    
    // Execute until expansion
    let mut staged = StagedConstruction::new(validated);
    let expansion_point = staged.execute_until_expansion().await.unwrap();
    
    // Provide expansion subgraph
    let subgraph = create_valid_subgraph_for_schema::<TestSchema>();
    staged.provide_expansion(subgraph).unwrap();
    
    // Complete expansion
    let expanded = staged.complete_expansion().unwrap();
    
    // Continue execution
    let executor = Executor::new(verifying_key);
    let result = executor.run(expanded).await;
    
    assert!(result.is_ok());
}

// Test escalation workflow
#[test]
fn test_escalation_via_contract() {
    let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
    
    let node = builder.add_node(NodeSpec {
        // Escalation thresholds embedded in contract
        directives: DirectiveSet::with_escalation_thresholds(3, 1, 1),
        ..valid_node_spec()
    });
    
    let validated = builder.validate().unwrap();
    
    // Execution respects embedded escalation rules
    let executor = Executor::new(verifying_key);
    
    // After 3 failures, autonomy should reduce
    // After reduction, escalation state triggered
}
```

---

### 24.2.6 Determinism Verification

```rust
#[test]
fn test_deterministic_construction() {
    // Same inputs → Same ValidatedGraph
    let builder1 = create_builder_with_ops(operations1);
    let builder2 = create_builder_with_ops(operations1);
    
    let validated1 = builder1.validate().unwrap();
    let validated2 = builder2.validate().unwrap();
    
    assert_eq!(validated1.validation_hash(), validated2.validation_hash());
}

#[test]
fn test_deterministic_execution() {
    // Same ValidatedGraph → Same execution result
    let validated1 = create_validated_graph_with_seed(12345);
    let validated2 = create_validated_graph_with_seed(12345);
    
    let executor = Executor::new(verifying_key);
    
    let result1 = executor.run(validated1).await;
    let result2 = executor.run(validated2).await;
    
    assert_eq!(result1, result2);
}
```

---

## 24.3 Test Organization

```
tests/
├── construction/           # GraphBuilder tests
│   ├── dag_tests.rs
│   ├── node_spec_tests.rs
│   ├── validation_tests.rs
│   └── security_pipeline_tests.rs
├── execution/              # Executor tests
│   ├── token_integrity_tests.rs
│   ├── state_machine_tests.rs
│   ├── resource_enforcement_tests.rs
│   └── zero_policy_tests.rs
├── expansion/              # StagedConstruction tests
│   ├── expansion_validation_tests.rs
│   ├── recursion_tests.rs
│   └── type_safety_tests.rs
├── integration/            # End-to-end tests
│   ├── construction_execution.rs
│   ├── expansion_workflow.rs
│   └── escalation_flow.rs
├── property/               # Property-based tests
│   ├── dag_properties.rs
│   ├── resource_properties.rs
│   ├── token_properties.rs
│   └── expansion_properties.rs
└── negative/               # Failure mode tests
    ├── invalid_construction.rs
    ├── integrity_failures.rs
    └── enforcement_triggers.rs
```

---

## 24.4 COA Simulator Test Integration

Since the COA Simulator is our primary testing mechanism:

```rust
// Simulator must test two-phase architecture
#[test]
fn test_simulator_uses_construction_phase() {
    let config = SimulatorConfig {
        test_construction: true,
        ..Default::default()
    };
    
    let report = run_simulator(config);
    
    // Verify construction-phase operations
    assert!(report.has_construction_operations());
    assert!(report.construction_validation_rate > 0.0);
}

#[test]
fn test_simulator_uses_execution_phase() {
    let config = SimulatorConfig {
        test_execution: true,
        ..Default::default()
    };
    
    let report = run_simulator(config);
    
    // Verify execution on pre-validated graphs
    assert!(report.has_execution_operations());
    assert_eq!(report.runtime_policy_violations, 0); // CRITICAL
}

#[test]
fn test_simulator_verifies_zero_runtime_policy() {
    let config = SimulatorConfig {
        verify_zero_runtime_policy: true,
        ..Default::default()
    };
    
    let report = run_simulator(config);
    
    // This is the key invariant
    assert_eq!(report.runtime_policy_validation_count, 0,
        "Simulator detected runtime policy validation - architecture violation!");
}
```

---

# END OF TEST SPECIFICATION
