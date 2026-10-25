use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger};
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth.oauth2_sessions.id",
    "coauth.oauth2_sessions.user_id",
    "coauth.oauth2_sessions.client_id",
    "coauth.oauth2_sessions.scope",
    "coauth.oauth2_sessions.human_name",
    "coauth.oauth2_sessions.created_at",
    "common.actions",
];
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let row_theme = theme.clone();
    let (body, data) = common::cursor(
        scope,
        theme.clone(),
        HEADERS,
        "coauth.oauth2_sessions.no_sessions",
        |cursor| async move { coauth::list_oauth2_sessions(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let action = if row.finished_at.is_none() {
                let id = row.id.clone();
                let form = ui::action_form_controlled(
                    scope,
                    &mutations,
                    row.id.clone(),
                    row_theme.clone(),
                    t("coauth.oauth2_sessions.finish"),
                    vec![],
                    Some(Danger::new(common::suffix(&id)).without_reason()),
                    move |_| {
                        let id = id.clone();
                        async move { coauth::finish_oauth2_session(&id).await }
                    },
                    move || data.reload(),
                );
                ui::compact_action(row_theme.clone(), t("coauth.oauth2_sessions.finish"), form)
            } else {
                common::muted("—")
            };
            vec![
                common::mono(row.id),
                common::mono(common::optional(row.user_id)),
                common::mono(common::optional(row.client_id)),
                common::cell(common::optional(row.scope)),
                common::cell(common::optional(row.human_name)),
                common::muted(common::optional(row.created_at)),
                action,
            ]
        },
    );
    common::page(
        "coauth.oauth2_sessions.title",
        "coauth.oauth2_sessions.subtitle",
        vec![common::refresh(theme, data)],
        vec![body],
    )
}
