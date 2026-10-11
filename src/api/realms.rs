use crate::api::client::{NO_BODY, api_client, build_url};
use crate::types::*;
use crate::utils::net::error::HttpError;

/// Cursor-paginated Realm list.
pub async fn list_realms(
    cursor: Option<&str>,
    limit: u64,
) -> Result<ListResponse<AdminRealmItem>, HttpError> {
    let limit_str = limit.max(1).to_string();
    let mut params: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(c) = cursor.filter(|c| !c.is_empty()) {
        params.push(("cursor", c));
    }
    let url = build_url("/_coland/admin/realms", &params)?;
    api_client(&url, "GET", NO_BODY).await
}

pub async fn get_realm(id: &str) -> Result<AdminRealmItem, HttpError> {
    let url = format!("/_coland/admin/realms/{}", urlencoding::encode(id));
    api_client(&url, "GET", NO_BODY).await
}
