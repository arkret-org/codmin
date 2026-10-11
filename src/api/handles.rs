//! HTTP client for the coland Handle admin surface (T6.2 §2).
//!
//! Endpoints:
//!
//! - `GET /_coland/admin/handles` — paginated list of `ak.handle.*` cells visible to the current
//!   admin scope.
//! - `GET /_coland/admin/handles/{id}` — single handle row.
//! - `GET /_coland/admin/handles/{id}/audit` — handle audit trail from the T3.2 audit table.
//! - `POST /_coland/admin/handles/{id}/revoke` — update issuer-local lifecycle state.
//! - `POST /_coland/admin/handles/{id}/reassign` — re-bind the issuer-local name to a stable
//!   subject ID.

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::types::*;
use crate::utils::destructive_reason::destructive_reason_error;
use crate::utils::net::error::HttpError;

pub async fn list_handles(
    page: u64,
    per_page: u64,
    search: &str,
) -> Result<ListResponse<AdminHandleRecord>, HttpError> {
    let url = build_url(
        "/_coland/admin/handles",
        &[
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
            ("search", search),
        ],
    )?;
    api_client(&url, "GET", NO_BODY).await
}

pub async fn get_handle(id: &str) -> Result<AdminHandleRecord, HttpError> {
    let url = format!("/_coland/admin/handles/{}", urlencoding::encode(id));
    api_client(&url, "GET", NO_BODY).await
}

pub async fn get_handle_audit(id: &str) -> Result<ListResponse<AdminHandleAuditEvent>, HttpError> {
    let url = format!("/_coland/admin/handles/{}/audit", urlencoding::encode(id));
    api_client(&url, "GET", NO_BODY).await
}

pub async fn revoke_handle(id: &str) -> Result<(), HttpError> {
    let url = format!("/_coland/admin/handles/{}/revoke", urlencoding::encode(id));
    api_client(&url, "POST", NO_BODY).await
}

pub async fn reassign_handle(
    id: &str,
    req: &AdminHandleReassignBody,
) -> Result<AdminHandleRecord, HttpError> {
    if let Some(error_key) = destructive_reason_error(&req.reason, true) {
        return Err(HttpError::message(error_key));
    }
    let url = format!(
        "/_coland/admin/handles/{}/reassign",
        urlencoding::encode(id)
    );
    api_client(&url, "POST", Some(req)).await
}
