use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sync::EngineClient;
use crate::Result;

#[derive(Debug, Deserialize)]
struct HealthResponse {
    ok: bool,
}

#[derive(Debug, Serialize)]
struct RunTaskBody<'a> {
    session_id: String,
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<&'a str>,
}

pub struct HttpEngineClient {
    base_url: String,
    http: reqwest::Client,
}

impl HttpEngineClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EngineClient for HttpEngineClient {
    async fn health(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let res = self.http.get(url).send().await.map_err(map_reqwest)?;
        if !res.status().is_success() {
            return Ok(false);
        }
        let body: HealthResponse = res.json().await.map_err(map_reqwest)?;
        Ok(body.ok)
    }

    async fn run_task(
        &self,
        session_id: Uuid,
        prompt: &str,
        model_id: Option<&str>,
    ) -> Result<()> {
        let url = format!("{}/v1/tasks/run", self.base_url);
        let body = RunTaskBody {
            session_id: session_id.to_string(),
            prompt,
            model_id,
        };
        let res = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(map_reqwest)?;
        if !res.status().is_success() {
            return Err(crate::NexusError::Engine(format!(
                "run_task failed: {}",
                res.status()
            )));
        }
        Ok(())
    }

    async fn pause_task(&self, session_id: Uuid) -> Result<()> {
        let url = format!("{}/v1/tasks/{}/pause", self.base_url, session_id);
        let res = self.http.post(url).send().await.map_err(map_reqwest)?;
        if !res.status().is_success() {
            return Err(crate::NexusError::Engine(format!(
                "pause failed: {}",
                res.status()
            )));
        }
        Ok(())
    }

    async fn resume_task(&self, session_id: Uuid) -> Result<()> {
        let url = format!("{}/v1/tasks/{}/resume", self.base_url, session_id);
        let res = self.http.post(url).send().await.map_err(map_reqwest)?;
        if !res.status().is_success() {
            return Err(crate::NexusError::Engine(format!(
                "resume failed: {}",
                res.status()
            )));
        }
        Ok(())
    }
}

fn map_reqwest(err: reqwest::Error) -> crate::NexusError {
    crate::NexusError::Engine(err.to_string())
}
