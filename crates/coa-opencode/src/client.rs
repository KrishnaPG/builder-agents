use crate::{
    AgentInfo, AgentRunInput, AgentRunOutput, AgentService, ModelInfo, SkillInfo,
};
use async_trait::async_trait;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct HttpAgentClient {
    base_url: String,
    client: Client,
}

impl HttpAgentClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl AgentService for HttpAgentClient {
    async fn list_agents(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client.get(&self.url("agents")).send().await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to list agents: {}", resp.status()).into());
        }
        let ids: Vec<String> = resp.json().await?;
        Ok(ids)
    }

    async fn get_agent_info(&self, agent_id: &str) -> Result<AgentInfo, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client
            .get(&self.url(&format!("agents/{}", agent_id)))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to get agent info for {}: {}", agent_id, resp.status()).into());
        }
        let info: AgentInfo = resp.json().await?;
        Ok(info)
    }

    async fn update_agent_config(&self, agent_id: &str, patch: AgentInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client
            .put(&self.url(&format!("agents/{}", agent_id)))
            .json(&patch)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to update agent config for {}: {}", agent_id, resp.status()).into());
        }
        Ok(())
    }

    async fn list_skills(&self) -> Result<Vec<SkillInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client.get(&self.url("skills")).send().await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to list skills: {}", resp.status()).into());
        }
        let skills: Vec<SkillInfo> = resp.json().await?;
        Ok(skills)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client.get(&self.url("models")).send().await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to list models: {}", resp.status()).into());
        }
        let models: Vec<ModelInfo> = resp.json().await?;
        Ok(models)
    }

    async fn run_agent(&self, agent_id: &str, input: AgentRunInput) -> Result<AgentRunOutput, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client
            .post(&self.url(&format!("agents/{}/run", agent_id)))
            .json(&input)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to run agent {}: {}", agent_id, resp.status()).into());
        }
        let output: AgentRunOutput = resp.json().await?;
        Ok(output)
    }

    async fn execute_skill(&self, skill_id: &str, input: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client
            .post(&self.url(&format!("skills/{}/execute", skill_id)))
            .json(&input)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to execute skill {}: {}", skill_id, resp.status()).into());
        }
        let result: serde_json::Value = resp.json().await?;
        Ok(result)
    }

    async fn reload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
