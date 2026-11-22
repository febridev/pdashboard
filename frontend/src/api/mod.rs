use crate::models::task::TasksResponse;

pub enum ApiError {
    /// The API answered 401: the user must (re)authenticate.
    Unauthorized,
    /// Anything else (request build failure, network, HTTP, parse errors).
    /// Rendered to the user instead of being mistaken for "not signed in".
    Other(String),
}

/// reqwest only accepts absolute URLs: a relative path fails with
/// "builder error: relative URL without a base" before any request is sent.
/// (That failure was previously swallowed, so the UI always showed the
/// sign-in card, signed in or not.)
pub fn absolute_url(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

/// The origin the browser loaded this page from ("" when unavailable,
/// e.g. outside a browser; the resulting request error is then rendered).
#[cfg(target_arch = "wasm32")]
fn origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn origin() -> String {
    String::new()
}

pub async fn fetch_tasks() -> Result<TasksResponse, ApiError> {
    let response = reqwest::get(absolute_url(&origin(), "/api/tasks"))
        .await
        .map_err(|e| ApiError::Other(e.to_string()))?;

    // Check for authentication errors
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ApiError::Unauthorized);
    }

    if !response.status().is_success() {
        return Err(ApiError::Other(format!("HTTP {}", response.status())));
    }

    let tasks: TasksResponse = response
        .json()
        .await
        .map_err(|e| ApiError::Other(e.to_string()))?;

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_origin_and_path() {
        assert_eq!(
            absolute_url("https://dash.example.com", "/api/tasks"),
            "https://dash.example.com/api/tasks"
        );
    }

    #[test]
    fn does_not_double_slash_when_origin_ends_with_slash() {
        assert_eq!(
            absolute_url("https://dash.example.com/", "/api/tasks"),
            "https://dash.example.com/api/tasks"
        );
    }

    #[test]
    fn keeps_port_from_origin() {
        assert_eq!(
            absolute_url("http://localhost:8081", "/api/tasks"),
            "http://localhost:8081/api/tasks"
        );
    }
}
