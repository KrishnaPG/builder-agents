//! Agent pool for managing agent lifecycle
//!
//! Provides efficient agent reuse and lifecycle management:
//! - Agent acquisition (create or reuse)
//! - Message passing to agents
//! - Pool statistics and monitoring

use crate::error::PoolError;
use crate::types::{AgentId, AgentSpec, Task};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Agent handle for communication
#[derive(Debug, Clone)]
pub struct AgentHandle {
    /// Agent ID
    pub id: AgentId,
    /// Agent specification
    pub spec: AgentSpec,
    /// Communication channel
    sender: mpsc::Sender<AgentMessage>,
}

impl AgentHandle {
    /// Send message to agent
    pub async fn send(&self, message: AgentMessage) -> Result<(), PoolError> {
        self.sender
            .send(message)
            .await
            .map_err(|_| PoolError::CommunicationFailed("channel closed".to_string()))
    }

    /// Get agent ID
    #[inline]
    #[must_use]
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Get agent spec
    #[inline]
    #[must_use]
    pub fn spec(&self) -> &AgentSpec {
        &self.spec
    }
}

/// Messages sent to agents
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// Execute a task
    Execute(Task),
    /// Shutdown agent
    Shutdown,
    /// Pause execution
    Pause,
    /// Resume execution
    Resume,
}

/// Messages received from agents
#[derive(Debug, Clone)]
pub enum AgentResponse {
    /// Task completed successfully
    TaskCompleted { task_id: String, result: TaskResult },
    /// Task failed
    TaskFailed { task_id: String, error: String },
    /// Agent ready
    Ready,
    /// Agent shutting down
    ShuttingDown,
}

/// Task execution result
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Output delta reference (placeholder)
    pub delta_ref: Option<String>,
    /// Execution metrics
    pub metrics: ExecutionMetrics,
}

/// Execution metrics
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetrics {
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Memory used in MB
    pub memory_used_mb: usize,
    /// Tokens consumed (if LLM-based)
    pub tokens_consumed: Option<usize>,
}

/// Pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total agents created
    pub total_created: usize,
    /// Currently active agents
    pub active_count: usize,
    /// Available agents in pool
    pub available_count: usize,
    /// Total tasks executed
    pub total_tasks_executed: usize,
    /// Cache hit rate (reused agents)
    pub reuse_rate: f64,
}

/// Agent pool for lifecycle management
#[derive(Debug)]
pub struct AgentPool {
    /// Maximum pool size
    max_size: usize,
    /// Available agents (LIFO for cache efficiency)
    available: Mutex<Vec<AgentHandle>>,
    /// Active agents
    active: DashMap<AgentId, AgentHandle>,
    /// Statistics
    stats: Mutex<PoolStats>,
}

