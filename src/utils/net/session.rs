//! Typed read-only views into the current admin session (
//! token hardening).
//!
//!
//! issued by coauth's `/oauth/token` endpoint. The bearer token is
//! not readable from JS (cookie has `HttpOnly` + `Secure` +
//! `SameSite=Strict`), so this module no longer exposes an
//! `access_token()` accessor — the API client sends the cookie
//! automatically via `credentials: "include"`. What we do still keep
//! in localStorage is a *non-secret* `session_active=1` marker so the
//! UI can decide whether to render the login page vs. the
//! authenticated layout. The marker carries no entropy and is safe to
//! read from JS.
//!
//! See `api::auth` for the write path (login / logout / refresh / OAuth
//! callback). This module is intentionally read-only and re-exports the
//! small number of cached read helpers from there so callers can
//! `use crate::utils::net::session::*` and get the full picture.

use crate::utils::storage;

/// Snapshot of the currently signed-in admin's display attributes,
/// derived from the OIDC userinfo we cache after a successful OAuth
/// callback. All fields are independently optional so partially
/// populated sessions (e.g. only `user_id` known) still render.
#[derive(Debug, Clone, Default)]
pub struct CurrentUser {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl CurrentUser {
    /// True if any of the identity fields are populated. Used by pages
    /// like `NotAuthorizedPage` that show a "Signed in as ..." block
    /// only when there's at least one piece of identity to display.
    pub fn has_identity(&self) -> bool {
        self.id.is_some() || self.display_name.is_some()
    }
}

pub fn current_user() -> CurrentUser {
    CurrentUser {
        id: storage::get_item("user_id").filter(|v| !v.is_empty()),
        display_name: storage::get_item("user_display_name").filter(|v| !v.is_empty()),
        avatar_url: storage::get_item("user_avatar_url").filter(|v| !v.is_empty()),
    }
}

/// Origin of the upstream coauth service we federate authentication
/// against. Populated by `pages::login` once `/config.json` has been
/// fetched.
pub fn coauth_public_url() -> Option<String> {
    storage::get_item("coauth_public_url")
}

/// Convenience for the sidebar / dashboard surfaces that hide the
/// "coauth" admin sections when the operator has not configured a
/// coauth public URL.
pub fn has_coauth() -> bool {
    coauth_public_url().is_some()
}

// NOTE: there is intentionally no client-side admin-scope / bridge
// gating here. The sidebar derives its visibility from the deployment
// facts above (`has_coauth()`); server-side RBAC is the only
// authorization authority (DEPLOYMENT.md).

/// OAuth client ID is public deployment configuration, not a credential.
pub fn oauth_client_id() -> String {
    storage::get_item("oauth_client_id")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| super::config::DEFAULT_OAUTH_CLIENT_ID.into())
}
