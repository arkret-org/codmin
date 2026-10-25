use glory::prelude::*;

use crate::components::ui::{self, View};
use crate::pages::station;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let coland = ui::resource(scope, || async {
        crate::api::server::get_coland_health().await
    });
    let coauth = ui::resource(scope, || async {
        if crate::utils::net::session::has_coauth() {
            crate::api::server::get_coauth_health().await
        } else {
            Ok(crate::api::server::HealthEnvelope::default())
        }
    });
    let score_coland = coland.data;
    let score_coauth = coauth.data;
    let score = scope.bond(move || {
        let states = [score_coland.get(), score_coauth.get()];
        let mut score = 0;
        let mut max = 0;
        let mut warnings = 0;
        for state in states.into_iter().flatten().flatten() {
            if let Some(h) = state.hardening {
                score += h.checklist_score;
                max += h.checklist_max;
                warnings += h.warnings.len();
            }
        }
        if max == 0 {
            t("hardening.no_services_reporting")
        } else {
            format!("Checklist {score}/{max}; warnings: {warnings}")
        }
    });
    let reload_c = coland.clone();
    let reload_a = coauth.clone();
    let tc = theme.clone();
    let ta = theme.clone();
    col().class("station-page").fill((
        ui::heading(t("hardening.title")),
        text().content(score),
        sapid::button()
            .theme(theme.clone())
            .label(t("common.refresh"))
            .on_press(move || {
                reload_c.reload();
                reload_a.reload();
            }),
        ui::resource_view(scope, theme.clone(), coland, move |_, state| {
            health(tc.clone(), "Coland", state.hardening)
        }),
        ui::resource_view(scope, theme, coauth, move |_, state| {
            if !crate::utils::net::session::has_coauth() {
                Box::new(text().content(t("hardening.coauth_not_configured"))) as View
            } else {
                health(ta.clone(), "Coauth", state.hardening)
            }
        }),
    ))
}
fn health(
    theme: sapid::Theme,
    name: &str,
    status: Option<crate::types::api::HardeningStatus>,
) -> View {
    let value = match status {
        Some(value) => serde_json::to_value(value).unwrap_or_default(),
        None => serde_json::Value::String(t("hardening.no_data")),
    };
    Box::new(
        sapid::card()
            .theme(theme.clone())
            .title(name.to_owned())
            .fill(station::value_view(theme, value)),
    )
}
