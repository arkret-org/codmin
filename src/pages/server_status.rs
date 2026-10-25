use glory::prelude::*;

use crate::api::server;
use crate::components::ui::{self, View};
use crate::types::ServerDescribeDocument;
use crate::utils::i18n::t;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let info = ui::resource(scope, || async { server::get_server_info().await });
    let describe = ui::resource(scope, || async { server::get_server_describe().await });
    let status = ui::resource(scope, || async { server::get_server_status().await });
    let (refresh_info, refresh_describe, refresh_status) =
        (info.clone(), describe.clone(), status.clone());
    let info_theme = theme.clone();
    let describe_theme = theme.clone();
    let probe_theme = theme.clone();
    col()
        .class("admin-page server-status-page")
        .style(
            Style::new()
                .display(Display::Flex)
                .flex_direction(FlexDirection::Column)
                .gap(24.0),
        )
        .fill((
            row()
                .class("page-header")
                .style(Style {
                    display: Some(Display::Flex),
                    flex_direction: Some(FlexDirection::Row),
                    justify: Some(Justify::Between),
                    align: Some(Align::Start),
                    gap: Some(16.0),
                    ..Style::new()
                })
                .fill((
                    col()
                        .class("page-heading")
                        .style(
                            Style::new()
                                .display(Display::Flex)
                                .flex_direction(FlexDirection::Column)
                                .gap(6.0),
                        )
                        .fill((
                            ui::heading(t("server_status.title")),
                            sapid::text()
                                .theme(theme.clone())
                                .variant(sapid::TextVariant::Muted)
                                .content(t("server_status.subtitle")),
                        )),
                    sapid::button()
                        .theme(theme.clone())
                        .class("page-actions")
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("common.refresh"))
                        .on_press(move || {
                            refresh_info.reload();
                            refresh_describe.reload();
                            refresh_status.reload();
                        }),
                )),
            ui::resource_content(scope, theme.clone(), info, move |_, info| {
                Box::new(
                    sapid::card()
                        .theme(info_theme.clone())
                        .class("server-info-card")
                        .title(t("server_status.server_info"))
                        .fill(info_grid(
                            info_theme.clone(),
                            vec![
                                (t("server_status.version"), info.server_version),
                                (
                                    t("server_status.protocol_version"),
                                    info.protocol_version.unwrap_or_else(|| "-".into()),
                                ),
                                (
                                    t("server_status.server_name"),
                                    info.server_name.unwrap_or_else(|| "-".into()),
                                ),
                                (
                                    t("server_status.uptime"),
                                    info.uptime
                                        .map(|value| format!("{value}s"))
                                        .unwrap_or_else(|| "-".into()),
                                ),
                            ],
                            "server-info-grid",
                            4,
                        )),
                )
            }),
            col()
                .class("service-descriptions")
                .style(
                    Style::new()
                        .display(Display::Flex)
                        .flex_direction(FlexDirection::Column)
                        .gap(16.0),
                )
                .fill((
                    sapid::title(2)
                        .theme(theme.clone())
                        .font_size(20.0)
                        .weight(600.0)
                        .content(t("server_status.services_title")),
                    ui::resource_content(scope, theme.clone(), describe, move |_, describe| {
                        describe_card(describe_theme.clone(), &describe)
                    }),
                )),
            col()
                .class("node-probe")
                .style(
                    Style::new()
                        .display(Display::Flex)
                        .flex_direction(FlexDirection::Column)
                        .gap(16.0),
                )
                .fill((
                    sapid::title(2)
                        .theme(theme.clone())
                        .font_size(20.0)
                        .weight(600.0)
                        .content(t("server_status.probe_title")),
                    ui::resource_content(scope, theme, status, move |_, status| {
                        Box::new(
                            col()
                                .style(
                                    Style::new()
                                        .display(Display::Flex)
                                        .flex_direction(FlexDirection::Column)
                                        .gap(16.0),
                                )
                                .fill((
                                    row()
                                        .style(Style {
                                            display: Some(Display::Flex),
                                            flex_direction: Some(FlexDirection::Row),
                                            gap: Some(16.0),
                                            align: Some(Align::Center),
                                            ..Style::new()
                                        })
                                        .fill((
                                            status_badge(
                                                probe_theme.clone(),
                                                if status.is_ok() {
                                                    t("server_status.reachable")
                                                } else {
                                                    t("server_status.unreachable")
                                                },
                                                status.is_ok(),
                                            ),
                                            sapid::text()
                                                .theme(probe_theme.clone())
                                                .variant(sapid::TextVariant::Muted)
                                                .font_size(12.0)
                                                .content(t("server_status.probe_hint")),
                                        )),
                                    info_grid(
                                        probe_theme.clone(),
                                        vec![
                                            (
                                                t("server_status.probe_service_id"),
                                                status.service_id.to_string(),
                                            ),
                                            (t("server_status.probe_storage"), status.storage),
                                            (
                                                t("server_status.probe_checked_by"),
                                                status.checked_by.to_string(),
                                            ),
                                            (
                                                t("server_status.probe_generated_at"),
                                                status.generated_at,
                                            ),
                                            (
                                                t("server_status.probe_realms"),
                                                status.counts.realms.to_string(),
                                            ),
                                            (
                                                t("server_status.probe_accounts"),
                                                status
                                                    .counts
                                                    .accounts
                                                    .map(|value| value.to_string())
                                                    .unwrap_or_else(|| "-".into()),
                                            ),
                                            (
                                                t("server_status.probe_devices"),
                                                status
                                                    .counts
                                                    .devices
                                                    .map(|value| value.to_string())
                                                    .unwrap_or_else(|| "-".into()),
                                            ),
                                        ],
                                        "server-probe-grid",
                                        3,
                                    ),
                                )),
                        )
                    }),
                )),
        ))
}

