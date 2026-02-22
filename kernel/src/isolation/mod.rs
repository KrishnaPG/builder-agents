//! Isolation Module (v2.0)
//!
//! Determines isolation level based on NodeSpec from the construction phase.
//! All policy decisions have already been validated - this module only
//! implements the isolation primitives.

use crate::api::{ApiExecutionError, ApiExecutionErrorKind, ExecutionResult, ExecutionRuntime, ResourceUsage};
use crate::autonomy::CapabilityToken;
use crate::types::v2::NodeSpecV2;
use crate::types::{AutonomyLevel, NodeId, WorkSpec};
use std::process::{Command, Stdio};
use std::thread;

/// Isolation executor (v2.0)
///
/// Determines isolation level from NodeSpec (pre-validated at construction)
/// rather than from the token at runtime.
pub struct Isolation;

impl Isolation {
    pub fn new() -> Self {
        Self
    }
    
    /// Determine isolation level from node spec
    ///
    /// In v2.0, this comes from the pre-validated NodeSpec, not the token.
    fn isolation_level_from_spec(spec: &NodeSpecV2) -> IsolationLevel {
        match spec.autonomy_ceiling {
            AutonomyLevel::L0 | AutonomyLevel::L1 | AutonomyLevel::L2 => {
                IsolationLevel::Thread
            }
            AutonomyLevel::L3 | AutonomyLevel::L4 | AutonomyLevel::L5 => {
                IsolationLevel::Subprocess
            }
        }
    }
    
    /// Execute work with the appropriate isolation
    ///
    /// # Arguments
    /// * `spec` - The pre-validated node specification
    /// * `work` - The work to execute
    pub fn execute_with_spec(
        &self,
        spec: &NodeSpecV2,
        work: WorkSpec,
    ) -> Result<String, ApiExecutionError> {
        match Self::isolation_level_from_spec(spec) {
            IsolationLevel::Thread => self.execute_in_thread(work),
            IsolationLevel::Subprocess => self.execute_in_subprocess(work),
        }
    }
    
    fn execute_in_thread(&self, work: WorkSpec) -> Result<String, ApiExecutionError> {
        let handle = thread::spawn(move || {
            println!("Executing work in thread: {:?}", work);
            "Thread execution completed".to_string()
        });
        
        match handle.join() {
            Ok(result) => Ok(result),
            Err(_) => Err(ApiExecutionError {
                node_id: None,
                kind: ApiExecutionErrorKind::IsolationFailure,
                message: "Thread panicked".to_string(),
            }),
        }
    }
    
    fn execute_in_subprocess(&self, work: WorkSpec) -> Result<String, ApiExecutionError> {
        let mut cmd = Command::new("echo");
        cmd.arg(format!("Executing work in subprocess: {:?}", work));
        cmd.env_clear();
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());
        
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    Err(ApiExecutionError {
                        node_id: None,
                        kind: ApiExecutionErrorKind::Internal,
                        message: "Subprocess failed".to_string(),
                    })
                }
            }
            Err(e) => Err(ApiExecutionError {
                node_id: None,
                kind: ApiExecutionErrorKind::IsolationFailure,
                message: format!("Failed to spawn subprocess: {}", e),
            }),
        }
    }
}

/// Isolation level determined at construction time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Thread-level isolation (L0-L2)
    Thread,
    /// Subprocess-level isolation (L3-L5)
    Subprocess,
}

/// Legacy implementation for backward compatibility in tests
impl ExecutionRuntime for Isolation {
    fn execute(
        &self,
        node_id: NodeId,
        token: &CapabilityToken,
        work: WorkSpec,
    ) -> Result<ExecutionResult, ApiExecutionError> {
        // Legacy: read from token (v1.x behavior)
        match token.autonomy_level {
            AutonomyLevel::L0 | AutonomyLevel::L1 | AutonomyLevel::L2 => {
                let result = self.execute_in_thread(work)?;
                Ok(ExecutionResult {
                    success: true,
                    node_id,
                    output: Some(result),
                    resource_usage: ResourceUsage::default(),
                })
            }
            AutonomyLevel::L3 | AutonomyLevel::L4 | AutonomyLevel::L5 => {
                let result = self.execute_in_subprocess(work)?;
                Ok(ExecutionResult {
                    success: true,
                    node_id,
                    output: Some(result),
                    resource_usage: ResourceUsage::default(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DirectiveSet;
    use std::collections::BTreeMap;

    fn create_spec(autonomy: AutonomyLevel) -> NodeSpecV2 {
        NodeSpecV2 {
            directives: DirectiveSet {
                directives: BTreeMap::new(),
            },
            autonomy_ceiling: autonomy,
            resource_bounds: crate::types::ResourceCaps {
                cpu_time_ms: 1000,
                memory_bytes: 1024 * 1024,
                token_limit: 1000,
                iteration_cap: 100,
            },
            expansion_type: None,
        }
    }

    #[test]
    fn test_isolation_level_from_spec() {
        let _isolation = Isolation::new();
        
        // L0-L2 -> Thread
        assert_eq!(
            Isolation::isolation_level_from_spec(&create_spec(AutonomyLevel::L0)),
            IsolationLevel::Thread
        );
        assert_eq!(
            Isolation::isolation_level_from_spec(&create_spec(AutonomyLevel::L2)),
            IsolationLevel::Thread
        );
        
        // L3-L5 -> Subprocess
        assert_eq!(
            Isolation::isolation_level_from_spec(&create_spec(AutonomyLevel::L3)),
            IsolationLevel::Subprocess
        );
        assert_eq!(
            Isolation::isolation_level_from_spec(&create_spec(AutonomyLevel::L5)),
            IsolationLevel::Subprocess
        );
    }

    #[test]
    fn test_execute_in_thread() {
        let isolation = Isolation::new();
        let work = WorkSpec {
            kind: "test".to_string(),
            payload: serde_json::json!("test"),
        };
        
        let result = isolation.execute_in_thread(work);
        assert!(result.is_ok());
    }
}
