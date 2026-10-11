//! T6.2 §2 — Handle management surface.
//!
//! Two screens:
//!
//! - [`HandleList`] is a paginated table of `ak.handle.*` cells with issuer / expiry /
//!   last-reassignment columns and inline revoke + reassign affordances.
//! - [`HandleShow`] is the per-handle detail page; it pulls the audit trail from
//!   `/_coland/admin/handles/{id}/audit` (T3.2's audit table) and exposes the revoke +
//!   force-reassign actions.
//!
//! Both columns render Coland's canonical `<localpart>:<domain>` value
//! verbatim, without adding a display sigil.

use arkret_identifiers::DidCoreId;
use dioxus::prelude::*;

use crate::api::handles;
use crate::components::dangerous_action_dialog::{DangerousActionDialog, confirmation_suffix};
use crate::components::did_input::DidCoreIdInput;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::*;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::input::{LabelFor, SearchInput};
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::pagination::Pagination;
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::router::Route;
use crate::types::AdminHandleReassignBody;
use crate::utils::destructive_reason::destructive_reason_error;
use crate::utils::i18n::t;

const PAGE_SIZE: u64 = 25;

#[component]
pub fn HandleList() -> Element {
    let mut search = use_signal(String::new);
    let mut page = use_signal(|| 1u64);
    let mut show_revoke = use_signal(|| None::<String>);
    let mut show_reassign = use_signal(|| None::<String>);
    let mut new_subject_id = use_signal(String::new);
    let mut reassign_reason = use_signal(String::new);
    let mut reassign_loading = use_signal(|| false);

    let page_val = *page.read();
    let search_val = search.read().clone();

    let mut data = use_resource(move || {
        let s = search_val.clone();
        async move { handles::list_handles(page_val, PAGE_SIZE, &s).await }
    });

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("handles.title"),
                description: t("handles.subtitle"),
            }

            SearchInput {
                placeholder: t("handles.search"),
                value: search.read().clone(),
                oninput: move |evt: FormEvent| {
                    search.set(evt.value());
                    page.set(1);
                },
            }

            match &*data.read() {
                Some(Ok(resp)) => rsx! {
                    div { class: "rounded-md border",
                        Table {
                            TableHeader {
                                TableRow {
                                    TableHead { {t("handles.canonical_uri")} }
                                    TableHead { {t("handles.aliases")} }
                                    TableHead { {t("handles.issuer")} }
                                    TableHead { {t("handles.expires_at")} }
                                    TableHead { {t("handles.last_reassignment")} }
                                    TableHead { {t("handles.status")} }
                                    TableHead { class: "text-right".to_string(), {t("common.actions")} }
                                }
                            }
                            TableBody {
                                if resp.data.is_empty() {
                                    TableRow {
                                        TableCell { class: "text-center text-muted-foreground py-8".to_string(), colspan: 99,
                                            {t("handles.empty")}
                                        }
                                    }
                                } else {
                                    for handle in resp.data.iter() {
                                        {
                                            let id = handle.id.clone();
                                            let canonical = handle.canonical_uri.clone();
                                            let aliases = handle.aliases.join(", ");
                                            let issuer_id = handle.issuer_id.to_string();
                                            let expires = handle.expires_at.clone().unwrap_or_else(|| "-".to_string());
                                            let last_reassign = handle.last_reassignment_at.clone().unwrap_or_else(|| "-".to_string());
                                            let status = handle.status.clone().unwrap_or_else(|| "active".to_string());
                                            let is_revoked = status == "revoked";
                                            let id_for_link = id.clone();
                                            let id_for_revoke = id.clone();
                                            let id_for_reassign = id.clone();

                                            rsx! {
                                                TableRow {
                                                    key: "{id}",
                                                    TableCell { class: "font-mono text-xs max-w-[260px] truncate".to_string(),
                                                        div { class: "flex flex-col",
                                                            Link { to: Route::HandleShow { handle_id: id_for_link.clone() },
                                                                class: "text-primary hover:underline",
                                                                "{canonical}"
                                                            }
                                                        }
                                                    }
                                                    TableCell { class: "max-w-[200px] truncate text-xs".to_string(), "{aliases}" }
                                                    TableCell { class: "font-mono text-xs max-w-[180px] truncate".to_string(), "{issuer_id}" }
                                                    TableCell { class: "text-xs text-muted-foreground".to_string(), "{expires}" }
                                                    TableCell { class: "text-xs text-muted-foreground".to_string(), "{last_reassign}" }
                                                    TableCell {
                                                        if is_revoked {
                                                            Badge { variant: BadgeVariant::Destructive, "{status}" }
                                                        } else {
                                                            Badge { variant: BadgeVariant::Success, "{status}" }
                                                        }
                                                    }
                                                    TableCell { class: "text-right space-x-1".to_string(),
                                                        Button {
                                                            variant: ButtonVariant::Ghost,
                                                            size: ButtonSize::Sm,
                                                            disabled: is_revoked,
                                                            onclick: {
                                                                let id = id_for_reassign.clone();
                                                                move |_| {
                                                                    new_subject_id.set(String::new());
                                                                    reassign_reason.set(String::new());
                                                                    show_reassign.set(Some(id.clone()));
                                                                }
                                                            },
                                                            {t("handles.reassign")}
                                                        }
                                                        Button {
                                                            variant: ButtonVariant::Ghost,
                                                            size: ButtonSize::Sm,
                                                            disabled: is_revoked,
                                                            onclick: {
                                                                let id = id_for_revoke.clone();
                                                                move |_| show_revoke.set(Some(id.clone()))
                                                            },
                                                            {t("handles.revoke")}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(total) = resp.total {
                        Pagination {
                            page: page_val,
                            total,
                            per_page: PAGE_SIZE,
                            on_page_change: move |p| page.set(p),
                        }
                    } else {
                        p { class: "text-sm text-muted-foreground", {t("pagination.total_unknown")} }
                    }
                },
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        on_retry: move |_| data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }

        DangerousActionDialog {
            open: show_revoke.read().is_some(),
            title: t("handles.revoke_title"),
            description: t("handles.revoke_body"),
            confirmation_phrase: confirmation_suffix(show_revoke.read().as_deref().unwrap_or(""), 4),
            confirm_text: t("handles.revoke"),
            cancel_text: t("common.cancel"),
            on_confirm: move |_| {
                if let Some(id) = show_revoke.read().clone() {
                    spawn(async move {
                        match handles::revoke_handle(&id).await {
                            Ok(_) => {
                                show_toast(&t("handles.revoke_ok"), ToastVariant::Success);
                                data.restart();
                            }
                            Err(e) => show_toast(&format!("{}: {}", t("handles.revoke_fail"), e.message), ToastVariant::Error),
                        }
                    });
                }
                show_revoke.set(None);
            },
            on_cancel: move |_| show_revoke.set(None),
        }

        DangerousActionDialog {
            open: show_reassign.read().is_some(),
            title: t("handles.reassign_title"),
            description: t("handles.reassign_body"),
            confirmation_phrase: confirmation_suffix(
                show_reassign.read().as_deref().unwrap_or(""),
                4,
            ),
            confirm_text: t("handles.reassign"),
            cancel_text: t("common.cancel"),
            reason: Some(reassign_reason.read().clone()),
            reason_required: true,
            busy: *reassign_loading.read()
                || DidCoreId::new(new_subject_id.read().trim().to_owned()).is_err(),
            on_reason_change: move |reason| reassign_reason.set(reason),
            on_cancel: move |_| show_reassign.set(None),
            on_confirm: move |_| {
                if let Some(id) = show_reassign.read().clone() {
                    let subject_id = DidCoreId::new(new_subject_id.read().trim().to_owned());
                    let reason = reassign_reason.read().trim().to_string();
                    let Ok(subject_id) = subject_id else { return; };
                    if destructive_reason_error(&reason, true).is_some() {
                        return;
                    }
                    reassign_loading.set(true);
                    spawn(async move {
                        let req = AdminHandleReassignBody {
                            new_subject_id: subject_id,
                            reason,
                        };
                        match handles::reassign_handle(&id, &req).await {
                            Ok(_) => {
                                show_toast(&t("handles.reassign_ok"), ToastVariant::Success);
                                show_reassign.set(None);
                                data.restart();
                            }
                            Err(e) => show_toast(&format!("{}: {}", t("handles.reassign_fail"), e.message), ToastVariant::Error),
                        }
                        reassign_loading.set(false);
                    });
                }
            },
            div { class: "space-y-1",
                LabelFor { r#for: "handle-new-subject".to_string(), {t("handles.new_subject_id")} }
                DidCoreIdInput {
                    value: new_subject_id.read().clone(),
                    oninput: move |evt: FormEvent| new_subject_id.set(evt.value()),
                }
            }
        }
    }
}

#[component]
pub fn HandleShow(handle_id: String) -> Element {
    let id = handle_id.clone();
    let id_audit = handle_id.clone();
    let mut handle_data = use_resource(move || {
        let id = id.clone();
        async move { handles::get_handle(&id).await }
    });
    let mut audit_data = use_resource(move || {
        let id = id_audit.clone();
        async move { handles::get_handle_audit(&id).await }
    });

    let mut show_revoke = use_signal(|| false);
    let mut show_reassign = use_signal(|| false);
    let mut new_subject_id = use_signal(String::new);
    let mut reassign_reason = use_signal(String::new);
    let mut reassign_loading = use_signal(|| false);

    let id_revoke = handle_id.clone();
    let id_reassign = handle_id.clone();

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("handles.detail_title"),
                description: handle_id.clone(),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        handle_data.restart();
                        audit_data.restart();
                    },
                    {t("common.refresh")}
                }
            }

            match &*handle_data.read() {
                Some(Ok(handle)) => {
                    let canonical = handle.canonical_uri.clone();
                    let aliases = if handle.aliases.is_empty() { "-".to_string() } else { handle.aliases.join(", ") };
                    let issuer_id = handle.issuer_id.to_string();
                    let subject_id = handle.subject_id.to_string();
                    let assigned = handle.assigned_at.clone().unwrap_or_else(|| "-".to_string());
                    let expires = handle.expires_at.clone().unwrap_or_else(|| "-".to_string());
                    let last_re = handle.last_reassignment_at.clone().unwrap_or_else(|| "-".to_string());
                    let status = handle.status.clone().unwrap_or_else(|| "active".to_string());
                    let is_revoked = status == "revoked";
                    rsx! {
                        Card {
                            CardHeader {
                                div { class: "flex items-start justify-between gap-2",
                                    div { class: "space-y-1",
                                        CardTitle { "{canonical}" }
                                        CardDescription { {format!("{}: {}", t("handles.aliases"), aliases)} }
                                    }
                                    if is_revoked {
                                        Badge { variant: BadgeVariant::Destructive, "{status}" }
                                    } else {
                                        Badge { variant: BadgeVariant::Success, "{status}" }
                                    }
                                }
                            }
                            CardContent {
                                div { class: "grid gap-3 sm:grid-cols-2",
                                    {kv_cell(t("handles.issuer"), issuer_id)}
                                    {kv_cell(t("handles.subject"), subject_id)}
                                    {kv_cell(t("handles.assigned_at"), assigned)}
                                    {kv_cell(t("handles.expires_at"), expires)}
                                    {kv_cell(t("handles.last_reassignment"), last_re)}
                                }
                                div { class: "mt-4 flex gap-2",
                                    Button {
                                        variant: ButtonVariant::Outline,
                                        disabled: is_revoked,
                                        onclick: move |_| {
                                            new_subject_id.set(String::new());
                                            reassign_reason.set(String::new());
                                            show_reassign.set(true);
                                        },
                                        {t("handles.reassign")}
                                    }
                                    Button {
                                        variant: ButtonVariant::Destructive,
                                        disabled: is_revoked,
                                        onclick: move |_| show_revoke.set(true),
                                        {t("handles.revoke")}
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    ErrorBanner { message: e.message.clone(), on_retry: move |_| handle_data.restart() }
                },
                None => rsx! { PageSkeleton {} },
            }

            Card {
                CardHeader {
                    CardTitle { {t("handles.audit_title")} }
                    CardDescription { {t("handles.audit_subtitle")} }
                }
                CardContent {
                    match &*audit_data.read() {
                        Some(Ok(resp)) => rsx! {
                            if resp.data.is_empty() {
                                p { class: "text-sm text-muted-foreground py-4 text-center",
                                    {t("handles.audit_empty")}
                                }
                            } else {
                                Table {
                                    TableHeader {
                                        TableRow {
                                            TableHead { {t("handles.audit_when")} }
                                            TableHead { {t("handles.audit_action")} }
                                            TableHead { {t("handles.audit_actor")} }
                                            TableHead { {t("handles.audit_reason")} }
                                        }
                                    }
                                    TableBody {
                                        for ev in resp.data.iter() {
                                            {
                                                let when = ev.timestamp.clone().unwrap_or_else(|| "-".to_string());
                                                let action = ev.action.clone();
                                                let actor = ev.actor_id.clone().unwrap_or_else(|| "-".to_string());
                                                let reason = ev.reason.clone().unwrap_or_else(|| "-".to_string());
                                                rsx! {
                                                    TableRow {
                                                        key: "{ev.id}",
                                                        TableCell { class: "text-xs text-muted-foreground".to_string(), "{when}" }
                                                        TableCell { class: "text-xs font-medium".to_string(), "{action}" }
                                                        TableCell { class: "text-xs font-mono max-w-[180px] truncate".to_string(), "{actor}" }
                                                        TableCell { class: "text-xs".to_string(), "{reason}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            ErrorBanner { message: e.message.clone(), on_retry: move |_| audit_data.restart() }
                        },
                        None => rsx! { PageSkeleton {} },
                    }
                }
            }
        }

        DangerousActionDialog {
            open: *show_revoke.read(),
            title: t("handles.revoke_title"),
            description: t("handles.revoke_body"),
            confirmation_phrase: confirmation_suffix(&id_revoke, 4),
            confirm_text: t("handles.revoke"),
            cancel_text: t("common.cancel"),
            on_confirm: move |_| {
                let id = id_revoke.clone();
                spawn(async move {
                    match handles::revoke_handle(&id).await {
                        Ok(_) => {
                            show_toast(&t("handles.revoke_ok"), ToastVariant::Success);
                            handle_data.restart();
                            audit_data.restart();
                        }
                        Err(e) => show_toast(&format!("{}: {}", t("handles.revoke_fail"), e.message), ToastVariant::Error),
                    }
                });
                show_revoke.set(false);
            },
            on_cancel: move |_| show_revoke.set(false),
        }

        DangerousActionDialog {
            open: *show_reassign.read(),
            title: t("handles.reassign_title"),
            description: t("handles.reassign_body"),
            confirmation_phrase: confirmation_suffix(&id_reassign, 4),
            confirm_text: t("handles.reassign"),
            cancel_text: t("common.cancel"),
            reason: Some(reassign_reason.read().clone()),
            reason_required: true,
            busy: *reassign_loading.read()
                || DidCoreId::new(new_subject_id.read().trim().to_owned()).is_err(),
            on_reason_change: move |reason| reassign_reason.set(reason),
            on_cancel: move |_| show_reassign.set(false),
            on_confirm: move |_| {
                let subject_id = DidCoreId::new(new_subject_id.read().trim().to_owned());
                let reason = reassign_reason.read().trim().to_string();
                let Ok(subject_id) = subject_id else { return; };
                if destructive_reason_error(&reason, true).is_some() {
                    return;
                }
                let id = id_reassign.clone();
                reassign_loading.set(true);
                spawn(async move {
                    let req = AdminHandleReassignBody {
                        new_subject_id: subject_id,
                        reason,
                    };
                    match handles::reassign_handle(&id, &req).await {
                        Ok(_) => {
                            show_toast(&t("handles.reassign_ok"), ToastVariant::Success);
                            show_reassign.set(false);
                            handle_data.restart();
                            audit_data.restart();
                        }
                        Err(e) => show_toast(&format!("{}: {}", t("handles.reassign_fail"), e.message), ToastVariant::Error),
                    }
                    reassign_loading.set(false);
                });
            },
            div { class: "space-y-1",
                LabelFor { r#for: "handle-detail-new-subject".to_string(), {t("handles.new_subject_id")} }
                DidCoreIdInput {
                    value: new_subject_id.read().clone(),
                    oninput: move |evt: FormEvent| new_subject_id.set(evt.value()),
                }
            }
        }

    }
}

fn kv_cell(label: String, value: String) -> Element {
    rsx! {
        div {
            p { class: "text-xs text-muted-foreground", "{label}" }
            p { class: "text-sm font-mono break-all", "{value}" }
        }
    }
}
