//! Capability grant admin surface — contract-authoritative types plus thin
//! display helpers.
//!
//! The row is the shared `coland_contracts::admin::CapabilitySummary` (D14
//! production projection of the authz read-index grant). codmin adds no
//! wire mirror; display-only conveniences live in [`CapabilitySummaryExt`].

pub use coland_contracts::admin::CapabilitySummary;

/// Display helpers for the shared [`CapabilitySummary`].
pub trait CapabilitySummaryExt {
    fn actions_display(&self) -> String;
    fn resource_display(&self) -> String;
}

impl CapabilitySummaryExt for CapabilitySummary {
    fn actions_display(&self) -> String {
        if self.actions.is_empty() {
            "-".to_string()
        } else {
            self.actions.join(", ")
        }
    }

    fn resource_display(&self) -> String {
        self.resource.clone().unwrap_or_else(|| "-".to_string())
    }
}

#[cfg(test)]
mod tests {
    use arkret_wire::CapabilityActionId;

    use super::{CapabilitySummary, CapabilitySummaryExt};

    #[test]
    fn summary_parses_production_wire_shape() {
        let summary: CapabilitySummary = serde_json::from_value(serde_json::json!({
            "grant_id": "ak:grant:AbhvODyrIRCskAIoS9IXLjMfD-Zsr8lwDpiCU_zLR4it",
            "realm_id": "ak:realm:Adzr6hWdpvvoBoHZ2PftHKATea_QnznMIDwV5k6POjOi",
            "issuer_id": "ak:did_core:web:issuer.example",
            "subject_id": "ak:did_core:web:subject.example",
            "resource": "realm",
            "actions": [CapabilityActionId::MESSAGE_CREATE],
            "revoked": false,
            "created_at": "2026-06-07T00:00:00.000Z"
        }))
        .expect("production-shaped grant summary should deserialize");

        assert_eq!(summary.issuer_id.as_str(), "ak:did_core:web:issuer.example");
        assert_eq!(
            summary.actions_display(),
            CapabilityActionId::MESSAGE_CREATE
        );
        assert_eq!(summary.resource_display(), "realm");
        assert!(!summary.revoked);
    }
}
