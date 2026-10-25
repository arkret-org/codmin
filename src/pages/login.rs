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
    let mut children: Vec<View> = vec![
        ui::heading("Arkret Admin"),
        Box::new(text().content(t("auth.sign_in_subtitle"))),
    ];
    if let Some(warning) = warning {
        children.push(Box::new(
            sapid::alert("logout-warning")
                .theme(theme.clone())
                .severity(sapid::AlertSeverity::Warning)
                .content(warning),
        ));
    }
    let form_theme = theme.clone();
    children.push(ui::resource_view(
        scope,
        theme,
        config,
        move |scope, _config| {
            Box::new(col().class("login-panel").fill((
                text().content(t("auth.oauth_hint")),
                ui::action_form(
                    scope,
                    form_theme.clone(),
                    t("auth.sign_in"),
                    vec![],
                    None,
                    |_| async { auth::start_oauth_login().await },
                    || {},
                ),
            )))
        },
    ));
    children.push(Box::new(text().content(t("auth.footer"))));
    col().role(Role::Main).class("auth-page").fill(children)
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
