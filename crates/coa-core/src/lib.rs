//! COA Core - Creator Orchestrator Agent
//!
//! The central intelligence that:
//! - Parses user intent into structured specifications
//! - Decomposes work into tasks
//! - Manages the agent lifecycle
//! - Handles construction failures with diagnostics
//! - Coordinates multi-agent composition
//!
//! # Example
//!
//! ```rust,ignore
//! use coa_core::{CreatorOrchestratorAgent, COAConfig, UserIntent};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = COAConfig::new();
//! let coa = CreatorOrchestratorAgent::new(config);
//!
//! let intent = UserIntent::new("Create a hello world function");
//! let result = coa.execute_intent(intent).await?;
//!
//! println!("Executed {} nodes", result.nodes_executed);
//! # Ok(())
//! # }
//! ```

#![warn(unreachable_pub)]
#![allow(missing_docs)]

// Core modules
pub mod agent_pool;
pub mod coa;
pub mod decomposition;
pub mod error;
pub mod types;

// Re-exports for convenience
pub use agent_pool::{AgentHandle, AgentMessage, AgentPool, PoolStats};
pub use coa::CreatorOrchestratorAgent;
pub use decomposition::TaskDecomposer;
pub use error::{
    ConstructionError, COAError, DecompositionError, Diagnostic, ErrorType, Goal, Location,
    PoolError, ResourceAmount, SuggestedFix,
};
pub use types::{
    AgentId, AgentSpec, ArtifactSummary, AutonomyLevel, COAConfig, Constraint, ExecutionResult,
    ExpansionType, IntentContext, OutputSpec, ResourceCaps, SpecFormat, Specification, Task,
    TaskId, UserIntent,
};

/// Prelude module for common imports
pub mod prelude {
    //! Common imports for working with COA Core
    pub use crate::{
        AgentHandle, AgentPool, AgentSpec, AutonomyLevel, COAConfig, CreatorOrchestratorAgent,
        ExecutionResult, Task, TaskDecomposer, TaskId, UserIntent,
    };
}

/// Version of this crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn coa_full_flow() {
        let config = COAConfig::new().with_max_agents(2);
        let coa = CreatorOrchestratorAgent::new(config);

        let intent = UserIntent::new("Create a simple function");

        // Note: This will fail at execution since we don't have real agents
        // But it tests the flow up to that point
        let result = coa.execute_intent(intent).await;
        assert!(result.is_err()); // Expected since agents aren't real
    }

    #[test]
    fn types_integration() {
        let task = Task::new(
            "tester",
            "test task",
            coa_artifact::SymbolPath::from_str("test").unwrap(),
        )
        .with_autonomy(AutonomyLevel::L3);

        assert_eq!(task.role, "tester");
        assert_eq!(task.autonomy, AutonomyLevel::L3);
    }
}
