//! Scheduler Module (v2.0)
//!
//! The scheduler works with pre-validated graphs from the construction phase.
//! All policy decisions have already been made - the scheduler only handles
//! execution ordering and timing.

use crate::api::{ExecutionResult, ResourceUsage, ScheduleToken, Scheduler, SchedulerError, SchedulerErrorKind};
use crate::autonomy::CapabilityToken;
use crate::executor::Executor;
use crate::types::v2::ValidatedGraph;
use crate::types::NodeId;
use ed25519_dalek::VerifyingKey;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// v2.0 Scheduler that works with ValidatedGraph
///
/// The scheduler only accepts pre-validated graphs and handles
/// execution ordering without any policy validation.
pub struct GraphScheduler {
    executor: Arc<Executor>,
}

impl GraphScheduler {
    /// Create a new graph scheduler
    pub fn new(verifying_key: VerifyingKey) -> Self {
        Self {
            executor: Arc::new(Executor::new(verifying_key)),
        }
    }
    
    /// Create with custom executor
    pub fn with_executor(executor: Arc<Executor>) -> Self {
        Self { executor }
    }
    
    /// Schedule a node from a validated graph for execution
    ///
    /// # Arguments
    /// * `graph` - The pre-validated graph containing the node
    /// * `node_id` - The node to schedule
    ///
    /// # Errors
    /// Returns error if node not found in graph or scheduling fails
    pub fn schedule(
        &self,
        graph: &ValidatedGraph,
        node_id: NodeId,
    ) -> Result<ScheduleToken, SchedulerError> {
        // Verify node exists in graph (integrity check, not policy)
        if graph.get_node_spec(node_id).is_none() {
            return Err(SchedulerError {
                kind: SchedulerErrorKind::NodeNotFound,
                message: format!("Node {:?} not found in graph", node_id),
            });
        }
        
        // Verify token exists (integrity check)
        if graph.get_node_token(node_id).is_none() {
            return Err(SchedulerError {
                kind: SchedulerErrorKind::NodeNotFound,
                message: format!("No token for node {:?}", node_id),
            });
        }
        
        Ok(ScheduleToken {
            node_id,
            sequence: 0, // Would be incremented in real implementation
        })
    }
    
    /// Execute a node from a validated graph
    ///
    /// This performs zero policy validation - all validation happened
    /// during the construction phase.
    pub async fn execute_node(
        &self,
        graph: &ValidatedGraph,
        node_id: NodeId,
    ) -> Result<ExecutionResult, SchedulerError> {
        // Get token (integrity check only)
        let _token = graph.get_node_token(node_id)
            .ok_or_else(|| SchedulerError {
                kind: SchedulerErrorKind::NodeNotFound,
                message: format!("No token for node {:?}", node_id),
            })?;
        
        // Execute through executor (zero policy checks)
        match self.executor.execute_single(graph, node_id).await {
            Ok(result) => Ok(ExecutionResult {
                success: result.success,
                node_id: result.node_id,
                output: Some(format!("Executed node {:?}", node_id)),
                resource_usage: ResourceUsage {
                    cpu_time_ms: result.execution_time_ms,
                    memory_bytes: result.resource_consumed.memory_bytes,
                    tokens_used: result.resource_consumed.token_limit,
                    iterations: result.resource_consumed.iteration_cap,
                },
            }),
            Err(e) => Err(SchedulerError {
                kind: SchedulerErrorKind::Timeout,
                message: format!("Execution failed: {:?}", e),
            }),
        }
    }
    
    /// Schedule and execute all nodes in topological order
    pub async fn execute_graph(
        &self,
        graph: ValidatedGraph,
    ) -> Result<Vec<ExecutionResult>, SchedulerError> {
        let mut results = Vec::new();
        
        // Get all node IDs from the graph
        let node_ids: Vec<NodeId> = graph.node_ids().collect();
        
        for node_id in node_ids {
            let result = self.execute_node(&graph, node_id).await?;
            results.push(result);
        }
        
        Ok(results)
    }
}

/// Legacy scheduler implementation (for backward compatibility in tests)
pub struct BasicScheduler;

impl BasicScheduler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Scheduler for BasicScheduler {
    fn schedule(
        &self,
        node_id: NodeId,
        _token: &CapabilityToken,
    ) -> Result<ScheduleToken, SchedulerError> {
        Ok(ScheduleToken {
            node_id,
            sequence: 0,
        })
    }

    fn cancel(&self, _schedule_token: ScheduleToken) -> Result<(), SchedulerError> {
        Ok(())
    }

    async fn wait_for_completion(
        &self,
        node_id: NodeId,
        timeout: Duration,
    ) -> Result<ExecutionResult, SchedulerError> {
        sleep(std::cmp::min(timeout, Duration::from_millis(10))).await;
        Ok(ExecutionResult {
            success: true,
            node_id,
            output: Some("Completed".to_string()),
            resource_usage: ResourceUsage::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construction::GraphBuilder;
    use crate::types::{
        AutonomyLevel, DirectiveSet, GraphType, ResourceCaps,
    };
    use crate::types::v2::NodeSpecV2;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;

    fn create_test_spec() -> NodeSpecV2 {
        NodeSpecV2 {
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

    #[test]
    fn test_scheduler_verifies_node_exists() {
        let signing_key = create_signing_key();
        let verifying_key = signing_key.verifying_key();
        
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        let n1 = builder.add_node(create_test_spec());
        let validated = builder.validate(&signing_key).unwrap();
        
        let scheduler = GraphScheduler::new(verifying_key);
        
        // Should succeed for existing node
        assert!(scheduler.schedule(&validated, n1).is_ok());
        
        // Should fail for non-existent node
        let fake_node = NodeId::new();
        assert!(scheduler.schedule(&validated, fake_node).is_err());
    }

    #[tokio::test]
    async fn test_scheduler_executes_node() {
        let signing_key = create_signing_key();
        let verifying_key = signing_key.verifying_key();
        
        let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
        let n1 = builder.add_node(create_test_spec());
        let validated = builder.validate(&signing_key).unwrap();
        
        let scheduler = GraphScheduler::new(verifying_key);
        
        let result = scheduler.execute_node(&validated, n1).await;
        assert!(result.is_ok());
        
        let execution = result.unwrap();
        assert!(execution.success);
        assert_eq!(execution.node_id, n1);
    }
}
