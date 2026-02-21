use crate::api::{ExecutionError, ExecutionErrorKind, ExecutionResult, ExecutionRuntime, ResourceUsage};
use crate::autonomy::CapabilityToken;
use crate::types::{AutonomyLevel, NodeId, WorkSpec};
use std::process::{Command, Stdio};
use std::thread;

pub struct Isolation;

impl Isolation {
    pub fn new() -> Self {
        Self
    }
}

impl ExecutionRuntime for Isolation {
    fn execute(
        &self,
        node_id: NodeId,
        token: &CapabilityToken,
        work: WorkSpec,
    ) -> Result<ExecutionResult, ExecutionError> {
        match token.autonomy_level {
            AutonomyLevel::L0 | AutonomyLevel::L1 | AutonomyLevel::L2 => {
                // Thread isolation
                let handle = thread::spawn(move || {
                    println!("Executing work in thread: {:?}", work);
                    "Thread execution completed".to_string()
                });
                
                match handle.join() {
                    Ok(result) => Ok(ExecutionResult {
                        success: true,
                        node_id,
                        output: Some(result),
                        resource_usage: ResourceUsage::default(),
                    }),
                    Err(_) => Err(ExecutionError {
                        node_id: Some(node_id),
                        kind: ExecutionErrorKind::IsolationFailure,
                        message: "Thread panicked".to_string(),
                    }),
                }
            }
            AutonomyLevel::L3 | AutonomyLevel::L4 | AutonomyLevel::L5 => {
                // Subprocess isolation
                let mut cmd = Command::new("echo");
                cmd.arg(format!("Executing work in subprocess: {:?}", work));
                cmd.env_clear();
                cmd.stdout(Stdio::piped());
                cmd.stdin(Stdio::piped());
                
                match cmd.output() {
                    Ok(output) => {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                            Ok(ExecutionResult {
                                success: true,
                                node_id,
                                output: Some(stdout),
                                resource_usage: ResourceUsage::default(),
                            })
                        } else {
                            Err(ExecutionError {
                                node_id: Some(node_id),
                                kind: ExecutionErrorKind::Internal,
                                message: "Subprocess failed".to_string(),
                            })
                        }
                    }
                    Err(e) => Err(ExecutionError {
                        node_id: Some(node_id),
                        kind: ExecutionErrorKind::IsolationFailure,
                        message: format!("Failed to spawn subprocess: {}", e),
                    }),
                }
            }
        }
    }
}
