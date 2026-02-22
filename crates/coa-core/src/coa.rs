//! Creator Orchestrator Agent (COA)
//!
//! The central intelligence that:
//! - Parses user intent into structured specifications
//! - Decomposes work into tasks
//! - Manages agent lifecycle
//! - Handles construction failures with diagnostics
//! - Coordinates multi-agent composition

use crate::agent_pool::{AgentPool, AgentHandle};
use crate::decomposition::TaskDecomposer;
use crate::error::{COAError, DecompositionError, Diagnostic, ErrorType, Goal, Location, SuggestedFix};
use crate::types::{AgentSpec, ArtifactSummary, COAConfig, ExecutionResult, Specification, Task, UserIntent};
use coa_artifact::{Artifact, ArtifactType, StructuralDelta};
use coa_constitutional::parsers::CodeArtifact;
use coa_composition::CompositionStrategy;
// Constitutional layer will be integrated when ready
// use coa_constitutional::ConstitutionalLayer;
use coa_symbol::SymbolRefIndex;
use std::str::FromStr;
use std::sync::Arc;

/// The central orchestrator
///
/// Owns the artifact namespace, manages agents, and coordinates work.
#[derive(Debug)]
pub struct CreatorOrchestratorAgent {
    /// Configuration
    config: COAConfig,
    /// Symbol namespace
    symbol_index: Arc<SymbolRefIndex>,
    /// Active agent pool
    agent_pool: AgentPool,
    /// Task decomposer
    decomposer: TaskDecomposer,
}

impl CreatorOrchestratorAgent {
    /// Create new COA instance
    #[inline]
    #[must_use]
    pub fn new(config: COAConfig) -> Self {
        Self {
            config: config.clone(),
            symbol_index: Arc::new(SymbolRefIndex::new()),
            agent_pool: AgentPool::new(config.max_concurrent_agents),
            decomposer: TaskDecomposer::default(),
        }
    }

    /// Execute high-level user intent
    ///
    /// This is the main entry point for user interactions.
    ///
    /// # Workflow
    /// 1. Parse intent into structured specification
    /// 2. Decompose into tasks
    /// 3. Create execution graph
    /// 4. Validate and execute
    /// 5. Handle failures with diagnostics
    ///
    /// # Arguments
    /// * `intent` - User intent (natural language)
    ///
    /// # Returns
    /// Execution result with artifacts produced
    pub async fn execute_intent(
        &self,
        intent: UserIntent,
    ) -> Result<ExecutionResult, COAError> {
        tracing::info!("Executing intent: {}", intent.description);

        // 1. Parse intent into structured specification
        let spec = self.parse_intent(intent).await?;
        tracing::debug!("Parsed specification: {:?}", spec.goal);

        // 2. Decompose into tasks
        let tasks = self.decompose(spec).await?;
        tracing::info!("Decomposed into {} tasks", tasks.len());

        // 3. Execute tasks through agent pool
        match self.execute_tasks(&tasks).await {
            Ok(result) => {
                tracing::info!("Execution completed: {} nodes executed", result.nodes_executed);
                Ok(result)
            }
            Err(e) => {
                tracing::error!("Execution failed: {}", e);
                // 4. Handle failure with diagnostics
                self.handle_execution_failure(e, &tasks).await
            }
        }
    }