pub(crate) fn info_grid(
    theme: sapid::Theme,
    values: Vec<(String, String)>,
    class: &str,
    columns: usize,
) -> View {
    Box::new(
        view()
            .class(class.to_string())
            .style(Style {
                display: Some(Display::Grid),
                grid_columns: Some(glory::style::GridTracks::Tracks(
                    vec![glory::style::GridTrack::Fr(1.0); columns],
                )),
                gap: Some(16.0),
                min_width: Some(Length::Px(0.0)),
                ..Style::new()
            })
            .fill(
                values
                    .into_iter()
                    .map(|(label, value)| {
                        col()
                            .class("server-info-cell")
                            .style(
                                Style::new()
                                    .display(Display::Flex)
                                    .flex_direction(FlexDirection::Column)
                                    .gap(4.0)
                                    .min_width(Length::Px(0.0)),
                            )
                            .fill((
                                sapid::text()
                                    .theme(theme.clone())
                                    .variant(sapid::TextVariant::Muted)
                                    .font_size(12.0)
                                    .content(label),
                                sapid::text()
                                    .theme(theme.clone())
                                    .font_size(14.0)
                                    .weight(600.0)
                                    .class("server-info-value")
                                    .content(value),
                            ))
                    })
                    .collect::<Vec<_>>(),
            ),
    )
}
pub(crate) fn status_badge(theme: sapid::Theme, label: String, ok: bool) -> View {
    Box::new(
        sapid::badge()
            .theme(theme.clone())
            .style(Style {
                color: Some(if ok { theme.success } else { theme.danger }),
                background: Some(if ok {
                    theme.success_soft
                } else {
                    theme.danger_soft
                }),
                ..Style::new()
            })
            .content(label),
    )
}
fn chips(theme: sapid::Theme, label: String, items: Vec<String>, tone: Option<bool>) -> View {
    let chips: Vec<View> = if items.is_empty() {
        vec![Box::new(text().content("-"))]
    } else {
        items
            .into_iter()
            .map(|item| {
                if let Some(ok) = tone {
                    status_badge(theme.clone(), item, ok)
                } else {
                    Box::new(sapid::badge().theme(theme.clone()).content(item)) as View
                }
            })
            .collect()
    };
    Box::new(
        col()
            .class("service-chip-section")
            .style(
                Style::new()
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Column)
                    .gap(6.0),
            )
            .fill((
                sapid::text()
                    .theme(theme)
                    .variant(sapid::TextVariant::Muted)
                    .font_size(12.0)
                    .weight(600.0)
                    .content(label),
                row()
                    .style(Style {
                        display: Some(Display::Flex),
                        flex_direction: Some(FlexDirection::Row),
                        flex_wrap: Some(glory::style::FlexWrap::Wrap),
                        gap: Some(6.0),
                        ..Style::new()
                    })
                    .fill(chips),
            )),
    )
}
pub(crate) fn describe_card(theme: sapid::Theme, describe: &ServerDescribeDocument) -> View {
    let mut body: Vec<View> = vec![];
    if describe.dev_mode_with_verified_profiles() {
        body.push(Box::new(
            sapid::alert("dev-verified-conflict")
                .theme(theme.clone())
                .severity(sapid::AlertSeverity::Danger)
                .title(t("server_status.dev_with_verified_title"))
                .content(t("server_status.dev_with_verified_detail")),
        ));
    }
    if describe.development_mode {
        body.push(Box::new(
            sapid::alert("development-mode")
                .theme(theme.clone())
                .severity(sapid::AlertSeverity::Danger)
                .content(t("server_status.dev_banner")),
        ));
    }
    let ext = |key: &str| describe.extra_str(&[key]).unwrap_or_else(|| "-".into());
    body.push(info_grid(
        theme.clone(),
        vec![
            (
                t("server_status.service_id"),
                describe.service_id.to_string(),
            ),
            (
                t("server_status.service_did"),
                describe.service_resolution.did.to_string(),
            ),
            (
                t("server_status.resolution_history_head"),
                describe.service_resolution.method_history_head.clone(),
            ),
            (
                t("server_status.resolution_version_id"),
                describe.service_resolution.version_id.clone(),
            ),
            (
                t("server_status.trust_domain"),
                describe.trust_domain.to_string(),
            ),
            (
                t("server_status.service_kind"),
                describe.service_kind.as_str().to_string(),
            ),
            (
                t("server_status.protocol_version"),
                describe.protocol_version.to_string(),
            ),
            (t("server_status.openapi_version"), ext("openapi_version")),
            (
                t("server_status.schema_registry"),
                ext("schema_registry_version"),
            ),
            (
                t("server_status.event_kind_registry"),
                ext("event_kind_registry_version"),
            ),
            (
                t("server_status.proof_verifier_mode"),
                ext("proof_verifier_mode"),
            ),
            (t("server_status.admin_auth_mode"), ext("admin_auth_mode")),
        ],
        "service-description-grid",
        2,
    ));
    body.push(chips(
        theme.clone(),
        t("server_status.profiles_label"),
        describe.supported_profiles.clone(),
        None,
    ));
    let known = [
        (
            arkret_wire::ProfileId::MEDIA_SERVICE_BINDING_V1,
            "server_status.profile.media_service_binding",
        ),
        (
            arkret_wire::ProfileId::KEY_BACKUP_MEMORY_HARD_V1,
            "server_status.profile.key_backup_memory_hard",
        ),
    ];
    body.push(Box::new(
        col()
            .class("optional-profile-declarations")
            .style(
                Style::new()
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Column)
                    .gap(6.0),
            )
            .fill((
                sapid::text()
                    .theme(theme.clone())
                    .font_size(12.0)
                    .weight(600.0)
                    .content(t("server_status.profile_declaration_title")),
                col()
                    .style(
                        Style::new()
                            .display(Display::Flex)
                            .flex_direction(FlexDirection::Column)
                            .gap(6.0),
                    )
                    .fill(
                        known
                            .into_iter()
                            .map(|(wire, key)| {
                                row()
                                    .style(Style {
                                        display: Some(Display::Flex),
                                        flex_direction: Some(FlexDirection::Row),
                                        gap: Some(8.0),
                                        align: Some(Align::Start),
                                        ..Style::new()
                                    })
                                    .fill((
                                        status_badge(
                                            theme.clone(),
                                            if describe
                                                .supported_profiles
                                                .iter()
                                                .any(|profile| profile == wire)
                                            {
                                                "declared".into()
                                            } else {
                                                "absent".into()
                                            },
                                            describe
                                                .supported_profiles
                                                .iter()
                                                .any(|profile| profile == wire),
                                        ),
                                        sapid::text()
                                            .theme(theme.clone())
                                            .font_size(12.0)
                                            .content(t(key)),
                                    ))
                            })
                            .collect::<Vec<_>>(),
                    ),
            )),
    ));
    body.push(chips(
        theme.clone(),
        t("server_status.features_label"),
        describe.supported_features.clone(),
        None,
    ));
    body.push(chips(
        theme.clone(),
        t("server_status.supported_operation_bundles"),
        describe.supported_operation_bundles.clone(),
        None,
    ));
    body.push(chips(
        theme.clone(),
        t("server_status.transport_bindings"),
        describe
            .transport_bindings
            .iter()
            .map(|binding| serde_json::to_string(binding).unwrap_or_default())
            .collect(),
        None,
    ));
    let limits = serde_json::to_string_pretty(&describe.limits).unwrap_or_else(|_| "-".into());
    let rate = describe
        .rate_limit_policy
        .as_ref()
        .and_then(|policy| serde_json::to_value(policy).ok())
        .unwrap_or_default();
    body.push(Box::new(
        view()
            .class("service-limits-grid")
            .style(Style {
                display: Some(Display::Grid),
                grid_columns: Some(glory::style::GridTracks::Tracks(
                    vec![glory::style::GridTrack::Fr(1.0); 2],
                )),
                gap: Some(16.0),
                ..Style::new()
            })
            .fill((
                code_panel(theme.clone(), t("server_status.limits_label"), limits),
                rate_panel(theme.clone(), &rate),
            )),
    ));
    body.push(chips(
        theme.clone(),
        t("server_status.verified_profiles"),
        describe
            .verified_profiles
            .iter()
            .map(|profile| profile.profile_id.clone())
            .collect(),
        Some(!describe.development_mode),
    ));
    if describe.development_mode && !describe.verified_profiles.is_empty() {
        body.push(Box::new(
            text().content(t("server_status.verified_blocked_dev")),
        ));
    }
    body.push(chips(
        theme.clone(),
        t("server_status.claimed_profiles"),
        describe.supported_profiles.clone(),
        None,
    ));
    body.push(chips(
        theme.clone(),
        t("server_status.interop_surfaces"),
        describe
            .interop_surfaces
            .iter()
            .map(|surface| surface.name.clone())
            .collect(),
        None,
    ));
    let mut weak = vec![];
    if ext("proof_verifier_mode").eq_ignore_ascii_case("development") {
        weak.push(t("server_status.dev_posture_verifier"));
    }
    if ext("admin_auth_mode").eq_ignore_ascii_case("development") {
        weak.push(t("server_status.dev_posture_admin"));
    }
    let plaintext = describe.plaintext_visibility_entries();
    if !plaintext.is_empty() {
        weak.push(format!(
            "{}: {}",
            t("server_status.dev_posture_plaintext"),
            plaintext.join(", ")
        ));
    }
    if !weak.is_empty() {
        body.push(Box::new(
            sapid::alert("runtime-posture")
                .theme(theme.clone())
                .severity(sapid::AlertSeverity::Danger)
                .title(t("server_status.dev_posture_title"))
                .content(weak.join("\n")),
        ));
    }
    Box::new(
        sapid::card()
            .theme(theme.clone())
            .class("service-description-card")
            .title(t("server_status.service_admin"))
            .description(t("server_status.service_admin_hint"))
            .header(status_badge(theme, t("server_status.online"), true))
            .fill(
                col()
                    .style(
                        Style::new()
                            .display(Display::Flex)
                            .flex_direction(FlexDirection::Column)
                            .gap(16.0),
                    )
                    .fill(body),
            ),
    )
}
fn code_panel(theme: sapid::Theme, title: String, value: String) -> View {
    Box::new(
        col()
            .class("server-code-panel")
            .style(
                Style::new()
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Column)
                    .gap(6.0)
                    .min_width(Length::Px(0.0)),
            )
            .fill((
                sapid::text()
                    .theme(theme.clone())
                    .font_size(12.0)
                    .weight(600.0)
                    .content(title),
                text()
                    .style(Style {
                        font_size: Some(11.0),
                        line_height: Some(16.0),
                        white_space: Some(WhiteSpace::PreWrap),
                        background: Some(theme.canvas),
                        padding: Some(Edges::all(8.0)),
                        radius: Some(4.0),
                        ..Style::new()
                    })
                    .content(value),
            )),
    )
}

