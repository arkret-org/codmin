//! coauth admin API client surface.
//!
//! Wire shapes that already live in the shared `coauth-admin-types`
//! crate are imported / re-exported from there. Anything still defined
//! inline here is on the migration list — when the corresponding coauth
//! admin handler graduates to a typed response, lift the struct into
//! `coauth-admin-types` and turn the local copy into a re-export.
//!
//! This module is the public facade; domain submodules remain private and
//! their public items are re-exported here as `crate::api::coauth::X`.

mod accounts;
mod audit_feed;
mod connector_health;
mod notifications;
mod pagination;
mod registration;
mod sessions;
mod upstream;
mod viewer;

pub use accounts::{
    AccountRiskActionExecuteRequestBody, AccountRiskActionProposalRequestBody,
    execute_account_risk_action, get_account_detail, list_accounts_cursor,
    remove_account_did_binding, revoke_account_claim, submit_account_risk_action,
};
pub use audit_feed::{AuditFeedFilter, list_audit_feed};
pub use connector_health::get_connector_health;
pub use notifications::{list_notification_channels, list_notification_templates};
pub use registration::{
    create_registration_token, list_registration_tokens, revoke_registration_token,
};
pub use sessions::{
    create_personal_session, finish_oauth2_session, list_oauth2_sessions, list_personal_sessions,
    regenerate_personal_session, revoke_personal_session,
};
pub use upstream::{
    CreateUpstreamProviderRequest, create_upstream_provider, delete_upstream_link,
    delete_upstream_provider, list_upstream_links, list_upstream_providers,
    toggle_upstream_provider,
};
pub use viewer::get_viewer;
