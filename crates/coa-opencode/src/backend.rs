use crate::{
    AgentInfo, AgentRunInput, AgentRunOutput, AgentService, ModelInfo, SkillInfo,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use reqwest::Client;

#[derive(Debug, Clone)]
pub enum OpencodeBackendMode {
    Cli {
        opencode_bin: String,
        working_dir: Option<PathBuf>,
    },
    Daemon {
        client: Client,
        base_url: String,
    },
}

#[derive(Debug, Clone)]
pub struct OpencodeBackend {
    mode: OpencodeBackendMode,
    agent_cache: Option<HashMap<String, AgentInfo>>,
    skill_cache: Option<Vec<SkillInfo>>,
}

impl Default for OpencodeBackend {
    fn default() -> Self {
        let mode_str = env::var("OPENCODE_BACKEND_MODE")
            .unwrap_or_else(|_| "daemon".to_string())
            .to_lowercase();
        match mode_str.as_str() {
            "cli" => {
                let bin = env::var("OPENCODE_BIN").unwrap_or_else(|_| "opencode".to_string());
                let work_dir = env::var("OPENCODE_WORKING_DIR")
                    .ok()
                    .map(PathBuf::from);
                Self {
                    mode: OpencodeBackendMode::Cli {
                        opencode_bin: bin,
                        working_dir: work_dir,
                    },
                    agent_cache: None,
                    skill_cache: None,
                }
            }
            _ => {
                let addr = env::var("OPENCODE_SERVER_ADDR")
                    .unwrap_or_else(|_| "http://localhost:8080".to_string());
                Self {
                    mode: OpencodeBackendMode::Daemon {
                        client: Client::new(),
                        base_url: addr.trim_end_matches('/').to_string(),
                    },
                    agent_cache: None,
                    skill_cache: None,
                }
            }
        }
    }
}

impl OpencodeBackend {
    pub fn new(mode: OpencodeBackendMode) -> Self {
        Self {
            mode,
            agent_cache: None,
            skill_cache: None,
        }
    }

    pub fn from_env() -> Self {
        Self::default()
    }

    fn load_agent_definitions(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let OpencodeBackendMode::Cli { ref working_dir, .. } = self.mode {
            let base = working_dir.as_ref().map(|p| p.as_path()).unwrap_or_else(|| Path::new(".opencode"));
            let agent_dir = base.join("agent");
            let mut agents = HashMap::new();
            if agent_dir.is_dir() {
                for entry in fs::read_dir(agent_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                        agents.insert(
                            id.clone(),
                            AgentInfo {
                                id: id.clone(),
                                name: id.replace('_', " ").title_case(),
                                description: Some(format!("Agent loaded from {}", path.display())),
                                model: "openai:gpt-4o".into(),
                                provider: "openai".into(),
                                temperature: Some(0.2),
                                top_p: None,
                                max_tokens: Some(4096),
                                system_prompt: None,
                                permission_ruleset: None,
                                skill_ids: vec![],
                                options: HashMap::new(),
                            },
                        );
                    }
                }
            }
            self.agent_cache = Some(agents);
        }
        Ok(())
    }

    fn load_skill_definitions(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let OpencodeBackendMode::Cli { ref working_dir, .. } = self.mode {
            let base = working_dir.as_ref().map(|p| p.as_path()).unwrap_or_else(|| Path::new(".opencode"));
            let skill_dir = base.join("skill");
            let mut skills = Vec::new();
            if skill_dir.is_dir() {
                for entry in fs::read_dir(skill_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("ts")
                        || path.extension().and_then(|s| s.to_str()) == Some("js")
                    {
                        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                        skills.push(SkillInfo {
                            id: id.clone(),
                            name: id.replace('_', " ").title_case(),
                            description: Some(format!("Skill loaded from {}", path.display())),
                            entrypoint: None,
                            options: HashMap::new(),
                        });
                    }
                }
            }
            self.skill_cache = Some(skills);
        }
        Ok(())
    }
}

