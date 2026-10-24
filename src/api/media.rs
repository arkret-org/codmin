use crate::api::client::{NO_BODY, api_client, build_url};
use crate::types::*;
use crate::utils::net::error::HttpError;

const FILTERED_MEDIA_FETCH_LIMIT: u64 = 1000;

pub async fn list_media(
    page: u64,
    per_page: u64,
    search: &str,
) -> Result<ListResponse<AdminMediaRow>, HttpError> {
    let needle = search.trim().to_ascii_lowercase();
    if !needle.is_empty() {
        return list_filtered_media(page, per_page.max(1), &needle).await;
    }
    let limit = per_page.max(1);
    let cursor = page.saturating_sub(1).saturating_mul(limit).to_string();
    fetch_media_page(Some(&cursor), limit).await
}

async fn fetch_media_page(
    cursor: Option<&str>,
    limit: u64,
) -> Result<ListResponse<AdminMediaRow>, HttpError> {
    let limit_str = limit.to_string();
    let mut parameters: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(cursor) = cursor.filter(|c| !c.is_empty()) {
        parameters.push(("cursor", cursor));
    }
    let url = build_url("/_coland/admin/media", &parameters)?;
    api_client(&url, "GET", NO_BODY).await
}

async fn list_filtered_media(
    page: u64,
    per_page: u64,
    needle: &str,
) -> Result<ListResponse<AdminMediaRow>, HttpError> {
    let mut cursor = None::<String>;
    let mut matched = Vec::new();

    loop {
        let resp = fetch_media_page(cursor.as_deref(), FILTERED_MEDIA_FETCH_LIMIT).await?;
        matched.extend(
            resp.data
                .into_iter()
                .filter(|row| media_row_matches(row, needle)),
        );
        match resp.next_cursor {
            Some(next) if !next.is_empty() && cursor.as_deref() != Some(next.as_str()) => {
                cursor = Some(next);
            }
            _ => break,
        }
    }

    let total = matched.len() as u64;
    let start = page.saturating_sub(1).saturating_mul(per_page) as usize;
    let data: Vec<AdminMediaRow> = matched
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();
    let next_cursor = if (start as u64).saturating_add(data.len() as u64) < total {
        Some(page.saturating_add(1).to_string())
    } else {
        None
    };
    Ok(ListResponse {
        data,
        total: Some(total),
        next_cursor,
    })
}

fn media_row_matches(row: &AdminMediaRow, needle: &str) -> bool {
    [
        Some(row.sha256.as_str()),
        row.filename.as_deref(),
        Some(row.media_type.as_str()),
        row.realm_id.as_deref(),
        Some(row.uploaded_by.as_str()),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.to_ascii_lowercase().contains(needle))
}

pub async fn get_media_statistics() -> Result<AdminMediaStatistics, HttpError> {
    api_client("/_coland/admin/media/statistics", "GET", NO_BODY).await
}

pub async fn list_media_by_actor() -> Result<AdminMediaByActorList, HttpError> {
    api_client("/_coland/admin/media/by-actor", "GET", NO_BODY).await
}
