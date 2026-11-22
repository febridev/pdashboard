use crate::models::task::TasksResponse;

pub async fn fetch_tasks() -> Result<TasksResponse, String> {
    let response = reqwest::get("/api/tasks")
        .await
        .map_err(|e| e.to_string())?;
    
    // Check for authentication errors
    if response.status() == 401 {
        // Redirect to Google OAuth login
        let _ = web_sys::window().map(|w| {
            w.location().set_href("/auth/google").ok();
        });
        return Err("Not authenticated".to_string());
    }
    
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    
    let tasks: TasksResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(tasks)
}
