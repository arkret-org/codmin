//! OAuth/OIDC + session lifecycle.
//!
//! The bearer credential lives in an httpOnly cookie that coauth's
//! `/oauth/token` endpoint sets via `Set-Cookie`. The cookie is
//! `HttpOnly + Secure + SameSite=Strict + __Host-` prefixed so it
//! cannot be read from JS, can only be sent to the issuing origin, and
//! cannot be smuggled across navigations from third-party iframes.
//!
//! This module's contract is therefore:
//!
//! - **Login** — `start_oauth_login` opens the OAuth authorize URL. PKCE verifier / state / nonce
//!   live in `sessionStorage` (still readable from JS, but they are *one-shot*: invalid after the
//!   callback consumes them, so leakage has zero replay value).
//! - **Callback** — `handle_oauth_callback` exchanges the code with `credentials: "include"`.
//!   coauth sets the session cookie. On success, codmin stores ONLY a non-secret `session_active=1`
//!   marker plus the userinfo block (id / display name / avatar). None of these carry entropy.
//! - **API calls** — `api::client::api_client` now sends every request with `credentials:
//!   "include"` so the cookie travels with the request automatically. There is no `Authorization:
//!   Bearer` header from the SPA.
//! - **Refresh** — when the cookie has expired the server returns 401, the client invokes
//!   `handle_unauthorized` which calls `/oauth/token` (grant_type=refresh_token) (also with
//!   `credentials: "include"`) and the server sets a new cookie. No JS-visible refresh_token.
//! - **Logout** — `logout` records the Station hard-logout, OAuth revoke, and coauth browser-cookie
//!   logout outcomes separately. JS-visible session state is cleared only after the cookie-owning
//!   coauth endpoint explicitly confirms browser-session termination.

use std::cell::Cell;

use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::RequestCredentials;

use crate::utils::net::error::HttpError;
use crate::utils::security::crypto::{base64url_encode, random_token};
use crate::utils::storage;

const OAUTH_CLIENT_ID: &str = "codmin";
const COAUTH_ADMIN_SCOPE: &str = "urn:coauth:admin";
const ARKRET_ADMIN_SCOPE: &str = "urn:arkret:admin:*";
const PKCE_VERIFIER_KEY: &str = "pkce_code_verifier";
const OAUTH_STATE_KEY: &str = "oauth_state";
const OAUTH_NONCE_KEY: &str = "oauth_nonce";

/// Non-secret marker localStorage key. Set to `"1"` after a successful
/// OAuth callback, removed on logout. Carries no entropy — the actual
/// bearer credential is in an httpOnly cookie.
pub(crate) const SESSION_ACTIVE_KEY: &str = "session_active";

#[derive(Debug)]
struct TextResponse {
    status: u16,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogoutStepStatus {
    Confirmed,
    Failed(String),
}

impl LogoutStepStatus {
    fn is_confirmed(&self) -> bool {
        matches!(self, Self::Confirmed)
    }

    fn failure(&self) -> Option<&str> {
        match self {
            Self::Confirmed => None,
            Self::Failed(message) => Some(message),
        }
    }
}

/// Result of the three independent logout responsibilities. The Principal
/// Server owns hard account-session cleanup, OAuth revoke is a separate token
/// revocation signal, and coauth alone can expire its HttpOnly browser cookie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoutReport {
    pub principal_logout: LogoutStepStatus,
    pub oauth_revoke: LogoutStepStatus,
    pub cookie_logout: LogoutStepStatus,
}

impl LogoutReport {
    /// The only safe gate for clearing the SPA marker: JavaScript cannot inspect
    /// the HttpOnly cookie, so a typed success from its owning endpoint is the
    /// confirmation boundary.
    pub fn local_cookie_cleared(&self) -> bool {
        self.cookie_logout.is_confirmed()
    }

    pub fn fully_confirmed(&self) -> bool {
        self.local_cookie_cleared()
            && self.principal_logout.is_confirmed()
            && self.oauth_revoke.is_confirmed()
    }

