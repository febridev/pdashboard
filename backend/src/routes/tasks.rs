use crate::models::response::TasksResponse;
use crate::models::task::DocumentTasks;
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
}

pub async fn get_tasks(State(state): State<AppState>) -> Json<TasksResponse> {
    let mut all_tasks = Vec::new();

    // Fetch all documents
    if let Ok(documents) = state.outline_client.list_documents().await {
        for doc_summary in documents {
            // Only fetch documents that have tasks
            if let Some(task_info) = doc_summary.tasks {
                if task_info.total > 0 {
                    // Fetch full document with content
                    if let Ok(doc) = state.outline_client.get_document(&doc_summary.id).await {
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
                }
            }
        }
    }

    Json(TasksResponse { tasks: all_tasks })
}
