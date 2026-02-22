//! Resource Management (v2.0)
//!
//! This module handles resource management with the v2.0 architecture:
//! - **Construction time**: Prove resource bounds are satisfiable
//! - **Runtime**: Enforce pre-declared limits (NOT validation)

use crate::error::ValidationError;
use crate::types::v2::{NodeSpecV2, SystemLimits};
use crate::types::ResourceCaps;

/// Resource proof - evidence that bounds are satisfiable
///
/// This is computed at construction time.
#[derive(Debug, Clone)]
pub struct ResourceProof {
    pub total_cpu_ms: u64,
    pub total_memory_bytes: u64,
    pub total_tokens: u64,
    pub total_iterations: u64,
    pub within_system_limits: bool,
}

/// Prove that resource bounds are satisfiable for a set of nodes
///
/// Called at construction time by `ConstructionValidator`.
/// Returns `ResourceProof` if bounds are provably satisfiable.
pub fn prove_resource_bounds(
    nodes: &[NodeSpecV2],
    system_limits: &SystemLimits,
) -> Result<ResourceProof, ValidationError> {
    let mut total_cpu = 0u64;
    let mut total_memory = 0u64;
    let mut total_tokens = 0u64;
    let mut total_iterations = 0u64;
    
    for node in nodes {
        let bounds = &node.resource_bounds;
        
        // Check for overflow
        total_cpu = total_cpu.checked_add(bounds.cpu_time_ms)
            .ok_or(ValidationError::ResourceBoundsNotProvable)?;
        total_memory = total_memory.checked_add(bounds.memory_bytes)
            .ok_or(ValidationError::ResourceBoundsNotProvable)?;
        total_tokens = total_tokens.checked_add(bounds.token_limit)
            .ok_or(ValidationError::ResourceBoundsNotProvable)?;
        total_iterations = total_iterations.checked_add(bounds.iteration_cap)
            .ok_or(ValidationError::ResourceBoundsNotProvable)?;
    }
    
    // Check against system limits
    let within_limits = total_cpu <= system_limits.max_resources.cpu_time_ms
        && total_memory <= system_limits.max_resources.memory_bytes
        && total_tokens <= system_limits.max_resources.token_limit
        && total_iterations <= system_limits.max_resources.iteration_cap;
    
    if !within_limits {
        return Err(ValidationError::ResourceBoundsNotProvable);
    }
    
    Ok(ResourceProof {
        total_cpu_ms: total_cpu,
        total_memory_bytes: total_memory,
        total_tokens,
        total_iterations,
        within_system_limits: within_limits,
    })
}

/// Validate resource caps against a limit
///
/// This is used for basic validation during construction.
pub fn validate_caps(caps: &ResourceCaps, limits: &ResourceCaps) -> Result<(), crate::error::ResourceError> {
    if caps.cpu_time_ms > limits.cpu_time_ms
        || caps.memory_bytes > limits.memory_bytes
        || caps.token_limit > limits.token_limit
        || caps.iteration_cap > limits.iteration_cap
    {
        Err(crate::error::ResourceError::CapExceeded)
    } else {
        Ok(())
    }
}

/// Container for runtime resource enforcement
///
/// Enforces pre-declared resource limits. This is NOT validation -
/// all validation happened at construction time.
#[derive(Debug, Clone)]
pub struct ResourceContainer {
    cpu_limit_ms: u64,
    memory_limit_bytes: u64,
    token_limit: u64,
    iteration_limit: u64,
    cpu_used_ms: u64,
    memory_used_bytes: u64,
    tokens_used: u64,
    iterations_used: u64,
}

impl ResourceContainer {
    /// Create a new resource container with the given limits
    pub fn new(limits: ResourceCaps) -> Self {
        Self {
            cpu_limit_ms: limits.cpu_time_ms,
            memory_limit_bytes: limits.memory_bytes,
            token_limit: limits.token_limit,
            iteration_limit: limits.iteration_cap,
            cpu_used_ms: 0,
            memory_used_bytes: 0,
            tokens_used: 0,
            iterations_used: 0,
        }
    }
    
