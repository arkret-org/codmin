//! Device admin surface — contract-authoritative types plus thin display
//! helpers.
//!
//! The row is the shared `coland_contracts::admin::AdminDevice` (D14
//! production projection); codmin keeps no wire mirror or field aliases.

pub use coland_contracts::admin::AdminDevice;

/// Display helpers for the shared [`AdminDevice`].
pub trait AdminDeviceExt {
    fn verification_label(&self) -> Option<&str>;
    /// RFC3339 rendering for the optional timestamps (`-` handled by the
    /// caller).
    fn created_at_display(&self) -> Option<String>;
    fn updated_at_display(&self) -> Option<String>;
    fn revoked_at_display(&self) -> Option<String>;
}

impl AdminDeviceExt for AdminDevice {
    fn verification_label(&self) -> Option<&str> {
        self.verification_state.as_deref()
    }

    fn created_at_display(&self) -> Option<String> {
        self.created_at.map(|ts| ts.to_rfc3339())
    }

    fn updated_at_display(&self) -> Option<String> {
        self.updated_at.map(|ts| ts.to_rfc3339())
    }

    fn revoked_at_display(&self) -> Option<String> {
        self.revoked_at.map(|ts| ts.to_rfc3339())
    }
}
