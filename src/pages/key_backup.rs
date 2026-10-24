//! B-C key-backup admin page (`/key-backup`).
//!
//! Two panes:
//!   1. Backup series list with series head status + per-series 3-class 409 counters
//!      (`series_chain_broken` / `series_seq_not_monotonic` / `series_predecessor_not_found`).
//!   2. Recovery policy history (read-only; spec `recovery_policy_summary` rows — policy publish
//!      requires a principal-signed `signature_proof` transcript the admin UI cannot mint).

use arkret_models_crypto::recovery_policy::RecoveryMethod;
use arkret_models_crypto::{KeyBackupSummary, RecoveryPolicySummary};
use dioxus::prelude::*;

use crate::api::key_backup;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::input::{Input, LabelFor};
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::table::*;
use crate::utils::i18n::t;

#[component]
pub fn KeyBackupList() -> Element {
    let mut series_filter = use_signal(String::new);
    let mut backup_class_filter = use_signal(String::new);
    let mut principal_filter = use_signal(String::new);

    let series_q = series_filter.read().clone();
    let class_q = backup_class_filter.read().clone();
    let mut series_data = use_resource(move || {
        let series_q = series_q.clone();
        let class_q = class_q.clone();
        async move {
            let series_id = if series_q.is_empty() {
                None
            } else {
                Some(series_q.as_str())
            };
            let backup_kind = if class_q.is_empty() {
                None
            } else {
                Some(class_q.as_str())
            };
            key_backup::list_backups(series_id, backup_kind).await
        }
    });

    let principal_q = principal_filter.read().clone();
    let mut policies_data = use_resource(move || {
        let principal_q = principal_q.clone();
        async move {
            let principal_id = if principal_q.trim().is_empty() {
                None
            } else {
                Some(
                    arkret_identifiers::DidCoreId::new(principal_q.trim()).map_err(|error| {
                        crate::utils::net::error::HttpError::message(format!(
                            "principal_id is invalid: {error}"
                        ))
                    })?,
                )
            };
            key_backup::list_recovery_policies(principal_id.as_ref()).await
        }
    });

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("key_backup.title"),
                description: t("key_backup.subtitle"),
            }

            // ── Series filter ──
            div { class: "rounded-md border bg-card p-4",
                div { class: "grid gap-3 md:grid-cols-2",
                    div { class: "space-y-1",
                        LabelFor { class: "text-xs text-muted-foreground".to_string(), "series_id" }
                        Input {
                            placeholder: "ak:backup_series:…".to_string(),
                            value: series_filter.read().clone(),
                            oninput: move |evt: FormEvent| series_filter.set(evt.value()),
                        }
                    }
                    div { class: "space-y-1",
                        LabelFor { class: "text-xs text-muted-foreground".to_string(), "backup_kind" }
                        Input {
                            placeholder: "e.g. secret_storage".to_string(),
                            value: backup_class_filter.read().clone(),
                            oninput: move |evt: FormEvent| backup_class_filter.set(evt.value()),
                        }
                    }
                }
                div { class: "mt-3",
                    Button {
                        size: ButtonSize::Sm,
                        onclick: move |_| series_data.restart(),
                        {t("key_backup.apply_filter")}
                    }
                }
            }

            div { class: "rounded-md border bg-card p-4",
                div { class: "space-y-1",
                    LabelFor { class: "text-xs text-muted-foreground".to_string(), "principal_id" }
                    Input {
                        placeholder: t("key_backup.current_principal_when_empty"),
                        value: principal_filter.read().clone(),
                        oninput: move |evt: FormEvent| principal_filter.set(evt.value()),
                    }
                }
                div { class: "mt-3 flex gap-2",
                    Button {
                        size: ButtonSize::Sm,
                        onclick: move |_| {
                            policies_data.restart();
                        },
                        {t("key_backup.apply_recovery_filter")}
                    }
                }
            }

            // ── Series table ──
            div { class: "rounded-md border",
                div { class: "p-3 border-b font-medium text-sm", {t("key_backup.backup_series")} }
                match &*series_data.read() {
                    Some(Ok(resp)) => rsx! {
                        Table {
                            TableHeader {
                                TableRow {
                                    TableHead { "backup_id" }
                                    TableHead { "actor_id" }
                                    TableHead { "backup_kind" }
                                    TableHead { "backup_version" }
                                    TableHead { "created_at" }
                                    TableHead { "ciphertext_digest" }
                                }
                            }
                            TableBody {
                                if resp.backups.is_empty() {
                                    TableRow {
                                        TableCell { colspan: 99,
                                            class: "text-center text-muted-foreground py-6".to_string(),
                                            {t("key_backup.no_series")}
                                        }
                                    }
                                } else {
                                    for s in resp.backups.iter() {
                                        {render_series_row(s)}
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        ErrorBanner {
                            message: e.message.clone(),
                            on_retry: move |_| series_data.restart(),
                        }
                    },
                    None => rsx! { PageSkeleton {} },
                }
            }

            // ── Recovery policy history (read-only) ──
            div { class: "rounded-md border",
                div { class: "p-3 border-b font-medium text-sm",
                    {t("key_backup.recovery_policies")}
                }
                match &*policies_data.read() {
                    Some(Ok(resp)) => rsx! {
                        Table {
                            TableHeader {
                                TableRow {
                                    TableHead { "policy_id" }
                                    TableHead { "version" }
                                    TableHead { "trust_domain" }
                                    TableHead { "methods" }
                                    TableHead { "issued_at" }
                                    TableHead { "expires_at" }
                                }
                            }
                            TableBody {
                                if resp.is_empty() {
                                    TableRow {
                                        TableCell { colspan: 99,
                                            class: "text-center text-muted-foreground py-6".to_string(),
                                            {t("key_backup.no_policies")}
                                        }
                                    }
                                } else {
                                    for p in resp.iter() {
                                        {render_policy_row(p)}
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        ErrorBanner {
                            message: e.message.clone(),
                            on_retry: move |_| policies_data.restart(),
                        }
                    },
                    None => rsx! { PageSkeleton {} },
                }
            }
        }
    }
}

fn render_series_row(s: &KeyBackupSummary) -> Element {
    let class = format!("{:?}", s.backup_kind);
    let created_at = s.created_at.to_rfc3339();
    rsx! {
        TableRow {
            key: "{s.backup_id}",
            TableCell { class: "font-mono text-xs max-w-[180px] truncate".to_string(), "{s.backup_id}" }
            TableCell { class: "font-mono text-xs max-w-[180px] truncate".to_string(), "{s.actor_id}" }
            TableCell { "{class}" }
            TableCell { "{s.backup_version}" }
            TableCell { class: "text-xs".to_string(), "{created_at}" }
            TableCell { class: "font-mono text-xs max-w-[180px] truncate".to_string(), "{s.ciphertext_digest}" }
        }
    }
}

fn render_policy_row(p: &RecoveryPolicySummary) -> Element {
    let methods = if p.methods.is_empty() {
        // Explicit-revocation policy: an empty method set means
        // recovery is disabled under this policy.
        "(revoked)".to_string()
    } else {
        p.methods
            .iter()
            .map(RecoveryMethod::kind_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let policy_id = p.policy_id.as_str();
    let issued_at = p.issued_at.to_rfc3339();
    let expires_at = p
        .expires_at
        .as_ref()
        .map(|expires_at| expires_at.to_rfc3339())
        .unwrap_or_else(|| "-".to_string());
    let trust_domain = p.trust_domain.as_str();
    rsx! {
        TableRow {
            key: "{policy_id}",
            TableCell { class: "font-mono text-xs".to_string(), "{policy_id}" }
            TableCell {
                Badge { variant: BadgeVariant::Secondary, "v{p.version}" }
            }
            TableCell { class: "font-mono text-xs max-w-[160px] truncate".to_string(), "{trust_domain}" }
            TableCell { class: "text-xs".to_string(), "{methods}" }
            TableCell { class: "text-xs".to_string(), "{issued_at}" }
            TableCell { class: "text-xs".to_string(), "{expires_at}" }
        }
    }
}
