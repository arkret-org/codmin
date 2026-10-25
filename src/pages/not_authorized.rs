use glory::prelude::*;

use crate::api::auth;
use crate::components::ui;
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;
use crate::utils::net::session;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let user = session::current_user();
    col().role(Role::Main).class("auth-page").fill((
        ui::heading(t("auth.not_admin")),
        text().content("Your account does not have server administrator privileges. Sign in as an administrator or ask an existing administrator to grant access."),
        ui::details(vec![
            ("Display name".into(), user.display_name.unwrap_or_default()),
            ("Account".into(), user.id.unwrap_or_default()),
        ]),
        ui::action_form(scope, theme, t("auth.sign_out"), vec![], None,
            |_| async {
                let report = auth::logout().await;
                if !report.local_cookie_cleared() {
                    return Err(HttpError::message(report.retry_message()));
                }
                let destination = report.login_warning_code().map(|warning| format!("/login?logout_warning={}", urlencoding::encode(&warning))).unwrap_or_else(|| "/login".into());
                if let Some(window) = web_sys::window() {
                    window.location().replace(&destination).map_err(|_| HttpError::message("Could not open sign-in"))?;
                }
                Ok(())
            }, || {}),
    ))
}
