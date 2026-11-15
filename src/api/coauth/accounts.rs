//! coauth admin accounts: summary/detail, managed DID bindings, claims,
//! session grants and the risk-action lifecycle.

use arkret_identifiers::DidCoreId;
use coauth_admin_types::{
    AdminAccountAttributes, AdminAccountClaimsOutcome, AdminAccountDidBinding,
    AdminAccountDidBindingsOutcome, SingleOutcome, SingleResource,
};
use serde::{Deserialize, Serialize};

use super::pagination::get_jsonapi_cursor_page_with_query;
use crate::api::client::{NO_BODY, NoBody, api_client};
use crate::types::CursorPage;
use crate::utils::net::error::HttpError;

const ACCOUNTS_PATH: &str = "/_coauth/admin/accounts";
const BRIDGE_DESCRIBE_PATH: &str = "/_coauth/admin/bridge/describe";
const COAUTH_INTEGRATION_DESCRIBE_PATH: &str = "/_coauth/account/integration/describe";

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CoauthAccountSummary {
    pub id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub primary_principal_id: Option<DidCoreId>,
    pub is_locked: bool,
    pub is_deactivated: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl CoauthAccountSummary {
    pub fn lifecycle_label(&self) -> &'static str {
        if self.is_deactivated {
            "Deactivated"
        } else if self.is_locked {
            "Locked"
        } else {
            "Active"
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CoauthManagedDidBinding {
    pub did: String,
    pub kind: DidBindingKind,
    pub state: DidBindingState,
    pub verification_status: DidBindingVerificationStatus,
    pub primary: bool,
    pub last_verified_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CoauthAccountClaim {
    pub id: String,
    pub claim_kind: String,
    pub value: Option<String>,
    pub state: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CoauthRiskActionHook {
    pub endpoint: String,
    pub approval_mode: String,
}

pub use coauth_admin_types::{
    AdminBridgeDescribe, DidBindingKind, DidBindingState, DidBindingVerificationStatus,
    IntegrationManifest,
};

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CoauthAccountDetail {
    pub account: CoauthAccountSummary,
    pub managed_dids: Vec<CoauthManagedDidBinding>,
    pub claims: Vec<CoauthAccountClaim>,
    pub risk_action_current: AccountRiskActionCurrentOutcome,
    pub risk_action_history: Vec<AccountRiskActionTransitionRecord>,
    pub risk_action_hook: CoauthRiskActionHook,
    pub admin_bridge: AdminBridgeDescribe,
    pub integration_manifest: IntegrationManifest,
}

pub use coauth_admin_types::{
    AccountRiskActionCurrentOutcome, AccountRiskActionExecuteRequestBody,
    AccountRiskActionHistoryOutcome, AccountRiskActionProposalOutcome,
    AccountRiskActionProposalRequestBody, AccountRiskActionTransitionRecord,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CoauthAccountRiskActionExecute {
    pub state_record_id: String,
    pub proposal_id: String,
    pub account_id: String,
    pub action: String,
    pub ticket: Option<String>,
    pub previous_state: String,
    pub execution_state: String,
    pub mutation_kind: String,
    pub state_revision: u64,
    pub transition_kind: String,
    pub executed_at: String,
    pub execution_note: Option<String>,
    pub account: coauth_admin_types::SingleOutcome<coauth_admin_types::AdminAccountAttributes>,
    pub allowed_next_transitions: Vec<String>,
}

/// Cursor-paginated account list. The caller passes back the opaque
/// `page[after]` value it received from the prior page's `links.next`.
///
/// `filter[search]` is the only free-text filter in coauth's account-list
/// contract. Pagination always goes through the shared JSON:API helper.
pub async fn list_accounts_cursor(
    cursor: Option<&str>,
    limit: u64,
    search: &str,
) -> Result<CursorPage<CoauthAccountSummary>, HttpError> {
    let search = search.trim();
    let query = [("filter[search]", search)];
    get_jsonapi_cursor_page_with_query(
        ACCOUNTS_PATH,
        cursor,
        limit,
        &query,
        map_admin_account_summary_resource,
    )
    .await
}

pub async fn get_account_detail(id: &str) -> Result<CoauthAccountDetail, HttpError> {
    let bridge_url = BRIDGE_DESCRIBE_PATH;
    let integration_manifest_url = COAUTH_INTEGRATION_DESCRIBE_PATH;
    let summary_url = format!("/_coauth/admin/accounts/{}", urlencoding::encode(id));
    let dids_url = format!("/_coauth/admin/accounts/{}/dids", urlencoding::encode(id));
    let claims_url = format!("/_coauth/admin/accounts/{}/claims", urlencoding::encode(id));
    let current_url = format!(
        "/_coauth/admin/accounts/{}/risk-action/current",
        urlencoding::encode(id)
    );
    let history_url = format!(
        "/_coauth/admin/accounts/{}/risk-action/history",
        urlencoding::encode(id)
    );
    let (summary, dids, claims, bridge, integration_manifest, current, history): (
        SingleOutcome<AdminAccountAttributes>,
        AdminAccountDidBindingsOutcome,
        AdminAccountClaimsOutcome,
        AdminBridgeDescribe,
        IntegrationManifest,
        SingleOutcome<AccountRiskActionCurrentOutcome>,
        AccountRiskActionHistoryOutcome,
    ) = futures_util::try_join!(
        api_client(&summary_url, "GET", NO_BODY),
        api_client(&dids_url, "GET", NO_BODY),
        api_client(&claims_url, "GET", NO_BODY),
        api_client(bridge_url, "GET", NO_BODY),
        api_client(integration_manifest_url, "GET", NO_BODY),
        api_client(&current_url, "GET", NO_BODY),
        api_client(&history_url, "GET", NO_BODY),
    )?;
    let account = map_admin_account_summary_resource(summary.data);
    Ok(CoauthAccountDetail {
        claims: claims
            .data
            .into_iter()
            .map(map_admin_account_claim)
            .collect(),
        risk_action_current: current.data.attributes,
        risk_action_history: history.data,
        managed_dids: dids.data.into_iter().map(map_admin_did_binding).collect(),
        account,
        risk_action_hook: CoauthRiskActionHook {
            endpoint: bridge.risk_action_path_template.replace("{account_id}", id),
            approval_mode: bridge.risk_action_approval_mode.clone(),
        },
        admin_bridge: bridge,
        integration_manifest,
    })
}

/// Remove a managed DID binding from an account. The DID is part of the
/// path so the request body is empty.
pub async fn remove_account_did_binding(account_id: &str, did: &str) -> Result<(), HttpError> {
    let url = format!(
        "/_coauth/admin/accounts/{}/dids/{}",
        urlencoding::encode(account_id),
        urlencoding::encode(did),
    );
    let _: NoBody = api_client(&url, "DELETE", NO_BODY).await?;
    Ok(())
}

/// Revoke a single claim by its record ULID.
pub async fn revoke_account_claim(claim_id: &str) -> Result<(), HttpError> {
    let url = format!(
        "/_coauth/admin/claims/{}/revoke",
        urlencoding::encode(claim_id),
    );
    let _: NoBody = api_client(&url, "POST", NO_BODY).await?;
    Ok(())
}

pub async fn submit_account_risk_action(
    id: &str,
    draft: &AccountRiskActionProposalRequestBody,
) -> Result<AccountRiskActionProposalOutcome, HttpError> {
    let url = format!(
        "/_coauth/admin/accounts/{}/risk-action",
        urlencoding::encode(id)
    );
    // The draft is `coauth_admin_types`' own request-body type, so it is
    // already the exact shape coauth deserializes.
    api_client(&url, "POST", Some(draft)).await
}

pub async fn execute_account_risk_action(
    id: &str,
    proposal_id: &str,
    draft: &AccountRiskActionExecuteRequestBody,
) -> Result<CoauthAccountRiskActionExecute, HttpError> {
    let url = format!(
        "/_coauth/admin/accounts/{}/risk-action/{}/execute",
        urlencoding::encode(id),
        urlencoding::encode(proposal_id)
    );
    api_client(&url, "POST", Some(draft)).await
}

fn map_admin_account_summary_resource(
    resource: SingleResource<AdminAccountAttributes>,
) -> CoauthAccountSummary {
    let attributes = resource.attributes;
    let is_locked = attributes.status.is_locked();
    let is_deactivated = attributes.status.is_deactivated();
    let primary_principal_id = attributes.effective_primary_id().cloned();
    CoauthAccountSummary {
        id: resource.id,
        username: Some(attributes.handle),
        display_name: attributes.display_name,
        avatar_url: attributes.avatar_url,
        primary_principal_id,
        is_locked,
        is_deactivated,
        created_at: attributes.created_at.map(|t| t.to_rfc3339()),
        updated_at: attributes.updated_at.map(|t| t.to_rfc3339()),
    }
}

fn map_admin_did_binding(binding: AdminAccountDidBinding) -> CoauthManagedDidBinding {
    CoauthManagedDidBinding {
        did: binding.did,
        kind: binding.kind,
        state: binding.state,
        verification_status: binding.verification_status,
        primary: binding.primary,
        last_verified_at: binding.last_verified_at.map(|t| t.to_rfc3339()),
    }
}

fn map_admin_account_claim(
    record: coauth_admin_types::AdminAccountClaimRecord,
) -> CoauthAccountClaim {
    CoauthAccountClaim {
        id: record.id,
        claim_kind: record.claim_kind,
        value: record.value,
        state: Some(record.state),
        source: Some(record.source),
    }
}
