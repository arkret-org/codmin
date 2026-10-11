//! Actor admin surface — contract-authoritative types plus thin display
//! helpers.
//!
//! The row is the shared `coland_contracts::admin::AdminActor` (D14
//! production projection). codmin keeps no wire mirror; security-relevant
//! fields are `Option` on the wire and rendered as their answered values (the
//! old "unwired" placeholder rendering is gone with the dev snapshot).

pub use arkret_models_collaboration::objects::account_status::AccountStatus;
pub use coland_contracts::admin::AdminActor;

/// Display helpers for the shared [`AdminActor`].
pub trait AdminActorExt {
    fn is_deactivated(&self) -> bool;
    /// Wire status string (`active` / `suspended` / ...), `-` when the
    /// server reported the lifecycle as unknown.
    fn status_display(&self) -> &'static str;
}

impl AdminActorExt for AdminActor {
    fn is_deactivated(&self) -> bool {
        matches!(
            self.status,
            Some(AccountStatus::Deactivated | AccountStatus::ErasurePending)
        )
    }

    fn status_display(&self) -> &'static str {
        self.status.map(AccountStatus::as_str).unwrap_or("-")
    }
}
