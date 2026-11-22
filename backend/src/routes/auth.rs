use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use rand::Rng;
use serde::Deserialize;
use std::time::Duration;

use crate::routes::tasks::{AppState, SessionData};

#[derive(Deserialize)]
pub struct AuthCallback {
    pub code: String,
    pub state: String,
}

pub struct RequireAuth;

#[axum::async_trait]
impl axum::extract::FromRequestParts<AppState> for RequireAuth {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract session cookie directly from headers
        let cookie_header = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let session_id = cookie_header
            .split(';')
            .find_map(|c| {
                let c = c.trim();
                if c.starts_with("session=") {
                    Some(c.trim_start_matches("session="))
                } else {
                    None
                }
            })
            .ok_or((StatusCode::UNAUTHORIZED, "Not authenticated"))?;

        // Validate session exists in server-side store
        let sessions = state.sessions.lock().unwrap();
        if sessions.contains_key(session_id) {
            Ok(RequireAuth)
        } else {
            Err((StatusCode::UNAUTHORIZED, "Invalid session"))
        }
    }
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
        .same_site(SameSite::Lax)
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
    let csrf_cookie = jar.get("csrf_state").ok_or("Missing CSRF cookie")?;

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
        .map_err(|e| {
            tracing::error!("Token exchange failed: {}", e);
            e.to_string()
        })?;

    let access_token = token_result.access_token().secret().clone();

    // Generate random session ID
    let session_id: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    // Store session server-side
    {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.insert(
            session_id.clone(),
            SessionData {
                access_token,
                created_at: std::time::Instant::now(),
            },
        );
    }

    tracing::info!("Session created, session_id={}", session_id);

    // Set session cookie with ID (not raw token)
    let session_cookie = Cookie::build(("session", session_id))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(7))
        .build();

    // Remove CSRF cookie, add session cookie
    let removal = Cookie::build(("csrf_state", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(0))
        .build();
    let jar = jar.remove(removal).add(session_cookie);

    Ok((jar, Redirect::to("/")))
}

pub async fn logout(jar: CookieJar) -> (CookieJar, Redirect) {
    let removal = Cookie::build(("session", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(0))
        .build();
    let jar = jar.remove(removal);
    (jar, Redirect::to("/"))
}
