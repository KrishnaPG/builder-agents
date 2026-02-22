//! Construction Phase (v2.0)
//!
//! This module contains all types and functions for the construction phase.
//! All policy validation happens here, producing a `ValidatedGraph` that
//! carries a cryptographic proof of validation.
//!
//! # Two-Phase Architecture
//!
//! 1. **Construction Phase** (this module):
//!    - Build DAG structure
//!    - Validate all policy constraints
//!    - Prove resource bounds
//!    - Issue capability tokens
//!    - Produce `ValidatedGraph`
//!
//! 2. **Execution Phase** (executor module):
//!    - Verify token integrity (cryptographic only)
//!    - Enforce pre-declared resource limits
//!    - Execute nodes
//!    - Zero policy validation

pub mod builder;
pub mod issuer;
pub mod validator;

pub use builder::{GraphBuilder, GraphBuilderError};
pub use issuer::{IssuedTokens, TokenIssuer};
pub use validator::{ConstructionValidator, ValidationContext};
