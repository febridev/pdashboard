use crate::models::task::DocumentTasks;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct TasksResponse {
    pub tasks: Vec<DocumentTasks>,
}
