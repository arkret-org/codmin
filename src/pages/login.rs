use glory::prelude::*;

use crate::api::auth;
use crate::components::ui::{self, View};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let config = ui::resource(scope, || async {
        let config = crate::utils::net::config::load_runtime_config()
            .await
            .map_err(|error| HttpError::message(error.to_string()))?;
        crate::utils::storage::set_item("oauth_client_id", &config.oauth_client_id);
        crate::utils::storage::set_item("coauth_public_url", &config.coauth_public_url);
        crate::utils::net::telemetry::set_endpoint(&config.telemetry_endpoint);
        Ok(config)
    });
    let warning = query("logout_warning")
        .as_deref()
        .and_then(logout_warning_message);
    let reload = config.clone();
    ui::reactive(config.data, move |scope, result| {
        if let Some(Err(error)) = result.as_ref() {
            let reload = reload.clone();
            return ui::auth_frame(
                512.0,
                vec![
                    ui::auth_brand(
                        theme.clone(),
                        t("config.error.title"),
                        t("config.error.subtitle"),
                        true,
                    ),
                    Box::new(
                        sapid::alert("config-error")
                            .theme(theme.clone())
                            .severity(sapid::AlertSeverity::Danger)
                            .class("auth-error-card")
                            .content(error.message.clone()),
                    ),
                    Box::new(
                        sapid::card()
                            .theme(theme.clone())
                            .class("auth-checklist")
                            .style(Style::new().padding(Edges::all(16.0)))
                            .fill((
                                sapid::text()
                                    .theme(theme.clone())
                                    .weight(500.0)
                                    .content(t("config.error.hint_title")),
                                text().content(t("config.error.hint_serve_path")),
                                text().content(t("config.error.hint_required_fields")),
                                text().content(t("config.error.hint_check_proxy")),
                            )),
                    ),
                    Box::new(
                        sapid::button()
                            .theme(theme.clone())
                            .label(t("common.retry"))
                            .on_press(move || reload.reload()),
                    ),
                ],
            );
        }
        let mut card: Vec<View> = Vec::new();
        if let Some(warning) = warning.as_ref() {
            card.push(Box::new(
                sapid::alert("logout-warning")
                    .theme(theme.clone())
                    .severity(sapid::AlertSeverity::Warning)
                    .content(warning.clone()),
            ));
        }
        card.push(Box::new(
            sapid::text()
                .theme(theme.clone())
                .class("auth-oauth-hint")
                .variant(sapid::TextVariant::Muted)
                .font_size(14.0)
                .content(t("auth.oauth_hint")),
        ));
        if result.is_some() {
            card.push(ui::action_form(
                scope,
                theme.clone(),
                t("auth.sign_in"),
                vec![],
                None,
                |_| async { auth::start_oauth_login().await },
                || {},
            ));
        } else {
            card.push(Box::new(
                sapid::button()
                    .theme(theme.clone())
                    .label(t("auth.sign_in"))
                    .loading(true)
                    .enabled(false),
            ));
        }
        ui::auth_frame(
            448.0,
            vec![
                ui::auth_brand(
                    theme.clone(),
                    "Arkret Admin".into(),
                    t("auth.sign_in_subtitle"),
                    false,
                ),
                Box::new(
                    sapid::card()
                        .theme(theme.clone())
                        .class("auth-login-card")
                        .shadow(sapid::ShadowSize::Small)
                        .style(Style::new().padding(Edges::all(24.0)).gap(16.0))
                        .fill(card),
                ),
                Box::new(
                    sapid::text()
                        .theme(theme.clone())
                        .class("auth-footer")
                        .variant(sapid::TextVariant::Muted)
                        .font_size(12.0)
                        .line_height(16.0)
                        .content(t("auth.footer")),
                ),
            ],
        )
    })
}
pub(super) fn query(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search)
        .ok()?
        .get(name)
}

fn logout_warning_message(code: &str) -> Option<String> {
    let stages: Vec<_> = code
        .split(',')
        .filter_map(|item| match item {
            "principal_logout" => Some("Station session cleanup"),
            "oauth_revoke" => Some("OAuth token revocation"),
            _ => None,
        })
        .collect();
    (!stages.is_empty()).then(|| format!(
        "This browser session was closed, but {} was not confirmed. Sign in again and retry sign-out if remote cleanup is still required.", stages.join(" and ")
    ))
}

#[cfg(test)]
mod tests {
    use super::logout_warning_message;
    #[test]
    fn logout_warning_codes_are_fixed_and_non_sensitive() {
        let message = logout_warning_message("principal_logout,oauth_revoke").unwrap();
        assert!(message.contains("Station session cleanup"));
        assert!(message.contains("OAuth token revocation"));
        assert!(logout_warning_message("server body here").is_none());
    }
}
