use crate::models::response::TasksResponse;
use crate::models::task::DocumentTasks;
use crate::routes::auth::RequireAuth;
use crate::services::auth::AuthService;
use crate::services::outline::OutlineClient;
use crate::services::parser::parse_tasks_from_markdown;
use axum::extract::State;
use axum::Json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub outline_client: OutlineClient,
    pub auth_service: Arc<AuthService>,
    pub csrf_states: Arc<Mutex<HashMap<String, Instant>>>,
    pub sessions: Arc<Mutex<HashMap<String, SessionData>>>,
}

#[derive(Clone)]
pub struct SessionData {
    pub access_token: String,
    pub created_at: Instant,
}

pub async fn get_tasks(
    _auth: RequireAuth,
    State(state): State<AppState>,
) -> Json<TasksResponse> {
    let mut all_tasks = Vec::new();

    match state.outline_client.list_documents().await {
        Ok(documents) => {
            for doc_summary in documents {
                if let Some(task_info) = doc_summary.tasks {
                    if task_info.total > 0 {
                        match state.outline_client.get_document(&doc_summary.id).await {
                            Ok(doc) => {
                                if let Some(text) = doc.text {
                                    let groups = parse_tasks_from_markdown(&text);
                                    if !groups.is_empty() {
                                        all_tasks.push(DocumentTasks {
                                            document_title: doc.title,
                                            groups,
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to fetch document {}: {}",
                                    doc_summary.id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to list documents from Outline: {}", e);
        }
    }

    Json(TasksResponse { tasks: all_tasks })
}
