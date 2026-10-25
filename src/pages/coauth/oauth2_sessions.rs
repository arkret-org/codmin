use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let row_theme = theme.clone();
    let (body, _) = common::cursor(
        scope,
        theme.clone(),
        |cursor| async move { coauth::list_oauth2_sessions(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let mut children = vec![common::record(&row)];
            if row.finished_at.is_none() {
                let id = row.id.clone();
                children.push(ui::action_form_controlled(
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
                ));
            }
            Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(row.id)
                    .fill(common::stack(row_theme.clone(), children)),
            )
        },
    );
    common::stack(
        theme,
        vec![common::heading("coauth.oauth2_sessions.title"), body],
    )
}
