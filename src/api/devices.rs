//! Device admin API — D14 production endpoint.
//!
//! `GET /_coland/admin/devices` is the typed cursor-paginated production
//! query (shared `AdminDeviceList` contract); `filter[name_or_id]` is
//! applied server-side.

use coland_contracts::admin::AdminDeviceList;

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::utils::net::error::HttpError;

pub async fn list_devices(
    cursor: Option<&str>,
    limit: u64,
    search: &str,
) -> Result<AdminDeviceList, HttpError> {
    let limit_str = limit.max(1).to_string();
    let mut params: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(cursor) = cursor.filter(|cursor| !cursor.is_empty()) {
        params.push(("cursor", cursor));
    }
    let search = search.trim();
    if !search.is_empty() {
        params.push(("filter[name_or_id]", search));
    }
    let url = build_url("/_coland/admin/devices", &params)?;
    api_client(&url, "GET", NO_BODY).await
}
