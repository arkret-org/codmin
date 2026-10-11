use crate::api::client::{NO_BODY, api_client};
use crate::types::*;
use crate::utils::net::error::HttpError;

const SERVER_INFO_PATH: &str = "/_coland/admin/server/info";
const SERVER_DESCRIBE_PATH: &str = "/_arkret/describe";
const SERVER_STATUS_PATH: &str = "/_coland/admin/server/status";
const SERVER_STATS_PATH: &str = "/_coland/admin/server/stats";

pub async fn get_server_info() -> Result<AdminServerInfo, HttpError> {
    api_client(SERVER_INFO_PATH, "GET", NO_BODY).await
}

pub async fn get_server_describe() -> Result<ServerDescribeDocument, HttpError> {
    let document: ServerDescribeDocument = api_client(SERVER_DESCRIBE_PATH, "GET", NO_BODY).await?;
    document
        .validate()
        .map_err(|error| HttpError::message(format!("invalid ServiceDescribe: {error}")))?;
    Ok(document)
}

pub async fn get_server_stats() -> Result<AdminServerStats, HttpError> {
    api_client(SERVER_STATS_PATH, "GET", NO_BODY).await
}

pub async fn get_server_status() -> Result<AdminServerStatus, HttpError> {
    api_client(SERVER_STATUS_PATH, "GET", NO_BODY).await
}

/// T8.3 — `/health` envelope deserialized for the hardening dashboard.
/// Each service exposes a `hardening` block (see `HardeningStatus`).
#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct HealthEnvelope {
    pub hardening: Option<HardeningStatus>,
}

/// T8.3 — fetch station (coland) `/health` for the hardening
/// dashboard. Returns the parsed envelope including the `hardening`
/// block; older coland builds without the field still parse cleanly
/// (Option::None).
pub async fn get_coland_health() -> Result<HealthEnvelope, HttpError> {
    api_client("/health", "GET", NO_BODY).await
}

/// T8.3 — fetch coauth `/health` against the configured upstream URL.
pub async fn get_coauth_health() -> Result<HealthEnvelope, HttpError> {
    let url = crate::utils::net::session::coauth_public_url()
        .map(|base| format!("{}/health", base.trim_end_matches('/')))
        .unwrap_or_else(|| "/health".to_string());
    api_client(&url, "GET", NO_BODY).await
}
