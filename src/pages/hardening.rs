//! T8.3 — aggregate production hardening dashboard.
//!
//! This page fetches a non-sensitive `hardening` block from each service
//! that Codmin can actually address (the local Coland plus the
//! configured Coauth upstream), then renders a per-service checklist
//! with green / red chips and an aggregate score.
//!
//! Hardening status is intentionally coarse — no paths, hostnames, or
//! token tails are returned by any service — so it's safe to publish
//! on the unauthenticated `/health` route.

use dioxus::prelude::*;

use crate::api::server;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::*;
use crate::components::ui::page_header::PageHeader;
use crate::types::api::HardeningStatus;
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

/// Build a sentinel `HttpError` for the "service not configured"
/// branch of the resource future. The dashboard treats this as a
/// `not_configured` empty state — distinct from an unreachable fetch.
fn http_skip(message: &str) -> HttpError {
    HttpError::message(message)
}

/// Aggregate view per service. `hardening` is `None` when the service
/// is reachable but the `/health` route did not return the new field
/// (older deployments), or `unreachable` is set when the fetch
/// failed.
struct ServiceHardening {
    /// Service slug used in URLs / DIDs (`coland`, `coauth`).
    slug: &'static str,
    /// i18n key for the user-facing label.
    label_key: &'static str,
    /// i18n key for the short paragraph describing the service's role.
    description_key: &'static str,
    hardening: Option<HardeningStatus>,
    /// When `Some`, the operator has not wired this service.
    not_configured_label: Option<String>,
    /// When `true`, the fetch failed (network / 5xx).
    unreachable: bool,
}

#[component]
pub fn HardeningDashboard() -> Element {
    let mut coland_health = use_resource(|| async { server::get_coland_health().await });
    let mut coauth_health = use_resource(|| async {
        if !crate::utils::net::session::has_coauth() {
            return Err(http_skip("coauth not configured"));
        }
        server::get_coauth_health().await
    });

    let services: Vec<ServiceHardening> = {
        let coland = coland_health.read();
        let coauth = coauth_health.read();
        vec![
            service_from_health(
                "coland",
                "hardening.svc_coland",
                "hardening.svc_coland_desc",
                coland.as_ref(),
                None,
            ),
            service_from_health(
                "coauth",
                "hardening.svc_coauth",
                "hardening.svc_coauth_desc",
                coauth.as_ref(),
                if crate::utils::net::session::has_coauth() {
                    None
                } else {
                    Some(t("hardening.coauth_not_configured"))
                },
            ),
        ]
    };

    let (total_score, total_max) = aggregate_score(&services);
    let any_warnings = services
        .iter()
        .any(|s| s.hardening.as_ref().is_some_and(|h| !h.warnings.is_empty()));

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("hardening.title"),
                description: t("hardening.subtitle"),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        coland_health.restart();
                        coauth_health.restart();
                    },
                    {t("common.refresh")}
                }
            }

            // Overall score chip. Renders red when any warning is
            // active or score is below max; green otherwise.
            Card {
                CardContent { class: "p-4".to_string(),
                    div { class: "flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between",
                        div { class: "space-y-1",
                            p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                                {t("hardening.aggregate_score")}
                            }
                            p { class: "text-2xl font-bold",
                                "{total_score} / {total_max}"
                            }
                            p { class: "text-xs text-muted-foreground",
                                {t("hardening.aggregate_score_hint")}
                            }
                        }
                        {aggregate_badge(total_score, total_max, any_warnings)}
                    }
                }
            }

            // Per-service breakdown.
            div { class: "grid gap-4 lg:grid-cols-2",
                for service in services.iter() {
                    {service_card(service)}
                }
            }
        }
    }
}

fn service_from_health(
    slug: &'static str,
    label_key: &'static str,
    description_key: &'static str,
    fetched: Option<&Result<server::HealthEnvelope, crate::utils::net::error::HttpError>>,
    not_configured_label: Option<String>,
) -> ServiceHardening {
    let (hardening, unreachable) = match (fetched, &not_configured_label) {
        (_, Some(_)) => (None, false),
        (Some(Ok(env)), _) => (env.hardening.clone(), false),
        (Some(Err(_)), _) => (None, true),
        (None, _) => (None, false),
    };
    ServiceHardening {
        slug,
        label_key,
        description_key,
        hardening,
        not_configured_label,
        unreachable,
    }
}

fn aggregate_score(services: &[ServiceHardening]) -> (u32, u32) {
    services.iter().fold((0u32, 0u32), |(score, max), s| {
        if let Some(h) = s.hardening.as_ref() {
            (score + h.checklist_score, max + h.checklist_max)
        } else {
            (score, max)
        }
    })
}

fn aggregate_badge(score: u32, max: u32, any_warnings: bool) -> Element {
    if max == 0 {
        return rsx! {
            Badge { variant: BadgeVariant::Secondary, {t("hardening.no_services_reporting")} }
        };
    }
    if score == max && !any_warnings {
        rsx! { Badge { variant: BadgeVariant::Success, {t("hardening.all_passing")} } }
    } else if score == 0 {
        rsx! { Badge { variant: BadgeVariant::Destructive, {t("hardening.all_failing")} } }
    } else {
        rsx! { Badge { variant: BadgeVariant::Destructive, {t("hardening.some_failing")} } }
    }
}

