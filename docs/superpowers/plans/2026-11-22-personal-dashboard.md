# Personal Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a personal dashboard with Dioxus + Axum that displays Outline tasks in a Kanban board with Google OAuth authentication.

**Architecture:** Full-stack Rust application with Axum backend (API proxy + OAuth) and Dioxus frontend (Kanban UI). Backend fetches documents from Outline API, parses markdown to extract tasks grouped by headers, and serves them via REST endpoints. Frontend displays tasks in two-column Kanban (Pending/Completed).

**Tech Stack:** Rust, Dioxus (frontend), Axum (backend), reqwest (HTTP client), pulldown-cmark (markdown parser), oauth2 (Google OAuth), Docker

**Spec:** `docs/superpowers/specs/2026-11-22-personal-dashboard-design.md`

## Global Constraints

- Rust edition 2021
- All API keys stored in environment variables only (never committed to git)
- Light color palette: background `#f8f9fa`, cards `#ffffff`, primary `#3b82f6`, text `#1f2937`
- Read-only task view (no updates to Outline)
- Tasks grouped by markdown headers within documents
- Google OAuth for authentication
- Docker deployment with docker-compose

## Review Focus

1. **Outline API rate limits**: Backend should handle rate limiting gracefully with retry logic
2. **Markdown parsing edge cases**: Nested headers, malformed checkboxes, empty documents
3. **OAuth state validation**: Prevent CSRF attacks during Google OAuth flow
4. **Session security**: HttpOnly, Secure cookies with proper expiration
5. **CORS configuration**: Restrict to allowed origins only in production

---

### Task 1: Backend Project Setup

**Files:**
- Create: `backend/Cargo.toml`
- Create: `backend/src/main.rs`
- Create: `backend/src/config.rs`
- Create: `backend/.env.example`

**Interfaces:**
- `config::Config` - loads environment variables
- `main::run_server()` - starts Axum server

- [ ] **Step 1: Create backend Cargo.toml with dependencies**

```toml
[package]
name = "pdashboard-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
dotenv = "0.15"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

- [ ] **Step 2: Create config.rs to load environment variables**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub outline_api_url: String,
    pub outline_api_key: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub session_secret: String,
    pub port: u16,
    pub host: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            outline_api_url: std::env::var("OUTLINE_API_URL").expect("OUTLINE_API_URL must be set"),
            outline_api_key: std::env::var("OUTLINE_API_KEY").expect("OUTLINE_API_KEY must be set"),
            google_client_id: std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set"),
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET must be set"),
            google_redirect_uri: std::env::var("GOOGLE_REDIRECT_URI").expect("GOOGLE_REDIRECT_URI must be set"),
            session_secret: std::env::var("SESSION_SECRET").expect("SESSION_SECRET must be set"),
            port: std::env::var("PORT").unwrap_or_else(|_| "3000".to_string()).parse().expect("PORT must be a number"),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        }
    }
}
```

- [ ] **Step 3: Create minimal main.rs**

```rust
mod config;

use axum::{routing::get, Router};
use config::Config;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let config = Config::from_env();
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }));

    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 4: Create .env.example template**

```env
OUTLINE_API_URL=https://notes.febridev.me/api
OUTLINE_API_KEY=ol_api_your_key_here
GOOGLE_CLIENT_ID=your_client_id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your_client_secret
GOOGLE_REDIRECT_URI=http://localhost:3000/auth/google/callback
SESSION_SECRET=your_random_secret_here
PORT=3000
HOST=0.0.0.0
```

- [ ] **Step 5: Verify backend compiles**

Run: `cd backend && cargo build`
Expected: Compilation succeeds without errors

- [ ] **Step 6: Commit**

```bash
git add backend/
git commit -m "feat: add backend project setup with Axum"
```

---

### Task 2: Outline API Client

**Files:**
- Create: `backend/src/services/mod.rs`
- Create: `backend/src/services/outline.rs`
- Create: `backend/src/models/mod.rs`
- Create: `backend/src/models/document.rs`

**Interfaces:**
- `services::outline::OutlineClient` - HTTP client for Outline API
- `models::document::Document` - Outline document structure
- Consumes: `config::Config`
- Produces: `OutlineClient::list_documents()`, `OutlineClient::get_document()`

- [ ] **Step 1: Add reqwest dependency to Cargo.toml**

Add to existing `backend/Cargo.toml` dependencies:
```toml
reqwest = { version = "0.11", features = ["json"] }
```

- [ ] **Step 2: Create Document model**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub text: Option<String>,
    pub tasks: Option<DocumentTasks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTasks {
    pub completed: u32,
    pub total: u32,
}

#[derive(Debug, Deserialize)]
pub struct OutlineResponse<T> {
    pub ok: bool,
    pub data: T,
}
```

