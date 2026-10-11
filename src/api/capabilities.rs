//! Capability admin API — D14 production endpoint.
//!
//! `GET /_coland/admin/capabilities` is the typed cursor-paginated
//! production query (shared `AdminCapabilityList` contract). Revoked tombstones are
//! included by default (`filter[state]=all` server default) so operators
//! see the full grant ledger.

use coland_contracts::admin::AdminCapabilityList;

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::utils::net::error::HttpError;

pub async fn list_capabilities(
    cursor: Option<&str>,
    limit: u64,
) -> Result<AdminCapabilityList, HttpError> {
    let limit_str = limit.max(1).to_string();
    let mut params: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(cursor) = cursor.filter(|cursor| !cursor.is_empty()) {
        params.push(("cursor", cursor));
    }
    let url = build_url("/_coland/admin/capabilities", &params)?;
    api_client(&url, "GET", NO_BODY).await
}