#[async_trait]
impl AgentService for OpencodeBackend {
    async fn list_agents(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { .. } => {
                let cache = self.agent_cache.as_ref().ok_or("Agent cache not loaded (call reload first)")?;
                Ok(cache.keys().cloned().collect())
            }
            OpencodeBackendMode::Daemon { client, base_url } => {
                let resp = client.get(&format!("{}/agents", base_url)).send().await?;
                if !resp.status().is_success() {
                    return Err(format!("Daemon list_agents failed: {}", resp.status()).into());
                }
                let ids: Vec<String> = resp.json().await?;
                Ok(ids)
            }
        }
    }

    async fn get_agent_info(&self, agent_id: &str) -> Result<AgentInfo, Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { .. } => {
                let cache = self.agent_cache.as_ref().ok_or("Agent cache not loaded (call reload first)")?;
                cache.get(agent_id)
                    .cloned()
                    .ok_or_else(|| format!("Unknown agent: {}", agent_id).into())
            }
            OpencodeBackendMode::Daemon { client, base_url } => {
                let resp = client
                    .get(&format!("{}/agents/{}", base_url, agent_id))
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Err(format!("Daemon get_agent_info failed: {}", resp.status()).into());
                }
                let info: AgentInfo = resp.json().await?;
                Ok(info)
            }
        }
    }

    async fn update_agent_config(&self, agent_id: &str, patch: AgentInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { .. } => {
                Err("runtime agent config update not implemented in CLI mode".into())
            }
            OpencodeBackendMode::Daemon { client, base_url } => {
                let resp = client
                    .put(&format!("{}/agents/{}", base_url, agent_id))
                    .json(&patch)
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Err(format!("Daemon update_agent_config failed: {}", resp.status()).into());
                }
                Ok(())
            }
        }
    }

    async fn list_skills(&self) -> Result<Vec<SkillInfo>, Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { .. } => {
                let cache = self.skill_cache.as_ref().ok_or("Skill cache not loaded (call reload first)")?;
                Ok(cache.clone())
            }
            OpencodeBackendMode::Daemon { client, base_url } => {
                let resp = client.get(&format!("{}/skills", base_url)).send().await?;
                if !resp.status().is_success() {
                    return Err(format!("Daemon list_skills failed: {}", resp.status()).into());
                }
                let skills: Vec<SkillInfo> = resp.json().await?;
                Ok(skills)
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { .. } => {
                Ok(vec![
                    ModelInfo {
                        provider: "openai".into(),
                        model: "gpt-4o".into(),
                        display_name: Some("GPT-4o".into()),
                    },
                    ModelInfo {
                        provider: "anthropic".into(),
                        model: "claude-3-5-sonnet-20240620".into(),
                        display_name: Some("Claude 3.5 Sonnet".into()),
                    },
                    ModelInfo {
                        provider: "ollama".into(),
                        model: "llama3:8b".into(),
                        display_name: Some("Llama 3 8B".into()),
                    },
                ])
            }
            OpencodeBackendMode::Daemon { client, base_url } => {
                let resp = client.get(&format!("{}/models", base_url)).send().await?;
                if !resp.status().is_success() {
                    return Err(format!("Daemon list_models failed: {}", resp.status()).into());
                }
                let models: Vec<ModelInfo> = resp.json().await?;
                Ok(models)
            }
        }
    }

    async fn run_agent(&self, agent_id: &str, input: AgentRunInput) -> Result<AgentRunOutput, Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { ref opencode_bin, ref working_dir } => {
                let request = serde_json::json!({
                    "agentId": agent_id,
                    "intent": input.intent,
                    "context": input.context,
                    "sessionId": input.session_id,
                    "overrides": input.overrides,
                });

                let req_file = tempfile::NamedTempFile::new()?;
                std::fs::write(req_file.path(), serde_json::to_string(&request)?)?;

                let mut cmd = Command::new(opencode_bin);
                cmd.arg("agent")
                    .arg("run")
                    .arg(agent_id)
                    .arg("--input")
                    .arg(req_file.path())
                    .arg("--output")
                    .arg("-");
                if let Some(ref dir) = working_dir {
                    cmd.current_dir(dir);
                }
                let output = cmd
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await?;

                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("opencode agent failed: {}", err).into());
                }

                let result: AgentRunOutput = serde_json::from_slice(&output.stdout)?;
                Ok(result)
            }
            OpencodeBackendMode::Daemon { client, base_url } => {
                let resp = client
                    .post(&format!("{}/agents/{}/run", base_url, agent_id))
                    .json(&input)
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Err(format!("Daemon run_agent failed: {}", resp.status()).into());
                }
                let output: AgentRunOutput = resp.json().await?;
                Ok(output)
            }
        }
    }

    async fn execute_skill(&self, skill_id: &str, input: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { .. } => {
                Err("direct skill execution not implemented in CLI mode".into())
            }
            OpencodeBackendMode::Daemon { client, base_url } => {
                let resp = client
                    .post(&format!("{}/skills/{}/execute", base_url, skill_id))
                    .json(&input)
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Err(format!("Daemon execute_skill failed: {}", resp.status()).into());
                }
                let result: serde_json::Value = resp.json().await?;
                Ok(result)
            }
        }
    }

    async fn reload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut mutable = self.clone();
        if let OpencodeBackendMode::Cli { .. } = mutable.mode {
            mutable.load_agent_definitions()?;
            mutable.load_skill_definitions()?;
        }
        Ok(())
    }
}

trait TitleCase {
    fn title_case(self) -> String;
}

impl TitleCase for String {
    fn title_case(self) -> String {
        self.split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    }
}
