use crate::models::response::TasksResponse;
use crate::models::task::DocumentTasks;
use crate::routes::auth::RequireAuth;
use crate::services::auth::AuthService;
use crate::services::live::TaskEvents;
use crate::services::outline::OutlineClient;
use crate::services::parser::parse_tasks_from_markdown;
use axum::extract::State;
use axum::http::{HeaderName, HeaderValue};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct AppState {
    pub outline_client: OutlineClient,
    pub auth_service: Arc<AuthService>,
    pub csrf_states: Arc<Mutex<HashMap<String, Instant>>>,
    pub sessions: Arc<Mutex<HashMap<String, SessionData>>>,
    pub task_events: Arc<TaskEvents>,
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

    // Check if a specific document ID is configured
    if let Some(doc_id) = &state.outline_client.config.outline_task_document_id {
        // Fetch the specific document
        match state.outline_client.get_document(doc_id).await {
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
                tracing::error!("Failed to fetch document {}: {}", doc_id, e);
            }
        }
    } else {
        // Fall back to listing all documents
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
    }

    Json(TasksResponse { tasks: all_tasks })
}

fn sse_event(snapshot: &TasksResponse) -> Event {
    Event::default()
        .event("tasks")
        .data(serde_json::to_string(snapshot).expect("serialize snapshot"))
}

/// SSE endpoint: sends the current snapshot immediately, then one `tasks`
/// event each time the poller publishes a changed snapshot. Keep-alive
/// comments keep proxies from closing the quiet connection.
pub async fn task_stream(
    _auth: RequireAuth,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    let mut rx = state.task_events.subscribe();
    let first = rx.borrow_and_update().clone();
    let stream = async_stream::stream! {
        if let Some(snapshot) = first {
            yield Ok::<Event, Infallible>(sse_event(&snapshot));
        }
        loop {
            if rx.changed().await.is_err() {
                break; // channel gone (server shutdown)
            }
            // Bind the clone to a local first: the Ref guard from
            // borrow_and_update() must drop before the yield, or the
            // generator's future is not Send.
            let changed = rx.borrow_and_update().clone();
            if let Some(snapshot) = changed {
                yield Ok(sse_event(&snapshot));
            }
        }
    };
    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    );
    (
        [(
            HeaderName::from_static("x-accel-buffering"),
            HeaderValue::from_static("no"),
        )],
        sse,
    )
}
