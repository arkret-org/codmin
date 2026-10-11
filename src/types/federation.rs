//! Federation admin surface — contract-authoritative row plus thin display
//! helpers.
//!
//! `GET /_coland/admin/federation` returns a cursor-paginated stream of
//! **federation operations** (not peer-health rows). The row is the shared
//! `coland_contracts::admin::AdminFederationOperation`, produced by coland's
//! `admin_federation_items`
//! (`coland/crates/http/src/routing/admin/collection.rs`).

pub use coland_contracts::admin::AdminFederationOperation;

/// Display helpers for the shared [`AdminFederationOperation`].
pub trait AdminFederationOperationExt {
    /// Wire string for the operation kind (`create` / `update` / ...).
    fn operation_kind_label(&self) -> String;
    /// Canonical Event kind wire string.
    fn canonical_kind_label(&self) -> String;
    fn created_at_display(&self) -> String;
}

impl AdminFederationOperationExt for AdminFederationOperation {
    fn operation_kind_label(&self) -> String {
        serde_json::to_value(&self.operation_kind)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| "-".to_owned())
    }

    fn canonical_kind_label(&self) -> String {
        self.canonical_kind.as_str().to_owned()
    }

    fn created_at_display(&self) -> String {
        self.created_at.to_rfc3339()
    }
}
