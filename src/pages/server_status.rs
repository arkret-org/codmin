use arkret_wire::ProfileId;
use dioxus::prelude::*;

use crate::api::server;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::*;
use crate::components::ui::error_banner::{ErrorBanner, resource_error};
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::types::ServerDescribeDocument;
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

#[component]
pub fn ServerStatus() -> Element {
    let mut info_data = use_resource(|| async { server::get_server_info().await });
    let mut status_data = use_resource(|| async { server::get_server_status().await });
    let mut admin_describe = use_resource(|| async { server::get_server_describe().await });
    let info_result = info_data.read().clone();
    let status_result = status_data.read().clone();
    let admin_describe_result = admin_describe.read().clone();

    let info = result_data(&info_result).cloned();
    let admin_d = result_data(&admin_describe_result).cloned();

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("server_status.title"),
                description: t("server_status.subtitle"),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        info_data.restart();
                        status_data.restart();
                        admin_describe.restart();
                    },
                    {t("common.refresh")}
                }
            }

            {resource_error(&info_result)}
            {resource_error(&admin_describe_result)}

            if let Some(info) = info.as_ref() {
                {
                    let version = info.server_version.clone();
                    let protocol = info.protocol_version.clone().unwrap_or_else(|| "-".to_string());
                    let server_name = info.server_name.clone().unwrap_or_else(|| "-".to_string());
                    let uptime = info.uptime.map(|u| format!("{u}s")).unwrap_or_else(|| "-".to_string());
                    rsx! {
                        Card {
                            CardHeader { CardTitle { {t("server_status.server_info")} } }
                            CardContent {
                                div { class: "grid gap-4 md:grid-cols-4",
                                    {info_cell(t("server_status.version"), version)}
                                    {info_cell(t("server_status.protocol_version"), protocol)}
                                    {info_cell(t("server_status.server_name"), server_name)}
                                    {info_cell(t("server_status.uptime"), uptime)}
                                }
                            }
                        }
                    }
                }
            }

            // Public protocol-role description. Authentication providers remain
            // private Station implementation details and have no describe card.
            div { class: "space-y-4",
                h2 { class: "text-xl font-semibold tracking-tight", {t("server_status.services_title")} }
                div { class: "grid gap-4",
                    {service_describe_card(
                        t("server_status.service_admin"),
                        t("server_status.service_admin_hint"),
                        admin_d.as_ref(),
                        None,
                    )}
                }
            }

            // Reachability probe (admin /server/status). The endpoint runs no
            // component health checks, so this panel must not present itself
            // as a component roll-up: the previous mirror invented an
            // `{ok, results}` shape the server never sent, every field
            // defaulted, and the page reported "Issues Detected" on every
            // healthy node without a word of explanation.
            div { class: "space-y-4",
                h2 { class: "text-xl font-semibold tracking-tight", {t("server_status.probe_title")} }
                match &status_result {
                    Some(Ok(status)) => rsx! {
                        div { class: "flex items-center gap-4 mb-4",
                            if status.is_ok() {
                                Badge { variant: BadgeVariant::Success, class: "text-base px-4 py-1".to_string(), {t("server_status.reachable")} }
                            } else {
                                Badge { variant: BadgeVariant::Destructive, class: "text-base px-4 py-1".to_string(), {t("server_status.unreachable")} }
                            }
                            span { class: "text-xs text-muted-foreground", {t("server_status.probe_hint")} }
                        }
                        div { class: "grid gap-4 md:grid-cols-2 lg:grid-cols-3",
                            {info_cell(t("server_status.probe_service_id"), status.service_id.to_string())}
                            {info_cell(t("server_status.probe_storage"), status.storage.clone())}
                            {info_cell(t("server_status.probe_checked_by"), status.checked_by.to_string())}
                            {info_cell(t("server_status.probe_generated_at"), status.generated_at.clone())}
                            {info_cell(t("server_status.probe_realms"), status.counts.realms.to_string())}
                            {info_cell(
                                t("server_status.probe_accounts"),
                                status.counts.accounts.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
                            )}
                            {info_cell(
                                t("server_status.probe_devices"),
                                status.counts.devices.map(|value| value.to_string()).unwrap_or_else(|| "-".to_string()),
                            )}
                        }
                    },
                    Some(Err(error)) => rsx! {
                        ErrorBanner {
                            message: error.message.clone(),
                            errcode: error.body.as_ref().map(|body| body.errcode.clone()),
                            request_id: error.request_id.clone(),
                            retry_after_ms: error.retry_after_ms,
                            on_retry: move |_| status_data.restart(),
                        }
                    },
                    None => rsx! { PageSkeleton {} },
                }
            }
        }
    }
}

