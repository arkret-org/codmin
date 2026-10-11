//! HTTP client for the coland Realm link-graph admin surface
//! (the `ak.realm.link` projection).
//!
//! A "Realm link" is a typed edge between two security boundaries.
//! Examples: `governed_by` (parent for capability inheritance),
//! `discoverable_from` (cross-realm directory hop), `mirror_of`
//! (federated mirror of another Realm).
//!
//! The admin surface returns the outbound and/or inbound edge list
//! for a single Realm so the operator can see who governs / mirrors /
//! discovers them and vice-versa. The page renders both typed rows and
//! their graph layout.

use crate::api::client::{NO_BODY, api_client};
use crate::types::{RealmLinkDirection, RealmLinkList};
use crate::utils::net::error::HttpError;

fn direction_query(direction: RealmLinkDirection) -> String {
    serde_json::to_value(direction)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "both".to_owned())
}

pub async fn list_realm_links(
    realm_id: &str,
    direction: RealmLinkDirection,
) -> Result<RealmLinkList, HttpError> {
    let url = format!(
        "/_coland/admin/realms/{}/links?direction={}",
        urlencoding::encode(realm_id),
        direction_query(direction),
    );
    api_client(&url, "GET", NO_BODY).await
}
