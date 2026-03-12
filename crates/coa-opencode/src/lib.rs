use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod backend;
pub mod client;
pub mod config;

pub use config::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub model: String,
    pub provider: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub permission_ruleset: Option<String>,
    pub skill_ids: Vec<String>,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunInput {
    pub intent: String,
    pub context: serde_json::Value,
    pub session_id: Option<String>,
    pub overrides: Option<AgentRunOverrides>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunOverrides {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub permission_ruleset: Option<String>,
    pub skill_ids: Option<Vec<String>>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunOutput {
    pub content: String,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub tool_calls: Vec<ToolCall>,
    pub messages: Vec<Message>,
    pub usage: Option<TokenUsage>,
    pub session_id: Option<String>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content_delta: Option<String>,
    pub tool_call_delta: Option<ToolCallDelta>,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    pub id: String,
    pub tool_name: String,
    pub argument_delta: String,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub entrypoint: Option<String>,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
    pub display_name: Option<String>,
}

#[async_trait]
pub trait AgentService: Send + Sync {
    async fn list_agents(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_agent_info(&self, agent_id: &str) -> Result<AgentInfo, Box<dyn std::error::Error + Send + Sync>>;
    async fn update_agent_config(&self, agent_id: &str, patch: AgentInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn list_skills(&self) -> Result<Vec<SkillInfo>, Box<dyn std::error::Error + Send + Sync>>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error + Send + Sync>>;
    async fn run_agent(&self, agent_id: &str, input: AgentRunInput) -> Result<AgentRunOutput, Box<dyn std::error::Error + Send + Sync>>;
    async fn execute_skill(&self, skill_id: &str, input: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
    async fn reload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