- [ ] **Step 3: Create OutlineClient**

```rust
use crate::config::Config;
use crate::models::document::{Document, OutlineResponse};
use reqwest::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct OutlineClient {
    client: Client,
    config: Arc<Config>,
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
```

- [ ] **Step 4: Update main.rs to initialize OutlineClient**

Add to `backend/src/main.rs`:
```rust
mod services;
mod models;

use services::outline::OutlineClient;
use std::sync::Arc;

// In main function, after config loading:
let config = Arc::new(config);
let outline_client = OutlineClient::new(config.clone());
```

- [ ] **Step 5: Verify compilation**

Run: `cd backend && cargo build`
Expected: Compilation succeeds

- [ ] **Step 6: Commit**

```bash
git add backend/src/services backend/src/models
git commit -m "feat: add Outline API client"
```

---

### Task 3: Markdown Task Parser

**Files:**
- Create: `backend/src/services/parser.rs`
- Create: `backend/src/models/task.rs`
- Test: `backend/tests/parser_test.rs`

**Interfaces:**
- `services::parser::parse_tasks_from_markdown(markdown: &str) -> Vec<TaskGroup>`
- `models::task::Task`, `models::task::TaskGroup`
- Consumes: markdown string from Outline documents
- Produces: structured task groups

- [ ] **Step 1: Create Task models**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub text: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGroup {
    pub header: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTasks {
    pub document_title: String,
    pub groups: Vec<TaskGroup>,
}
```

- [ ] **Step 2: Write failing test for parser**

Create `backend/tests/parser_test.rs`:
```rust
use pdashboard_backend::services::parser::parse_tasks_from_markdown;
use pdashboard_backend::models::task::{TaskStatus};

#[test]
fn test_parse_tasks_with_headers() {
    let markdown = r#"# Project Alpha

## TODO
- [ ] Setup database
- [ ] Design API

## Completed
- [x] Initial planning
- [x] Send emails
"#;

    let groups = parse_tasks_from_markdown(markdown);
    
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].header, "TODO");
    assert_eq!(groups[0].tasks.len(), 2);
    assert_eq!(groups[0].tasks[0].text, "Setup database");
    assert_eq!(groups[0].tasks[0].status, TaskStatus::Pending);
    
    assert_eq!(groups[1].header, "Completed");
    assert_eq!(groups[1].tasks.len(), 2);
    assert_eq!(groups[1].tasks[0].status, TaskStatus::Completed);
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd backend && cargo test parser_test`
Expected: FAIL with "module not found" or "function not found"

- [ ] **Step 4: Implement parser**

Create `backend/src/services/parser.rs`:
```rust
use crate::models::task::{Task, TaskGroup, TaskStatus};

