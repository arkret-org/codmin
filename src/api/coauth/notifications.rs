//! coauth admin notification channels and templates.

pub use coauth_admin_types::{NotificationChannelStatus, NotificationTemplateEntry};

use crate::api::client::{NO_BODY, api_client};
use crate::utils::net::error::HttpError;

const NOTIFICATION_CHANNELS_PATH: &str = "/_coauth/admin/notification-channels";
const NOTIFICATION_TEMPLATES_PATH: &str = "/_coauth/admin/notification-templates";

pub async fn list_notification_channels() -> Result<Vec<NotificationChannelStatus>, HttpError> {
    let resp: coauth_admin_types::NotificationChannelsOutcome =
        api_client(NOTIFICATION_CHANNELS_PATH, "GET", NO_BODY).await?;
    Ok(resp.channels)
}

/// Returns the static catalog of known notification template keys. The
/// backend endpoint is non-paginated — it returns the full known-keys
/// list every call — so the codmin UI now flat-lists the entries
/// instead of pretending there's a paging cursor.
pub async fn list_notification_templates() -> Result<Vec<NotificationTemplateEntry>, HttpError> {
    let resp: coauth_admin_types::NotificationTemplatesOutcome =
        api_client(NOTIFICATION_TEMPLATES_PATH, "GET", NO_BODY).await?;
    Ok(resp.templates)
}
