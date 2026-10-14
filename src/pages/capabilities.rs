use dioxus::prelude::*;

use crate::api::capabilities::{self, CapabilityFilters};
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::Button;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::input::Input;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::pagination::CursorPagination;
use crate::components::ui::table::*;
use crate::types::CapabilitySummaryExt;
use crate::utils::i18n::t;

const PAGE_SIZE: u64 = 25;

#[component]
pub fn CapabilityList() -> Element {
    rsx! { CapabilityLedger { title: t("capabilities.title"), description: t("capabilities.subtitle") } }
}

#[component]
pub fn CapabilityLedger(title: String, description: String) -> Element {
    let mut cursor_stack = use_signal(|| vec![None::<String>]);
    let mut realm_input = use_signal(String::new);
    let mut actor_input = use_signal(String::new);
    let mut state_input = use_signal(|| "all".to_owned());
    let mut filters = use_signal(CapabilityFilters::default);
    let mut filter_error = use_signal(|| None::<String>);

    let mut data = use_resource(move || {
        let cursor = cursor_stack.read().last().cloned().unwrap_or(None);
        let filters = filters();
        async move { capabilities::list_capabilities(cursor.as_deref(), PAGE_SIZE, &filters).await }
    });

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title,
                description,
            }

            div { class: "flex flex-wrap items-end gap-3", "data-testid": "accepted-capability-filters",
                div {
                    label { r#for: "capability-realm", {t("capabilities.filter_realm")} }
                    Input { id: "capability-realm", value: realm_input(), oninput: move |event: FormEvent| realm_input.set(event.value()) }
                }
                div {
                    label { r#for: "capability-actor", {t("capabilities.filter_actor")} }
                    Input { id: "capability-actor", value: actor_input(), oninput: move |event: FormEvent| actor_input.set(event.value()) }
                }
                div {
                    label { r#for: "capability-state", {t("authz_caps.status")} }
                    select { id: "capability-state", value: state_input(), onchange: move |event| state_input.set(event.value()),
                        option { value: "all", {t("capabilities.filter_all")} }
                        option { value: "active", {t("capabilities.active")} }
                        option { value: "revoked", {t("capabilities.revoked")} }
                        option { value: "relinquished", {t("capabilities.relinquished")} }
                    }
                }
                Button { onclick: move |_| {
                    match CapabilityFilters::parse(&realm_input(), &actor_input(), &state_input()) {
                        Ok(parsed) => { filter_error.set(None); data.clear(); cursor_stack.set(vec![None]); filters.set(parsed); }
                        Err(error) => filter_error.set(Some(error)),
                    }
                }, {t("capabilities.apply_filters")} }
            }
            if let Some(error) = filter_error() { ErrorBanner { message: error } }
            match &*data.read() {
                Some(Ok(resp)) => {
                    let next_cursor = resp.next_cursor.clone();
                    let stack_depth = cursor_stack.read().len();
                    rsx! {
                        div { class: "rounded-md border",
                            Table {
                                TableHeader {
                                    TableRow {
                                        TableHead { {t("capabilities.id")} }
                                        TableHead { {t("capabilities.filter_realm")} }
                                        TableHead { {t("capabilities.revision")} }
                                        TableHead { {t("capabilities.grantor_id")} }
                                        TableHead { {t("capabilities.grantee_id")} }
                                        TableHead { {t("capabilities.capability")} }
                                        TableHead { {t("capabilities.scope")} }
                                        TableHead { {t("capabilities.granted_at")} }
                                        TableHead { {t("capabilities.expires_at")} }
                                        TableHead { {t("authz_caps.status")} }
                                    }
                                }
                                TableBody {
                                    if resp.capabilities.is_empty() {
                                        TableRow {
                                            TableCell { class: "text-center text-muted-foreground py-8".to_string(), colspan: 99,
                                                {t("capabilities.no_capabilities")}
                                            }
                                        }
                                    } else {
                                        for cap in resp.capabilities.iter() {
                                            {
                                                let id = cap.selector.grant_id.to_string();
                                                let issuer = cap.issuer_display();
                                                let subject_id = cap.subject_display();
                                                let actions = cap.actions_display();
                                                let resource = cap.resource_display();
                                                let issued_at = cap.value.issued_at.to_rfc3339();
                                                let expires = cap.expiry_display();
                                                let revoked = cap.value.status != arkret_models_collaboration::governance::grant_constraint::CapabilityGrantStatus::Active;
                                                let status = match cap.value.status {
                                                    arkret_models_collaboration::governance::grant_constraint::CapabilityGrantStatus::Active => t("capabilities.active"),
                                                    arkret_models_collaboration::governance::grant_constraint::CapabilityGrantStatus::Revoked => t("capabilities.revoked"),
                                                    arkret_models_collaboration::governance::grant_constraint::CapabilityGrantStatus::Relinquished => t("capabilities.relinquished"),
                                                };
                                                let realm = cap.source_stream_ref.realm_id().to_string();
                                                let revision = arkret_canonical::canonical_json_string(&cap.revision).unwrap_or_else(|_| "unavailable".to_owned());
                                                rsx! {
                                                    TableRow {
                                                        key: "{id}",
                                                        TableCell { class: "font-medium".to_string(), "{id}" }
                                                        TableCell { class: "font-mono text-xs".to_string(), "{realm}" }
                                                        TableCell { class: "font-mono text-xs".to_string(), "{revision}" }
                                                        TableCell { class: "max-w-[180px] truncate".to_string(), span { title: "{issuer}", "{issuer}" } }
                                                        TableCell { class: "max-w-[180px] truncate".to_string(), span { title: "{subject_id}", "{subject_id}" } }
                                                        TableCell { "{actions}" }
                                                        TableCell { class: "max-w-[260px] truncate".to_string(), "{resource}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{issued_at}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{expires}" }
                                                        TableCell {
                                                            if revoked {
                                                                Badge { variant: BadgeVariant::Destructive, "{status}" }
                                                            } else {
                                                                Badge { variant: BadgeVariant::Success, "{status}" }
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

                        CursorPagination {
                            depth: stack_depth,
                            has_next: next_cursor.is_some(),
                            on_prev: move |_| {
                                let mut new_stack = cursor_stack.read().clone();
                                if new_stack.len() > 1 {
                                    new_stack.pop();
                                    data.clear();
                                    cursor_stack.set(new_stack);
                                }
                            },
                            on_next: move |_| {
                                if let Some(c) = next_cursor.clone() {
                                    let mut new_stack = cursor_stack.read().clone();
                                    new_stack.push(Some(c));
                                    data.clear();
                                    cursor_stack.set(new_stack);
                                }
                            },
                        }
                    }
                },
                Some(Err(e)) => {
                    let reset_cursor=e.status==410;
                    rsx! { ErrorBanner {
                        message:e.message.clone(),
                        on_retry:move |_| { data.clear(); if reset_cursor { cursor_stack.set(vec![None]); } data.restart(); },
                    } }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}