pub fn parse_tasks_from_markdown(markdown: &str) -> Vec<TaskGroup> {
    let mut groups: Vec<TaskGroup> = Vec::new();
    let mut current_header = String::new();
    let mut current_tasks: Vec<Task> = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Check for header (H1-H6)
        if trimmed.starts_with('#') {
            // Save previous group if exists
            if !current_tasks.is_empty() && !current_header.is_empty() {
                groups.push(TaskGroup {
                    header: current_header.clone(),
                    tasks: current_tasks.clone(),
                });
                current_tasks.clear();
            }
            
            // Extract header text (remove # symbols)
            current_header = trimmed.trim_start_matches('#').trim().to_string();
        }
        // Check for task items
        else if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") {
            let status = if trimmed.starts_with("- [x]") {
                TaskStatus::Completed
            } else {
                TaskStatus::Pending
            };

            let text = trimmed
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim()
                .to_string();

            current_tasks.push(Task { text, status });
        }
    }

    // Add final group
    if !current_tasks.is_empty() && !current_header.is_empty() {
        groups.push(TaskGroup {
            header: current_header,
            tasks: current_tasks,
        });
    }

    groups
}
```

- [ ] **Step 5: Export parser module**

Update `backend/src/services/mod.rs`:
```rust
pub mod outline;
pub mod parser;
```

- [ ] **Step 6: Export models**

Update `backend/src/models/mod.rs`:
```rust
pub mod document;
pub mod task;
```

- [ ] **Step 7: Add lib.rs to expose modules for testing**

Create `backend/src/lib.rs`:
```rust
pub mod config;
pub mod models;
pub mod services;
```

- [ ] **Step 8: Update Cargo.toml for lib + bin**

Add to `backend/Cargo.toml`:
```toml
[lib]
name = "pdashboard_backend"
path = "src/lib.rs"

[[bin]]
name = "pdashboard-server"
path = "src/main.rs"
```

- [ ] **Step 9: Run test to verify it passes**

Run: `cd backend && cargo test parser_test`
Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add backend/src/services/parser.rs backend/src/models/task.rs backend/tests/
git commit -m "feat: add markdown task parser with tests"
```

---

### Task 4: Task API Endpoints

**Files:**
- Create: `backend/src/routes/mod.rs`
- Create: `backend/src/routes/tasks.rs`
- Create: `backend/src/models/response.rs`
- Modify: `backend/src/main.rs`

**Interfaces:**
- `routes::tasks::get_tasks()` - GET /api/tasks
- Consumes: `OutlineClient`, `parse_tasks_from_markdown()`
- Produces: JSON response with grouped tasks

- [ ] **Step 1: Create response models**

```rust
use crate::models::task::DocumentTasks;
use serde::Serialize;

#[derive(Serialize)]
pub struct TasksResponse {
    pub tasks: Vec<DocumentTasks>,
}
```

- [ ] **Step 2: Create tasks route handler**

```rust
use crate::models::response::TasksResponse;
use crate::models::task::DocumentTasks;
use crate::services::outline::OutlineClient;
use crate::services::parser::parse_tasks_from_markdown;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub outline_client: OutlineClient,
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
```

- [ ] **Step 3: Update main.rs to wire up routes**

```rust
mod routes;

use routes::tasks::{get_tasks, AppState};
use axum::extract::State;

// In main function:
let state = AppState {
    outline_client: outline_client.clone(),
};

let app = Router::new()
    .route("/health", get(|| async { "OK" }))
    .route("/api/tasks", get(get_tasks))
    .with_state(state);
```

- [ ] **Step 4: Verify compilation**

Run: `cd backend && cargo build`
Expected: Compilation succeeds

- [ ] **Step 5: Test endpoint manually (optional)**

Run: `cd backend && cargo run`
Then in another terminal: `curl http://localhost:3000/api/tasks`
Expected: JSON response with tasks from Outline (requires valid API key)

- [ ] **Step 6: Commit**

```bash
git add backend/src/routes backend/src/main.rs
git commit -m "feat: add task API endpoints"
```

---

### Task 5: Frontend Project Setup

**Files:**
- Create: `frontend/Cargo.toml`
- Create: `frontend/src/main.rs`
- Create: `frontend/index.html`

**Interfaces:**
- Dioxus app entry point
- Consumes: Backend API endpoints
- Produces: Web UI

- [ ] **Step 1: Create frontend Cargo.toml**

```toml
[package]
name = "pdashboard-frontend"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus = { version = "0.5", features = ["web"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
```

- [ ] **Step 2: Create minimal Dioxus app**

```rust
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            h1 { "Personal Dashboard" }
            p { "Loading tasks..." }
        }
    }
}
```

- [ ] **Step 3: Create index.html**

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Personal Dashboard</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
</head>
<body>
    <div id="main"></div>
    <script src="/assets/dioxus/pdashboard-frontend.js"></script>
