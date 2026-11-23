use crate::config::Config;
use crate::models::document::{Document, OutlineResponse};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

/// Errors from the Outline API, with "the document is gone" separated from
/// transient failures so the poller can clear the board on the former and
/// keep its last snapshot on the latter.
#[derive(Debug)]
pub enum OutlineError {
    Http(reqwest::Error),
    NotFound,
    Api(String),
}

impl std::fmt::Display for OutlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutlineError::Http(e) => write!(f, "{}", e),
            OutlineError::NotFound => write!(f, "document not found"),
            OutlineError::Api(code) => write!(f, "Outline API error: {}", code),
        }
    }
}

/// Lenient envelope for `documents.info`: Outline reports errors in the
/// body (`ok: false`, `error: <code>`, no `data`) instead of HTTP status,
/// so `data` must be optional for those responses to parse at all.
#[derive(Debug, Deserialize)]
pub struct DocumentEnvelope {
    ok: Option<bool>,
    error: Option<String>,
    data: Option<Document>,
}

/// Classify a parsed envelope: `data` wins; a `not_found` error code means
/// the document was deleted; anything else is a transient API error.
pub fn classify_envelope(env: &DocumentEnvelope) -> Result<Document, OutlineError> {
    if let Some(doc) = &env.data {
        return Ok(doc.clone());
    }
    let code = env.error.clone().unwrap_or_default();
    if code.contains("not_found") {
        return Err(OutlineError::NotFound);
    }
    Err(OutlineError::Api(code))
}

#[derive(Clone)]
pub struct OutlineClient {
    client: Client,
    pub config: Arc<Config>,
}

impl OutlineClient {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub async fn list_documents(&self) -> Result<Vec<Document>, reqwest::Error> {
        let url = format!("{}/documents.list", self.config.outline_api_url);
        let response: OutlineResponse<Vec<Document>> = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.outline_api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.data)
    }

    pub async fn get_document(&self, id: &str) -> Result<Document, OutlineError> {
        let url = format!("{}/documents.info", self.config.outline_api_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.outline_api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "id": id }))
            .send()
            .await
            .map_err(OutlineError::Http)?;
        // Some deployments also surface deletion as a real HTTP 404.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(OutlineError::NotFound);
        }
        let env: DocumentEnvelope = response.json().await.map_err(OutlineError::Http)?;
        classify_envelope(&env)
    }
}
