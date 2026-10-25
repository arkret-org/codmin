use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger, Field, View};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;
pub fn page(scope: &Scope, theme: sapid::Theme, account_id: String) -> impl Widget + use<> {
    let mutations = ui::MutationController::new(scope);
    let account = account_id.clone();
    let data = ui::resource(scope, move || {
        let account = account.clone();
        async move { coauth::get_account_detail(&account).await }
    });
    let mutation = data.clone();
    let row_theme = theme.clone();
    let body = ui::resource_view(scope, theme.clone(), data, move |scope, detail| {
        let mut children: Vec<View> = vec![
            Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(t("coauth_account_detail.account_summary"))
                    .fill(common::record(&detail.account)),
            ),
            Box::new(
                link()
                    .target(format!(
                        "/coauth/accounts/{}/devices",
                        urlencoding::encode(&account_id)
                    ))
                    .label(t("coauth_devices.title")),
            ),
            ui::heading(t("did_binding_panel.title")),
        ];
        if detail.managed_dids.is_empty() {
            children.push(Box::new(text().content(t("did_binding_panel.empty"))));
        }
        for binding in detail.managed_dids {
            let did = binding.did.clone();
            let account = account_id.clone();
            let changed = mutation.clone();
            let remove = ui::action_form_controlled(
                scope,
                &mutations,
                format!("did:{}", binding.did),
                row_theme.clone(),
                t("did_binding_panel.remove"),
                vec![],
                Some(Danger::new(common::suffix(&did)).without_reason()),
                move |_| {
                    let did = did.clone();
                    let account = account.clone();
                    async move { coauth::remove_account_did_binding(&account, &did).await }
                },
                move || changed.reload(),
            );
            children.push(Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(binding.did.clone())
                    .fill((common::record(&binding), remove)),
            ));
        }
        children.push(ui::heading(t("claims_panel.title")));
        if detail.claims.is_empty() {
            children.push(Box::new(text().content(t("claims_panel.empty"))));
        }
        for claim in detail.claims {
            if claim
                .state
                .as_deref()
                .is_some_and(|state| state.contains("revoked"))
            {
                children.push(Box::new(
                    sapid::card()
                        .theme(row_theme.clone())
                        .title(claim.claim_kind.clone())
                        .fill((
                            common::record(&claim),
                            text().content(t("claims_panel.already_revoked")),
                        )),
                ));
                continue;
            }
            let id = claim.id.clone();
            let changed = mutation.clone();
            let remove = ui::action_form_controlled(
                scope,
                &mutations,
                format!("claim:{}", claim.id),
                row_theme.clone(),
                t("common.revoke"),
                vec![],
                Some(Danger::new(common::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move { coauth::revoke_account_claim(&id).await }
                },
                move || changed.reload(),
            );
            children.push(Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(claim.claim_kind.clone())
                    .fill((common::record(&claim), remove)),
            ));
        }
        children.push(ui::heading(t("risk_action_panel.current_state_title")));
        children.push(common::record(&detail.risk_action_current));
        children.push(ui::heading(t("risk_action_panel.history_title")));
        children.push(common::record(&detail.risk_action_history));
        children.push(ui::heading(t("risk_action_panel.hook_title")));
        children.push(common::record(&detail.risk_action_hook));
        let current = &detail.risk_action_current;
        let allowed = |name: &str| {
            current
                .allowed_next_transitions
                .iter()
                .any(|v| v.eq_ignore_ascii_case(name))
        };
        if current.lifecycle_state.is_empty()
            || current.lifecycle_state == "idle"
            || allowed("submit")
            || allowed("propose")
            || allowed("queue")
        {
            for action in ["lock", "disable", "reset_recovery", "erase"] {
                let account = account_id.clone();
                let changed = mutation.clone();
                children.push(ui::action_form_controlled(
                    scope,
                    &mutations,
                    format!("risk:{}", account_id),
                    row_theme.clone(),
                    t(&format!("risk_action_panel.queue_{action}")),
                    vec![Field::new(t("risk_action_panel.ticket_label"), "")],
                    Some(Danger::new(format!("QUEUE {}", action.to_uppercase()))),
                    move |values| {
                        let account = account.clone();
                        async move {
                            if values[0].trim().is_empty() {
                                return Err(HttpError::message("A ticket reference is required"));
                            }
                            coauth::submit_account_risk_action(
                                &account,
                                &coauth::AccountRiskActionProposalRequestBody {
                                    action: action.into(),
                                    reason: Some(values[1].clone()),
                                    ticket: Some(values[0].trim().into()),
                                    approved_by: None,
                                },
                            )
                            .await
                            .map(|_| ())
                        }
                    },
                    move || changed.reload(),
                ));
            }
        }
        if allowed("approve") {
            children.push(Box::new(
                text().content(t("risk_action_panel.signing_unavailable")),
            ));
            children.push(Box::new(
                sapid::button()
                    .theme(row_theme.clone())
                    .label(t("risk_action_panel.approve_last"))
                    .enabled(false),
            ));
        }
        if allowed("execute")
            && let (Some(proposal), Some(action)) =
                (current.proposal_id.clone(), current.action.clone())
        {
            let account = account_id.clone();
            let changed = mutation.clone();
            let ticket = current.ticket.clone();
            children.push(ui::action_form_controlled(
                scope,
                &mutations,
                format!("risk:{}", account_id),
                row_theme.clone(),
                t("risk_action_panel.execute_approved"),
                vec![],
                Some(Danger::new(format!("EXECUTE {}", action.to_uppercase()))),
                move |values| {
                    let account = account.clone();
                    let proposal = proposal.clone();
                    let action = action.clone();
                    let ticket = ticket.clone();
                    async move {
                        coauth::execute_account_risk_action(
                            &account,
                            &proposal,
                            &coauth::AccountRiskActionExecuteRequestBody {
                                action,
                                ticket,
                                execution_note: Some(values[0].clone()),
                            },
                        )
                        .await
                        .map(|_| ())
                    }
                },
                move || changed.reload(),
            ));
        }
        children.push(ui::heading(t("coauth_account_detail.integration_manifest")));
        children.push(common::record(&detail.integration_manifest));
        children.push(ui::heading(t(
            "coauth_account_detail.admin_bridge_contract",
        )));
        children.push(common::record(&detail.admin_bridge));
        common::stack(row_theme.clone(), children)
    });
    common::stack(
        theme,
        vec![
            common::heading("coauth_account_detail.title"),
            Box::new(
                link()
                    .target("/coauth/accounts")
                    .label(t("coauth_account_detail.back")),
            ),
            body,
        ],
    )
}
