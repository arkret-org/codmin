use glory::prelude::*;

use crate::api::server;
use crate::components::ui::{self, View};
use crate::types::{AdminServerInfo, AdminServerStats, AdminServerStatus, ServerDescribeDocument};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;
#[derive(Clone)]
struct DashboardData {
    stats: std::result::Result<AdminServerStats, HttpError>,
    info: std::result::Result<AdminServerInfo, HttpError>,
    describe: std::result::Result<ServerDescribeDocument, HttpError>,
    status: std::result::Result<AdminServerStatus, HttpError>,
}
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let data = ui::resource(scope, || async {
        Ok(DashboardData {
            stats: server::get_server_stats().await,
            info: server::get_server_info().await,
            describe: server::get_server_describe().await,
            status: server::get_server_status().await,
        })
    });
    let refresh = data.clone();
    let theme_content = theme.clone();
    col().class("admin-page dashboard-page").fill((
        row().class("page-header").fill((
            col().class("page-heading").fill((
                ui::heading(t("dashboard.title")),
                text()
                    .class("page-description")
                    .content(t("dashboard.welcome")),
            )),
            sapid::button()
                .theme(theme.clone())
                .variant(sapid::ButtonVariant::Secondary)
                .size(sapid::Size::Small)
                .label(t("common.refresh"))
                .on_press(move || refresh.reload()),
        )),
        ui::resource_content(scope, theme.clone(), data, move |_, data| {
            dashboard_content(theme_content.clone(), data)
        }),
    ))
}
fn metric(value: String, title: String, subtitle: String, icon: &str, online: bool) -> View {
    Box::new(
        col().class("stat-cell").fill((
            row()
                .class("stat-heading")
                .fill((text().content(title), crate::appearance::icon(icon))),
            text().class("stat-value").content(value),
            text()
                .class(if online {
                    "stat-caption online"
                } else {
                    "stat-caption"
                })
                .content(subtitle),
        )),
    )
}
fn dashboard_content(theme: sapid::Theme, data: DashboardData) -> View {
    let stats = data.stats.as_ref().ok();
    let info = data.info.as_ref().ok();
    let describe = data.describe.as_ref().ok();
    let version = info
        .map(|i| i.server_version.clone())
        .unwrap_or_else(|| "—".into());
    let avg = crate::utils::net::perf::average_latency();
    let mut children: Vec<View> = vec![];
    for error in [
        data.stats.as_ref().err(),
        data.info.as_ref().err(),
        data.describe.as_ref().err(),
        data.status.as_ref().err(),
    ]
    .into_iter()
    .flatten()
    {
        children.push(Box::new(
            sapid::alert("dashboard-error")
                .theme(theme.clone())
                .severity(sapid::AlertSeverity::Danger)
                .content(error.message.clone()),
        ));
    }
    if describe.is_some_and(|d| d.development_mode) {
        children.push(Box::new(
            sapid::alert("dashboard-development")
                .theme(theme.clone())
                .severity(sapid::AlertSeverity::Warning)
                .content(t("server_status.dev_banner_dashboard")),
        ));
    }
    children.push(Box::new(view().class("dashboard-stats").fill(vec![
            metric(
                version.clone(),
                t("server.version"),
                t("dashboard.server_online"),
                "server",
                info.is_some(),
            ),
            metric(
                stats
                    .map(|s| s.actor_count.to_string())
                    .unwrap_or_else(|| "—".into()),
                t("nav.actors"),
                t("dashboard.total_actors"),
                "users",
                false,
            ),
            metric(
                stats
                    .map(|s| s.realm_count.to_string())
                    .unwrap_or_else(|| "—".into()),
                t("nav.realms"),
                t("dashboard.total_realms"),
                "shield",
                false,
            ),
            metric(
                stats
                    .map(|s| s.active_actor_count.to_string())
                    .unwrap_or_else(|| "—".into()),
                t("dashboard.active_users"),
                t("dashboard.active_actors"),
                "users",
                false,
            ),
            metric(
                stats
                    .map(|s| s.federation_peer_count.to_string())
                    .unwrap_or_else(|| "—".into()),
                t("nav.federation"),
                t("dashboard.federation_peers"),
                "globe",
                false,
            ),
            metric(
                if avg > 0. {
                    format!("{avg:.0}ms")
                } else {
                    "—".into()
                },
                t("dashboard.api_latency"),
                t("dashboard.avg_api_response"),
                "activity",
                false,
            ),
        ])));
    if let Some(info) = info {
        children.push(card(
            theme.clone(),
            t("dashboard.server_info"),
            None,
            vec![
                (t("server.version"), version),
                (
                    t("dashboard.protocol_version"),
                    describe
                        .map(|d| d.protocol_version.to_string())
                        .or_else(|| info.protocol_version.clone())
                        .unwrap_or_else(|| "—".into()),
                ),
                (
                    t("dashboard.server_name"),
                    info.server_name.clone().unwrap_or_else(|| "—".into()),
                ),
            ],
            "metadata-grid three-columns server-summary-grid",
        ));
    }
    let known = |value: Option<String>| value.unwrap_or_else(|| "—".into());
    children.push(card(
        theme.clone(),
        t("server.features"),
        Some(t("dashboard.discovery_source")),
        vec![
            (
                t("dashboard.station_did"),
                known(describe.map(|d| d.service_resolution.did.to_string())),
            ),
            (
                t("dashboard.coauth_issuer_did"),
                known(describe.and_then(coauth_issuer_did)),
            ),
            (
                t("dashboard.delegated_resolver"),
                known(describe.and_then(identity_registry_endpoint)),
            ),
            (
                t("dashboard.supported_profiles"),
                known(describe.map(|d| join_or_dash(&d.supported_profiles))),
            ),
            (
                t("dashboard.schema_registry"),
                known(describe.and_then(schema_registry_version)),
            ),
            (
                t("dashboard.event_kind_registry"),
                known(describe.and_then(event_kind_registry_version)),
            ),
            (
                t("dashboard.openapi_version"),
                known(describe.and_then(|d| d.extra_str(&["openapi_version"]))),
            ),
        ],
        "metadata-grid four-columns",
    ));
    let verified = describe
        .map(|d| {
            d.verified_profiles
                .iter()
                .map(|p| p.profile_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let interop = describe
        .map(|d| {
            d.interop_surfaces
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    children.push(card(
        theme,
        t("dashboard.conformance_status"),
        Some(t("dashboard.conformance_desc")),
        vec![
            (
                t("dashboard.overall_conformance"),
                known(describe.and_then(conformance_level)),
            ),
            (
                t("dashboard.health"),
                health_summary(data.status.as_ref().ok()),
            ),
            (t("dashboard.verified_profiles"), join_or_dash(&verified)),
            (
                t("dashboard.claimed_profiles"),
                describe
                    .map(|d| join_or_dash(&d.supported_profiles))
                    .unwrap_or_else(|| "—".into()),
            ),
            (t("dashboard.interop_surfaces"), join_or_dash(&interop)),
        ],
        "metadata-grid conformance-grid",
    ));
    Box::new(col().class("dashboard-content").fill(children))
}
fn card(
    theme: sapid::Theme,
    title: String,
    description: Option<String>,
    values: Vec<(String, String)>,
    class: &str,
) -> View {
    let mut children: Vec<View> = vec![];
    if let Some(description) = description {
        children.push(Box::new(
            text().class("page-description").content(description),
        ));
    }
    children.push(Box::new(
        view().class(class.to_owned()).fill(
            values
                .into_iter()
                .map(|(label, value)| {
                    col().class("metadata-cell").fill((
                        text().class("metadata-label").content(label),
                        text().class("metadata-value").content(value),
                    ))
                })
                .collect::<Vec<_>>(),
        ),
    ));
    Box::new(
        sapid::card()
            .theme(theme)
            .class("dashboard-card")
            .title(title)
            .fill(children),
    )
}
pub(crate) fn description(scope: &Scope, theme: sapid::Theme) -> View {
    let resource = ui::resource(scope, || async { server::get_server_describe().await });
    ui::resource_view(scope, theme.clone(), resource, move |_, describe| {
        card(
            theme.clone(),
            t("server.features"),
            Some(t("dashboard.discovery_source")),
            vec![
                (
                    t("dashboard.station_did"),
                    describe.service_resolution.did.to_string(),
                ),
                (
                    t("dashboard.protocol_version"),
                    describe.protocol_version.to_string(),
                ),
                (
                    t("dashboard.supported_profiles"),
                    join_or_dash(&describe.supported_profiles),
                ),
            ],
            "metadata-grid three-columns",
        )
    })
}
fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn join_or_dash(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

fn coauth_issuer_did(describe: &ServerDescribeDocument) -> Option<String> {
    // `auth_metadata` is the SDK-canonical `AuthMetadata`; coauth carries its
    // issuer DID in the type's flattened `extra` (`additionalProperties`), so
    // serialize to JSON before walking for the `issuer_did` key.
    let auth_metadata = serde_json::to_value(&describe.auth_metadata).unwrap_or_default();
    json_string(&auth_metadata, &["issuer_did"])
        .or_else(|| non_empty(describe.service_resolution.did.to_string()))
}

fn identity_registry_endpoint(describe: &ServerDescribeDocument) -> Option<String> {
    // coauth-proprietary top-level extension block (not part of the spec
    // ServiceDescribe shape) — read via the extension envelope.
    describe.extra_str(&[
        "x_coauth_identity_registry_resolver",
        "delegated_resolver",
        "resolver",
    ])
}

fn schema_registry_version(describe: &ServerDescribeDocument) -> Option<String> {
    describe.extra_str(&["schema_registry_version"])
}

fn event_kind_registry_version(describe: &ServerDescribeDocument) -> Option<String> {
    describe
        .extra_str(&["event_kind_registry_version"])
        .or_else(|| describe.extra_str(&["registry", "event_kind_registry_version"]))
}

fn conformance_level(describe: &ServerDescribeDocument) -> Option<String> {
    serde_json::to_value(&describe.limits)
        .ok()
        .and_then(|limits| json_string(&limits, &["profile_status", "conformance"]))
}

/// `GET /_coland/admin/server/status` is a reachability probe: it answers
/// `ok` or it fails the request. It runs no component health checks, so the
/// dashboard reports reachability and never invents a component roll-up.
fn health_summary(status: Option<&AdminServerStatus>) -> String {
    match status {
        Some(status) if status.is_ok() => t("dashboard.health_healthy"),
        Some(_) => t("dashboard.health_issues_detected"),
        None => t("common.unknown"),
    }
}

fn json_string(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToOwned::to_owned)
}
