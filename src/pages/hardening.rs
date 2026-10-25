use glory::prelude::*;

use super::server_status::status_badge;
use crate::api::server::{self, HealthEnvelope};
use crate::components::ui::{self, Resource, View};
use crate::types::api::HardeningStatus;
use crate::utils::i18n::t;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let coland = ui::resource(scope, || async { server::get_coland_health().await });
    let configured = crate::utils::net::session::has_coauth();
    let coauth = ui::resource(scope, move || async move {
        if configured {
            server::get_coauth_health().await
        } else {
            Ok(HealthEnvelope::default())
        }
    });
    let aggregate = scope.cage((0u32, 0u32, false));
    let (station_data, auth_data) = (coland.data, coauth.data);
    scope.effect(move || {
        let mut score = 0;
        let mut max = 0;
        let mut warning = false;
        for state in [station_data.get(), auth_data.get()]
            .into_iter()
            .flatten()
            .flatten()
        {
            if let Some(state) = state.hardening {
                score += state.checklist_score;
                max += state.checklist_max;
                warning |= !state.warnings.is_empty();
            }
        }
        aggregate.revise(|value| *value = (score, max, warning));
    });
    let (refresh_station, refresh_auth) = (coland.clone(), coauth.clone());
    let summary_theme = theme.clone();
    col()
        .class("admin-page hardening-page")
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
                            ui::heading(t("hardening.title")),
                            sapid::text()
                                .theme(theme.clone())
                                .variant(sapid::TextVariant::Muted)
                                .content(t("hardening.subtitle")),
                        )),
                    sapid::button()
                        .theme(theme.clone())
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("common.refresh"))
                        .on_press(move || {
                            refresh_station.reload();
                            refresh_auth.reload();
                        }),
                )),
            ui::reactive(aggregate, move |_, (score, max, warnings)| {
                let caption = if max == 0 {
                    t("hardening.no_services_reporting")
                } else if score == max && !warnings {
                    t("hardening.all_passing")
                } else if score == 0 {
                    t("hardening.all_failing")
                } else {
                    t("hardening.some_failing")
                };
                Box::new(
                    sapid::card()
                        .theme(summary_theme.clone())
                        .class("hardening-aggregate")
                        .style(Style::new().padding(Edges::all(16.0)))
                        .fill(
                            row()
                                .style(Style {
                                    display: Some(Display::Flex),
                                    flex_direction: Some(FlexDirection::Row),
                                    justify: Some(Justify::Between),
                                    align: Some(Align::Center),
                                    gap: Some(16.0),
                                    flex_wrap: Some(glory::style::FlexWrap::Wrap),
                                    ..Style::new()
                                })
                                .fill((
                                    col()
                                        .style(
                                            Style::new()
                                                .display(Display::Flex)
                                                .flex_direction(FlexDirection::Column)
                                                .gap(4.0),
                                        )
                                        .fill((
                                            sapid::text()
                                                .theme(summary_theme.clone())
                                                .font_size(12.0)
                                                .weight(600.0)
                                                .variant(sapid::TextVariant::Muted)
                                                .content(t("hardening.aggregate_score")),
                                            sapid::title(2)
                                                .theme(summary_theme.clone())
                                                .font_size(24.0)
                                                .weight(700.0)
                                                .content(format!("{score} / {max}")),
                                            sapid::text()
                                                .theme(summary_theme.clone())
                                                .font_size(12.0)
                                                .variant(sapid::TextVariant::Muted)
                                                .content(t("hardening.aggregate_score_hint")),
                                        )),
                                    if max == 0 {
                                        Box::new(
                                            sapid::badge()
                                                .theme(summary_theme.clone())
                                                .content(caption),
                                        ) as View
                                    } else {
                                        status_badge(
                                            summary_theme.clone(),
                                            caption,
                                            score == max && !warnings,
                                        )
                                    },
                                )),
                        ),
                )
            }),
            view()
                .class("hardening-services-grid")
                .style(Style {
                    display: Some(Display::Grid),
                    grid_columns: Some(glory::style::GridTracks::Tracks(
                        vec![glory::style::GridTrack::Fr(1.0); 2],
                    )),
                    gap: Some(16.0),
                    ..Style::new()
                })
                .fill((
                    service_card(
                        theme.clone(),
                        coland,
                        "coland",
                        "hardening.svc_coland",
                        "hardening.svc_coland_desc",
                        true,
                    ),
                    service_card(
                        theme,
                        coauth,
                        "coauth",
                        "hardening.svc_coauth",
                        "hardening.svc_coauth_desc",
                        configured,
                    ),
                )),
        ))
}
fn service_card(
    theme: sapid::Theme,
    source: Resource<HealthEnvelope>,
    slug: &'static str,
    label: &'static str,
    description: &'static str,
    configured: bool,
) -> View {
    let retry = source.clone();
    ui::reactive(source.data, move |_, result| {
        let (badge, body): (View, View) = if !configured {
            (
                Box::new(
                    sapid::badge()
                        .theme(theme.clone())
                        .content(t("hardening.not_configured")),
                ),
                Box::new(text().content(t("hardening.coauth_not_configured"))),
            )
        } else {
            match result {
                None => (
                    Box::new(
                        sapid::badge()
                            .theme(theme.clone())
                            .content(t("common.loading")),
                    ),
                    Box::new(text().content(t("common.loading"))),
                ),
                Some(Err(error)) => {
                    let retry = retry.clone();
                    (
                        status_badge(theme.clone(), t("hardening.unreachable"), false),
                        Box::new(
                            col()
                                .style(
                                    Style::new()
                                        .display(Display::Flex)
                                        .flex_direction(FlexDirection::Column)
                                        .gap(12.0),
                                )
                                .fill((
                                    sapid::alert(format!("health-error-{slug}"))
                                        .theme(theme.clone())
                                        .severity(sapid::AlertSeverity::Danger)
                                        .title(t("hardening.fetch_failed"))
                                        .content(error.message),
                                    sapid::button()
                                        .theme(theme.clone())
                                        .label(t("common.retry"))
                                        .on_press(move || retry.reload()),
                                )),
                        ),
                    )
                }
                Some(Ok(health)) => match health.hardening {
                    None => (
                        Box::new(
                            sapid::badge()
                                .theme(theme.clone())
                                .content(t("hardening.no_data")),
                        ),
                        Box::new(text().content(t("hardening.no_hardening_block"))),
                    ),
                    Some(state) => {
                        let ok = state.warnings.is_empty();
                        let caption = if ok {
                            t("hardening.ok")
                        } else {
                            format!(
                                "{} {}",
                                state.warnings.len(),
                                t("hardening.warnings_suffix")
                            )
                        };
                        (
                            status_badge(theme.clone(), caption, ok),
                            checklist(theme.clone(), slug, state),
                        )
                    }
                },
            }
        };
        Box::new(
            sapid::card()
                .theme(theme.clone())
                .class("hardening-service-card")
                .header(
                    row()
                        .style(Style {
                            display: Some(Display::Flex),
                            flex_direction: Some(FlexDirection::Row),
                            justify: Some(Justify::Between),
                            gap: Some(12.0),
                            align: Some(Align::Start),
                            ..Style::new()
                        })
                        .fill((
                            col()
                                .style(
                                    Style::new()
                                        .display(Display::Flex)
                                        .flex_direction(FlexDirection::Column)
                                        .gap(4.0),
                                )
                                .fill((
                                    sapid::title(2)
                                        .theme(theme.clone())
                                        .font_size(18.0)
                                        .weight(600.0)
                                        .content(t(label)),
                                    sapid::text()
                                        .theme(theme.clone())
                                        .font_size(12.0)
                                        .variant(sapid::TextVariant::Muted)
                                        .content(t(description)),
                                    sapid::text()
                                        .theme(theme.clone())
                                        .font_size(10.0)
                                        .variant(sapid::TextVariant::Muted)
                                        .content(slug),
                                )),
                            badge,
                        )),
                )
                .fill(body),
        )
    })
}
fn checklist(theme: sapid::Theme, slug: &str, state: HardeningStatus) -> View {
    let admin = if state.admin_auth_mode.trim().is_empty() {
        "-".into()
    } else {
        state.admin_auth_mode
    };
    let rotation = if state.provider_credential_rotation.trim().is_empty() {
        "-".into()
    } else {
        state.provider_credential_rotation
    };
    let checks = [
        ("hardening.check_dev_mode_disabled", !state.development_mode),
        ("hardening.check_tls_enabled", state.tls_enabled),
        (
            "hardening.check_pq_hybrid_tls",
            state.pq_hybrid_tls_probe_verified,
        ),
        (
            "hardening.check_csp_configured",
            state.csp_header_configured,
        ),
        ("hardening.check_cors_strict", state.cors_strict),
        (
            "hardening.check_secret_manager",
            state.secret_manager_in_use,
        ),
        ("hardening.check_log_redaction", state.log_redaction_enabled),
        (
            "hardening.check_admin_auth_prod",
            !matches!(admin.as_str(), "development" | "closed" | "-"),
        ),
        ("hardening.check_rate_limit", state.rate_limit_enabled),
        (
            "hardening.check_credential_rotation",
            !matches!(rotation.as_str(), "none" | "-"),
        ),
    ];
    let mut parts: Vec<View> = vec![
        Box::new(
            row()
                .style(Style {
                    display: Some(Display::Flex),
                    flex_direction: Some(FlexDirection::Row),
                    justify: Some(Justify::Between),
                    ..Style::new()
                })
                .fill((
                    sapid::text()
                        .theme(theme.clone())
                        .font_size(12.0)
                        .weight(600.0)
                        .content(t("hardening.score")),
                    text().content(format!(
                        "{} / {}",
                        state.checklist_score, state.checklist_max
                    )),
                )),
        ),
        Box::new(
            view()
                .class("hardening-checklist-grid")
                .style(Style {
                    display: Some(Display::Grid),
                    grid_columns: Some(glory::style::GridTracks::Tracks(
                        vec![glory::style::GridTrack::Fr(1.0); 2],
                    )),
                    gap: Some(6.0),
                    ..Style::new()
                })
                .fill(
                    checks
                        .into_iter()
                        .map(|(key, ok)| {
                            status_badge(
                                theme.clone(),
                                format!("{} {}", if ok { "✓" } else { "✗" }, t(key)),
                                ok,
                            )
                        })
                        .collect::<Vec<_>>(),
                ),
        ),
        super::server_status::info_grid(
            theme.clone(),
            vec![
                ("admin_auth_mode".into(), admin),
                ("provider_credential_rotation".into(), rotation),
            ],
            "hardening-mode-grid",
            2,
        ),
    ];
    if !state.warnings.is_empty() {
        parts.push(Box::new(
            sapid::alert(format!("hardening-warnings-{slug}"))
                .theme(theme)
                .severity(sapid::AlertSeverity::Danger)
                .title(t("hardening.failing_checks"))
                .content(state.warnings.join("\n")),
        ));
    }
    Box::new(
        col()
            .style(
                Style::new()
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Column)
                    .gap(12.0),
            )
            .fill(parts),
    )
}
