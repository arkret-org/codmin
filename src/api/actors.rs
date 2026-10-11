//! Actor admin API — D14 production endpoints.
//!
//! `GET /_coland/admin/actors` is the typed cursor-paginated production
//! query (shared `AdminActorList` contract); search is applied server-side
//! via `filter[search]`. Account deactivation addresses accounts by DID.

use coland_contracts::admin::{AdminActor, AdminActorList};

use crate::api::client::{NO_BODY, NoBody, api_client, build_url};
use crate::utils::net::error::HttpError;

pub async fn list_actors(
    cursor: Option<&str>,
    limit: u64,
    search: &str,
) -> Result<AdminActorList, HttpError> {
    let limit_str = limit.max(1).to_string();
    let mut params: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(cursor) = cursor.filter(|cursor| !cursor.is_empty()) {
        params.push(("cursor", cursor));
    }
    let search = search.trim();
    if !search.is_empty() {
        params.push(("filter[search]", search));
    }
    let url = build_url("/_coland/admin/actors", &params)?;
    api_client(&url, "GET", NO_BODY).await
}

pub async fn get_actor(id: &str) -> Result<AdminActor, HttpError> {
    let url = format!("/_coland/admin/actors/{}", urlencoding::encode(id));
    api_client(&url, "GET", NO_BODY).await
}

/// Deactivate an account by DID (account-lifecycle endpoints address
/// accounts by principal DID, not by the surrogate `ak:account:` row id).
pub async fn deactivate_account(did: &str) -> Result<(), HttpError> {
    let url = format!(
        "/_coland/admin/accounts/{}/deactivate",
        urlencoding::encode(did)
    );
    let body = serde_json::json!({});
    let _: NoBody = api_client(&url, "POST", Some(&body)).await?;
    Ok(())
}
