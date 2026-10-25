use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger, Field};
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth.personal_sessions.id",
    "coauth.personal_sessions.user_id",
    "coauth.personal_sessions.name",
    "coauth.personal_sessions.created_at",
    "coauth.personal_sessions.last_active",
    "common.actions",
];
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let reveal = scope.cage(String::new());
    let row_theme = theme.clone();
    let (body, data) = common::cursor(
        scope,
        theme.clone(),
        HEADERS,
        "coauth.personal_sessions.no_sessions",
        |cursor| async move { coauth::list_personal_sessions(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let id = row.id.clone();
            let changed = data.clone();
            let revoke = ui::action_form_controlled(
                scope,
                &mutations,
                row.id.clone(),
                row_theme.clone(),
                t("coauth.personal_sessions.revoke"),
                vec![],
                Some(Danger::new(common::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move { coauth::revoke_personal_session(&id).await }
                },
                move || changed.reload(),
            );
            let id = row.id.clone();
            let regenerate = ui::action_form_controlled(
                scope,
                &mutations,
                row.id.clone(),
                row_theme.clone(),
                t("coauth.personal_sessions.regenerate"),
                vec![],
                Some(Danger::new(common::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move {
                        let response = coauth::regenerate_personal_session(&id).await?;
                        reveal.revise(|value| *value = response.access_token.unwrap_or_default());
                        Ok(())
                    }
                },
                move || data.reload(),
            );
            vec![
                common::mono(row.id),
                common::mono(common::optional(row.user_id)),
                common::cell(common::optional(row.name)),
                common::muted(common::optional(row.created_at)),
                common::muted(common::optional(row.last_active_at)),
                common::group(vec![
                    ui::compact_action(
                        row_theme.clone(),
                        t("coauth.personal_sessions.regenerate"),
                        common::stack(
                            row_theme.clone(),
                            vec![
                                common::muted(t("coauth.personal_sessions.regenerate_confirm")),
                                regenerate,
                            ],
                        ),
                    ),
                    ui::compact_action(
                        row_theme.clone(),
                        t("coauth.personal_sessions.revoke"),
                        common::stack(
                            row_theme.clone(),
                            vec![
                                common::muted(t("coauth.personal_sessions.revoke_confirm")),
                                revoke,
                            ],
                        ),
                    ),
                ]),
            ]
        },
    );
    let changed = data.clone();
    let create = ui::action_form(
        scope,
        theme.clone(),
        t("common.create"),
        vec![Field::new(t("coauth.personal_sessions.name"), "")],
        None,
        move |values| async move {
            if values[0].trim().is_empty() {
                return Err(crate::utils::net::error::HttpError::message(
                    "Session name is required",
                ));
            }
            let response = coauth::create_personal_session(values[0].trim()).await?;
            reveal.revise(|value| *value = response.access_token.unwrap_or_default());
            Ok(())
        },
        move || changed.reload(),
    );
    common::page(
        "coauth.personal_sessions.title",
        "coauth.personal_sessions.subtitle",
        vec![
            common::refresh(theme.clone(), data),
            ui::compact_action(theme.clone(), t("coauth.personal_sessions.create"), create),
        ],
        vec![token_reveal(scope, theme, reveal), body],
    )
}
fn token_reveal(_scope: &Scope, theme: sapid::Theme, reveal: Cage<String>) -> ui::View {
    ui::reactive(reveal, move |scope, token| {
        if token.is_empty() {
            return Box::new(group());
        }
        let status = scope.cage(String::new());
        let weak = scope.downgrade();
        Box::new(
            sapid::card()
                .theme(theme.clone())
                .title(t("coauth.personal_sessions.token_reveal_title"))
                .fill((
                    text().content(t("coauth.personal_sessions.token_reveal_warning")),
                    text().class("one-shot-token").content(token.clone()),
                    sapid::button()
                        .theme(theme.clone())
                        .label(t("common.copy"))
                        .on_press(move || {
                            let Some(scope) = weak.upgrade() else {
                                return;
                            };
                            let token = token.clone();
                            let _ = scope.owner().spawn(async move {
                                let result = if let Some(window) = web_sys::window() {
                                    wasm_bindgen_futures::JsFuture::from(
                                        window.navigator().clipboard().write_text(&token),
                                    )
                                    .await
                                    .map(|_| ())
                                    .map_err(|_| "Failed to copy token")
                                } else {
                                    Err("Clipboard is unavailable")
                                };
                                status.revise(|v| {
                                    *v = match result {
                                        Ok(()) => t("coauth.personal_sessions.token_copied"),
                                        Err(e) => e.into(),
                                    }
                                });
                            });
                        }),
                    text().content(status),
                    sapid::button()
                        .theme(theme.clone())
                        .label(t("common.close"))
                        .on_press(move || reveal.revise(String::clear)),
                )),
        )
    })
}