fn result_data<T>(result: &Option<Result<T, HttpError>>) -> Option<&T> {
    result.as_ref().and_then(|value| value.as_ref().ok())
}

fn info_cell(label: String, value: String) -> Element {
    rsx! {
        div {
            p { class: "text-xs text-muted-foreground", "{label}" }
            p { class: "text-sm font-semibold break-all", "{value}" }
        }
    }
}

fn service_describe_card(
    title: String,
    description: String,
    describe: Option<&ServerDescribeDocument>,
    not_configured_label: Option<String>,
) -> Element {
    let badge = match (describe.is_some(), not_configured_label.is_some()) {
        (true, _) => rsx! { Badge { variant: BadgeVariant::Success, {t("server_status.online")} } },
        (false, true) => {
            rsx! { Badge { variant: BadgeVariant::Secondary, {t("server_status.not_configured")} } }
        }
        (false, false) => {
            rsx! { Badge { variant: BadgeVariant::Destructive, {t("server_status.unreachable")} } }
        }
    };

    let body = match (describe, not_configured_label) {
        (Some(d), _) => describe_body(d),
        (None, Some(label)) => rsx! {
            p { class: "text-sm text-muted-foreground", "{label}" }
        },
        (None, None) => rsx! {
            p { class: "text-sm text-muted-foreground", {t("server_status.fetch_failed")} }
        },
    };

    rsx! {
        Card {
            CardHeader {
                div { class: "flex items-center justify-between gap-2",
                    div { class: "space-y-1",
                        CardTitle { class: "text-lg".to_string(), {title} }
                        CardDescription { {description} }
                    }
                    {badge}
                }
            }
            CardContent { {body} }
        }
    }
}