</body>
</html>
```

- [ ] **Step 4: Verify frontend compiles**

Run: `cd frontend && cargo build --target wasm32-unknown-unknown`
Expected: Compilation succeeds

- [ ] **Step 5: Commit**

```bash
git add frontend/
git commit -m "feat: add frontend project setup with Dioxus"
```

---

### Task 6: Kanban Component

**Files:**
- Create: `frontend/src/components/mod.rs`
- Create: `frontend/src/components/kanban.rs`
- Modify: `frontend/src/main.rs`

**Interfaces:**
- `components::kanban::KanbanBoard` - displays tasks in two columns
- Consumes: task data from API
- Produces: rendered Kanban UI

- [ ] **Step 1: Create Kanban component**

```rust
use dioxus::prelude::*;

#[component]
pub fn KanbanBoard() -> Element {
    rsx! {
        div {
            class: "kanban-board",
            style: "display: flex; gap: 20px; padding: 20px;",
            
            // Pending column
            div {
                class: "kanban-column",
                style: "flex: 1; background: #f8f9fa; padding: 15px; border-radius: 8px;",
                h2 { "Pending" }
                p { "Tasks will appear here" }
            }
            
            // Completed column
            div {
                class: "kanban-column",
                style: "flex: 1; background: #f8f9fa; padding: 15px; border-radius: 8px;",
                h2 { "Completed" }
                p { "Tasks will appear here" }
            }
        }
    }
}
```

- [ ] **Step 2: Update main.rs to use Kanban component**

```rust
mod components;

use components::kanban::KanbanBoard;