    /// Parse natural language intent into structured spec
    async fn parse_intent(&self, intent: UserIntent) -> Result<Specification, COAError> {
        // In a real implementation, this would:
        // 1. Use LLM to extract structured information
        // 2. Parse response into Specification
        // 3. Validate the specification

        let desc = intent.description.to_lowercase();

        let logging_like = desc.contains("logging") || desc.contains("log ");
        let mentions_handlers = desc.contains("handler") || desc.contains("endpoint");
        let mentions_all = desc.contains("all ");

        // For now, create a simple specification based on keywords
        let goal = if logging_like && mentions_handlers && mentions_all {
            Goal::ModifyExisting
        } else if desc.contains("create")
            || desc.contains("new")
            || desc.contains("add")
        {
            Goal::CreateNew
        } else if desc.contains("modify")
            || desc.contains("update")
            || desc.contains("change")
        {
            Goal::ModifyExisting
        } else if desc.contains("refactor") {
            Goal::Refactor
        } else if desc.contains("analyze") {
            Goal::Analyze
        } else if desc.contains("optimize") {
            Goal::Optimize
        } else {
            Goal::CreateNew // Default
        };

        let artifact_type = if intent.description.contains("function")
            || intent.description.contains("struct")
            || intent.description.contains("class")
        {
            "code"
        } else if intent.description.contains("config") || intent.description.contains("setting")
        {
            "config"
        } else if intent.description.contains("spec") || intent.description.contains("document")
        {
            "spec"
        } else {
            "code" // Default
        };

        let target_path = intent
            .context
            .as_ref()
            .and_then(|c| c.targets.first())
            .map(|t| coa_artifact::SymbolPath::from_str(t).unwrap_or_default())
            .unwrap_or_default();

        let spec = Specification::new(goal, artifact_type, target_path)
            .with_criteria(vec![intent.description.clone()]);

        Ok(spec)
    }

    /// Decompose specification into tasks
    async fn decompose(&self, spec: Specification) -> Result<Vec<Task>, COAError> {
        self.decomposer
            .decompose(spec, &self.symbol_index)
            .await
            .map_err(COAError::from)
    }

    /// Execute tasks through agent pool
    async fn execute_tasks(&self, tasks: &[Task]) -> Result<ExecutionResult, COAError> {
        let mut completed = Vec::new();
        let mut artifacts = Vec::new();
        let start_time = std::time::Instant::now();

        for task in tasks {
            // Spawn agent for task
            let agent = self.spawn_agent(task).await?;

            // Execute task
            match self.execute_task(&agent, task).await {
                Ok(artifact) => {
                    completed.push(task.id);
                    artifacts.push(ArtifactSummary {
                        artifact_type: task
                            .expected_output
                            .as_ref()
                            .map(|o| format!("{:?}", o))
                            .unwrap_or_else(|| "unknown".to_string()),
                        path: task.target_artifact.to_string(),
                        hash: artifact.hash().to_string(),
                    });
                }
                Err(e) => {
                    return Err(COAError::AgentFailed(format!(
                        "Task {} failed: {}",
                        task.id, e
                    )));
                }
            }

            // Release agent back to pool
            self.agent_pool.release(agent).await;
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            nodes_executed: tasks.len(),
            execution_time_ms,
            artifacts_produced: artifacts,
            tasks_completed: completed,
        })
    }

    /// Spawn an agent for a task
    async fn spawn_agent(&self, task: &Task) -> Result<AgentHandle, COAError> {
        let agent_spec = AgentSpec::from_task(task);
        self.agent_pool
            .acquire(agent_spec)
            .await
            .map_err(COAError::from)
    }

    /// Execute single task through agent
    async fn execute_task(
        &self,
        _agent: &AgentHandle,
        task: &Task,
    ) -> Result<Artifact<CodeArtifact>, COAError> {
        // In a real implementation, this would:
        // 1. Send task to agent via channel
        // 2. Wait for response
        // 3. Collect delta from agent
        // 4. Apply delta through constitutional layer

        // For now, return placeholder error
        Err(COAError::AgentFailed(format!(
            "Task execution not fully implemented: {}",
            task.id
        )))
    }

    /// Handle execution failure with diagnostics
    async fn handle_execution_failure(
        &self,
        error: COAError,
        tasks: &[Task],
    ) -> Result<ExecutionResult, COAError> {
        tracing::warn!("Handling execution failure: {}", error);

        // Generate diagnostic
        let diagnostic = self.generate_diagnostic(&error, tasks).await;

        // Generate suggested fixes
        let fixes = self.suggest_fixes(&error, tasks).await;

        // Check if we can auto-apply
        if self.config.auto_apply_fixes && fixes.iter().any(|f| f.auto_applicable) {
            tracing::info!("Auto-applying fix");
            // In a real implementation, apply the fix and retry
            // For now, just return the error
        }

        // Escalate to human
        Err(COAError::RequiresHumanIntervention {
            error: Box::new(error),
            diagnostic,
            suggested_fixes: fixes,
        })
    }

    /// Generate diagnostic for failure
    async fn generate_diagnostic(&self, error: &COAError, tasks: &[Task]) -> Diagnostic {
        let (error_type, location) = match error {
            COAError::DecompositionFailed(_) => (
                ErrorType::Decomposition,
                Location::Task(tasks.first().map(|t| t.id.to_string()).unwrap_or_default()),
            ),
            COAError::ConstructionFailed(_) => (ErrorType::Construction, Location::GraphConstruction),
            COAError::AgentFailed(_) => (
                ErrorType::Agent,
                Location::Agent("unknown".to_string()),
            ),
            _ => (ErrorType::Unknown, Location::Unknown),
        };

        Diagnostic::new(error_type, location)
            .with_context(crate::error::Context::empty().add("task_count", tasks.len().to_string()))
    }

    /// Suggest fixes for failure
    async fn suggest_fixes(&self, error: &COAError, _tasks: &[Task]) -> Vec<SuggestedFix> {
        let mut fixes = Vec::new();

        match error {
            COAError::PoolError(crate::error::PoolError::PoolExhausted(max)) => {
                fixes.push(
                    SuggestedFix::new(
                        format!("Increase max concurrent agents from {}", max),
                        0.9,
                    )
                    .auto_applicable(),
                );
            }
            COAError::DecompositionFailed(DecompositionError::RecursionDepthExceeded) => {
                fixes.push(SuggestedFix::new(
                    "Increase max decomposition depth",
                    0.8,
                ));
            }
            _ => {
                fixes.push(SuggestedFix::new(
                    "Retry with modified parameters",
                    0.5,
                ));
            }
        }

        fixes
    }

    /// Get configuration
    #[inline]
    #[must_use]
    pub fn config(&self) -> &COAConfig {
        &self.config
    }

    /// Get symbol index
    #[inline]
    #[must_use]
    pub fn symbol_index(&self) -> &SymbolRefIndex {
        &self.symbol_index
    }

    /// Get agent pool stats
    pub async fn pool_stats(&self) -> crate::agent_pool::PoolStats {
        self.agent_pool.stats().await
    }
}

