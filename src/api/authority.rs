//! HTTP client for the Realm authority-commit admin surface.
//!
//! An administrator inspecting a Realm needs the same three facts a joining
//! client needs: which Station governs the Realm right now, how that authority
//! was reached from genesis, and where the Realm commit stream head sits. All
//! three arrive in one nonce-bound `RealmAuthorityBundle`
//! (`sync/authority-commit-log.md` sections 7 and 8); codmin renders it and never
//! re-derives authority locally.

use arkret_wire::{
    Base64UrlString, RealmAuthorityBundle, RealmAuthorityBundleRequestBody, RealmId,
};

use crate::api::client::api_client;
use crate::utils::net::error::HttpError;
use crate::utils::security::crypto::random_token;

const AUTHORITY_BUNDLE_PATH: &str = "/_arkret/open/realm-authority/bundle";

/// Fetch the current authority bundle for one Realm.
///
/// The nonce is generated per request. A Station that replays a previously
/// signed assertion cannot satisfy it, so a stale current-authority answer is
/// detectable rather than silently cached.
pub async fn get_realm_authority_bundle(realm_id: &str) -> Result<RealmAuthorityBundle, HttpError> {
    let realm_id = RealmId::new(realm_id.to_owned())
        .map_err(|error| HttpError::message(format!("invalid Realm id: {error}")))?;
    let nonce = Base64UrlString::new(random_token(24))
        .map_err(|error| HttpError::message(format!("could not build a bundle nonce: {error}")))?;
    let request = RealmAuthorityBundleRequestBody { realm_id, nonce };
    request.validate().map_err(|error| {
        HttpError::message(format!("invalid authority bundle request: {error}"))
    })?;
    api_client(AUTHORITY_BUNDLE_PATH, "POST", Some(&request)).await
}
