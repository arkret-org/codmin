use crate::api::client::{NO_BODY, api_client, build_url};
use crate::types::*;
use crate::utils::net::error::HttpError;

/// Cursor-paginated federation operation stream from
/// `GET /_coland/admin/federation`. The backend returns
/// `federation_operation` rows; `search` is a best-effort client-side filter
/// over `operation_id` / `realm_id` (the endpoint does not yet support a
/// server-side `filter` parameter).
pub async fn list_federation_operations(
    cursor: Option<&str>,
    limit: u64,
    search: &str,
) -> Result<ListResponse<AdminFederationOperation>, HttpError> {
    let limit_str = limit.max(1).to_string();
    let mut parameters: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(c) = cursor.filter(|c| !c.is_empty()) {
        parameters.push(("cursor", c));
    }
    let url = build_url("/_coland/admin/federation", &parameters)?;
    let mut resp: ListResponse<AdminFederationOperation> = api_client(&url, "GET", NO_BODY).await?;
    let needle = search.trim().to_ascii_lowercase();
    let data = resp
        .data
        .drain(..)
        .filter(|op| {
            needle.is_empty()
                || op
                    .operation_id
                    .as_str()
                    .to_ascii_lowercase()
                    .contains(&needle)
                || op.realm_id.as_str().to_ascii_lowercase().contains(&needle)
        })
        .collect::<Vec<_>>();
    Ok(ListResponse {
        data,
        total: resp.total,
        next_cursor: resp.next_cursor,
    })
}

/// Looks up a single federation operation by `operation_id`. There is no
/// per-operation detail endpoint, so this walks the paginated listing until
/// it finds the match (bounded by `MAX_PAGES`).
pub async fn get_federation_operation(
    operation_id: &str,
) -> Result<AdminFederationOperation, HttpError> {
    const PAGE_SIZE: u64 = 100;
    const MAX_PAGES: usize = 50;

    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let page = list_federation_operations(cursor.as_deref(), PAGE_SIZE, operation_id).await?;
        if let Some(op) = page
            .data
            .into_iter()
            .find(|op| op.operation_id.as_str() == operation_id)
        {
            return Ok(op);
        }
        cursor = page.next_cursor;
        if cursor.as_deref().is_none_or(str::is_empty) {
            break;
        }
    }
    Err(HttpError::message(
        "federation operation detail is not present in the paginated listing",
    ))
}
