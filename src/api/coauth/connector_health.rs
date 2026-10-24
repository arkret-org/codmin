//! coauth admin connector health.

pub use coauth_admin_types::ConnectorHealthView;

use crate::api::client::{NO_BODY, api_client};
use crate::utils::net::error::HttpError;

const CONNECTOR_HEALTH_PATH: &str = "/_coauth/admin/connector-health";

pub async fn get_connector_health() -> Result<Vec<ConnectorHealthView>, HttpError> {
    let resp: coauth_admin_types::ConnectorHealthReport =
        api_client(CONNECTOR_HEALTH_PATH, "GET", NO_BODY).await?;
    Ok(resp.providers)
}
