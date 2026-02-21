use crate::api::{ExecutionResult, ResourceUsage, ScheduleToken, Scheduler, SchedulerError};
use crate::autonomy::CapabilityToken;
use crate::types::NodeId;
use std::time::Duration;
use tokio::time::sleep;

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
        _node_id: NodeId,
        _token: &CapabilityToken,
    ) -> Result<ScheduleToken, SchedulerError> {
        Ok(ScheduleToken {
            node_id: _node_id,
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
