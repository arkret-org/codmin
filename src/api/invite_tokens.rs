use crate::api::client::{NO_BODY, api_client, build_url};
use crate::types::*;
use crate::utils::net::error::HttpError;

pub async fn list_invite_tokens(
    page: u64,
    per_page: u64,
) -> Result<ListResponse<AdminInviteTokenItem>, HttpError> {
    let limit = per_page.max(1);
    let cursor = page.saturating_sub(1).saturating_mul(limit).to_string();
    let url = build_url(
        "/_coland/admin/invite-tokens",
        &[("limit", &limit.to_string()), ("cursor", &cursor)],
    )?;
    api_client(&url, "GET", NO_BODY).await
}
