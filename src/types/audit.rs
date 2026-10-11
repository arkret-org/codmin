//! Audit admin surface — contract-authoritative types plus thin display
//! helpers.
//!
//! The row is the shared `coland_contracts::admin::AdminAuditEntry` (D14
//! production projection, mirroring the durable audit record). AKP-0008
//! attribution (`executed_by` / `authorization_ref`) travels
//! inside `payload`; the helpers below surface it for the table columns.

pub use coland_contracts::admin::AdminAuditEntry;

/// Display helpers for the shared [`AdminAuditEntry`].
pub trait AdminAuditEntryExt {
    fn payload_str(&self, key: &str) -> Option<String>;
    /// AKP-0008 — executing DID for agent-attributed envelopes.
    fn executed_by(&self) -> Option<String>;
    /// AKP-0008 — authorizing grant reference.
    fn authorization_ref(&self) -> Option<String>;
}

impl AdminAuditEntryExt for AdminAuditEntry {
    fn payload_str(&self, key: &str) -> Option<String> {
        self.payload
            .as_ref()
            .and_then(|payload| payload.get(key))
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    }

    fn executed_by(&self) -> Option<String> {
        self.payload_str("executed_by")
    }

    fn authorization_ref(&self) -> Option<String> {
        self.payload_str("authorization_ref")
    }
}