fn service_card(service: &ServiceHardening) -> Element {
    let header_badge = if service.not_configured_label.is_some() {
        rsx! { Badge { variant: BadgeVariant::Secondary, {t("hardening.not_configured")} } }
    } else if service.unreachable {
        rsx! { Badge { variant: BadgeVariant::Destructive, {t("hardening.unreachable")} } }
    } else if let Some(h) = service.hardening.as_ref() {
        if h.warnings.is_empty() {
            rsx! { Badge { variant: BadgeVariant::Success, {t("hardening.ok")} } }
        } else {
            let warn_text = format!("{} {}", h.warnings.len(), t("hardening.warnings_suffix"));
            rsx! { Badge { variant: BadgeVariant::Destructive, "{warn_text}" } }
        }
    } else {
        rsx! { Badge { variant: BadgeVariant::Secondary, {t("hardening.no_data")} } }
    };

    let body = if let Some(label) = service.not_configured_label.as_ref() {
        rsx! { p { class: "text-sm text-muted-foreground", "{label}" } }
    } else if service.unreachable {
        rsx! {
            p { class: "text-sm text-destructive",
                {t("hardening.fetch_failed")}
            }
        }
    } else if let Some(h) = service.hardening.as_ref() {
        render_checklist(h)
    } else {
        rsx! {
            p { class: "text-sm text-muted-foreground",
                {t("hardening.no_hardening_block")}
            }
        }
    };

    let service_label = t(service.label_key);
    let service_description = t(service.description_key);

    rsx! {
        Card {
            CardHeader {
                div { class: "flex items-start justify-between gap-2",
                    div { class: "space-y-1",
                        CardTitle { class: "text-lg".to_string(), "{service_label}" }
                        CardDescription { "{service_description}" }
                        p { class: "text-[10px] font-mono text-muted-foreground", "{service.slug}" }
                    }
                    {header_badge}
                }
            }
            CardContent { {body} }
        }
    }
}

fn render_checklist(h: &HardeningStatus) -> Element {
    let admin_auth = empty_as_dash(&h.admin_auth_mode);
    let rotation = empty_as_dash(&h.provider_credential_rotation);

    // Per-check rows. Each row is a chip + label; chip turns red when
    // the corresponding flag is `false` (or `development_mode` is
    // true / admin_auth is `"development"` / `"closed"` / rotation is
    // `"none"`).
    let rows: Vec<(&str, bool)> = vec![
        ("hardening.check_dev_mode_disabled", !h.development_mode),
        ("hardening.check_tls_enabled", h.tls_enabled),
        (
            "hardening.check_pq_hybrid_tls",
            h.pq_hybrid_tls_probe_verified,
        ),
        ("hardening.check_csp_configured", h.csp_header_configured),
        ("hardening.check_cors_strict", h.cors_strict),
        ("hardening.check_secret_manager", h.secret_manager_in_use),
        ("hardening.check_log_redaction", h.log_redaction_enabled),
        (
            "hardening.check_admin_auth_prod",
            !matches!(admin_auth.as_str(), "development" | "closed" | "-"),
        ),
        ("hardening.check_rate_limit", h.rate_limit_enabled),
        (
            "hardening.check_credential_rotation",
            !matches!(rotation.as_str(), "none" | "-"),
        ),
    ];

    rsx! {
        div { class: "space-y-3",
            // Header score line.
            div { class: "flex items-center justify-between",
                p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                    {t("hardening.score")}
                }
                p { class: "text-sm font-mono",
                    "{h.checklist_score} / {h.checklist_max}"
                }
            }

            // Free-form chips listing each check.
            ul { class: "grid gap-1 sm:grid-cols-2",
                for (label, ok) in rows.iter() {
                    li {
                        class: if *ok {
                            "inline-flex items-center gap-1 rounded-md border border-green-600/30 bg-green-600/10 px-2 py-1 text-xs text-green-700 dark:text-green-300"
                        } else {
                            "inline-flex items-center gap-1 rounded-md border border-red-600/30 bg-red-600/10 px-2 py-1 text-xs text-red-700 dark:text-red-300"
                        },
                        span { class: "font-mono",
                            if *ok { "\u{2713}" } else { "\u{2717}" }
                        }
                        span { {t(label)} }
                    }
                }
            }

            // Coarse strings (admin_auth_mode, rotation) shown beside
            // the chip row.
            div { class: "grid gap-2 sm:grid-cols-2 pt-1",
                div {
                    p { class: "text-[10px] uppercase tracking-wider text-muted-foreground", "admin_auth_mode" }
                    p { class: "text-xs font-mono", "{admin_auth}" }
                }
                div {
                    p { class: "text-[10px] uppercase tracking-wider text-muted-foreground", "provider_credential_rotation" }
                    p { class: "text-xs font-mono", "{rotation}" }
                }
            }

            // Warnings list.
            if !h.warnings.is_empty() {
                div { class: "rounded-md border border-red-600 bg-red-600/10 px-3 py-2 text-xs space-y-1",
                    role: "alert",
                    p { class: "font-semibold text-red-700 dark:text-red-300",
                        {t("hardening.failing_checks")}
                    }
                    ul { class: "list-disc pl-5 text-red-700 dark:text-red-200",
                        for warning in h.warnings.iter() {
                            li { "{warning}" }
                        }
                    }
                }
            }
        }
    }
}

fn empty_as_dash(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_owned()
    } else {
        value.to_owned()
    }
}
