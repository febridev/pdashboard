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
