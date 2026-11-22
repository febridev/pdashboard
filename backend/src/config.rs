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