    /// Track CPU time usage
    pub fn track_cpu(&mut self, ms: u64) -> Result<(), crate::error::ExecutionError> {
        self.cpu_used_ms = self.cpu_used_ms.saturating_add(ms);
        if self.cpu_used_ms > self.cpu_limit_ms {
            Err(crate::error::ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
    
    /// Track memory usage
    pub fn track_memory(&mut self, bytes: u64) -> Result<(), crate::error::ExecutionError> {
        self.memory_used_bytes = self.memory_used_bytes.saturating_add(bytes);
        if self.memory_used_bytes > self.memory_limit_bytes {
            Err(crate::error::ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
    
    /// Track token usage
    pub fn track_tokens(&mut self, count: u64) -> Result<(), crate::error::ExecutionError> {
        self.tokens_used = self.tokens_used.saturating_add(count);
        if self.tokens_used > self.token_limit {
            Err(crate::error::ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
    
    /// Track iteration count
    pub fn track_iterations(&mut self, count: u64) -> Result<(), crate::error::ExecutionError> {
        self.iterations_used = self.iterations_used.saturating_add(count);
        if self.iterations_used > self.iteration_limit {
            Err(crate::error::ExecutionError::ResourceEnforcementTriggered)
        } else {
            Ok(())
        }
    }
    
    /// Get current usage
    pub fn current_usage(&self) -> ResourceCaps {
        ResourceCaps {
            cpu_time_ms: self.cpu_used_ms,
            memory_bytes: self.memory_used_bytes,
            token_limit: self.tokens_used,
            iteration_cap: self.iterations_used,
        }
    }
    
    /// Get remaining capacity
    pub fn remaining(&self) -> ResourceCaps {
        ResourceCaps {
            cpu_time_ms: self.cpu_limit_ms.saturating_sub(self.cpu_used_ms),
            memory_bytes: self.memory_limit_bytes.saturating_sub(self.memory_used_bytes),
            token_limit: self.token_limit.saturating_sub(self.tokens_used),
            iteration_cap: self.iteration_limit.saturating_sub(self.iterations_used),
        }
    }
    
    /// Check if resource is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.cpu_used_ms >= self.cpu_limit_ms
            || self.memory_used_bytes >= self.memory_limit_bytes
            || self.tokens_used >= self.token_limit
            || self.iterations_used >= self.iteration_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AutonomyLevel;
    use std::collections::BTreeMap;

    fn create_test_node(cpu_ms: u64, memory: u64) -> NodeSpecV2 {
        NodeSpecV2 {
            directives: crate::types::DirectiveSet {
                directives: BTreeMap::new(),
            },
            autonomy_ceiling: AutonomyLevel::L3,
            resource_bounds: ResourceCaps {
                cpu_time_ms: cpu_ms,
                memory_bytes: memory,
                token_limit: 1000,
                iteration_cap: 100,
            },
            expansion_type: None,
        }
    }

    #[test]
    fn test_prove_bounds_within_limits() {
        let nodes = vec![
            create_test_node(1000, 1024 * 1024),
            create_test_node(2000, 2 * 1024 * 1024),
        ];
        
        let limits = SystemLimits {
            max_autonomy: AutonomyLevel::L5,
            max_resources: ResourceCaps {
                cpu_time_ms: 10000,
                memory_bytes: 100 * 1024 * 1024,
                token_limit: 100000,
                iteration_cap: 10000,
            },
            max_nodes: 1000,
            max_edges: 10000,
        };
        
        let proof = prove_resource_bounds(&nodes, &limits);
        assert!(proof.is_ok());
        
        let proof = proof.unwrap();
        assert_eq!(proof.total_cpu_ms, 3000);
        assert_eq!(proof.total_memory_bytes, 3 * 1024 * 1024);
        assert!(proof.within_system_limits);
    }

    #[test]
    fn test_prove_bounds_exceeds_limits() {
        let nodes = vec![
            create_test_node(10000, 1024 * 1024),
            create_test_node(20000, 2 * 1024 * 1024),
        ];
        
        let limits = SystemLimits {
            max_autonomy: AutonomyLevel::L5,
            max_resources: ResourceCaps {
                cpu_time_ms: 5000, // Too low
                memory_bytes: 100 * 1024 * 1024,
                token_limit: 100000,
                iteration_cap: 10000,
            },
            max_nodes: 1000,
            max_edges: 10000,
        };
        
        let proof = prove_resource_bounds(&nodes, &limits);
        assert!(proof.is_err());
    }

    #[test]
    fn test_resource_container_tracks_usage() {
        let limits = ResourceCaps {
            cpu_time_ms: 100,
            memory_bytes: 1024,
            token_limit: 50,
            iteration_cap: 10,
        };
        
        let mut container = ResourceContainer::new(limits);
        
        // Track usage within limits
        assert!(container.track_cpu(50).is_ok());
        assert!(container.track_memory(512).is_ok());
        
        // Check remaining
        let remaining = container.remaining();
        assert_eq!(remaining.cpu_time_ms, 50);
        assert_eq!(remaining.memory_bytes, 512);
        
        // Exceed limit
        assert!(container.track_cpu(100).is_err());
    }

    #[test]
    fn test_resource_container_is_exhausted() {
        let limits = ResourceCaps {
            cpu_time_ms: 100,
            memory_bytes: 1024,
            token_limit: 50,
            iteration_cap: 10,
        };
        
        let mut container = ResourceContainer::new(limits);
        
        assert!(!container.is_exhausted());
        
        container.track_cpu(100).ok();
        assert!(container.is_exhausted());
    }
}
