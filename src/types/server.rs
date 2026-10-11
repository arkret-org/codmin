//! Server status / info / stats / describe admin surfaces.
//!
//! `/_coland/admin/server/{info,stats,status}` are deployment-local operator
//! endpoints whose shapes live in `coland_contracts::admin`; codmin keeps no
//! wire mirror of them.
//!
//! The `/_arkret/describe` payload is the SDK-authoritative
//! [`ServiceDescribe`] (`arkret_models_discovery`). codmin does not mirror it
//! either; service-proprietary top-level extensions are read from the SDK
//! model's flattened `extensions` map.

// (The `VerifiedProfileEntry` /
// `InteropSurfaceEntry` element types are reachable through the SDK
// directly; codmin views consume them via the `ServiceDescribe`
// fields and need no local re-export.)
pub use arkret_models_discovery::ServiceDescribe;
pub use coland_contracts::admin::{AdminServerInfo, AdminServerStats, AdminServerStatus};
use serde::{Deserialize, Serialize};

/// `/_arkret/describe` response envelope.
///
/// The protocol-authoritative fields deserialize into the SDK
/// [`ServiceDescribe`] (strict: `service_id: ServiceId`, an exact
/// `service_resolution` commitment, `trust_domain: TrustDomainId`,
/// `development_mode: bool`, …).
/// Any additional top-level keys a service emits beyond the spec shape
/// land in [`ServiceDescribe::extensions`]; views read them via
/// [`Self::extra_str`] so the wire type itself never grows hand-written
/// mirrors of upstream extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerDescribeDocument {
    pub description: ServiceDescribe,
}

impl std::ops::Deref for ServerDescribeDocument {
    type Target = ServiceDescribe;

    fn deref(&self) -> &ServiceDescribe {
        &self.description
    }
}

impl ServerDescribeDocument {
    /// Enforce the SDK's semantic checks, including DID-to-core-id
    /// consistency for the published service resolution commitment.
    pub fn validate(&self) -> arkret_wire::Result<()> {
        self.description.validate()
    }

    /// Walk `path` inside the extension envelope.
    pub fn extra_value<'a>(&'a self, path: &[&str]) -> Option<&'a serde_json::Value> {
        let mut current = path
            .first()
            .and_then(|key| self.description.extensions.get(*key))?;
        for key in &path[1..] {
            current = current.get(*key)?;
        }
        Some(current)
    }

    /// Walk `path` inside the extension envelope and return the string
    /// leaf, if any.
    pub fn extra_str(&self, path: &[&str]) -> Option<String> {
        self.extra_value(path)?.as_str().map(ToOwned::to_owned)
    }

    /// When `development_mode == true` and `verified_profiles` is
    /// non-empty the server is making contradictory claims (relaxed
    /// proof verifier breaks the verification chain). The UI surfaces a
    /// red warning banner in this case.
    pub fn dev_mode_with_verified_profiles(&self) -> bool {
        self.description.development_mode && !self.description.verified_profiles.is_empty()
    }

    /// Render the canonical `plaintext_visibility` advertisement as a list
    /// of the declared plaintext data-class names for display. An empty
    /// list means the service claims no plaintext / reversible-derived
    /// classes (the E2EE default).
    pub fn plaintext_visibility_entries(&self) -> Vec<String> {
        self.description
            .plaintext_visibility
            .data_classes
            .iter()
            .filter_map(|class| {
                serde_json::to_value(class)
                    .ok()
                    .and_then(|v| v.as_str().map(ToOwned::to_owned))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AdminServerStatus, ServerDescribeDocument};

    /// Minimal payload satisfying every required `ServiceDescribe` field.
    fn base_describe() -> serde_json::Value {
        json!({
            "service_id": "ak:did_core:web:coland.local",
            "service_resolution": {
                "did": "did:web:coland.local",
                "method_history_head": "fixture-head",
                "version_id": "fixture-v1"
            },
            "trust_domain": "ak:trust_domain:coland.local",
            "service_kind": "station",
            "protocol_version": "1.0",
            "supported_profiles": [],
            "supported_operation_bundles": ["ak.operation_bundle.station.describe.v1"],
            "transport_bindings": [{
                "kind": "http_json",
                "base_url": "https://coland.local/",
                "extension_profile_required": null
            }],
            "supported_features": [],
            "auth_metadata": {},
            "limits": {
                "profile_status": {
                    "conformance": "limited_reference",
                    "implemented_surfaces": ["station", "events_api_minimal"]
                }
            },
            "rate_limit_policy": { "policy_version": "1", "entries": [] },
            "plaintext_visibility": { "max_visibility": "private_plaintext", "data_classes": ["message_content"] },
            "verified_profiles": [],
            "interop_surfaces": [{
                "name": "federation.bridge",
                "kind": "external_interop"
            }],
            "development_mode": false,
        })
    }

    #[test]
    fn describe_document_parses_sdk_authoritative_shape() {
        let describe: ServerDescribeDocument =
            serde_json::from_value(base_describe()).expect("coland describe should deserialize");

        assert_eq!(describe.service_id.as_str(), "ak:did_core:web:coland.local");
        describe
            .validate()
            .expect("describe semantics should validate");
        assert_eq!(
            describe.trust_domain.as_str(),
            "ak:trust_domain:coland.local"
        );
        assert_eq!(describe.service_kind, arkret_wire::ServiceKind::Station);
        assert!(!describe.development_mode);
        assert_eq!(describe.interop_surfaces[0].name, "federation.bridge");
        assert_eq!(
            describe.limits.extensions["profile_status"]["conformance"],
            "limited_reference"
        );
        assert_eq!(
            describe.plaintext_visibility_entries(),
            vec!["message_content".to_string()]
        );
        assert!(!describe.dev_mode_with_verified_profiles());
    }

    #[test]
    fn describe_document_captures_service_extensions_in_extra() {
        let mut payload = base_describe();
        let object = payload.as_object_mut().unwrap();
        // coauth-proprietary top-level extension fields.
        object.insert(
            "x_coauth_identity_registry_resolver".to_owned(),
            json!({
                "mode": "delegated_resolver",
                "endpoint": "https://auth.example.com/api/v1/identity/resolve",
                "delegated_resolver": {
                    "kind": "public_did_resolver",
                    "resolver": "https://resolver.example.com/"
                }
            }),
        );
        object.insert(
            "x_coauth_admin_audience".to_owned(),
            json!("urn:coauth:admin"),
        );

        let describe: ServerDescribeDocument =
            serde_json::from_value(payload).expect("extended describe should deserialize");

        assert_eq!(
            describe.extra_str(&[
                "x_coauth_identity_registry_resolver",
                "delegated_resolver",
                "resolver"
            ]),
            Some("https://resolver.example.com/".to_string())
        );
        assert_eq!(
            describe.extra_str(&["x_coauth_admin_audience"]),
            Some("urn:coauth:admin".to_string())
        );
        assert!(!describe.description.extensions.contains_key("service_id"));
    }

    #[test]
    fn server_status_parses_the_producer_shape() {
        let status: AdminServerStatus = serde_json::from_value(json!({
            "status": "ok",
            "service_id": "ak:did_core:web:coland.local",
            "storage": "postgres",
            "development_mode": true,
            "checked_by": "ak:did_core:web:alice.example",
            "generated_at": "2026-08-14T00:00:00.000Z",
            "counts": { "accounts": 4, "devices": null, "realms": 2 },
        }))
        .expect("coland status should deserialize");

        assert!(status.is_ok());
        assert_eq!(status.counts.devices, None);
    }
}
