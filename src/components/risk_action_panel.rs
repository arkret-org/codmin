use dioxus::prelude::*;

use crate::api::coauth;
use crate::components::dangerous_action_dialog::DangerousActionDialog;
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::input::{Input, LabelFor};
use crate::utils::i18n::t;

/// Render the risk-action state-machine panel.
///
/// Called from `AccountDetailPage` with borrowed references to avoid
/// duplicating the data fetch.  Maintains its own signals for
/// proposal/approve/execute workflow state.
pub fn risk_action_panel<F>(
    account_id: &str,
    current: &coauth::AccountRiskActionCurrentOutcome,
    history: &[coauth::AccountRiskActionTransitionRecord],
    hook: &coauth::CoauthRiskActionHook,
    bridge: &coauth::AdminBridgeDescribe,
    on_mutated: F,
) -> Element
where
    F: FnMut() + Clone + 'static,
{
    let account_id = account_id.to_string();

    let mut action_status = use_signal(String::new);
    let mut last_proposal = use_signal(|| Option::<coauth::AccountRiskActionProposalOutcome>::None);
    let mut last_approval = use_signal(|| Option::<coauth::AccountRiskActionApprovalOutcome>::None);
    let mut proposal_reason = use_signal(String::new);
    let mut proposal_ticket = use_signal(String::new);
    let mut pending_proposal_action = use_signal::<Option<String>>(|| None);
    let mut approve_in_flight = use_signal(|| false);
    let mut execute_in_flight = use_signal(|| false);

    // State machine gating
    let allowed_transitions = &current.allowed_next_transitions;
    let lifecycle_state = current.lifecycle_state.as_str();
    let is_idle = lifecycle_state.is_empty() || lifecycle_state == "idle";
    let allowed_contains = |needle: &str| -> bool {
        allowed_transitions
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(needle))
    };
    let can_submit = is_idle
        || allowed_contains("submit")
        || allowed_contains("propose")
        || allowed_contains("queue");
    let can_approve = allowed_contains("approve");
    let approval_signing_available = risk_action_approval_signing_available();
    let can_execute = allowed_contains("execute");
    let allowed_summary = if allowed_transitions.is_empty() {
        t("risk_action_panel.allowed_none")
    } else {
        allowed_transitions.join(", ")
    };
    let proposal_reason_ready = !proposal_reason.read().trim().is_empty();
    let proposal_ticket_ready = !proposal_ticket.read().trim().is_empty();
    let can_queue_proposal = can_submit && proposal_reason_ready && proposal_ticket_ready;

    rsx! {
        div { class: "rounded-lg border p-4 space-y-4",
            if !action_status().is_empty() {
                div { class: "rounded-md border p-3 text-sm text-muted-foreground",
                    "{action_status}"
                }
            }

            h2 { class: "text-base font-semibold", {t("risk_action_panel.current_state_title")} }
            if current.lifecycle_state == "idle" {
                p { class: "text-sm text-muted-foreground",
                    {t("risk_action_panel.no_state_recorded")}
                }
            } else {
                {current_state_grid(current)}
            }

            h2 { class: "text-base font-semibold mt-4", {t("risk_action_panel.history_title")} }
            if history.is_empty() {
                p { class: "text-sm text-muted-foreground",
                    {t("risk_action_panel.history_empty")}
                }
            } else {
                div { class: "space-y-3",
                    for entry in history.iter() {
                        {history_entry_card(entry)}
                    }
                }
            }

            h2 { class: "text-base font-semibold mt-4", {t("risk_action_panel.hook_title")} }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_approval_mode")}
                span { class: "font-mono", "{hook.approval_mode}" }
            }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_endpoint")}
                span { class: "font-mono", "{hook.endpoint}" }
            }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_current_template")}
                span { class: "font-mono", "{bridge.risk_action_current_path_template}" }
            }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_history_template")}
                span { class: "font-mono", "{bridge.risk_action_history_path_template}" }
            }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_approve_template")}
                span { class: "font-mono", "{bridge.risk_action_approve_path_template}" }
            }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_execute_template")}
                span { class: "font-mono", "{bridge.risk_action_execute_path_template}" }
            }
            // The three example payloads are shared typed structures.
            // Round-trip them through `serde_json` for compact display.
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_proposal_example")}
                span { class: "font-mono", "{serde_json::to_string(&bridge.risk_action_examples.proposal_request).unwrap_or_default()}" }
            }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_approve_example")}
                span { class: "font-mono", "{serde_json::to_string(&bridge.risk_action_examples.approve_request).unwrap_or_default()}" }
            }
            p { class: "text-sm text-muted-foreground",
                {t("risk_action_panel.hook_execute_example")}
                span { class: "font-mono", "{serde_json::to_string(&bridge.risk_action_examples.execute_request).unwrap_or_default()}" }
            }
            if let Some(proposal) = last_proposal() {
                div { class: "rounded-md border p-3 space-y-1 text-sm text-muted-foreground",
                    div { {t("risk_action_panel.last_proposal")} span { class: "font-mono", "{proposal.proposal_id}" } }
                    div { {t("risk_action_panel.last_state_record")} span { class: "font-mono", "{proposal.state_record_id}" } }
                    div { {t("risk_action_panel.label_action")} span { class: "font-mono", "{proposal.action}" } }
                    div { {t("risk_action_panel.last_state")} span { class: "font-mono", "{proposal.proposal_state}" } }
                    div { {t("risk_action_panel.last_revision")} span { class: "font-mono", "{proposal.state_revision}" } }
                }
            }

            div { class: "rounded-md border p-3 text-xs text-muted-foreground",
                div { class: "font-medium text-foreground", {t("risk_action_panel.state_machine_title")} }
                div { {t("risk_action_panel.state_machine_lifecycle")} span { class: "font-mono", "{lifecycle_state}" } }
                div { {t("risk_action_panel.state_machine_allowed_next")} span { class: "font-mono", "{allowed_summary}" } }
                div { class: "mt-1",
                    {t("risk_action_panel.state_machine_help")}
                }
                if !approval_signing_available {
                    div { class: "mt-1 text-amber-700",
                        {t("risk_action_panel.signing_unavailable")}
                    }
                }
            }

            div { class: "grid gap-3 rounded-md border p-3 md:grid-cols-2",
                div { class: "space-y-1",
                    LabelFor { r#for: "risk-action-reason".to_string(), {t("risk_action_panel.reason_label")} }
                    Input {
                        id: "risk-action-reason".to_string(),
                        placeholder: t("risk_action_panel.reason_placeholder"),
                        value: proposal_reason.read().clone(),
                        required: true,
                        oninput: move |evt: FormEvent| proposal_reason.set(evt.value()),
                    }
                }
                div { class: "space-y-1",
                    LabelFor { r#for: "risk-action-ticket".to_string(), {t("risk_action_panel.ticket_label")} }
                    Input {
                        id: "risk-action-ticket".to_string(),
                        placeholder: t("risk_action_panel.ticket_placeholder"),
                        value: proposal_ticket.read().clone(),
                        required: true,
                        oninput: move |evt: FormEvent| proposal_ticket.set(evt.value()),
                    }
                }
            }

            div { class: "flex flex-wrap gap-2",
                for action_label in [("lock", t("risk_action_panel.queue_lock")), ("disable", t("risk_action_panel.queue_disable")), ("reset_recovery", t("risk_action_panel.queue_reset_recovery")), ("erase", t("risk_action_panel.queue_erase"))] {
                    Button {
                        key: "{action_label.0}",
                        variant: ButtonVariant::Outline,
                        disabled: !can_queue_proposal,
                        onclick: {
                            let action = action_label.0.to_string();
                            move |_| {
                                if can_queue_proposal {
                                    pending_proposal_action.set(Some(action.clone()));
                                }
                            }
                        },
                        "{action_label.1}"
                    }
                }
                if let Some(proposal) = last_proposal() {
                    Button {
                        variant: ButtonVariant::Secondary,
                        disabled: !can_approve || !approval_signing_available || *approve_in_flight.read(),
                        onclick: {
                            let account_id = account_id.clone();
                            let on_mutated = on_mutated.clone();
                            move |_| {
                                if *approve_in_flight.read() {
                                    return;
                                }
                                approve_in_flight.set(true);
                                let account_id = account_id.clone();
                                let proposal = proposal.clone();
                                let mut on_mutated = on_mutated.clone();
                                spawn(async move {
                                    let draft = build_risk_action_approval_draft(&proposal);
                                    match coauth::approve_account_risk_action(&account_id, &proposal.proposal_id, &draft).await {
                                        Ok(approval) => {
                                            last_approval.set(Some(approval.clone()));
                                            action_status.set(format_risk_action_approval_status(&approval));
                                            on_mutated();
                                        }
                                        Err(error) => action_status.set(t("risk_action_panel.approval_failed").replace("{error}", &error.message)),
                                    }
                                    approve_in_flight.set(false);
                                });
                            }
                        },
                        if *approve_in_flight.read() { {t("risk_action_panel.approving")} } else { {t("risk_action_panel.approve_last")} }
                    }
                }
                if let Some(approval) = last_approval() {
                    Button {
                        variant: ButtonVariant::Secondary,
                        disabled: !can_execute || *execute_in_flight.read(),
                        onclick: {
                            let account_id = account_id.clone();
                            let on_mutated = on_mutated.clone();
                            move |_| {
                                if *execute_in_flight.read() {
                                    return;
                                }
                                execute_in_flight.set(true);
                                let account_id = account_id.clone();
                                let approval = approval.clone();
                                let mut on_mutated = on_mutated.clone();
                                spawn(async move {
                                    let draft = build_risk_action_execute_draft(&approval);
                                    match coauth::execute_account_risk_action(&account_id, &approval.proposal_id, &draft).await {
                                        Ok(execution) => {
                                            action_status.set(format_risk_action_execute_status(&execution));
                                            on_mutated();
                                        }
                                        Err(error) => action_status.set(t("risk_action_panel.execute_failed").replace("{error}", &error.message)),
                                    }
                                    execute_in_flight.set(false);
                                });
                            }
                        },
                        if *execute_in_flight.read() { {t("risk_action_panel.executing")} } else { {t("risk_action_panel.execute_approved")} }
                    }
                }
            }

            if let Some(action) = pending_proposal_action() {
                {
                    let phrase = risk_action_phrase(&action);
                    let description = t("risk_action_panel.dialog_description")
                        .replace("{action}", &action)
                        .replace("{account}", &account_id)
                        .replace("{phrase}", &phrase);
                    rsx! {
                        DangerousActionDialog {
                            open: true,
                            title: t("risk_action_panel.dialog_title").replace("{action}", &action),
                            description,
                            confirmation_phrase: phrase,
                            confirm_text: t("risk_action_panel.dialog_confirm"),
                            on_cancel: move |_| pending_proposal_action.set(None),
                            on_confirm: {
                                let account_id = account_id.clone();
                                let action = action.clone();
                                let on_mutated = on_mutated.clone();
                                move |_| {
                                    let account_id = account_id.clone();
                                    let action = action.clone();
                                    let mut on_mutated = on_mutated.clone();
                                    let reason = proposal_reason.read().trim().to_string();
                                    let ticket = proposal_ticket.read().trim().to_string();
                                    if reason.is_empty() || ticket.is_empty() {
                                        action_status.set(t("risk_action_panel.reason_ticket_required"));
                                        pending_proposal_action.set(None);
                                        return;
                                    }
                                    pending_proposal_action.set(None);
                                    spawn(async move {
                                        let draft = build_risk_action_draft(&action, reason, ticket);
                                        match coauth::submit_account_risk_action(&account_id, &draft).await {
                                            Ok(proposal) => {
                                                last_proposal.set(Some(proposal.clone()));
                                                last_approval.set(None);
                                                action_status.set(format_risk_action_status(&proposal));
                                                on_mutated();
                                            }
                                            Err(error) => action_status.set(t("risk_action_panel.proposal_failed").replace("{action}", &action).replace("{error}", &error.message)),
                                        }
                                    });
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

fn current_state_grid(current: &coauth::AccountRiskActionCurrentOutcome) -> Element {
    let proposal_id = current
        .proposal_id
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let state_record_id = current
        .state_record_id
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let action = current.action.clone().unwrap_or_else(|| "-".to_string());
    let last_operation = current
        .last_operation
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let transition_kind = current
        .transition_kind
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let previous_state = current
        .previous_state
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let state_revision = current
        .state_revision
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let allowed_next = if current.allowed_next_transitions.is_empty() {
        "-".to_string()
    } else {
        current.allowed_next_transitions.join(", ")
    };
    let ticket = current.ticket.clone().unwrap_or_else(|| "-".to_string());
    let recorded_at = current
        .recorded_at
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "-".to_string());
    let recorded_by = current
        .recorded_by
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "-".to_string());
    let recorded_by_handle = current
        .recorded_by_handle
        .clone()
        .unwrap_or_else(|| "-".to_string());
    rsx! {
        div { class: "grid gap-2 text-sm md:grid-cols-2",
            {detail_row(&t("risk_action_panel.detail_lifecycle_state"), &current.lifecycle_state)}
            {detail_row(&t("risk_action_panel.detail_state_record_id"), &state_record_id)}
            {detail_row(&t("risk_action_panel.detail_proposal_id"), &proposal_id)}
            {detail_row(&t("risk_action_panel.detail_action"), &action)}
            {detail_row(&t("risk_action_panel.detail_last_operation"), &last_operation)}
            {detail_row(&t("risk_action_panel.detail_transition_kind"), &transition_kind)}
            {detail_row(&t("risk_action_panel.detail_previous_state"), &previous_state)}
            {detail_row(&t("risk_action_panel.detail_state_revision"), &state_revision)}
            {detail_row(&t("risk_action_panel.detail_allowed_next"), &allowed_next)}
            {detail_row(&t("risk_action_panel.detail_ticket"), &ticket)}
            {detail_row(&t("risk_action_panel.detail_recorded_at"), &recorded_at)}
            {detail_row(&t("risk_action_panel.detail_recorded_by"), &recorded_by)}
            {detail_row(&t("risk_action_panel.detail_recorded_by_handle"), &recorded_by_handle)}
        }
    }
}

fn history_entry_card(entry: &coauth::AccountRiskActionTransitionRecord) -> Element {
    let state_record_id = entry
        .state_record_id
        .clone()
        .unwrap_or_else(|| "missing".to_string());
    let proposal_id = entry
        .proposal_id
        .clone()
        .unwrap_or_else(|| "missing".to_string());
    let action = entry
        .action
        .clone()
        .unwrap_or_else(|| "missing".to_string());
    let previous_state = entry
        .previous_state
        .clone()
        .unwrap_or_else(|| "missing".to_string());
    let state_revision = entry
        .state_revision
        .map(|v| v.to_string())
        .unwrap_or_else(|| "missing".to_string());
    let recorded_at = entry
        .recorded_at
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "missing".to_string());
    let recorded_by = entry
        .recorded_by
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "missing".to_owned());

    rsx! {
        div { class: "rounded-md border p-3 space-y-1 text-sm text-muted-foreground",
            div { {t("risk_action_panel.label_state_record_id")} span { class: "font-mono", "{state_record_id}" } }
            div { {t("risk_action_panel.label_proposal_id")} span { class: "font-mono", "{proposal_id}" } }
            div { {t("risk_action_panel.label_action")} span { class: "font-mono", "{action}" } }
            div { {t("risk_action_panel.label_transition")} span { class: "font-mono", "{entry.transition_kind}" } }
            div { {t("risk_action_panel.label_previous_state")} span { class: "font-mono", "{previous_state}" } }
            div { {t("risk_action_panel.label_next_state")} span { class: "font-mono", "{entry.next_state}" } }
            div { {t("risk_action_panel.label_state_revision")} span { class: "font-mono", "{state_revision}" } }
            div { {t("risk_action_panel.label_ticket")} span { class: "font-mono", "{entry.ticket.as_deref().unwrap_or(\"missing\")}" } }
            div { {t("risk_action_panel.label_recorded_at")} span { class: "font-mono", "{recorded_at}" } }
            div { {t("risk_action_panel.label_recorded_by")} span { class: "font-mono", "{recorded_by}" } }
            div { {t("risk_action_panel.label_recorded_by_handle")} span { class: "font-mono", "{entry.recorded_by_handle.as_deref().unwrap_or(\"missing\")}" } }
            div { {t("risk_action_panel.label_approval_note")} span { class: "font-mono", "{entry.approval_note.as_deref().unwrap_or(\"missing\")}" } }
            div { {t("risk_action_panel.label_execution_note")} span { class: "font-mono", "{entry.execution_note.as_deref().unwrap_or(\"missing\")}" } }
        }
    }
}

fn detail_row(label: &str, value: &str) -> Element {
    rsx! {
        div { class: "flex items-start justify-between gap-4 border-b pb-2 last:border-b-0 last:pb-0",
            span { class: "text-sm text-muted-foreground", "{label}" }
            span { class: "text-sm text-right break-all", "{value}" }
        }
    }
}

fn build_risk_action_draft(
    action: &str,
    reason: String,
    ticket: String,
) -> coauth::AccountRiskActionProposalRequestBody {
    coauth::AccountRiskActionProposalRequestBody {
        action: action.to_string(),
        reason: Some(reason),
        ticket: Some(ticket),
        approved_by: None,
    }
}

fn build_risk_action_approval_draft(
    proposal: &coauth::AccountRiskActionProposalOutcome,
) -> coauth::AccountRiskActionApprovalRequestBody {
    coauth::AccountRiskActionApprovalRequestBody {
        action: proposal.action.clone(),
        ticket: proposal.ticket.clone(),
        approved_by: None,
        approval_note: Some(format!(
            "codmin approval for proposal {} action {}",
            proposal.proposal_id, proposal.action
        )),
        approval_proof_jws: String::new(),
    }
}

fn risk_action_approval_signing_available() -> bool {
    // coauth requires a detached Ed25519 JWS over the approval transcript.
    // codmin currently authenticates with a bearer token and does not possess
    // the admin DID private key, so the UI must not submit unverifiable
    // approvals.
    false
}

fn build_risk_action_execute_draft(
    approval: &coauth::AccountRiskActionApprovalOutcome,
) -> coauth::AccountRiskActionExecuteRequestBody {
    coauth::AccountRiskActionExecuteRequestBody {
        action: approval.action.clone(),
        ticket: approval.ticket.clone(),
        execution_note: Some(format!(
            "codmin execute proposal {} action {}",
            approval.proposal_id, approval.action
        )),
    }
}

fn format_risk_action_status(proposal: &coauth::AccountRiskActionProposalOutcome) -> String {
    format!(
        "{}\nstate_record_id={}\nproposal_id={}\naction={}\nproposal_state={}\nstate_revision={}\ntransition_kind={}\napproval_mode={}\nrequested_at={}\nrequested_by={}\nrequested_by_handle={}\nticket={}\napproved_by={}",
        t("risk_action_panel.status_queued"),
        proposal.state_record_id,
        proposal.proposal_id,
        proposal.action,
        proposal.proposal_state,
        proposal.state_revision,
        proposal.transition_kind,
        proposal.approval_mode,
        proposal
            .requested_at
            .map(|dt| dt.to_rfc3339())
            .as_deref()
            .unwrap_or("missing"),
        proposal
            .requested_by
            .as_ref()
            .map(|value| value.as_str())
            .unwrap_or("missing"),
        proposal.requested_by_handle.as_deref().unwrap_or("missing"),
        proposal.ticket.as_deref().unwrap_or("missing"),
        proposal
            .approved_by
            .as_ref()
            .map(arkret_identifiers::DidCoreId::as_str)
            .unwrap_or("pending"),
    )
}

fn risk_action_phrase(action: &str) -> String {
    format!("QUEUE {}", action.to_ascii_uppercase())
}

fn format_risk_action_approval_status(
    approval: &coauth::AccountRiskActionApprovalOutcome,
) -> String {
    format!(
        "{}\nstate_record_id={}\nproposal_id={}\naction={}\napproval_state={}\nstate_revision={}\ntransition_kind={}\napproved_at={}\napproved_by={}\napproved_by_handle={}\napproval_note={}",
        t("risk_action_panel.status_approved"),
        approval.state_record_id,
        approval.proposal_id,
        approval.action,
        approval.approval_state,
        approval.state_revision,
        approval.transition_kind,
        approval
            .approved_at
            .map(|dt| dt.to_rfc3339())
            .as_deref()
            .unwrap_or("missing"),
        approval
            .approved_by
            .as_ref()
            .map(arkret_identifiers::DidCoreId::as_str)
            .unwrap_or("missing"),
        approval.approved_by_handle.as_deref().unwrap_or("missing"),
        approval.approval_note.as_deref().unwrap_or("missing"),
    )
}

fn format_risk_action_execute_status(execution: &coauth::CoauthAccountRiskActionExecute) -> String {
    format!(
        "{}\nstate_record_id={}\nproposal_id={}\naction={}\nexecution_state={}\nstate_revision={}\ntransition_kind={}\nexecuted_at={}\nexecution_note={}",
        t("risk_action_panel.status_executed"),
        execution.state_record_id,
        execution.proposal_id,
        execution.action,
        execution.execution_state,
        execution.state_revision,
        execution.transition_kind,
        execution.executed_at,
        execution.execution_note.as_deref().unwrap_or("missing"),
    )
}
