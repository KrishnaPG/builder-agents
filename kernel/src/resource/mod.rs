use crate::error::ResourceError;
use crate::types::ResourceCaps;

pub fn validate_caps(caps: &ResourceCaps, limits: &ResourceCaps) -> Result<(), ResourceError> {
    if caps.cpu_time_ms > limits.cpu_time_ms
        || caps.memory_bytes > limits.memory_bytes
        || caps.token_limit > limits.token_limit
        || caps.iteration_cap > limits.iteration_cap
    {
        return Err(ResourceError::LimitExceeded);
    }
    Ok(())
}
