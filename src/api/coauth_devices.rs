//! HTTP client for the per-account device admin surface.
//!
//! Endpoints:
//!
//! - `GET /_coauth/admin/accounts/{account_id}/devices`
//! - `POST /_coauth/admin/accounts/{account_id}/devices/{device_id}/revoke`

use crate::api::client::{NO_BODY, api_client};
use crate::types::coauth_devices::CoauthDeviceRow;
use crate::utils::destructive_reason::destructive_reason_error;
use crate::utils::net::error::HttpError;

pub async fn list_account_devices(account_id: &str) -> Result<Vec<CoauthDeviceRow>, HttpError> {
    let path = format!(
        "/_coauth/admin/accounts/{}/devices",
        urlencoding::encode(account_id)
    );
    let resp: coauth_admin_types::DeviceListResBody = api_client(&path, "GET", NO_BODY).await?;
    Ok(resp.data)
}

pub async fn revoke_account_device(
    account_id: &str,
    device_id: &str,
    reason: &str,
) -> Result<(), HttpError> {
    let reason = reason.trim();
    if let Some(error_key) = destructive_reason_error(reason, true) {
        return Err(HttpError::message(error_key));
    }
    let url = format!(
        "/_coauth/admin/accounts/{}/devices/{}/revoke",
        urlencoding::encode(account_id),
        urlencoding::encode(device_id),
    );
    let body = coauth_admin_types::RevokeDeviceRequestBody {
        reason: reason.to_owned(),
        approval_proof: None,
    };
    let _: coauth_admin_types::DeviceRevocationStatusView =
        api_client(&url, "POST", Some(&body)).await?;
    Ok(())
}
