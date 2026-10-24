use dioxus::prelude::*;

use crate::api::coauth;
use crate::components::claims_panel::ClaimsPanel;
use crate::components::did_binding_panel::DidBindingPanel;
use crate::components::risk_action_panel;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::router::Route;
use crate::utils::i18n::t;

#[component]
pub fn AccountDetailPage(account_id: String) -> Element {
    let account_id_for_resource = account_id.clone();
    let mut data = use_resource(move || {
        let account_id_for_resource = account_id_for_resource.clone();
        async move { coauth::get_account_detail(&account_id_for_resource).await }
    });
    // Mutation panels (DID binding add/remove, claim revoke) need to
    // restart this resource on success; clone the signal so each panel
    // gets its own owned handle.
    let mut data_for_dids = data;
    let mut data_for_claims = data;
    let mut data_for_risk = data;

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("coauth_account_detail.title"),
                description: t("coauth_account_detail.description"),
                Link {
                    to: Route::CoauthAccountList {},
                    class: "inline-flex h-10 items-center rounded-md border px-4 text-sm font-medium transition-colors hover:bg-accent".to_string(),
                    {t("coauth_account_detail.back")}
                }
            }

            match &*data.read() {
                Some(Ok(detail)) => {
                    let summary = &detail.account;
                    let display_name = summary.display_name.clone().unwrap_or_else(|| summary.id.clone());
                    let handle = summary.username.clone().unwrap_or_else(|| "-".to_string());
                    let created_at = summary.created_at.clone().unwrap_or_else(|| "-".to_string());
                    let updated_at = summary.updated_at.clone().unwrap_or_else(|| "-".to_string());
                    let primary_principal_id = summary
                        .primary_principal_id
                        .as_ref()
                        .map_or("-", arkret_identifiers::DidCoreId::as_str);
                    let lifecycle = summary.lifecycle_label();
                    let integration_dependencies = if detail.integration_manifest.dependencies.is_empty() {
                        t("coauth_account_detail.none")
                    } else {
                        detail
                            .integration_manifest
                            .dependencies
                            .iter()
                            .map(|dependency| {
                                format!(
                                    "{}:{}@{}",
                                    dependency.service, dependency.purpose, dependency.discovery_path
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" | ")
                    };
                    let integration_surfaces = if detail.integration_manifest.surfaces.is_empty() {
                        t("coauth_account_detail.none")
                    } else {
                        detail
                            .integration_manifest
                            .surfaces
                            .iter()
                            .map(|surface| {
                                format!("{} {} {}", surface.method, surface.path, surface.contract)
                            })
                            .collect::<Vec<_>>()
                            .join(" | ")
                    };
                    rsx! {
                        div { class: "rounded-lg border p-4 space-y-2",
                            div { class: "text-lg font-semibold", "{display_name}" }
                            div { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.bridge_description")}
                            }
                        }

                        div { class: "grid gap-4 md:grid-cols-2",
                            {section_block(
                                t("coauth_account_detail.account_summary"),
                                rsx! {
                                    {detail_row(t("coauth_account_detail.account_id"), &summary.id)}
                                    {detail_row(t("coauth_account_detail.handle"), &handle)}
                                    {detail_row(
                                        t("coauth_account_detail.primary_principal_id"),
                                        primary_principal_id,
                                    )}
                                },
                            )}
                            {section_block(
                                t("coauth_account_detail.lifecycle"),
                                rsx! {
                                    {detail_row(t("coauth_account_detail.status"), lifecycle)}
                                    {detail_row(t("coauth_account_detail.locked"), &bool_label(summary.is_locked))}
                                    {detail_row(t("coauth_account_detail.deactivated"), &bool_label(summary.is_deactivated))}
                                    {detail_row(t("coauth_account_detail.created_at"), &created_at)}
                                    {detail_row(t("coauth_account_detail.updated_at"), &updated_at)}
                                },
                            )}
                        }

                        DidBindingPanel {
                            account_id: account_id.clone(),
                            bindings: detail.managed_dids.clone(),
                            on_mutated: move |_| data_for_dids.restart(),
                        }

                        ClaimsPanel {
                            account_id: account_id.clone(),
                            claims: detail.claims.clone(),
                            on_mutated: move |_| data_for_claims.restart(),
                        }

                        {risk_action_panel::risk_action_panel(
                            &account_id,
                            &detail.risk_action_current,
                            &detail.risk_action_history,
                            &detail.risk_action_hook,
                            &detail.admin_bridge,
                            move || data_for_risk.restart(),
                        )}

                        div { class: "rounded-lg border p-4 space-y-4",
                            h2 { class: "text-base font-semibold", {t("coauth_account_detail.integration_manifest")} }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.contract")}
                                span { class: "font-mono", "{detail.integration_manifest.contract}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.service")}
                                span { class: "font-mono", "{detail.integration_manifest.service}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.kind")}
                                span { class: "font-mono", "{detail.integration_manifest.service_kind}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.describe_path")}
                                span { class: "font-mono", "{detail.integration_manifest.describe_path}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.dependencies")}
                                span { class: "font-mono", "{integration_dependencies}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.surfaces")}
                                span { class: "font-mono", "{integration_surfaces}" }
                            }
                            h2 { class: "text-base font-semibold", {t("coauth_account_detail.admin_bridge_contract")} }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.contract")}
                                span { class: "font-mono", "{detail.admin_bridge.contract}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.api_base")}
                                span { class: "font-mono", "{detail.admin_bridge.api_base_path}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.accounts")}
                                span { class: "font-mono", "{detail.admin_bridge.accounts_path}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.account_detail_template")}
                                span { class: "font-mono", "{detail.admin_bridge.account_detail_path_template}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.did_bindings_template")}
                                span { class: "font-mono", "{detail.admin_bridge.account_dids_path_template}" }
                            }
                            p { class: "text-sm text-muted-foreground",
                                {t("coauth_account_detail.claims_template")}
                                span { class: "font-mono", "{detail.admin_bridge.account_claims_path_template}" }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        errcode: e.body.as_ref().map(|body| body.code.clone()),
                        request_id: e.request_id.clone(),
                        retry_after_ms: e.retry_after_ms,
                        on_retry: move |_| data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}

fn section_block(title: String, content: Element) -> Element {
    rsx! {
        div { class: "rounded-lg border p-4 space-y-3",
            h2 { class: "text-base font-semibold", "{title}" }
            {content}
        }
    }
}

fn detail_row(label: String, value: &str) -> Element {
    rsx! {
        div { class: "flex items-start justify-between gap-4 border-b pb-2 last:border-b-0 last:pb-0",
            span { class: "text-sm text-muted-foreground", "{label}" }
            span { class: "text-sm text-right break-all", "{value}" }
        }
    }
}

fn bool_label(value: bool) -> String {
    if value {
        t("common.yes")
    } else {
        t("common.no")
    }
}
