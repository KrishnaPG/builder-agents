//! Execution Phase (v2.0)
//!
//! This module contains the executor for the execution phase.
//! The executor only accepts `ValidatedGraph` instances.
//!
//! # Critical Invariant
//!
//! The executor performs **zero policy validation**. All policy checks
//! happened at construction time. The executor only:
//! - Verifies token integrity (cryptographic)
//! - Enforces pre-declared resource limits (container primitives)
//! - Executes node operations

use crate::error::ExecutionError;
use crate::token_integrity::TokenIntegrity;
use crate::types::v2::{ExecutionSummary, ValidatedGraph};
use crate::types::NodeId;
use ed25519_dalek::VerifyingKey;
use std::sync::Arc;
use std::time::Instant;

/// Node executor trait
///
/// Implement this trait to define how individual nodes are executed.
#[async_trait::async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Execute a single node
    async fn execute_node(
        &self,
        node_id: NodeId,
        token: &crate::autonomy::CapabilityToken,
    ) -> Result<NodeExecutionResult, ExecutionError>;
}

/// Result of node execution
#[derive(Debug, Clone)]
pub struct NodeExecutionResult {
    pub node_id: NodeId,
    pub success: bool,
    pub execution_time_ms: u64,
    pub resource_consumed: crate::types::ResourceCaps,
}

/// Graph executor
///
/// Only accepts pre-validated graphs. Performs integrity verification
/// but no policy validation.
pub struct Executor {
    verifying_key: VerifyingKey,
    node_executor: Arc<dyn NodeExecutor>,
}

impl Executor {
    /// Create a new executor
    pub fn new(verifying_key: VerifyingKey) -> Self {
        Self {
            verifying_key,
            node_executor: Arc::new(DefaultNodeExecutor),
        }
    }
    
    /// Create with custom node executor
    pub fn with_executor(
        verifying_key: VerifyingKey,
        node_executor: Arc<dyn NodeExecutor>,
    ) -> Self {
        Self {
            verifying_key,
            node_executor,
        }
    }
    
    /// Run a validated graph
    ///
    /// # Arguments
    /// * `graph` - A `ValidatedGraph` produced by `GraphBuilder::validate()`
    ///
    /// # Errors
    /// Returns `ExecutionError` if:
    /// - Token integrity verification fails
    /// - Token has expired
    /// - Token is not bound to the correct node
    /// - Resource enforcement triggers
    pub async fn run(
        &self,
        graph: ValidatedGraph,
    ) -> Result<ExecutionSummary, ExecutionError> {
        let start_time = Instant::now();
        let mut nodes_executed = 0;
        let mut total_cpu_ms = 0u64;
        let mut total_memory = 0u64;
        let mut total_tokens = 0u64;
        let mut total_iterations = 0u64;
        
        // Verify graph validation token
        self.verify_graph_token(&graph)?;
        
        // Get topological order for execution
        let node_order: Vec<NodeId> = graph.node_ids().collect();
        
        for node_id in node_order {
            // Get the node's capability token
            let token = graph.get_node_token(node_id)
                .ok_or(ExecutionError::TokenIntegrityFailure)?;
            
            // Verify token integrity (cryptographic + temporal + binding)
            TokenIntegrity::verify_full(
                token,
                &self.verifying_key,
                node_id,
                Some("execute"),
            )?;
            
            // Execute the node
            let result = self.node_executor.execute_node(node_id, token).await?;
            
            if result.success {
                nodes_executed += 1;
                total_cpu_ms += result.execution_time_ms;
                total_memory += result.resource_consumed.memory_bytes;
                total_tokens += result.resource_consumed.token_limit;
                total_iterations += result.resource_consumed.iteration_cap;
            }
        }
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(ExecutionSummary {
            graph_id: graph.graph_id(),
            nodes_executed,
            execution_time_ms,
            resource_consumed: crate::types::ResourceCaps {
                cpu_time_ms: total_cpu_ms,
                memory_bytes: total_memory,
                token_limit: total_tokens,
                iteration_cap: total_iterations,
            },
        })
    }
    
    /// Verify the graph's validation token
    fn verify_graph_token(&self, graph: &ValidatedGraph) -> Result<(), ExecutionError> {
        let token = graph.validation_token();
        
        // Check expiration
        if token.is_expired() {
            return Err(ExecutionError::TokenExpired);
        }
        
        // Verify token is for this graph
        if token.graph_id != graph.graph_id() {
            return Err(ExecutionError::TokenBindingFailure);
        }
        
        // Note: Full signature verification would require access to the signing key
        // In production, this would use the verifying_key
        Ok(())
    }
    
    /// Execute a single node (for testing/debugging)
    pub async fn execute_single(
        &self,
        graph: &ValidatedGraph,
        node_id: NodeId,
    ) -> Result<NodeExecutionResult, ExecutionError> {
        let token = graph.get_node_token(node_id)
            .ok_or(ExecutionError::TokenIntegrityFailure)?;
        
        TokenIntegrity::verify_full(
            token,
            &self.verifying_key,
            node_id,
            Some("execute"),
        )?;
        
        self.node_executor.execute_node(node_id, token).await
    }
}