    /// Fixed, non-sensitive codes suitable for the login-page query string
    /// after local cookie removal has already succeeded.
    pub fn login_warning_code(&self) -> Option<String> {
        if !self.local_cookie_cleared() {
            return None;
        }
        let mut failed = Vec::new();
        if !self.principal_logout.is_confirmed() {
            failed.push("principal_logout");
        }
        if !self.oauth_revoke.is_confirmed() {
            failed.push("oauth_revoke");
        }
        (!failed.is_empty()).then(|| failed.join(","))
    }

    /// User-facing retry guidance for the authenticated surface. This is used
    /// only when cookie removal was not confirmed and the session marker stays
    /// present, so the same button remains a valid retry path.
    pub fn retry_message(&self) -> String {
        let failures = [
            self.principal_logout.failure(),
            self.oauth_revoke.failure(),
            self.cookie_logout.failure(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("; ");
        format!(
            "Sign-out was not confirmed and your local session marker was kept. Retry sign-out. {failures}"
        )
    }
}

fn build_oauth_scope() -> String {
    format!("{COAUTH_ADMIN_SCOPE} {ARKRET_ADMIN_SCOPE}")
}

fn coauth_public_base() -> Option<String> {
    crate::utils::net::session::coauth_public_url()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
}

fn make_err(msg: String) -> HttpError {
    HttpError::message(msg)
}

fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "browser API call failed".to_string())
}

fn browser_window() -> Result<web_sys::Window, HttpError> {
    web_sys::window().ok_or_else(|| make_err("Browser window is unavailable".into()))
}

fn session_storage() -> Result<web_sys::Storage, HttpError> {
    browser_window()?
        .session_storage()
        .map_err(js_error)
        .map_err(make_err)?
        .ok_or_else(|| make_err("Session storage is unavailable".into()))
}

fn current_origin() -> Result<String, HttpError> {
    browser_window()?
        .location()
        .origin()
        .map_err(js_error)
        .map_err(make_err)
}

fn redirect_to_login() {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_href("/login");
    }
}

async fn read_text_response(response: gloo_net::http::Response) -> Result<TextResponse, HttpError> {
    let status = response.status();
    let text = response.text().await.map_err(|e| make_err(e.to_string()))?;
    Ok(TextResponse { status, text })
}

async fn send_form_post(url: &str, body: &str) -> Result<TextResponse, HttpError> {
    let builder = Request::post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        // S5: every OAuth call must carry the httpOnly cookie so the
        // server can both set the cookie on /token and read it on
        // /refresh and /revoke.
        .credentials(RequestCredentials::Include);

    let response = builder
        .body(body.to_string())
        .map_err(|e| make_err(e.to_string()))?
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;

    read_text_response(response).await
}

async fn send_oauth_form_request(path: &str, body: &str) -> Result<TextResponse, HttpError> {
    send_form_post(path, body).await
}

fn generate_code_verifier() -> String {
    random_token(32)
}

async fn compute_code_challenge(verifier: &str) -> Result<String, HttpError> {
    let crypto = browser_window()?
        .crypto()
        .map_err(js_error)
        .map_err(make_err)?;
    let subtle = crypto.subtle();
    let data = js_sys::Uint8Array::from(verifier.as_bytes());
    let promise = subtle
        .digest_with_str_and_buffer_source("SHA-256", &data)
        .map_err(js_error)
        .map_err(make_err)?;
    let result = JsFuture::from(promise)
        .await
        .map_err(js_error)
        .map_err(make_err)?;
    let buffer = result
        .dyn_into::<js_sys::ArrayBuffer>()
        .map_err(|_| make_err("SHA-256 digest returned an unexpected value".into()))?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    Ok(base64url_encode(&bytes))
}

