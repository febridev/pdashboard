use crate::models::task::TasksResponse;

pub async fn fetch_tasks() -> Result<TasksResponse, String> {
    let response = reqwest::get("/api/tasks")
        .await
        .map_err(|e| e.to_string())?;
    
    let tasks: TasksResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(tasks)
}
