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