pub async fn start_oauth_login() -> Result<(), HttpError> {
    let verifier = generate_code_verifier();
    let challenge = compute_code_challenge(&verifier).await?;
    let state = generate_code_verifier();
    let nonce = generate_code_verifier();
    let scope = build_oauth_scope();

    let session = session_storage()?;
    session
        .set_item(PKCE_VERIFIER_KEY, &verifier)
        .map_err(js_error)
        .map_err(make_err)?;
    session
        .set_item(OAUTH_STATE_KEY, &state)
        .map_err(js_error)
        .map_err(make_err)?;
    session
        .set_item(OAUTH_NONCE_KEY, &nonce)
        .map_err(js_error)
        .map_err(make_err)?;

    let redirect_uri = {
        let origin = current_origin()?;
        format!("{origin}/oauth/callback")
    };

    let coauth_base =
        coauth_public_base().ok_or_else(|| make_err("Missing coauth public URL".into()))?;

    let auth_url = format!(
        "{coauth_base}/authorize?response_type=code\
         &client_id={OAUTH_CLIENT_ID}\
         &redirect_uri={}\
         &code_challenge={challenge}\
         &code_challenge_method=S256\
         &scope={}\
         &state={}\
         &nonce={}",
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&scope),
        urlencoding::encode(&state),
        urlencoding::encode(&nonce),
    );

    browser_window()?
        .location()
        .set_href(&auth_url)
        .map_err(js_error)
        .map_err(make_err)?;
    Ok(())
}