impl Default for CreatorOrchestratorAgent {
    fn default() -> Self {
        Self::new(COAConfig::default())
    }
}

/// Apply composition strategy to collected deltas
#[allow(dead_code)]
fn compose_deltas<T: ArtifactType, S: CompositionStrategy>(
    base: &Artifact<T>,
    deltas: Vec<StructuralDelta<T>>,
    strategy: &S,
    index: &SymbolRefIndex,
) -> Result<Artifact<T>, COAError> {
    // Validate composition
    strategy
        .validate(&deltas, index)
        .map_err(|e| COAError::CompositionFailed(e))?;

    // Compose
    strategy
        .compose(base, &deltas)
        .map_err(|e| COAError::CompositionFailed(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn coa_creation() {
        let config = COAConfig::new();
        let coa = CreatorOrchestratorAgent::new(config);

        assert_eq!(coa.config().max_concurrent_agents, 10);
    }

    #[tokio::test]
    async fn coa_parse_intent_create() {
        let coa = CreatorOrchestratorAgent::default();

        let intent = UserIntent::new("Create a new authentication function");
        let spec = coa.parse_intent(intent).await.unwrap();

        assert!(matches!(spec.goal, Goal::CreateNew));
        assert_eq!(spec.artifact_type, "code");
    }

    #[tokio::test]
    async fn coa_parse_intent_modify() {
        let coa = CreatorOrchestratorAgent::default();

        let intent = UserIntent::new("Update the config settings");
        let spec = coa.parse_intent(intent).await.unwrap();

        assert!(matches!(spec.goal, Goal::ModifyExisting));
    }

    #[tokio::test]
    async fn coa_parse_intent_refactor() {
        let coa = CreatorOrchestratorAgent::default();

        let intent = UserIntent::new("Refactor the utils module");
        let spec = coa.parse_intent(intent).await.unwrap();

        assert!(matches!(spec.goal, Goal::Refactor));
    }

    #[tokio::test]
    async fn coa_default_config() {
        let coa = CreatorOrchestratorAgent::default();
        assert_eq!(coa.config().max_concurrent_agents, 10);
    }
}
