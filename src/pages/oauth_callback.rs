use glory::prelude::*;

use crate::api::auth;
use crate::components::ui;
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let status = ui::resource(scope, || async {
        if let Some(error) = super::login::query("error") {
            return Err(HttpError::message(format!(
                "OAuth authorization failed: {error}"
            )));
        }
        let code = super::login::query("code")
            .ok_or_else(|| HttpError::message("Missing OAuth authorization code"))?;
        let state = super::login::query("state");
        auth::handle_oauth_callback(&code, state.as_deref()).await?;
        if let Some(window) = web_sys::window() {
            window
                .location()
                .replace("/")
                .map_err(|_| HttpError::message("Could not open the dashboard"))?;
        }
        Ok(())
    });
    ui::auth_frame(
        448.0,
        vec![
            ui::auth_brand(
                theme.clone(),
                "Arkret Admin".into(),
                t("auth.completing_login"),
                false,
            ),
            ui::reactive(status.data, move |_scope, status| match status {
                None => Box::new(text().content(t("common.loading"))),
                Some(Ok(())) => Box::new(text().content("Opening dashboard…")),
                Some(Err(error)) => Box::new(
                    col().fill((
                        sapid::alert("oauth-error")
                            .theme(theme.clone())
                            .severity(sapid::AlertSeverity::Danger)
                            .content(error.message),
                        link().target("/login").label(t("auth.sign_in")),
                    )),
                ),
            }),
        ],
    )
}
