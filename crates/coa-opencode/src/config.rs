use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provider-agnostic agent configuration.
/// Users work with this, not Opencode internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub model: ModelConfig,
    pub behavior: BehaviorConfig,
    pub skills: Vec<SkillAssignment>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub system_prompt: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub timeout_seconds: Option<u64>,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            temperature: Some(0.7),
            top_p: None,
            max_tokens: Some(4096),
            presence_penalty: None,
            frequency_penalty: None,
            stop_sequences: vec![],
            timeout_seconds: Some(300),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAssignment {
    pub skill_id: String,
    pub enabled: bool,
    pub config: Option<serde_json::Value>,
}

/// Backend configuration that's provider-agnostic.
/// Maps to Opencode, OpenAI, or other backends internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub default_agent: Option<String>,
    pub max_concurrent_runs: usize,
    pub request_timeout_ms: u64,
    pub retry_policy: RetryPolicy,
    pub provider_overrides: HashMap<String, ProviderConfig>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            default_agent: None,
            max_concurrent_runs: 10,
            request_timeout_ms: 300_000,
            retry_policy: RetryPolicy::default(),
            provider_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub organization: Option<String>,
    pub extra_headers: HashMap<String, String>,
}