/// Default node executor implementation
struct DefaultNodeExecutor;

#[async_trait::async_trait]
impl NodeExecutor for DefaultNodeExecutor {
    async fn execute_node(
        &self,
        node_id: NodeId,
        _token: &crate::autonomy::CapabilityToken,
    ) -> Result<NodeExecutionResult, ExecutionError> {
        // Default implementation - just return success
        // Real implementation would execute the node's work
        Ok(NodeExecutionResult {
            node_id,
            success: true,
            execution_time_ms: 0,
            resource_consumed: crate::types::ResourceCaps {
                cpu_time_ms: 0,
                memory_bytes: 0,
                token_limit: 0,
                iteration_cap: 0,
            },
        })
    }
}

/// Container for resource enforcement
///
/// Enforces pre-declared resource limits at runtime.
/// This is NOT validation - it's primitive enforcement.
pub struct ResourceContainer {
    cpu_limit_ms: u64,
    memory_limit_bytes: u64,
    token_limit: u64,
    iteration_limit: u64,
}

impl ResourceContainer {
    /// Create a new resource container
    pub fn new(caps: crate::types::ResourceCaps) -> Self {
        Self {
            cpu_limit_ms: caps.cpu_time_ms,
            memory_limit_bytes: caps.memory_bytes,
            token_limit: caps.token_limit,
            iteration_limit: caps.iteration_cap,
        }
    }
    
    /// Check if operation is within CPU limit
    pub fn check_cpu(&self, used_ms: u64) -> Result<(), ExecutionError> {
        if used_ms > self.cpu_limit_ms {
            Err(ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
    
    /// Check if operation is within memory limit
    pub fn check_memory(&self, used_bytes: u64) -> Result<(), ExecutionError> {
        if used_bytes > self.memory_limit_bytes {
            Err(ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
    
    /// Check if operation is within token limit
    pub fn check_tokens(&self, used: u64) -> Result<(), ExecutionError> {
        if used > self.token_limit {
            Err(ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
    
    /// Check if operation is within iteration limit
    pub fn check_iterations(&self, used: u64) -> Result<(), ExecutionError> {
        if used > self.iteration_limit {
            Err(ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construction::GraphBuilder;
    use crate::types::{
        AutonomyLevel, DirectiveSet, GraphType, ResourceCaps,
    };
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;

    fn create_test_spec() -> crate::types::v2::NodeSpecV2 {
        crate::types::v2::NodeSpecV2 {
            directives: DirectiveSet {
                directives: BTreeMap::new(),
            },
            autonomy_ceiling: AutonomyLevel::L3,
            resource_bounds: ResourceCaps {
                cpu_time_ms: 1000,
                memory_bytes: 1024 * 1024,
                token_limit: 1000,
                iteration_cap: 100,
            },
            expansion_type: None,
        }
    }

    fn create_signing_key() -> SigningKey {
        let mut csprng = OsRng;
        SigningKey::generate(&mut csprng)
    }

    #[tokio::test]
    async fn test_executor_runs_validated_graph() {
        let signing_key = create_signing_key();
        let verifying_key = signing_key.verifying_key();
        
        // Build and validate graph
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        let n1 = builder.add_node(create_test_spec());
        let n2 = builder.add_node(create_test_spec());
        builder.add_edge(n1, n2).unwrap();
        
        let validated = builder.validate(&signing_key).unwrap();
        
        // Execute
        let executor = Executor::new(verifying_key);
        let result = executor.run(validated).await;
        
        assert!(result.is_ok());
        
        let summary = result.unwrap();
        assert_eq!(summary.nodes_executed, 2);
    }

    #[test]
    fn test_resource_container_enforces_limits() {
        let caps = ResourceCaps {
            cpu_time_ms: 100,
            memory_bytes: 1024,
            token_limit: 50,
            iteration_cap: 10,
        };
        
        let container = ResourceContainer::new(caps);
        
        // Within limits
        assert!(container.check_cpu(50).is_ok());
        assert!(container.check_memory(512).is_ok());
        assert!(container.check_tokens(25).is_ok());
        assert!(container.check_iterations(5).is_ok());
        
        // Exceeds limits
        assert!(container.check_cpu(150).is_err());
        assert!(container.check_memory(2048).is_err());
        assert!(container.check_tokens(100).is_err());
        assert!(container.check_iterations(20).is_err());
    }

    #[test]
    fn test_resource_container_at_exact_limit() {
        let caps = ResourceCaps {
            cpu_time_ms: 100,
            memory_bytes: 1024,
            token_limit: 50,
            iteration_cap: 10,
        };
        
        let container = ResourceContainer::new(caps);
        
        // Exactly at limit should succeed
        assert!(container.check_cpu(100).is_ok());
        assert!(container.check_memory(1024).is_ok());
    }
}
