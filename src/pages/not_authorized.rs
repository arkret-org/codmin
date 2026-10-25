use glory::prelude::*;

use crate::api::auth;
use crate::components::ui;
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;
use crate::utils::net::session;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let user = session::current_user();
    let sign_out = ui::action_form(
        scope,
        theme.clone(),
        t("auth.sign_out"),
        vec![],
        None,
        |_| async {
            let report = auth::logout().await;
            if !report.local_cookie_cleared() {
                return Err(HttpError::message(report.retry_message()));
            }
            let destination = report
                .login_warning_code()
                .map(|warning| format!("/login?logout_warning={}", urlencoding::encode(&warning)))
                .unwrap_or_else(|| "/login".into());
            if let Some(window) = web_sys::window() {
                window
                    .location()
                    .replace(&destination)
                    .map_err(|_| HttpError::message("Could not open sign-in"))?;
            }
            Ok(())
        },
        || {},
    );
    let mut children: Vec<ui::View> = vec![ui::auth_brand(theme.clone(), t("auth.not_admin"), "Your account does not have server administrator privileges, so the admin dashboard cannot be shown. Sign out and log in as an administrator, or ask an existing admin to grant you access.".into(), true)];
    if user.has_identity() {
        let mut identity: Vec<ui::View> = vec![Box::new(
            sapid::text()
                .theme(theme.clone())
                .variant(sapid::TextVariant::Muted)
                .font_size(12.0)
                .content("Signed in as"),
        )];
        if let Some(name) = user.display_name {
            identity.push(Box::new(
                sapid::text()
                    .theme(theme.clone())
                    .weight(500.0)
                    .content(name),
            ));
        }
        if let Some(id) = user.id {
            identity.push(Box::new(
                sapid::text()
                    .theme(theme.clone())
                    .font_size(12.0)
                    .content(id),
            ));
        }
        children.push(Box::new(
            sapid::card()
                .theme(theme.clone())
                .class("auth-identity-card")
                .style(Style::new().padding(Edges::all(16.0)))
                .fill(identity),
        ));
    }
    children.push(sign_out);
    ui::auth_frame(448.0, children)
}
