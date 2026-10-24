//! Realm `media_service` read-only operations view.
//!
//! Renders the effective `ak.component.realm.media_service.v1` cell for a
//! single Realm: the media service DID plus each focus in `foci[]`
//! (`focus_id`, `type`, `region`, `token_endpoint`, `connect_url`,
//! `capabilities`, `health_url`, `cascade_group`).
//!
//! This view is **read-only** in codmin: mutating `media_service` is a
//! general-management action that strands through events / inkson, not
//! the operations console. The page therefore only pulls the effective
//! cell via `GET /_coland/admin/realms/{id}/media-service` and renders it.

use arkret_models_collaboration::events_payloads::MediaServiceFocus;
use dioxus::prelude::*;

use crate::api::media_service;
use crate::components::selection_required::{is_placeholder_resource_id, selection_required_state};
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::*;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::{BreadcrumbItem, Breadcrumbs, PageHeader};
use crate::router::Route;
use crate::utils::i18n::t;

#[component]
pub fn MediaServicePage(realm_id: String) -> Element {
    if is_placeholder_resource_id(&realm_id) {
        return selection_required_state("Realm");
    }

    let realm_id_label = realm_id.clone();
    let id_for_fetch = realm_id.clone();
    let mut media_data = use_resource(move || {
        let id = id_for_fetch.clone();
        async move { media_service::get_realm_media_service(&id).await }
    });

    rsx! {
        div { class: "space-y-6",
            Breadcrumbs {
                items: vec![
                    BreadcrumbItem { label: t("nav.realms"), route: Some(Route::RealmShow { realm_id: realm_id.clone() }) },
                    BreadcrumbItem { label: t("media_service.title"), route: None },
                ],
            }

            PageHeader {
                title: t("media_service.title"),
                description: format!("{}: {}", t("nav.realms"), realm_id_label),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| media_data.restart(),
                    {t("common.refresh")}
                }
            }

            // Read-only operations view. Realm media_service writes strand
            // through events / inkson, not codmin.
            div {
                class: "rounded-md border bg-muted/40 px-3 py-2 text-sm text-muted-foreground",
                role: "note",
                {t("media_service.readonly_note")}
            }

            match &*media_data.read() {
                Some(Ok(cell)) => {
                    let service_id = cell.service_id.clone().unwrap_or_else(|| "-".to_string());
                    let foci = cell.foci.clone();
                    rsx! {
                        Card {
                            CardHeader {
                                CardTitle { {t("media_service.epoch_title")} }
                                CardDescription { {t("media_service.subtitle")} }
                            }
                            CardContent {
                                div { class: "space-y-4",
                                    div { class: "space-y-1",
                                        p { class: "text-xs text-muted-foreground", {t("media_service.service_id")} }
                                        p { class: "text-sm font-mono break-all", "{service_id}" }
                                    }
                                }
                            }
                        }

                        Card {
                            CardHeader {
                                CardTitle { {t("media_service.foci_label")} }
                            }
                            CardContent {
                                div { class: "space-y-3",
                                    if foci.is_empty() {
                                        p { class: "text-sm text-muted-foreground", {t("media_service.empty")} }
                                    } else {
                                        for (idx, focus) in foci.iter().enumerate() {
                                            div { key: "{focus.focus_id}",
                                                {render_focus_row(idx, focus)}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    ErrorBanner { message: e.message.clone(), on_retry: move |_| media_data.restart() }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}

fn render_focus_row(idx: usize, focus: &MediaServiceFocus) -> Element {
    let focus_id = focus.focus_id.clone();
    let focus_kind = match focus.focus_kind {
        arkret_models_collaboration::objects::media::MediaBackendKind::Livekit => "livekit",
        arkret_models_collaboration::objects::media::MediaBackendKind::Mediasoup => "mediasoup",
        arkret_models_collaboration::objects::media::MediaBackendKind::Janus => "janus",
        arkret_models_collaboration::objects::media::MediaBackendKind::ArkretNative => {
            "arkret_native"
        }
        arkret_models_collaboration::objects::media::MediaBackendKind::MoqRelay => "moq_relay",
    };
    let region = focus
        .region
        .as_ref()
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| "-".to_owned());
    // Both are normative required fields (`media-service-binding.md` §2), so a
    // focus that reaches this page always carries them: a descriptor missing
    // either one fails the shared contract's decode before it gets here.
    let token_endpoint = focus.token_endpoint.clone();
    let connect_url = focus.connect_url.clone();
    let health_url = focus.health_url.clone().unwrap_or_else(|| "-".to_string());
    let cascade_group = focus
        .cascade_group
        .as_ref()
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| "-".to_owned());
    let capabilities = focus.capabilities.clone();

    rsx! {
        div {
            class: "rounded-md border p-3 space-y-2",
            div { class: "flex items-center justify-between",
                p { class: "text-xs text-muted-foreground", {t("media_service.focus_index").replace("{n}", &(idx + 1).to_string())} }
            }
            div { class: "grid gap-2 md:grid-cols-2",
                div { class: "space-y-1",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_id")} }
                    p { class: "text-sm font-mono break-all", "{focus_id}" }
                }
                div { class: "space-y-1",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_backend")} }
                    p { class: "text-sm font-mono", "{focus_kind}" }
                }
                div { class: "space-y-1",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_region")} }
                    p { class: "text-sm font-mono break-all", "{region}" }
                }
                div { class: "space-y-1 md:col-span-2",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_token_endpoint")} }
                    p { class: "text-sm font-mono break-all", "{token_endpoint}" }
                }
                div { class: "space-y-1 md:col-span-2",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_connect_url")} }
                    p { class: "text-sm font-mono break-all", "{connect_url}" }
                }
                div { class: "space-y-1 md:col-span-2",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_health_endpoint")} }
                    p { class: "text-sm font-mono break-all", "{health_url}" }
                }
                div { class: "space-y-1",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_cascade_group")} }
                    p { class: "text-sm font-mono break-all", "{cascade_group}" }
                }
                div { class: "space-y-1 md:col-span-2",
                    p { class: "text-xs text-muted-foreground", {t("media_service.focus_capabilities")} }
                    if capabilities.is_empty() {
                        p { class: "text-sm text-muted-foreground", "-" }
                    } else {
                        div { class: "flex flex-wrap gap-1",
                            for capability in capabilities.iter() {
                                Badge {
                                    key: "{capability}",
                                    variant: BadgeVariant::Secondary,
                                    class: "font-mono text-xs".to_string(),
                                    "{capability}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
