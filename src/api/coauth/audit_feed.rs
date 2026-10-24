//! coauth admin audit feed.

use coauth_admin_types::{AuditEntry, AuditFeedOutcome};

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::utils::net::error::HttpError;

const AUDIT_FEED_PATH: &str = "/_coauth/admin/audit-feed";

/// Filters supported by `/_coauth/admin/audit-feed`. Empty fields are dropped
/// before encoding.
#[derive(Debug, Clone, Default)]
pub struct AuditFeedFilter {
    pub actor_user_id: Option<String>,
    pub target_type: Option<String>,
}

impl AuditFeedFilter {
    fn into_query(self) -> Vec<(&'static str, String)> {
        [
            ("admin_user_id", self.actor_user_id),
            ("resource_type", self.target_type),
        ]
        .into_iter()
        .filter_map(|(k, v)| {
            v.map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(|s| (k, s))
        })
        .collect()
    }
}

pub async fn list_audit_feed(
    limit: u64,
    filter: AuditFeedFilter,
) -> Result<Vec<AuditEntry>, HttpError> {
    let limit_str = limit.max(1).to_string();
    let mut parameters: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    let owned = filter.into_query();
    for (k, v) in owned.iter() {
        parameters.push((k, v.as_str()));
    }
    let url = build_url(AUDIT_FEED_PATH, &parameters)?;
    let resp: AuditFeedOutcome = api_client(&url, "GET", NO_BODY).await?;
    Ok(resp.data)
}

#[cfg(test)]
mod tests {
    use super::AuditFeedFilter;

    #[test]
    fn target_type_uses_backend_query_contract_name() {
        let query = AuditFeedFilter {
            target_type: Some("account".to_owned()),
            ..Default::default()
        }
        .into_query();

        assert_eq!(query, vec![("resource_type", "account".to_owned())]);
    }
}
