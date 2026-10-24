//! Audit admin API — D14 production endpoint.
//!
//! `GET /_coland/admin/audit` is the typed production query (shared
//! `AdminAuditList` contract, newest first). All filtering happens server-side via
//! `filter[...]` / `since` / `until`; the old fetch-everything-then-filter
//! client loop is gone. `target_type` / `target_id` / `source_ip` /
//! `effective_scope` are not part of the durable audit record and are no
//! longer query dimensions.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use coland_contracts::admin::AdminAuditList;

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::utils::net::error::HttpError;

/// Server-side filter set for `/_coland/admin/audit`. Empty fields are
/// dropped before encoding so the wire form only carries what the operator
/// actually filtered on.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub action: Option<String>,
    pub actor_id: Option<String>,
    pub realm_id: Option<String>,
    /// AKP-0007 event-kind filter (`ak.circle.create`, ...);
    /// matches the action or the payload `kind`/`type` server-side.
    pub kind: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

impl AuditFilter {
    fn into_query(self) -> Vec<(&'static str, String)> {
        let field = |value: Option<String>| {
            value
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let mut query = Vec::new();
        for (key, value) in [
            ("filter[action]", field(self.action)),
            ("filter[actor_id]", field(self.actor_id)),
            ("filter[realm_id]", field(self.realm_id)),
            ("filter[kind]", field(self.kind)),
        ] {
            if let Some(value) = value {
                query.push((key, value));
            }
        }
        for (key, value) in [("since", field(self.since)), ("until", field(self.until))] {
            // The endpoint takes RFC3339 only; normalize the
            // datetime-local input forms the filter UI produces.
            if let Some(value) = value.as_deref().and_then(parse_audit_time) {
                query.push((key, value.to_rfc3339()));
            }
        }
        query
    }
}

pub async fn list_audit_entries(
    cursor: Option<&str>,
    limit: u64,
    filter: AuditFilter,
) -> Result<AdminAuditList, HttpError> {
    let limit_str = limit.max(1).to_string();
    let query = filter.into_query();
    let mut parameters: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(cursor) = cursor.filter(|cursor| !cursor.is_empty()) {
        parameters.push(("cursor", cursor));
    }
    for (key, value) in &query {
        parameters.push((key, value.as_str()));
    }
    let url = build_url("/_coland/admin/audit", &parameters)?;
    api_client(&url, "GET", NO_BODY).await
}

/// Parse the filter UI's time inputs: RFC3339, or the two
/// `datetime-local` shapes (`%Y-%m-%dT%H:%M[:%S]`, treated as UTC).
fn parse_audit_time(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|dt| Utc.from_utc_datetime(&dt))
        })
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
                .ok()
                .map(|dt| Utc.from_utc_datetime(&dt))
        })
}

#[cfg(test)]
mod tests {
    use arkret_wire::event_kind_str;

    use super::*;

    #[test]
    fn filter_normalizes_datetime_local_bounds_to_rfc3339() {
        let query = AuditFilter {
            since: Some("2026-06-21T10:00".to_owned()),
            until: Some("2026-06-21T11:00:30".to_owned()),
            ..Default::default()
        }
        .into_query();
        assert_eq!(
            query,
            vec![
                ("since", "2026-06-21T10:00:00+00:00".to_owned()),
                ("until", "2026-06-21T11:00:30+00:00".to_owned()),
            ]
        );
    }

    #[test]
    fn filter_drops_empty_fields_and_unparsable_times() {
        let query = AuditFilter {
            action: Some("  ".to_owned()),
            kind: Some(event_kind_str::CIRCLE_CREATE.to_owned()),
            since: Some("yesterday".to_owned()),
            ..Default::default()
        }
        .into_query();
        assert_eq!(
            query,
            vec![("filter[kind]", event_kind_str::CIRCLE_CREATE.to_owned())]
        );
    }
}