pub async fn handle_oauth_callback(code: &str, state: Option<&str>) -> Result<(), HttpError> {
    let session = session_storage()?;
    let verifier = session
        .get_item(PKCE_VERIFIER_KEY)
        .ok()
        .flatten()
        .ok_or_else(|| make_err("Missing PKCE verifier".into()))?;
    let expected_state = session
        .get_item(OAUTH_STATE_KEY)
        .ok()
        .flatten()
        .ok_or_else(|| make_err("Missing OAuth state".into()))?;
    if state != Some(expected_state.as_str()) {
        session.remove_item(PKCE_VERIFIER_KEY).ok();
        session.remove_item(OAUTH_STATE_KEY).ok();
        session.remove_item(OAUTH_NONCE_KEY).ok();
        return Err(make_err("OAuth state validation failed".into()));
    }
    // Capture nonce before cleanup; any id_token echo is diagnostic only.
    let expected_nonce = session.get_item(OAUTH_NONCE_KEY).ok().flatten();
    session.remove_item(PKCE_VERIFIER_KEY).ok();
    session.remove_item(OAUTH_STATE_KEY).ok();
    session.remove_item(OAUTH_NONCE_KEY).ok();

    let redirect_uri = {
        let origin = current_origin()?;
        format!("{origin}/oauth/callback")
    };

    let form_body = format!(
        "grant_type=authorization_code\
         &code={}\
         &redirect_uri={}\
         &client_id={OAUTH_CLIENT_ID}\
         &code_verifier={}",
        urlencoding::encode(code),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&verifier),
    );

    let response = send_oauth_form_request("/oauth/token", &form_body).await?;
    if response.status >= 400 {
        return Err(make_err(format!(
            "Token exchange failed ({}): {}",
            response.status, response.text
        )));
    }

    // S5: the bearer credential rides back as an httpOnly cookie set by
    // coauth, NOT in the JSON body. We still parse the body for
    // optional metadata (id_token nonce, expiry hint, scope) but never
    // touch `access_token` / `refresh_token` even if the server
    // returns them on the wire.
    let token_resp: TokenResponse =
        serde_json::from_str(&response.text).map_err(|e| make_err(e.to_string()))?;

    // The bearer session is established by coauth's token endpoint and
    // then confirmed through `get_viewer()` below. This SPA does not
    // verify id_token signatures, so id_token claims are never used as
    // authorization input. When an id_token is present, compare its
    // unsigned nonce only as a diagnostic for provider drift; PKCE and
    // `state` remain the client-side replay/CSRF controls.
    if let Some(expected) = &expected_nonce {
        match &token_resp.id_token {
            Some(id_token) => match extract_unsigned_id_token_nonce(id_token) {
                Some(nonce_in_token) if &nonce_in_token == expected => {
                    log::debug!("OAuth id_token carried the expected nonce");
                }
                Some(_) => {
                    log::warn!(
                        "OAuth id_token nonce mismatch ignored; id_token is unsigned in the SPA and get_viewer remains authoritative"
                    );
                }
                None => {
                    log::warn!(
                        "OAuth id_token carried no readable nonce; id_token is unsigned in the SPA and get_viewer remains authoritative"
                    );
                }
            },
            None => {
                log::warn!(
                    "OAuth token response carried no id_token; PKCE + state remain enforced"
                );
            }
        }
    }

    // Mark the session as active for the JS-visible UI checks. The
    // value `"1"` carries no entropy.
    storage::set_item(SESSION_ACTIVE_KEY, "1");
    persist_token_expiry(token_resp.expires_in);

    match crate::api::coauth::get_viewer().await {
        Ok(viewer_resp) => {
            storage::set_item("user_id", viewer_resp.principal_id.as_str());
            if let Some(profile) = viewer_resp.profile.as_ref() {
                storage::set_item("user_display_name", &profile.display_name);
            }
            storage::remove_item("user_avatar_url");
        }
        Err(err) => {
            log::warn!("OAuth viewer fetch failed after token exchange: {}", err);
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct TokenResponse {
    expires_in: Option<u64>,
    id_token: Option<String>,
}

/// Extract an unsigned `nonce` claim from a JWT id_token payload.
/// Returns `None` if the token is malformed or has no nonce claim.
fn extract_unsigned_id_token_nonce(id_token: &str) -> Option<String> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload_bytes = crate::utils::security::crypto::base64url_decode(parts[1])?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;
    payload.get("nonce")?.as_str().map(String::from)
}

pub async fn handle_unauthorized() -> bool {
    if refresh_oauth_token_singleflight().await {
        return true;
    }
    clear_session_marker();
    redirect_to_login();
    false
}

thread_local! {
    static REFRESH_IN_FLIGHT: Cell<bool> = const { Cell::new(false) };
}

struct RefreshFlightGuard;

impl RefreshFlightGuard {
    fn acquire() -> Option<Self> {
        let acquired = REFRESH_IN_FLIGHT.with(|flag| {
            if flag.get() {
                false
            } else {
                flag.set(true);
                true
            }
        });
        acquired.then_some(Self)
    }
}

impl Drop for RefreshFlightGuard {
    fn drop(&mut self) {
        REFRESH_IN_FLIGHT.with(|flag| flag.set(false));
    }
}

fn refresh_in_flight() -> bool {
    REFRESH_IN_FLIGHT.with(|flag| flag.get())
}

async fn wait_for_refresh() {
    while refresh_in_flight() {
        TimeoutFuture::new(25).await;
    }
}

async fn refresh_oauth_token_singleflight() -> bool {
    if let Some(_guard) = RefreshFlightGuard::acquire() {
        return refresh_oauth_token().await;
    }
    wait_for_refresh().await;
    is_authenticated() && !token_expires_soon()
}

pub async fn refresh_oauth_token() -> bool {
    // S5: the refresh token is in the httpOnly cookie. Just hit
    // /oauth/token (refresh) with credentials: "include" — coauth pulls the
    // refresh token from the cookie, mints a new pair, and ships back
    // a new cookie. The JSON body carries no secrets.
    let form_body = format!("grant_type=refresh_token&client_id={OAUTH_CLIENT_ID}");

    let response = match send_oauth_form_request("/oauth/token", &form_body).await {
        Ok(r) => r,
        Err(_) => return false,
    };

    if response.status >= 400 {
        return false;
    }

    let token_resp: TokenResponse = match serde_json::from_str(&response.text) {
        Ok(t) => t,
        Err(_) => return false,
    };

    storage::set_item(SESSION_ACTIVE_KEY, "1");
    persist_token_expiry(token_resp.expires_in);
    true
}

pub async fn verify_admin() -> Result<bool, HttpError> {
    if !is_authenticated() {
        return Err(make_err("Not authenticated".into()));
    }

    let viewer = crate::api::coauth::get_viewer().await?;
    storage::set_item(
        "is_admin",
        if viewer.is_server_admin {
            "true"
        } else {
            "false"
        },
    );
    Ok(viewer.is_server_admin)
}

pub fn cached_is_admin() -> Option<bool> {
    storage::get_item("is_admin").map(|v| v == "true")
}

fn persist_token_expiry(expires_in: Option<u64>) {
    if let Some(expires_in) = expires_in {
        storage::set_item(
            "token_expires_at_ms",
            &((js_sys::Date::now() as u64) + expires_in.saturating_mul(1000)).to_string(),
        );
    } else {
        storage::remove_item("token_expires_at_ms");
    }
}

pub fn token_expiry_ms() -> Option<u64> {
    storage::get_item("token_expires_at_ms").and_then(|value| value.parse().ok())
}

/// Window before access-token expiry where we proactively spend a
/// refresh round-trip rather than gambling that the next admin call will
/// land before the server starts rejecting with 401.
const TOKEN_REFRESH_LEEWAY_MS: u64 = 60_000;

/// True if the cached access token is within `TOKEN_REFRESH_LEEWAY_MS`
/// of expiry (or has no recorded expiry — treat as eager refresh).
pub fn token_expires_soon() -> bool {
    match token_expiry_ms() {
        Some(expiry) => {
            let now = js_sys::Date::now() as u64;
            expiry.saturating_sub(now) <= TOKEN_REFRESH_LEEWAY_MS
        }
        None => false,
    }
}

/// Issue a refresh round-trip if we're inside the leeway window. Called
/// from `api::client::api_client` ahead of every admin call so the
/// happy-path stays a single request and we don't rely on the 401 retry.
pub async fn refresh_if_expiring_soon() {
    if !is_authenticated() {
        return;
    }
    if !token_expires_soon() {
        return;
    }
    let _ = refresh_oauth_token_singleflight().await;
}

async fn send_credentialed_empty_post(path: &str) -> Result<TextResponse, HttpError> {
    let response = Request::post(path)
        .header("Accept", "application/json")
        .credentials(RequestCredentials::Include)
        .send()
        .await
        .map_err(|error| make_err(error.to_string()))?;
    read_text_response(response).await
}

fn failed_request(label: &str, error: HttpError) -> LogoutStepStatus {
    if error.status == 0 {
        LogoutStepStatus::Failed(format!("{label} request failed before a response"))
    } else {
        LogoutStepStatus::Failed(format!("{label} request failed with HTTP {}", error.status))
    }
}

fn rejected_response(label: &str, status: u16) -> LogoutStepStatus {
    LogoutStepStatus::Failed(format!("{label} returned HTTP {status}"))
}

fn classify_oauth_revoke(response: Result<TextResponse, HttpError>) -> LogoutStepStatus {
    match response {
        Ok(response) if (200..300).contains(&response.status) => LogoutStepStatus::Confirmed,
        Ok(response) => rejected_response("OAuth revoke", response.status),
        Err(error) => failed_request("OAuth revoke", error),
    }
}

fn classify_principal_logout(response: Result<TextResponse, HttpError>) -> LogoutStepStatus {
    let response = match response {
        Ok(response) if (200..300).contains(&response.status) => response,
        Ok(response) => return rejected_response("Station logout", response.status),
        Err(error) => return failed_request("Station logout", error),
    };
    let Ok(outcome) =
        serde_json::from_str::<arkret_models_identity::AccountLogoutOutcome>(&response.text)
    else {
        return LogoutStepStatus::Failed("Station logout returned an invalid response".to_owned());
    };
    if outcome.revoked {
        LogoutStepStatus::Confirmed
    } else {
        LogoutStepStatus::Failed("Station logout did not confirm session revocation".to_owned())
    }
}

fn classify_cookie_logout(response: Result<TextResponse, HttpError>) -> LogoutStepStatus {
    let response = match response {
        Ok(response) if (200..300).contains(&response.status) => response,
        Ok(response) => return rejected_response("Browser cookie logout", response.status),
        Err(error) => return failed_request("Browser cookie logout", error),
    };
    let Ok(outcome) = serde_json::from_str::<coauth_account_types::LogoutOutcome>(&response.text)
    else {
        return LogoutStepStatus::Failed(
            "Browser cookie logout returned an invalid response".to_owned(),
        );
    };
    if outcome.status == "success" {
        LogoutStepStatus::Confirmed
    } else {
        LogoutStepStatus::Failed("Browser cookie logout did not confirm cookie removal".to_owned())
    }
}

pub async fn logout() -> LogoutReport {
    // The protocol hard-logout is authoritative for the account grant and
    // Principal-side device session. It runs before the standalone OAuth
    // revoke so token invalidation cannot prevent Principal-side cleanup.
    let principal_logout = classify_principal_logout(
        send_credentialed_empty_post("/_arkret/gate/account/logout").await,
    );

    // OAuth revocation remains a separate result. RFC 7009 success is conveyed
    // by a 2xx response, including the already-revoked/unknown-token case.
    let body = format!("client_id={OAUTH_CLIENT_ID}");
    let oauth_revoke = classify_oauth_revoke(send_oauth_form_request("/oauth/revoke", &body).await);

    // coauth owns the HttpOnly browser-session cookie. Always attempt this
    // final local termination even when an upstream cleanup step failed; only
    // its typed success can authorize clearing the JS-visible marker.
    let cookie_logout =
        classify_cookie_logout(send_credentialed_empty_post("/_coauth/account/auth/logout").await);

    let report = LogoutReport {
        principal_logout,
        oauth_revoke,
        cookie_logout,
    };

    finalize_logout_report(&report);
    report
}

fn finalize_logout_report(report: &LogoutReport) -> bool {
    if !report.local_cookie_cleared() {
        return false;
    }
    clear_session_marker();
    true
}

/// Drop every JS-visible piece of session state so the next user on
/// the same browser doesn't inherit a logged-in UI shell. The actual
/// bearer cookie is cleared by the server's Set-Cookie response — this
/// helper handles ONLY the markers that survive in localStorage.
fn clear_session_marker() {
    const KEYS_TO_CLEAR: &[&str] = &[
        SESSION_ACTIVE_KEY,
        "is_admin",
        "token_expires_at_ms",
        "user_id",
        "user_display_name",
        "user_avatar_url",
    ];
    for key in KEYS_TO_CLEAR {
        storage::remove_item(key);
    }
}

/// True if the JS-visible session marker is set. The actual bearer
/// credential lives in an httpOnly cookie; this is just a hint for
/// rendering the login page vs. the authenticated layout. The server
/// is the ultimate source of truth on every API call.
pub fn is_authenticated() -> bool {
    storage::get_item(SESSION_ACTIVE_KEY).is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        LogoutReport, LogoutStepStatus, SESSION_ACTIVE_KEY, TextResponse, build_oauth_scope,
        classify_cookie_logout, classify_oauth_revoke, classify_principal_logout,
        extract_unsigned_id_token_nonce,
    };
    use crate::utils::net::error::HttpError;
    use crate::utils::security::crypto::base64url_encode;

    #[test]
    fn oauth_scope_contains_admin_scopes() {
        let scope = build_oauth_scope();
        assert!(scope.contains("urn:coauth:admin"));
        assert!(scope.contains("urn:arkret:admin:*"));
        assert!(!scope.contains("urn:ak:admin"));
    }

    #[test]
    fn session_marker_key_is_non_secret_namespace() {
        assert_eq!(SESSION_ACTIVE_KEY, "session_active");
        assert_ne!(SESSION_ACTIVE_KEY, "access_token");
        assert_ne!(SESSION_ACTIVE_KEY, "refresh_token");
    }

    #[test]
    fn unsigned_id_token_nonce_decoder_is_diagnostic_only() {
        let payload = base64url_encode(br#"{"nonce":"n-123"}"#);
        let token = format!("header.{payload}.signature");
        assert_eq!(
            extract_unsigned_id_token_nonce(&token),
            Some("n-123".to_string())
        );
    }

    #[test]
    fn oauth_revoke_failure_is_not_swallowed() {
        let status = classify_oauth_revoke(Ok(TextResponse {
            status: 503,
            text: String::new(),
        }));
        assert_eq!(
            status,
            LogoutStepStatus::Failed("OAuth revoke returned HTTP 503".to_owned())
        );

        let transport = classify_oauth_revoke(Err(HttpError::message("offline")));
        assert!(matches!(transport, LogoutStepStatus::Failed(_)));
    }

    #[test]
    fn principal_logout_requires_explicit_revocation_confirmation() {
        let not_revoked = classify_principal_logout(Ok(TextResponse {
            status: 200,
            text: r#"{"ok":true,"revoked":false}"#.to_owned(),
        }));
        assert!(matches!(not_revoked, LogoutStepStatus::Failed(_)));

        let confirmed = classify_principal_logout(Ok(TextResponse {
            status: 200,
            text: r#"{"ok":true,"revoked":true}"#.to_owned(),
        }));
        assert_eq!(confirmed, LogoutStepStatus::Confirmed);
    }

    #[test]
    fn cookie_logout_requires_typed_success_confirmation() {
        let malformed = classify_cookie_logout(Ok(TextResponse {
            status: 200,
            text: "{}".to_owned(),
        }));
        assert!(matches!(malformed, LogoutStepStatus::Failed(_)));

        let rejected = classify_cookie_logout(Ok(TextResponse {
            status: 200,
            text: r#"{"status":"failed"}"#.to_owned(),
        }));
        assert!(matches!(rejected, LogoutStepStatus::Failed(_)));

        let confirmed = classify_cookie_logout(Ok(TextResponse {
            status: 200,
            text: r#"{"status":"success"}"#.to_owned(),
        }));
        assert_eq!(confirmed, LogoutStepStatus::Confirmed);
    }

    #[test]
    fn unconfirmed_cookie_keeps_marker_gate_closed_and_exposes_retry() {
        let report = LogoutReport {
            principal_logout: LogoutStepStatus::Confirmed,
            oauth_revoke: LogoutStepStatus::Confirmed,
            cookie_logout: LogoutStepStatus::Failed(
                "Browser cookie logout returned HTTP 502".to_owned(),
            ),
        };
        assert!(!report.local_cookie_cleared());
        assert!(!report.fully_confirmed());
        assert!(report.login_warning_code().is_none());
        assert!(report.retry_message().contains("Retry sign-out"));
        assert!(!super::finalize_logout_report(&report));
    }

    #[test]
    fn confirmed_cookie_allows_local_logout_but_preserves_upstream_warning() {
        let report = LogoutReport {
            principal_logout: LogoutStepStatus::Failed(
                "Station logout returned HTTP 503".to_owned(),
            ),
            oauth_revoke: LogoutStepStatus::Failed("OAuth revoke returned HTTP 503".to_owned()),
            cookie_logout: LogoutStepStatus::Confirmed,
        };
        assert!(report.local_cookie_cleared());
        assert!(!report.fully_confirmed());
        assert_eq!(
            report.login_warning_code().as_deref(),
            Some("principal_logout,oauth_revoke")
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::*;

    use super::{
        LogoutReport, LogoutStepStatus, SESSION_ACTIVE_KEY, compute_code_challenge,
        finalize_logout_report, is_authenticated,
    };

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test(async)]
    async fn compute_code_challenge_matches_rfc_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = compute_code_challenge(verifier).await.unwrap();
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[wasm_bindgen_test]
    fn marker_is_kept_until_cookie_logout_is_confirmed() {
        crate::utils::storage::set_item(SESSION_ACTIVE_KEY, "1");
        let unconfirmed = LogoutReport {
            principal_logout: LogoutStepStatus::Confirmed,
            oauth_revoke: LogoutStepStatus::Confirmed,
            cookie_logout: LogoutStepStatus::Failed(
                "Browser cookie logout returned HTTP 503".to_owned(),
            ),
        };
        finalize_logout_report(&unconfirmed);
        assert!(is_authenticated());

        let confirmed = LogoutReport {
            cookie_logout: LogoutStepStatus::Confirmed,
            ..unconfirmed
        };
        finalize_logout_report(&confirmed);
        assert!(!is_authenticated());
    }
}
