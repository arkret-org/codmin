use glory::prelude::*;

use crate::components::ui::View;
use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    col().class("station-page").fill((
        station::read(scope, theme.clone(), t("dashboard.title"), || async {
            Document::default().section(
                "Server statistics",
                &crate::api::server::get_server_stats().await?,
            )
        }),
        station::read(
            scope,
            theme.clone(),
            t("server_status.server_info"),
            || async {
                Document::default().section(
                    "Server information",
                    &crate::api::server::get_server_info().await?,
                )
            },
        ),
        description(scope, theme.clone()),
        station::read(scope, theme, t("server_status.probe_title"), || async {
            Document::default().section(
                "Admin endpoint reachability",
                &crate::api::server::get_server_status().await?,
            )
        }),
    ))
}
pub(crate) fn description(scope: &Scope, theme: sapid::Theme) -> View {
    station::read(scope, theme, t("server_status.services_title"), || async {
        let info = crate::api::server::get_server_describe().await?;
        let warning = if info.dev_mode_with_verified_profiles() {
            "WARNING: Development mode contradicts the advertised verified profiles."
        } else if info.development_mode {
            "Development mode is enabled. Relaxed verification must not be treated as production conformance."
        } else {
            "Production verification mode"
        };
        Document::default()
            .section("Verification status", &warning)?
            .section("Service description", &info)
    })
}
