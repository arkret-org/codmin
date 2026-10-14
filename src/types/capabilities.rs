//! Display-only helpers for SDK-owned accepted grant current rows.

pub use coland_contracts::admin::CapabilitySummary;

pub trait CapabilitySummaryExt {
    fn actions_display(&self) -> String;
    fn resource_display(&self) -> String;
    fn issuer_display(&self) -> String;
    fn subject_display(&self) -> String;
    fn expiry_display(&self) -> String;
}

impl CapabilitySummaryExt for CapabilitySummary {
    fn actions_display(&self) -> String {
        self.value.actions.join(", ")
    }
    fn resource_display(&self) -> String {
        arkret_canonical::canonical_json_string(&self.value.resources)
            .unwrap_or_else(|_| "unavailable".to_owned())
    }
    fn issuer_display(&self) -> String {
        arkret_canonical::canonical_json_string(&self.value.issuer_id)
            .unwrap_or_else(|_| "unavailable".to_owned())
    }
    fn subject_display(&self) -> String {
        arkret_canonical::canonical_json_string(&self.value.subject)
            .unwrap_or_else(|_| "unavailable".to_owned())
    }
    fn expiry_display(&self) -> String {
        self.value.constraints.iter()
            .filter(|constraint| constraint.constraint_kind == arkret_models_collaboration::governance::grant_constraint::GrantConstraintKind::Temporal)
            .filter_map(|constraint| constraint.expires_at).min()
            .map(|at| at.to_rfc3339()).unwrap_or_else(|| "-".to_owned())
    }
}
