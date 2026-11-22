use axum::extract::{Query, State};
use axum::response::Redirect;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::Deserialize;
use std::time::Duration;

use crate::routes::tasks::AppState;

#[derive(Deserialize)]
pub struct AuthCallback {
    pub code: String,
    pub state: String,
}

pub async fn google_login(State(state): State<AppState>) -> (CookieJar, Redirect) {
    let csrf_token = CsrfToken::new_random();
    let token_value = csrf_token.secret().clone();

    // Store CSRF token with timestamp
    {
        let mut states = state.csrf_states.lock().unwrap();
        states.insert(token_value.clone(), std::time::Instant::now());
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
    let cookie = Cookie::build(("csrf_state", token_value))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::minutes(5))
        .build();

    let jar = CookieJar::new().add(cookie);
    (jar, Redirect::temporary(auth_url.as_str()))
}

pub async fn google_callback(
    State(state): State<AppState>,
    Query(params): Query<AuthCallback>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), String> {
    // Validate CSRF token from cookie
    let csrf_cookie = jar
        .get("csrf_state")
        .ok_or("Missing CSRF cookie")?;

    {
        let mut states = state.csrf_states.lock().unwrap();
        let stored_time = states
            .remove(csrf_cookie.value())
            .ok_or("Invalid CSRF state")?;

        // Check expiration (5 minutes)
        if stored_time.elapsed() > Duration::from_secs(300) {
            return Err("CSRF token expired".to_string());
        }
    }

    // Verify state parameter matches cookie
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
    let session_cookie = Cookie::build(("session", access_token.to_string()))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::days(7))
        .build();

    // Remove CSRF cookie, add session cookie
    let jar = jar
        .remove(Cookie::from("csrf_state"))
        .add(session_cookie);

    Ok((jar, Redirect::to("/")))
}

pub async fn logout(jar: CookieJar) -> (CookieJar, Redirect) {
    let jar = jar.remove(Cookie::from("session"));
    (jar, Redirect::to("/"))
}
