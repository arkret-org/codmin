//! Read-only Coland service-route operations client.

use crate::api::client::{NO_BODY, api_client};
use crate::types::{AdminServiceRouteDetail, AdminServiceRouteList};
use crate::utils::net::error::HttpError;

pub async fn list_service_routes(
    cursor: Option<&str>,
    limit: usize,
) -> Result<AdminServiceRouteList, HttpError> {
    let mut url = format!("/_coland/admin/service-routes?limit={limit}");
    if let Some(cursor) = cursor {
        url.push_str("&cursor=");
        url.push_str(&urlencoding::encode(cursor));
    }
    api_client(&url, "GET", NO_BODY).await
}

pub async fn get_service_route(
    service_id: &str,
    service_kind: &str,
) -> Result<AdminServiceRouteDetail, HttpError> {
    let url = format!(
        "/_coland/admin/service-routes/{}/{}",
        urlencoding::encode(service_id),
        urlencoding::encode(service_kind)
    );
    api_client(&url, "GET", NO_BODY).await
}