impl AgentPool {
    /// Create new agent pool
    #[inline]
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            available: Mutex::new(Vec::new()),
            active: DashMap::new(),
            stats: Mutex::new(PoolStats::default()),
        }
    }

    /// Acquire an agent (reuse or create)
    ///
    /// # Arguments
    /// * `spec` - Agent specification
    ///
    /// # Returns
    /// Agent handle for communication
    ///
    /// # Errors
    /// - `PoolError::PoolExhausted` if max agents active
    pub async fn acquire(&self, spec: AgentSpec) -> Result<AgentHandle, PoolError> {
        // Try to find matching available agent
        let mut available = self.available.lock().await;

        if let Some(idx) = available.iter().position(|a| a.spec.role == spec.role) {
            // Reuse agent
            let agent = available.remove(idx);
            self.active.insert(agent.id, agent.clone());

            let mut stats = self.stats.lock().await;
            stats.available_count = available.len();
            stats.active_count = self.active.len();

            return Ok(agent);
        }

        drop(available);

        // Check capacity
        if self.active.len() >= self.max_size {
            return Err(PoolError::PoolExhausted(self.max_size));
        }

        // Create new agent
        let agent = self.create_agent(spec).await?;
        self.active.insert(agent.id, agent.clone());

        let mut stats = self.stats.lock().await;
        stats.total_created += 1;
        stats.active_count = self.active.len();

        Ok(agent)
    }

    /// Release agent back to pool
    ///
    /// # Arguments
    /// * `agent` - Agent to release
    pub async fn release(&self, agent: AgentHandle) {
        self.active.remove(&agent.id);

        let mut available = self.available.lock().await;
        if available.len() < self.max_size {
            available.push(agent);
        }
        // Else: drop agent

        let mut stats = self.stats.lock().await;
        stats.available_count = available.len();
        stats.active_count = self.active.len();
    }

    /// Shutdown specific agent
    pub async fn shutdown_agent(&self, agent_id: AgentId) -> Result<(), PoolError> {
        if let Some((_, agent)) = self.active.remove(&agent_id) {
            let _ = agent.send(AgentMessage::Shutdown).await;
        }

        let mut available = self.available.lock().await;
        if let Some(idx) = available.iter().position(|a| a.id == agent_id) {
            let agent = available.remove(idx);
            let _ = agent.send(AgentMessage::Shutdown).await;
        }

        let mut stats = self.stats.lock().await;
        stats.available_count = available.len();
        stats.active_count = self.active.len();

        Ok(())
    }

    /// Shutdown all agents
    pub async fn shutdown_all(&self) {
        // Shutdown active agents
        for entry in self.active.iter() {
            let _ = entry.value().send(AgentMessage::Shutdown).await;
        }
        self.active.clear();

        // Shutdown available agents
        let mut available = self.available.lock().await;
        for agent in available.drain(..) {
            let _ = agent.send(AgentMessage::Shutdown).await;
        }

        let mut stats = self.stats.lock().await;
        stats.available_count = 0;
        stats.active_count = 0;
    }

    /// Get pool statistics
    #[inline]
    #[must_use]
    pub async fn stats(&self) -> PoolStats {
        self.stats.lock().await.clone()
    }

    /// Get active agent count
    #[inline]
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Create new agent
    async fn create_agent(&self, spec: AgentSpec) -> Result<AgentHandle, PoolError> {
        let id = AgentId::new();
        let (tx, rx) = mpsc::channel(100);

        // Spawn agent task
        tokio::spawn(agent_task(id, spec.clone(), rx));

        Ok(AgentHandle {
            id,
            spec,
            sender: tx,
        })
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new(10)
    }
}

/// Agent task (runs in separate tokio task)
async fn agent_task(
    _id: AgentId,
    _spec: AgentSpec,
    mut rx: mpsc::Receiver<AgentMessage>,
) {
    // Agent lifecycle loop
    while let Some(msg) = rx.recv().await {
        match msg {
            AgentMessage::Execute(_task) => {
                // Execute task (placeholder)
                // In real implementation, this would:
                // 1. Set up execution context
                // 2. Run agent logic
                // 3. Produce delta
                // 4. Send response
            }
            AgentMessage::Shutdown => break,
            AgentMessage::Pause => {
                // Pause execution
            }
            AgentMessage::Resume => {
                // Resume execution
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coa_artifact::SymbolPath;

    #[tokio::test]
    async fn agent_pool_acquire_and_release() {
        let pool = AgentPool::new(2);

        let spec = AgentSpec::new("tester");

        let agent1 = pool.acquire(spec.clone()).await.unwrap();
        let agent2 = pool.acquire(spec.clone()).await.unwrap();

        // Third acquire should fail
        let result = pool.acquire(spec.clone()).await;
        assert!(matches!(result, Err(PoolError::PoolExhausted(2))));

        // Release one
        pool.release(agent1).await;

        // Now acquire should succeed
        let agent3 = pool.acquire(spec).await;
        assert!(agent3.is_ok());
    }

    #[tokio::test]
    async fn agent_pool_reuse() {
        let pool = AgentPool::new(2);

        let spec = AgentSpec::new("tester");

        // Acquire and release
        let agent1 = pool.acquire(spec.clone()).await.unwrap();
        let id1 = agent1.id;
        pool.release(agent1).await;

        // Acquire again - should get same agent
        let agent2 = pool.acquire(spec).await.unwrap();
        let id2 = agent2.id;

        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn agent_pool_stats() {
        let pool = AgentPool::new(2);

        let spec = AgentSpec::new("tester");

        let agent = pool.acquire(spec).await.unwrap();

        let stats = pool.stats().await;
        assert_eq!(stats.active_count, 1);
        assert_eq!(stats.total_created, 1);

        pool.release(agent).await;

        let stats = pool.stats().await;
        assert_eq!(stats.active_count, 0);
        assert_eq!(stats.available_count, 1);
    }

    #[tokio::test]
    async fn agent_handle_send() {
        let pool = AgentPool::new(1);
        let spec = AgentSpec::new("tester");

        let agent = pool.acquire(spec).await.unwrap();

        // Send shutdown message
        let result = agent.send(AgentMessage::Shutdown).await;
        assert!(result.is_ok());
    }
}