fn describe_body(describe: &ServerDescribeDocument) -> Element {
    let service_id = describe.service_id.to_string();
    let service_did = describe.service_resolution.did.to_string();
    let resolution_history_head = describe.service_resolution.method_history_head.clone();
    let resolution_version_id = describe.service_resolution.version_id.clone();
    // `trust_domain` is a required (validated) ServerDescribe
    // field in the SDK type, so it is always present here.
    let trust_domain = describe.trust_domain.to_string();
    let service_kind = describe.service_kind.as_str().to_owned();
    let protocol = describe.protocol_version.to_string();
    // Service-proprietary top-level extensions (not part of the spec
    // ServiceDescribe shape) — read via the extension envelope.
    let openapi = describe
        .extra_str(&["openapi_version"])
        .unwrap_or_else(|| "-".to_string());
    let schema = describe
        .extra_str(&["schema_registry_version"])
        .unwrap_or_else(|| "-".to_string());
    let event_kind = describe
        .extra_str(&["event_kind_registry_version"])
        .unwrap_or_else(|| "-".to_string());

    let profiles = describe.supported_profiles.clone();
    let features = describe.supported_features.clone();
    let operation_bundles = describe.supported_operation_bundles.clone();
    // ServiceDescribe carries strong-typed `transport_bindings`; render each
    // as JSON so the free-form chip section keeps reading `kind` / `base_url`.
    let bindings: Vec<serde_json::Value> = describe
        .transport_bindings
        .iter()
        .map(|binding| serde_json::to_value(binding).unwrap_or(serde_json::Value::Null))
        .collect();

    // T1.4 — surface the runtime posture. These are coland extension
    // fields (emitted on `/health`), so they come from the extension envelope and usually render
    // as "-" on the spec-shaped `/_arkret/describe`. `development_mode`
    // is rendered separately below (with red styling) so the operator
    // can't miss it.
    let proof_verifier_mode = describe
        .extra_str(&["proof_verifier_mode"])
        .unwrap_or_else(|| "-".to_string());
    let admin_auth_mode = describe
        .extra_str(&["admin_auth_mode"])
        .unwrap_or_else(|| "-".to_string());
    let development_mode_label = if describe.development_mode {
        t("server_status.dev_banner")
    } else {
        "production".to_string()
    };
    let development_mode_is_dev = describe.development_mode;
    // When development_mode + verified_profiles are both present,
    // the server is making contradictory claims. Surface a loud red
    // banner above the rest of the card.
    let dev_with_verified = describe.dev_mode_with_verified_profiles();

    // ServiceDescribe carries a typed `rate_limit_policy` (or
    // `rate_limit_policy_id`); there is no free-form top-level `rate_limit`.
    // Render it as JSON so the existing free-form panel keeps working.
    let rate_limit_value = describe
        .rate_limit_policy
        .as_ref()
        .map(|policy| serde_json::to_value(policy).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);
    let rate_limit_text = if rate_limit_value.is_null() {
        "-".to_string()
    } else {
        serde_json::to_string_pretty(&rate_limit_value)
            .unwrap_or_else(|_| rate_limit_value.to_string())
    };
    let limits_value =
        serde_json::to_value(&describe.limits).unwrap_or_else(|_| serde_json::json!({}));
    let limits_text = if limits_value
        .as_object()
        .is_none_or(serde_json::Map::is_empty)
    {
        "-".to_string()
    } else {
        serde_json::to_string_pretty(&limits_value).unwrap_or_else(|_| limits_value.to_string())
    };

    rsx! {
        div { class: "space-y-4",
            // Verified-profiles-in-dev-mode contradiction
            // banner. Renders ABOVE the dev-mode chip so operators can't
            // miss the most important wire-correctness issue.
            if dev_with_verified {
                div {
                    class: "rounded-md border-2 border-red-600 bg-red-600/15 px-3 py-2 text-sm font-semibold text-red-700 dark:text-red-200 space-y-1",
                    role: "alert",
                    p {
                        span { class: "mr-2", "\u{26A0}" }
                        {t("server_status.dev_with_verified_title")}
                    }
                    p { class: "text-xs font-normal",
                        {t("server_status.dev_with_verified_detail")}
                    }
                }
            }
            // Red posture chip — only renders when coland reports
            // `development_mode == true`. Production deployments see the
            // normal grid below with no extra chrome.
            if development_mode_is_dev {
                div {
                    class: "rounded-md border border-red-600 bg-red-600/10 px-3 py-2 text-sm font-semibold text-red-700 dark:text-red-300",
                    role: "alert",
                    span { class: "mr-2", "\u{26A0}" }
                    "{development_mode_label}"
                }
            }
            div { class: "grid gap-3 sm:grid-cols-2",
                {info_cell(t("server_status.service_id"), service_id)}
                {info_cell(t("server_status.service_did"), service_did)}
                {info_cell(t("server_status.resolution_history_head"), resolution_history_head)}
                {info_cell(t("server_status.resolution_version_id"), resolution_version_id)}
                {info_cell(t("server_status.trust_domain"), trust_domain)}
                {info_cell(t("server_status.service_kind"), service_kind)}
                {info_cell(t("server_status.protocol_version"), protocol)}
                {info_cell(t("server_status.openapi_version"), openapi)}
                {info_cell(t("server_status.schema_registry"), schema)}
                {info_cell(t("server_status.event_kind_registry"), event_kind)}
                {info_cell(t("server_status.proof_verifier_mode"), proof_verifier_mode)}
                {info_cell(t("server_status.admin_auth_mode"), admin_auth_mode)}
            }

            // Registered ServerDescribe declarations render as flat strips
            // of mono chips; conformance buckets render separately below.
            {chip_section(t("server_status.profiles_label"), &profiles)}
            {profile_declaration_section(&profiles)}
            {chip_section(t("server_status.features_label"), &features)}
            {chip_section(t("server_status.supported_operation_bundles"), &operation_bundles)}

            {binding_chip_section(t("server_status.transport_bindings"), &bindings)}

            // Limits + rate_limit as raw JSON. Free-form per
            // SDK; UI cannot assume a fixed key set.
            div { class: "grid gap-3 sm:grid-cols-2",
                div { class: "space-y-1",
                    p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                        {t("server_status.limits_label")}
                    }
                    pre { class: "font-mono text-[11px] bg-muted/50 rounded p-2 overflow-x-auto",
                        "{limits_text}"
                    }
                }
                {rate_limit_panel(t("server_status.rate_limit_label"), &rate_limit_value, rate_limit_text)}
            }

            // T6.2 §1 — conformance posture grouped into four buckets.
            // verified profiles flip to ❌ when development_mode is on,
            // since coland's relaxed proof verifier voids the
            // verification claim.
            {conformance_section(describe, development_mode_is_dev)}

            // T6.2 §6 — when dev posture is active, surface
            // plaintext_visibility in the same red card so the operator
            // sees every weak knob in one place.
            {dev_posture_card(describe)}
        }
    }
}

