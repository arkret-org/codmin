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
        |cursor| async move { coauth::list_upstream_links(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let id = row.id.clone();
            let remove = ui::action_form_controlled(
                scope,
                &mutations,
                row.id.clone(),
                row_theme.clone(),
                t("common.delete"),
                vec![],
                Some(Danger::new(common::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move { coauth::delete_upstream_link(&id).await }
                },
                move || data.reload(),
            );
            Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(row.id.clone())
                    .fill((common::record(&row), remove)),
            )
        },
    );
    common::stack(
        theme,
        vec![common::heading("coauth.upstream_links.title"), body],
    )
}
