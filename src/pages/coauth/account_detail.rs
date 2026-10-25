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
    let changed = data.clone();
    let detail_theme = theme.clone();
    let body = common::resource_body(theme.clone(), data.clone(), move |scope, detail| {
        let summary = &detail.account;
        let display_name = summary
            .display_name
            .clone()
            .unwrap_or_else(|| summary.id.clone());
        let summary_panel = common::panel(
            detail_theme.clone(),
            t("coauth_account_detail.account_summary"),
            common::fields(vec![
                ("coauth_account_detail.account_id", summary.id.clone()),
                (
                    "coauth_account_detail.handle",
                    common::optional(summary.username.clone()),
                ),
                (
                    "coauth_account_detail.primary_principal_id",
                    summary
                        .primary_principal_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "—".into()),
                ),
            ]),
        );
        let lifecycle = common::panel(
            detail_theme.clone(),
            t("coauth_account_detail.lifecycle"),
            common::fields(vec![
                (
                    "coauth_account_detail.status",
                    summary.lifecycle_label().into(),
                ),
                (
                    "coauth_account_detail.locked",
                    t(if summary.is_locked {
                        "common.yes"
                    } else {
                        "common.no"
                    }),
                ),
                (
                    "coauth_account_detail.deactivated",
                    t(if summary.is_deactivated {
                        "common.yes"
                    } else {
                        "common.no"
                    }),
                ),
                (
                    "coauth_account_detail.created_at",
                    common::optional(summary.created_at.clone()),
                ),
                (
                    "coauth_account_detail.updated_at",
                    common::optional(summary.updated_at.clone()),
                ),
            ]),
        );
        let mut children: Vec<View> = vec![
            Box::new(
                sapid::card()
                    .theme(detail_theme.clone())
                    .class("account-identity")
                    .fill((
                        text().class("account-display-name").content(display_name),
                        common::muted(t("coauth_account_detail.bridge_description")),
                    )),
            ),
            Box::new(
                view()
                    .class("coauth-detail-grid")
                    .fill((summary_panel, lifecycle)),
            ),
        ];
        let mut bindings: Vec<View> = Vec::new();
        for binding in detail.managed_dids {
            let did = binding.did.clone();
            let account = account_id.clone();
            let changed = changed.clone();
            let form = ui::action_form_controlled(
                scope,
                &mutations,
                format!("did:{}", binding.did),
                detail_theme.clone(),
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
            bindings.push(Box::new(view().class("coauth-inline-record").fill((
                view().class("record-header").fill((
                    text().class("mono record-title").content(binding.did),
                    ui::compact_action(detail_theme.clone(), t("did_binding_panel.remove"), form),
                )),
                common::attributes(
                    vec![
                        ("did_binding_panel.label_kind", binding.kind.label().into()),
                        (
                            "did_binding_panel.label_state",
                            binding.state.label().into(),
                        ),
                        (
                            "did_binding_panel.label_verification",
                            binding.verification_status.label().into(),
                        ),
                        (
                            "did_binding_panel.label_primary",
                            t(if binding.primary {
                                "did_binding_panel.primary_yes"
                            } else {
                                "did_binding_panel.primary_no"
                            }),
                        ),
                        (
                            "did_binding_panel.label_last_verified",
                            common::optional(binding.last_verified_at),
                        ),
                    ],
                    "coauth-attributes-five",
                ),
            ))));
        }
        if bindings.is_empty() {
            bindings.push(common::muted(t("did_binding_panel.empty")));
        }
        children.push(common::panel(
            detail_theme.clone(),
            t("did_binding_panel.title"),
            common::stack(detail_theme.clone(), bindings),
        ));
        let mut claims: Vec<View> = Vec::new();
        for claim in detail.claims {
            let action = if claim
                .state
                .as_deref()
                .is_some_and(|value| value.contains("revoked"))
            {
                common::muted(t("claims_panel.already_revoked"))
            } else {
                let id = claim.id.clone();
                let changed = changed.clone();
                let form = ui::action_form_controlled(
                    scope,
                    &mutations,
                    format!("claim:{}", claim.id),
                    detail_theme.clone(),
                    t("common.revoke"),
                    vec![],
                    Some(Danger::new(common::suffix(&id)).without_reason()),
                    move |_| {
                        let id = id.clone();
                        async move { coauth::revoke_account_claim(&id).await }
                    },
                    move || changed.reload(),
                );
                ui::compact_action(detail_theme.clone(), t("common.revoke"), form)
            };
            claims.push(Box::new(view().class("coauth-inline-record").fill((
                view().class("record-header").fill((
                    text().class("record-title").content(claim.claim_kind),
                    action,
                )),
                common::attributes(
                    vec![
                        ("claims_panel.label_value", common::optional(claim.value)),
                        ("claims_panel.label_state", common::optional(claim.state)),
                        ("claims_panel.label_source", common::optional(claim.source)),
                    ],
                    "coauth-attributes-three",
                ),
            ))));
        }
        if claims.is_empty() {
            claims.push(common::muted(t("claims_panel.empty")));
        }
        children.push(common::panel(
            detail_theme.clone(),
            t("claims_panel.title"),
            common::stack(detail_theme.clone(), claims),
        ));

        let current = &detail.risk_action_current;
        let mut risk: Vec<View> = vec![];
        if current.lifecycle_state == "idle" {
            risk.push(common::muted(t("risk_action_panel.no_state_recorded")));
        } else {
            risk.push(current_fields(current));
        }
        risk.push(Box::new(
            text()
                .class("section-heading")
                .kind(TextKind::Heading(3))
                .content(t("risk_action_panel.history_title")),
        ));
        if detail.risk_action_history.is_empty() {
            risk.push(common::muted(t("risk_action_panel.history_empty")));
        }
        for entry in &detail.risk_action_history {
            risk.push(Box::new(
                view()
                    .class("coauth-inline-record risk-history-entry")
                    .fill(history_fields(entry)),
            ));
        }
        risk.push(Box::new(
            text()
                .class("section-heading")
                .kind(TextKind::Heading(3))
                .content(t("risk_action_panel.hook_title")),
        ));
        risk.push(common::fields(vec![
            (
                "risk_action_panel.hook_approval_mode",
                detail.risk_action_hook.approval_mode.clone(),
            ),
            (
                "risk_action_panel.hook_endpoint",
                detail.risk_action_hook.endpoint.clone(),
            ),
            (
                "risk_action_panel.hook_current_template",
                detail
                    .admin_bridge
                    .risk_action_current_path_template
                    .clone(),
            ),
            (
                "risk_action_panel.hook_history_template",
                detail
                    .admin_bridge
                    .risk_action_history_path_template
                    .clone(),
            ),
            (
                "risk_action_panel.hook_approve_template",
                detail
                    .admin_bridge
                    .risk_action_approve_path_template
                    .clone(),
            ),
            (
                "risk_action_panel.hook_execute_template",
                detail
                    .admin_bridge
                    .risk_action_execute_path_template
                    .clone(),
            ),
        ]));
        let examples = common::fields(vec![
            (
                "risk_action_panel.hook_proposal_example",
                serde_json::to_string(&detail.admin_bridge.risk_action_examples.proposal_request)
                    .unwrap_or_default(),
            ),
            (
                "risk_action_panel.hook_approve_example",
                serde_json::to_string(&detail.admin_bridge.risk_action_examples.approve_request)
                    .unwrap_or_default(),
            ),
            (
                "risk_action_panel.hook_execute_example",
                serde_json::to_string(&detail.admin_bridge.risk_action_examples.execute_request)
                    .unwrap_or_default(),
            ),
        ]);
        risk.push(examples);
        let allowed = |name: &str| {
            current
                .allowed_next_transitions
                .iter()
                .any(|value| value.eq_ignore_ascii_case(name))
        };
        let allowed_summary = if current.allowed_next_transitions.is_empty() {
            t("risk_action_panel.allowed_none")
        } else {
            current.allowed_next_transitions.join(", ")
        };
        risk.push(Box::new(
            view().class("risk-workflow-summary").fill((
                text()
                    .class("section-heading")
                    .content(t("risk_action_panel.state_machine_title")),
                common::fields(vec![
                    (
                        "risk_action_panel.state_machine_lifecycle",
                        current.lifecycle_state.clone(),
                    ),
                    (
                        "risk_action_panel.state_machine_allowed_next",
                        allowed_summary,
                    ),
                ]),
                common::muted(t("risk_action_panel.state_machine_help")),
                text()
                    .class("text-warning")
                    .content(t("risk_action_panel.signing_unavailable")),
            )),
        ));
        let mut actions: Vec<View> = vec![];
        if current.lifecycle_state.is_empty()
            || current.lifecycle_state == "idle"
            || allowed("submit")
            || allowed("propose")
            || allowed("queue")
        {
            for action in ["lock", "disable", "reset_recovery", "erase"] {
                let account = account_id.clone();
                let changed = changed.clone();
                let label = t(&format!("risk_action_panel.queue_{action}"));
                let form = ui::action_form_controlled(
                    scope,
                    &mutations,
                    format!("risk:{}", account_id),
                    detail_theme.clone(),
                    label.clone(),
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
                );
                actions.push(ui::compact_action(detail_theme.clone(), label, form));
            }
        }
        if allowed("approve") {
            actions.push(Box::new(
                sapid::button()
                    .theme(detail_theme.clone())
                    .label(t("risk_action_panel.approve_last"))
                    .enabled(false),
            ));
        }
        if allowed("execute")
            && let (Some(proposal), Some(action)) =
                (current.proposal_id.clone(), current.action.clone())
        {
            let account = account_id.clone();
            let changed = changed.clone();
            let ticket = current.ticket.clone();
            let form = ui::action_form_controlled(
                scope,
                &mutations,
                format!("risk:{}", account_id),
                detail_theme.clone(),
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
            );
            actions.push(ui::compact_action(
                detail_theme.clone(),
                t("risk_action_panel.execute_approved"),
                form,
            ));
        }
        risk.push(common::group(actions));
        children.push(common::panel(
            detail_theme.clone(),
            t("risk_action_panel.current_state_title"),
            common::stack(detail_theme.clone(), risk),
        ));

        let manifest = &detail.integration_manifest;
        let bridge = &detail.admin_bridge;
        let dependencies = if manifest.dependencies.is_empty() {
            t("coauth_account_detail.none")
        } else {
            manifest
                .dependencies
                .iter()
                .map(|value| {
                    format!(
                        "{}:{}@{}",
                        value.service, value.purpose, value.discovery_path
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let surfaces = if manifest.surfaces.is_empty() {
            t("coauth_account_detail.none")
        } else {
            manifest
                .surfaces
                .iter()
                .map(|value| format!("{} {} {}", value.method, value.path, value.contract))
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let manifest_fields = common::fields(vec![
            ("coauth_account_detail.contract", manifest.contract.clone()),
            ("coauth_account_detail.service", manifest.service.clone()),
            ("coauth_account_detail.kind", manifest.service_kind.clone()),
            (
                "coauth_account_detail.describe_path",
                manifest.describe_path.clone(),
            ),
            ("coauth_account_detail.dependencies", dependencies),
            ("coauth_account_detail.surfaces", surfaces),
        ]);
        let bridge_fields = common::fields(vec![
            ("coauth_account_detail.contract", bridge.contract.clone()),
            (
                "coauth_account_detail.api_base",
                bridge.api_base_path.clone(),
            ),
            (
                "coauth_account_detail.accounts",
                bridge.accounts_path.clone(),
            ),
            (
                "coauth_account_detail.account_detail_template",
                bridge.account_detail_path_template.clone(),
            ),
            (
                "coauth_account_detail.did_bindings_template",
                bridge.account_dids_path_template.clone(),
            ),
            (
                "coauth_account_detail.claims_template",
                bridge.account_claims_path_template.clone(),
            ),
        ]);
        children.push(common::panel(
            detail_theme.clone(),
            t("coauth_account_detail.integration_manifest"),
            common::stack(
                detail_theme.clone(),
                vec![
                    manifest_fields,
                    Box::new(
                        text()
                            .class("section-heading")
                            .kind(TextKind::Heading(2))
                            .content(t("coauth_account_detail.admin_bridge_contract")),
                    ),
                    bridge_fields,
                ],
            ),
        ));
        common::stack(detail_theme.clone(), children)
    });
    common::page(
        "coauth_account_detail.title",
        "coauth_account_detail.description",
        vec![
            Box::new(
                link()
                    .class("table-link")
                    .target("/coauth/accounts")
                    .label(t("coauth_account_detail.back")),
            ),
            common::refresh(theme, data),
        ],
        vec![body],
    )
}
fn current_fields(value: &coauth_admin_types::AccountRiskActionCurrentOutcome) -> View {
    common::attributes(
        vec![
            (
                "risk_action_panel.detail_lifecycle_state",
                value.lifecycle_state.clone(),
            ),
            (
                "risk_action_panel.detail_state_record_id",
                common::optional(value.state_record_id.clone()),
            ),
            (
                "risk_action_panel.detail_proposal_id",
                common::optional(value.proposal_id.clone()),
            ),
            (
                "risk_action_panel.detail_action",
                common::optional(value.action.clone()),
            ),
            (
                "risk_action_panel.detail_last_operation",
                common::optional(value.last_operation.clone()),
            ),
            (
                "risk_action_panel.detail_transition_kind",
                common::optional(value.transition_kind.clone()),
            ),
            (
                "risk_action_panel.detail_previous_state",
                common::optional(value.previous_state.clone()),
            ),
            (
                "risk_action_panel.detail_state_revision",
                value
                    .state_revision
                    .map_or_else(|| "—".into(), |n| n.to_string()),
            ),
            (
                "risk_action_panel.detail_allowed_next",
                value.allowed_next_transitions.join(", "),
            ),
            (
                "risk_action_panel.detail_ticket",
                common::optional(value.ticket.clone()),
            ),
            (
                "risk_action_panel.detail_recorded_at",
                value
                    .recorded_at
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_else(|| "—".into()),
            ),
            (
                "risk_action_panel.detail_recorded_by",
                value
                    .recorded_by
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "—".into()),
            ),
            (
                "risk_action_panel.detail_recorded_by_handle",
                common::optional(value.recorded_by_handle.clone()),
            ),
        ],
        "coauth-attributes-two",
    )
}
fn history_fields(value: &coauth_admin_types::AccountRiskActionTransitionRecord) -> View {
    common::attributes(
        vec![
            (
                "risk_action_panel.label_state_record_id",
                common::optional(value.state_record_id.clone()),
            ),
            (
                "risk_action_panel.label_proposal_id",
                common::optional(value.proposal_id.clone()),
            ),
            (
                "risk_action_panel.label_action",
                common::optional(value.action.clone()),
            ),
            (
                "risk_action_panel.label_transition",
                value.transition_kind.clone(),
            ),
            (
                "risk_action_panel.label_previous_state",
                common::optional(value.previous_state.clone()),
            ),
            (
                "risk_action_panel.label_next_state",
                value.next_state.clone(),
            ),
            (
                "risk_action_panel.label_state_revision",
                value
                    .state_revision
                    .map_or_else(|| "—".into(), |n| n.to_string()),
            ),
            (
                "risk_action_panel.label_ticket",
                common::optional(value.ticket.clone()),
            ),
            (
                "risk_action_panel.label_recorded_at",
                value
                    .recorded_at
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_else(|| "—".into()),
            ),
            (
                "risk_action_panel.label_recorded_by",
                value
                    .recorded_by
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "—".into()),
            ),
            (
                "risk_action_panel.label_recorded_by_handle",
                common::optional(value.recorded_by_handle.clone()),
            ),
            (
                "risk_action_panel.label_approval_note",
                common::optional(value.approval_note.clone()),
            ),
            (
                "risk_action_panel.label_execution_note",
                common::optional(value.execution_note.clone()),
            ),
        ],
        "coauth-attributes-two",
    )
}
