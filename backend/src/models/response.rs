use crate::models::task::DocumentTasks;
use serde::Serialize;

#[derive(Serialize)]
pub struct TasksResponse {
    pub tasks: Vec<DocumentTasks>,
}