#[component]
fn App() -> Element {
    rsx! {
        div {
            style: "background: #f8f9fa; min-height: 100vh; padding: 20px;",
            h1 { 
                style: "color: #1f2937; margin-bottom: 20px;",
                "Personal Dashboard" 
            }
            KanbanBoard {}
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cd frontend && cargo build --target wasm32-unknown-unknown`
Expected: Compilation succeeds

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components
git commit -m "feat: add Kanban board component"
```

---

### Task 7: API Integration

**Files:**
- Create: `frontend/src/api/mod.rs`
- Create: `frontend/src/models/task.rs`
- Modify: `frontend/src/components/kanban.rs`

**Interfaces:**
- `api::fetch_tasks()` - calls backend API
- `models::task::*` - matches backend task models
- Consumes: Backend `/api/tasks` endpoint
- Produces: Task data for rendering

- [ ] **Step 1: Create task models matching backend**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub text: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGroup {
    pub header: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTasks {
    pub document_title: String,
    pub groups: Vec<TaskGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksResponse {
    pub tasks: Vec<DocumentTasks>,
}
```

- [ ] **Step 2: Create API client**

```rust
use crate::models::task::TasksResponse;

pub async fn fetch_tasks() -> Result<TasksResponse, String> {
    let response = reqwest::get("http://localhost:3000/api/tasks")
        .await
        .map_err(|e| e.to_string())?;
    
    let tasks: TasksResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(tasks)
}
```

- [ ] **Step 3: Update Kanban component to fetch and display tasks**

```rust
use dioxus::prelude::*;
use crate::api::fetch_tasks;
use crate::models::task::{Task, TaskStatus, DocumentTasks};

#[component]
pub fn KanbanBoard() -> Element {
    let mut tasks_data = use_signal(|| Vec::new());
    let mut loading = use_signal(|| true);

    use_future(async move {
        if let Ok(response) = fetch_tasks().await {
            tasks_data.set(response.tasks);
        }
        loading.set(false);
    });

    // Collect all pending tasks with document/header context
    let pending_items: Vec<(&str, &str, &Task)> = tasks_data
        .iter()
        .flat_map(|doc| {
            doc.groups.iter().flat_map(move |group| {
                group.tasks.iter()
                    .filter(|t| t.status == TaskStatus::Pending)
                    .map(move |t| (doc.document_title.as_str(), group.header.as_str(), t))
            })
        })
        .collect();

    // Collect all completed tasks with document/header context
    let completed_items: Vec<(&str, &str, &Task)> = tasks_data
        .iter()
        .flat_map(|doc| {
            doc.groups.iter().flat_map(move |group| {
                group.tasks.iter()
                    .filter(|t| t.status == TaskStatus::Completed)
                    .map(move |t| (doc.document_title.as_str(), group.header.as_str(), t))
            })
        })
        .collect();

    rsx! {
        div {
            class: "kanban-board",
            style: "display: flex; gap: 20px; padding: 20px;",
            
            // Pending column
            div {
                class: "kanban-column",
                style: "flex: 1; background: #ffffff; padding: 15px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h2 { 
                    style: "color: #1f2937; margin-bottom: 15px;",
                    "Pending ({pending_items.len()})" 
                }
                if loading() {
                    p { "Loading..." }
                } else {
                    for (doc_title, header, task) in pending_items.iter() {
                        div {
                            key: "{doc_title}-{header}-{task.text}",
                            style: "background: #f8f9fa; padding: 10px; margin-bottom: 8px; border-radius: 4px; border-left: 3px solid #f59e0b;",
                            div { 
                                style: "font-size: 11px; color: #6b7280; margin-bottom: 4px;",
                                "{doc_title} › {header}"
                            }
                            div {
                                style: "color: #1f2937;",
                                "{task.text}"
                            }
                        }
                    }
                }
            }
            
            // Completed column
            div {
                class: "kanban-column",
                style: "flex: 1; background: #ffffff; padding: 15px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h2 { 
                    style: "color: #1f2937; margin-bottom: 15px;",
                    "Completed ({completed_items.len()})" 
                }
                if loading() {
                    p { "Loading..." }
                } else {
                    for (doc_title, header, task) in completed_items.iter() {
                        div {
                            key: "{doc_title}-{header}-{task.text}",
                            style: "background: #f8f9fa; padding: 10px; margin-bottom: 8px; border-radius: 4px; border-left: 3px solid #10b981;",
                            div { 
                                style: "font-size: 11px; color: #6b7280; margin-bottom: 4px;",
                                "{doc_title} › {header}"
                            }
                            div {
                                style: "color: #6b7280; text-decoration: line-through;",
                                "{task.text}"
                            }
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd frontend && cargo build --target wasm32-unknown-unknown`
Expected: Compilation succeeds

- [ ] **Step 5: Commit**

```bash
git add frontend/src/api frontend/src/models
git commit -m "feat: add API integration to fetch tasks"
```

---

### Task 8: Google OAuth Backend

**Files:**
- Create: `backend/src/routes/auth.rs`
- Create: `backend/src/services/auth.rs`
- Add dependency: `oauth2` crate
- Modify: `backend/src/main.rs`

**Interfaces:**
- `routes::auth::google_login()` - GET /auth/google
- `routes::auth::google_callback()` - GET /auth/google/callback
- Consumes: Google OAuth configuration
- Produces: Session cookie on successful auth

- [ ] **Step 1: Add OAuth dependencies to Cargo.toml**

```toml
oauth2 = "4.4"
axum-extra = { version = "0.9", features = ["cookie"] }
rand = "0.8"
time = "0.3"
```

- [ ] **Step 2: Create auth service**

```rust
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl,
};
use crate::config::Config;
use std::sync::Arc;

pub struct AuthService {
    pub client: BasicClient,
}

impl AuthService {
    pub fn new(config: Arc<Config>) -> Self {
        let client = BasicClient::new(
            ClientId::new(config.google_client_id.clone()),
            Some(ClientSecret::new(config.google_client_secret.clone())),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(config.google_redirect_uri.clone()).unwrap());

        Self { client }
    }
}
```

- [ ] **Step 3: Create auth routes**

```rust
use crate::services::auth::AuthService;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct AuthState {
    pub auth_service: Arc<AuthService>,
    pub csrf_states: Arc<Mutex<HashMap<String, Instant>>>,
}

#[derive(Deserialize)]
pub struct AuthCallback {
    pub code: String,
    pub state: String,
}

pub async fn google_login(State(state): State<AuthState>) -> (CookieJar, Redirect) {
    let csrf_token = CsrfToken::new_random();
    let token_value = csrf_token.secret().clone();
    
    // Store CSRF token with expiration (5 minutes)
    {
        let mut states = state.csrf_states.lock().unwrap();
        states.insert(token_value.clone(), Instant::now());
    }

    let (auth_url, _) = state
        .auth_service
        .client
        .authorize_url(|| csrf_token)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    // Store CSRF token in cookie for validation
    let cookie = Cookie::build("csrf_state", token_value)
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::minutes(5))
        .finish();

    let jar = CookieJar::new().add(cookie);
    (jar, Redirect::temporary(auth_url.as_str()))
}

pub async fn google_callback(
    State(state): State<AuthState>,
    Query(params): Query<AuthCallback>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), String> {
    // Validate CSRF token
    let csrf_cookie = jar
        .get("csrf_state")
        .ok_or("Missing CSRF cookie")?;
    
    {
        let mut states = state.csrf_states.lock().unwrap();
        let stored_time = states
            .remove(csrf_cookie.value())
            .ok_or("Invalid CSRF state")?;
        
        // Check expiration
        if stored_time.elapsed() > Duration::from_secs(300) {
            return Err("CSRF token expired".to_string());
        }
    }

    // Verify state parameter matches
    if params.state != csrf_cookie.value() {
        return Err("CSRF state mismatch".to_string());
    }

    // Exchange code for token
    let token_result = state
        .auth_service
        .client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| e.to_string())?;

    // Create session cookie
    let access_token = token_result.access_token().secret();
    let session_cookie = Cookie::build("session", access_token.to_string())
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::days(7))
        .finish();

    // Remove CSRF cookie
    let jar = jar
        .remove(Cookie::named("csrf_state"))
        .add(session_cookie);
    
    Ok((jar, Redirect::to("/")))
}

pub async fn logout(jar: CookieJar) -> (CookieJar, Redirect) {
    let jar = jar.remove(Cookie::named("session"));
    (jar, Redirect::to("/"))
}
```

- [ ] **Step 4: Update main.rs to add auth routes**

```rust
use routes::auth::{google_login, google_callback, logout, AuthState};
use routes::tasks::AppState;
use services::auth::AuthService;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_http::cors::{CorsLayer, Any};

// Combined app state
#[derive(Clone)]
struct AppState2 {
    tasks: AppState,
    auth: AuthState,
}

// In main function:
let auth_service = Arc::new(AuthService::new(config.clone()));
let auth_state = AuthState {
    auth_service,
    csrf_states: Arc::new(Mutex::new(HashMap::new())),
};

let app_state = AppState2 {
    tasks: AppState {
        outline_client: outline_client.clone(),
    },
    auth: auth_state,
};

// CORS configuration
let cors = CorsLayer::new()
    .allow_origin(["http://localhost:8080".parse().unwrap()])
    .allow_methods(Any())
    .allow_headers(Any())
    .allow_credentials(true);

let app = Router::new()
    .route("/health", get(|| async { "OK" }))
    .route("/api/tasks", get(get_tasks))
    .route("/auth/google", get(google_login))
    .route("/auth/google/callback", get(google_callback))
    .route("/auth/logout", get(logout))
    .layer(cors)
    .with_state(app_state);
```

Note: Route handlers need to extract their respective state from the combined state using custom extractors or by adjusting handler signatures.

- [ ] **Step 5: Verify compilation**

Run: `cd backend && cargo build`
Expected: Compilation succeeds

- [ ] **Step 6: Commit**

```bash
git add backend/src/routes/auth.rs backend/src/services/auth.rs
git commit -m "feat: add Google OAuth authentication"
```

---

### Task 9: Docker Setup

**Files:**
- Create: `backend/Dockerfile`
- Create: `frontend/Dockerfile`
- Create: `docker-compose.yml`
- Create: `.dockerignore`

**Interfaces:**
- Multi-stage Docker builds
- Consumes: Backend and frontend code
- Produces: Containerized application

- [ ] **Step 1: Create backend Dockerfile**

```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/pdashboard-server /app/server
EXPOSE 3000
CMD ["/app/server"]
```

- [ ] **Step 2: Create frontend Dockerfile**

```dockerfile
# Build stage
FROM rust:1.75 as builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY index.html ./
RUN dx build --release

# Runtime stage
FROM nginx:alpine
COPY --from=builder /app/target/dx/pdashboard-frontend/release/web /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

- [ ] **Step 3: Create docker-compose.yml**

```yaml
version: '3.8'

services:
  backend:
    build: ./backend
    ports:
      - "3000:3000"
    env_file:
      - .env
    restart: unless-stopped

  frontend:
    build: ./frontend
    ports:
      - "8080:80"
    depends_on:
      - backend
    restart: unless-stopped
```

- [ ] **Step 4: Create .dockerignore**

```
target/
.env
.git
.gitignore
*.md
docs/
```

- [ ] **Step 5: Test Docker build**

Run: `docker-compose build`
Expected: Both images build successfully

- [ ] **Step 6: Commit**

```bash
git add Dockerfile docker-compose.yml .dockerignore
git commit -m "feat: add Docker setup for deployment"
```

---

### Task 10: Documentation

**Files:**
- Create: `README.md`
- Create: `.env.example` (root level)

**Interfaces:**
- User documentation
- Consumes: Project structure and setup requirements
- Produces: Clear setup instructions

- [ ] **Step 1: Create README.md**

```markdown
# Personal Dashboard

A personal dashboard built with Rust (Dioxus + Axum) that displays tasks from Outline in a Kanban board.

## Features

- Kanban board with Pending and Completed columns
- Tasks fetched from Outline API
- Grouped by markdown headers
- Google OAuth authentication
- Docker deployment

## Setup

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose (for deployment)
- Outline API key
- Google OAuth credentials

### Configuration

1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```

2. Fill in your credentials:
   - `OUTLINE_API_KEY`: Get from Outline Settings → API & Apps
   - `GOOGLE_CLIENT_ID` & `GOOGLE_CLIENT_SECRET`: Create in Google Cloud Console
   - `GOOGLE_REDIRECT_URI`: e.g., `http://localhost:3000/auth/google/callback`
   - `SESSION_SECRET`: Random string for session encryption

### Local Development

```bash
# Run backend
cd backend
cargo run

# Run frontend (separate terminal)
cd frontend
dx serve
```

### Docker Deployment

```bash
docker-compose up -d
```

The dashboard will be available at `http://localhost:8080`.

### Reverse Proxy

Example nginx configuration:

```nginx
server {
    listen 443 ssl;
    server_name dashboard.yourdomain.com;
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    location /api {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Architecture

- **Backend**: Axum server (Rust)
  - Outline API integration
  - Markdown task parser
  - Google OAuth
  - REST API endpoints

- **Frontend**: Dioxus web app (Rust/WASM)
  - Kanban board UI
  - Task display
  - Light color palette

## License

MIT
```

- [ ] **Step 2: Create root .env.example**

```env
# Outline API
OUTLINE_API_URL=https://notes.febridev.me/api
OUTLINE_API_KEY=ol_api_your_key_here

# Google OAuth
GOOGLE_CLIENT_ID=your_client_id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your_client_secret
GOOGLE_REDIRECT_URI=http://localhost:3000/auth/google/callback

# Security
SESSION_SECRET=generate_a_random_secret_string_here

# Server
PORT=3000
HOST=0.0.0.0
```

- [ ] **Step 3: Create .gitignore**

```
/target/
/.env
*.log
.DS_Store
```

- [ ] **Step 4: Commit**

```bash
git add README.md .env.example .gitignore
git commit -m "docs: add README and configuration templates"
```

---

## Implementation Complete

All tasks are now defined. The plan covers:
1. Backend setup with Axum
2. Outline API integration
3. Markdown task parsing
4. Task API endpoints
5. Frontend setup with Dioxus
6. Kanban UI component
7. API integration
8. Google OAuth authentication
9. Docker deployment
10. Documentation

Each task is self-contained with tests where applicable, and follows TDD principles.
