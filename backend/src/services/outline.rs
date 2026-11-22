use crate::config::Config;
use crate::models::document::{Document, OutlineResponse};
use reqwest::Client;
use std::sync::Arc;

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

    pub async fn get_document(&self, id: &str) -> Result<Document, reqwest::Error> {
        let url = format!("{}/documents.info", self.config.outline_api_url);
        let response: OutlineResponse<Document> = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.outline_api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "id": id }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.data)
    }
}