fn rate_panel(theme: sapid::Theme, value: &serde_json::Value) -> View {
    let mut children = vec![code_panel(
        theme.clone(),
        t("server_status.rate_limit_label"),
        if value.is_null() {
            "-".into()
        } else {
            serde_json::to_string_pretty(value).unwrap_or_default()
        },
    )];
    if let Some(fields) = value.as_object() {
        let number = |value: &serde_json::Value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|value| value.parse::<f64>().ok()))
        };
        let base = fields
            .get("limit")
            .and_then(number)
            .or_else(|| fields.get("capacity").and_then(number));
        for key in [
            "limit",
            "burst",
            "capacity",
            "tokens",
            "remaining",
            "requests",
        ] {
            if let Some(value) = fields.get(key).and_then(number) {
                let percent = (value / base.unwrap_or(value).max(1.0) * 100.0).clamp(0.0, 100.0);
                children.push(Box::new(
                    col()
                        .style(
                            Style::new()
                                .display(Display::Flex)
                                .flex_direction(FlexDirection::Column)
                                .gap(4.0),
                        )
                        .fill((
                            sapid::text()
                                .theme(theme.clone())
                                .font_size(11.0)
                                .content(format!("{key}: {value}")),
                            sapid::progress(percent)
                                .theme(theme.clone())
                                .name(format!("rate limit {key}"))
                                .width(Length::Percent(100.0)),
                        )),
                ));
            }
        }
        if let Some(window) = fields
            .get("window")
            .or_else(|| fields.get("window_seconds"))
            .or_else(|| fields.get("refill_interval_seconds"))
            .and_then(number)
        {
            children.push(Box::new(
                sapid::text()
                    .theme(theme)
                    .font_size(11.0)
                    .content(format!("window: {window}s")),
            ));
        }
    }
    Box::new(
        col()
            .class("rate-limit-panel")
            .style(
                Style::new()
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Column)
                    .gap(8.0),
            )
            .fill(children),
    )
}
