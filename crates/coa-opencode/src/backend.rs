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
                }
            }
        }
    }
}

impl OpencodeBackend {
    pub fn new(mode: OpencodeBackendMode) -> Self {
        Self {
            mode,
        }
    }

    pub fn from_env() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AgentService for OpencodeBackend {
    async fn list_agents(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        match &self.mode {
            OpencodeBackendMode::Cli { ref opencode_bin, ref working_dir } => {
                let mut cmd = Command::new(opencode_bin);
                cmd.arg("agent")
                    .arg("list");
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
                    return Err(format!("opencode agent list failed: {}", err).into());
                }

                let ids: Vec<String> = serde_json::from_slice(&output.stdout)?;
                Ok(ids)
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
            OpencodeBackendMode::Cli { ref opencode_bin, ref working_dir } => {
                let mut cmd = Command::new(opencode_bin);
                cmd.arg("agent")
                    .arg("info")
                    .arg(agent_id);
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
                    return Err(format!("opencode agent info failed: {}", err).into());
                }

                let info: AgentInfo = serde_json::from_slice(&output.stdout)?;
                Ok(info)
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
            OpencodeBackendMode::Cli { ref opencode_bin, ref working_dir } => {
                let mut cmd = Command::new(opencode_bin);
                cmd.arg("agent")
                    .arg("update")
                    .arg(agent_id);
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
                    return Err(format!("opencode agent update failed: {}", err).into());
                }
                Ok(())
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
            OpencodeBackendMode::Cli { ref opencode_bin, ref working_dir } => {
                let mut cmd = Command::new(opencode_bin);
                cmd.arg("skill")
                    .arg("list");
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
                    return Err(format!("opencode skill list failed: {}", err).into());
                }

                let skills: Vec<SkillInfo> = serde_json::from_slice(&output.stdout)?;
                Ok(skills)
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
            OpencodeBackendMode::Cli { ref opencode_bin, ref working_dir } => {
                let mut cmd = Command::new(opencode_bin);
                cmd.arg("model")
                    .arg("list");
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
                    return Err(format!("opencode model list failed: {}", err).into());
                }

                let models: Vec<ModelInfo> = serde_json::from_slice(&output.stdout)?;
                Ok(models)
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
                    return Err(format!("opencode agent run failed: {}", err).into());
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
            OpencodeBackendMode::Cli { ref opencode_bin, ref working_dir } => {
                let mut cmd = Command::new(opencode_bin);
                cmd.arg("skill")
                    .arg("execute")
                    .arg(skill_id);
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
                    return Err(format!("opencode skill execute failed: {}", err).into());
                }

                let result: serde_json::Value = serde_json::from_slice(&output.stdout)?;
                Ok(result)
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
        // Reload is handled by the opencode CLI itself when needed
        // For daemon mode, the server handles its own reload
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
