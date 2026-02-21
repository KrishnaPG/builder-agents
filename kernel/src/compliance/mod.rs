use crate::api::{ComplianceError, ComplianceInterface, ComplianceReport, PolicyScope, PolicySnapshot, ProposedAction, ResourceAvailability};
use crate::error::{ComplianceViolation, KernelError};
use crate::types::{now_timestamp, ResourceCaps};

pub struct Compliance {
    default_caps: ResourceCaps,
}

impl Compliance {
    pub fn new() -> Self {
        Self {
            default_caps: ResourceCaps {
                cpu_time_ms: 10000,
                memory_bytes: 1024 * 1024 * 1024, // 1GB
                token_limit: 100000,
                iteration_cap: 1000,
            },
        }
    }

    pub fn with_caps(default_caps: ResourceCaps) -> Self {
        Self { default_caps }
    }
}

impl ComplianceInterface for Compliance {
    fn validate_action(&self, action: ProposedAction) -> Result<ComplianceReport, ComplianceError> {
        let mut violations = Vec::new();
        let mut resource_check_passed = true;
        let policy_check_passed = true;

        // Check resource caps if specified
        if let Some(caps) = action.requested_caps {
            // Check for obviously excessive values (near u64::MAX)
            const REASONABLE_MAX_CPU: u64 = 24 * 60 * 60 * 1000; // 24 hours in ms
            const REASONABLE_MAX_MEMORY: u64 = 1024 * 1024 * 1024 * 100; // 100 GB
            const REASONABLE_MAX_TOKENS: u64 = 1_000_000_000;
            const REASONABLE_MAX_ITERATIONS: u64 = 1_000_000;
            
            if caps.cpu_time_ms > self.default_caps.cpu_time_ms
                || caps.memory_bytes > self.default_caps.memory_bytes
                || caps.token_limit > self.default_caps.token_limit
                || caps.iteration_cap > self.default_caps.iteration_cap
                || caps.cpu_time_ms > REASONABLE_MAX_CPU
                || caps.memory_bytes > REASONABLE_MAX_MEMORY
                || caps.token_limit > REASONABLE_MAX_TOKENS
                || caps.iteration_cap > REASONABLE_MAX_ITERATIONS
            {
                resource_check_passed = false;
                violations.push(ComplianceViolation::PolicyViolation);
            }
        }

        let approved = violations.is_empty() && resource_check_passed && policy_check_passed;

        Ok(ComplianceReport {
            approved,
            violations: violations.clone(),
            resource_check_passed,
            policy_check_passed,
            timestamp: now_timestamp(),
        })
    }

    fn query_policy(&self, _scope: PolicyScope) -> Result<PolicySnapshot, KernelError> {
        Ok(PolicySnapshot {
            max_autonomy_level: crate::types::AutonomyLevel::L5,
            default_caps: self.default_caps,
            require_token_for_all_actions: true,
            timestamp: now_timestamp(),
        })
    }

    fn check_resources(&self, caps: ResourceCaps) -> Result<ResourceAvailability, KernelError> {
        crate::resource::validate_caps(&caps, &self.default_caps).map_err(KernelError::Resource)?;
        
        Ok(ResourceAvailability {
            available: true,
            cpu_time_remaining_ms: self.default_caps.cpu_time_ms.saturating_sub(caps.cpu_time_ms),
            memory_remaining_bytes: self.default_caps.memory_bytes.saturating_sub(caps.memory_bytes),
            tokens_remaining: self.default_caps.token_limit.saturating_sub(caps.token_limit),
            iterations_remaining: self.default_caps.iteration_cap.saturating_sub(caps.iteration_cap),
        })
    }
}