fn binding_chip_section(label: String, bindings: &[serde_json::Value]) -> Element {
    if bindings.is_empty() {
        return rsx! {
            div { class: "space-y-1",
                p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                    "{label}"
                }
                p { class: "text-xs text-muted-foreground", "-" }
            }
        };
    }

    rsx! {
        div { class: "space-y-1",
            p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                "{label}"
            }
            div { class: "flex flex-wrap gap-1",
                for binding in bindings.iter() {
                    {
                        let binding_type = binding
                            .get("type")
                            .or_else(|| binding.get("kind"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("binding");
                        let endpoint = binding
                            .get("endpoint")
                            .or_else(|| binding.get("url"))
                            .or_else(|| binding.get("service_endpoint"))
                            .and_then(|v| v.as_str());
                        let label = endpoint
                            .map(|e| format!("{binding_type}: {e}"))
                            .unwrap_or_else(|| binding_type.to_string());
                        rsx! {
                            Badge {
                                variant: BadgeVariant::Secondary,
                                class: "font-mono text-xs max-w-full truncate".to_string(),
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn rate_limit_panel(
    label: String,
    rate_limit: &serde_json::Value,
    fallback_text: String,
) -> Element {
    let bars = rate_limit_bars(rate_limit);

    rsx! {
        div { class: "space-y-2",
            p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                "{label}"
            }
            if bars.is_empty() {
                pre { class: "font-mono text-[11px] bg-muted/50 rounded p-2 overflow-x-auto",
                    "{fallback_text}"
                }
            } else {
                div { class: "space-y-2 rounded-md bg-muted/50 p-2",
                    for bar in bars.iter() {
                        {
                            let width = bar.percent.clamp(4, 100);
                            rsx! {
                                div { class: "space-y-1",
                                    div { class: "flex items-center justify-between gap-2 text-[11px]",
                                        span { class: "font-mono", "{bar.label}" }
                                        span { class: "text-muted-foreground", "{bar.value}" }
                                    }
                                    div {
                                        class: "h-2 overflow-hidden rounded-full bg-background",
                                        role: "meter",
                                        aria_valuemin: "0",
                                        aria_valuemax: "100",
                                        aria_valuenow: "{bar.percent}",
                                        aria_label: format!("rate limit {}", bar.label),
                                        div {
                                            class: "h-full rounded-full bg-primary",
                                            style: "width: {width}%;"
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
}

#[derive(Debug, Clone)]
struct RateLimitBar {
    label: String,
    value: String,
    percent: i32,
}

fn rate_limit_bars(rate_limit: &serde_json::Value) -> Vec<RateLimitBar> {
    let Some(obj) = rate_limit.as_object() else {
        return Vec::new();
    };
    if obj
        .get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return vec![RateLimitBar {
            label: "disabled".to_string(),
            value: "off".to_string(),
            percent: 100,
        }];
    }

    let keys = [
        "limit",
        "burst",
        "capacity",
        "tokens",
        "remaining",
        "requests",
    ];
    let mut bars = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(value) = obj.get(key).and_then(number_value) {
            let base = obj
                .get("limit")
                .and_then(number_value)
                .or_else(|| obj.get("capacity").and_then(number_value))
                .unwrap_or(value)
                .max(1.0);
            bars.push(RateLimitBar {
                label: key.to_string(),
                value: format_number(value),
                percent: ((value / base) * 100.0).round() as i32,
            });
        }
    }
    if let Some(window) = obj
        .get("window")
        .or_else(|| obj.get("window_seconds"))
        .or_else(|| obj.get("refill_interval_seconds"))
        .and_then(number_value)
    {
        bars.push(RateLimitBar {
            label: "window".to_string(),
            value: format!("{}s", format_number(window)),
            percent: 100,
        });
    }
    bars
}

fn number_value(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn format_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as u64)
    } else {
        format!("{value:.2}")
    }
}

/// Uniform helper for list-shaped ServerDescribe fields. Returns an empty
/// fragment when the slice is empty so the card does not gain empty rows.
fn chip_section(label: String, items: &[String]) -> Element {
    if items.is_empty() {
        return rsx! {
            div { class: "space-y-1",
                p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                    "{label}"
                }
                p { class: "text-xs text-muted-foreground", "-" }
            }
        };
    }
    rsx! {
        div { class: "space-y-1",
            p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                "{label}"
            }
            div { class: "flex flex-wrap gap-1",
                for item in items.iter() {
                    Badge { variant: BadgeVariant::Secondary, class: "font-mono text-xs".to_string(), "{item}" }
                }
            }
        }
    }
}

/// T6.2 §1 — render the four conformance buckets with their colour
/// codes. When `dev_mode_active` is true, verified profiles are crossed
/// out and labelled "unavailable in dev mode" because the relaxed
/// proof verifier breaks the verification chain.
fn conformance_section(describe: &ServerDescribeDocument, dev_mode_active: bool) -> Element {
    let verified = describe
        .verified_profiles
        .iter()
        .map(|profile| profile.profile_id.clone())
        .collect::<Vec<_>>();
    let claimed = describe.supported_profiles.clone();
    let interop = describe
        .interop_surfaces
        .iter()
        .map(|surface| surface.name.clone())
        .collect::<Vec<_>>();

    if verified.is_empty() && claimed.is_empty() && interop.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "space-y-3 pt-2 border-t border-border/40",
            p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                {t("server_status.conformance_label")}
            }
            if !verified.is_empty() {
                {conformance_bucket(
                    t("server_status.verified_profiles"),
                    &verified,
                    if dev_mode_active { ConformanceTone::VerifiedBlockedByDev } else { ConformanceTone::Verified },
                )}
            }
            if !claimed.is_empty() {
                {conformance_bucket(
                    t("server_status.claimed_profiles"),
                    &claimed,
                    ConformanceTone::Claimed,
                )}
            }
            if !interop.is_empty() {
                {conformance_bucket(
                    t("server_status.interop_surfaces"),
                    &interop,
                    ConformanceTone::Interop,
                )}
            }
        }
    }
}

#[derive(Copy, Clone)]
enum ConformanceTone {
    Verified,
    VerifiedBlockedByDev,
    Claimed,
    Interop,
}

fn conformance_bucket(label: String, items: &[String], tone: ConformanceTone) -> Element {
    let (chip_class, prefix, suffix_label) = match tone {
        ConformanceTone::Verified => (
            "bg-green-600/10 text-green-700 dark:text-green-300 border border-green-600/30",
            "\u{2713}",
            None,
        ),
        ConformanceTone::VerifiedBlockedByDev => (
            "bg-red-600/10 text-red-700 dark:text-red-300 border border-red-600/30 line-through",
            "\u{274C}",
            Some(t("server_status.verified_blocked_dev")),
        ),
        ConformanceTone::Claimed => (
            "bg-yellow-600/10 text-yellow-700 dark:text-yellow-300 border border-yellow-600/30",
            "\u{1F7E1}",
            None,
        ),
        ConformanceTone::Interop => (
            "bg-gray-500/10 text-gray-700 dark:text-gray-300 border border-gray-500/30",
            "\u{1F50C}",
            None,
        ),
    };

    rsx! {
        div { class: "space-y-1",
            p { class: "text-[11px] font-medium text-muted-foreground", "{label}" }
            div { class: "flex flex-wrap gap-1",
                for item in items.iter() {
                    span {
                        class: "inline-flex items-center gap-1 rounded-md px-2 py-0.5 font-mono text-xs {chip_class}",
                        span { "{prefix}" }
                        span { "{item}" }
                    }
                }
            }
            if let Some(suffix) = suffix_label.as_ref() {
                p { class: "text-[11px] italic text-muted-foreground", "{suffix}" }
            }
        }
    }
}

/// T6.2 §6 — surface every weak runtime knob as a red posture card.
/// Renders nothing on a clean production server.
fn dev_posture_card(describe: &ServerDescribeDocument) -> Element {
    let verifier_dev = describe
        .extra_str(&["proof_verifier_mode"])
        .map(|m| m.eq_ignore_ascii_case("development"))
        .unwrap_or(false);
    let admin_dev = describe
        .extra_str(&["admin_auth_mode"])
        .map(|m| m.eq_ignore_ascii_case("development"))
        .unwrap_or(false);
    let plaintext = describe.plaintext_visibility_entries();

    if !verifier_dev && !admin_dev && plaintext.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "rounded-md border border-red-600 bg-red-600/10 dark:bg-red-600/20 px-3 py-2 text-xs space-y-1",
            role: "alert",
            p { class: "font-semibold text-red-700 dark:text-red-300",
                {t("server_status.dev_posture_title")}
            }
            ul { class: "list-disc pl-5 text-red-700 dark:text-red-200",
                if verifier_dev {
                    li { {t("server_status.dev_posture_verifier")} }
                }
                if admin_dev {
                    li { {t("server_status.dev_posture_admin")} }
                }
                if !plaintext.is_empty() {
                    li {
                        {format!("{}: {}", t("server_status.dev_posture_plaintext"), plaintext.join(", "))}
                    }
                }
            }
        }
    }
}

/// Renders a single-line status row per known optional profile so the
/// operator can see at a glance which of them the server has declared in
/// `ak.server.read.describe.supported_profiles`. The list of known
/// profiles is held here (not in i18n) because the wire ids come from the
/// SDK's generated profile registry and the i18n value is only the human
/// label.
fn profile_declaration_section(profiles: &[String]) -> Element {
    // (wire profile id, i18n key for the description copy).
    let known: &[(&str, &str)] = &[
        (
            ProfileId::MEDIA_SERVICE_BINDING_V1,
            "server_status.profile.media_service_binding",
        ),
        (
            ProfileId::KEY_BACKUP_MEMORY_HARD_V1,
            "server_status.profile.key_backup_memory_hard",
        ),
    ];

    rsx! {
        div { class: "space-y-1",
            p { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
                {t("server_status.profile_declaration_title")}
            }
            ul { class: "text-xs space-y-1",
                for (wire, key) in known.iter() {
                    {
                        let declared = profiles.iter().any(|p| p == wire);
                        let copy = t(key);
                        rsx! {
                            li { class: "flex items-start gap-2",
                                if declared {
                                    span { class: "rounded bg-green-600/15 px-1.5 py-0.5 font-mono text-green-700 dark:text-green-300",
                                        "declared"
                                    }
                                } else {
                                    span { class: "rounded bg-muted px-1.5 py-0.5 font-mono text-muted-foreground",
                                        "absent"
                                    }
                                }
                                span { class: "font-mono", "{wire}" }
                                span { class: "text-muted-foreground", "— {copy}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
