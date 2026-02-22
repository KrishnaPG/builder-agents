//! COA Kernel (coa-kernel) - v2.0
//!
//! Safe-by-construction architecture with two-phase design:
//! 1. **Construction Phase**: Build and validate graphs
//! 2. **Execution Phase**: Execute pre-validated graphs
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use coa_kernel::prelude::*;
//!
//! // Construction phase
//! let mut builder = GraphBuilder::new(GraphType::ProductionDAG);
//! let n1 = builder.add_node(spec1);
//! let n2 = builder.add_node(spec2);
//! builder.add_edge(n1, n2)?;
//!
//! let validated = builder.validate(&signing_key)?;
//!
//! // Execution phase
//! let executor = Executor::new(verifying_key);
//! let result = executor.run(validated).await?;
//! ```

// Core modules
pub mod api;
pub mod autonomy;
pub mod dag;
pub mod directives;
pub mod error;
pub mod isolation;
pub mod logging;
pub mod resource;
pub mod scheduler;
pub mod state_machine;
pub mod types;

// v2.0 modules
pub mod construction;
pub mod executor;
pub mod expansion;
pub mod token_integrity;
pub mod validated_graph;

// Test harness
pub mod test_harness;

// Re-exports
pub use api::*;
pub use error::*;
pub use types::*;

/// Re-export v2.0 types for convenience
pub mod prelude {
    pub use crate::construction::{GraphBuilder, GraphBuilderError, ConstructionValidator, TokenIssuer, ValidationContext};
    pub use crate::executor::{Executor, NodeExecutor, NodeExecutionResult, ResourceContainer};
    pub use crate::error::{ExecutionError, ValidationError};
    pub use crate::expansion::{ExpansionBuilder, ExpansionPoint, StagedConstruction};
    pub use crate::types::v2::ExpansionSchema;
    pub use crate::token_integrity::TokenIntegrity;
    pub use crate::types::v2::{
        ExecutionSummary, ExpansionType, IntegrityVerification, NodeSpecV2, SubgraphSpec,
        SystemLimits, ValidatedGraph, ValidationToken,
    };
    pub use crate::types::{AutonomyLevel, GraphType, ResourceCaps, NodeId, GraphId};
    pub use crate::validated_graph::{ResourceProof, ValidationReport};
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if running with strict debugging enabled
pub const fn strict_debug() -> bool {
    cfg!(feature = "strict-debug")
}
