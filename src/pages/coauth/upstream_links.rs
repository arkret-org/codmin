use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger};
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth.upstream_links.id",
    "coauth.upstream_links.user_id",
    "coauth.upstream_links.provider_id",
    "coauth.upstream_links.subject",
    "coauth.upstream_links.created_at",
    "common.actions",
];
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let row_theme = theme.clone();
    let (body, data) = common::cursor(
        scope,
        theme.clone(),
        HEADERS,
        "coauth.upstream_links.no_links",
        |cursor| async move { coauth::list_upstream_links(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let id = row.id.clone();
            let form = ui::action_form_controlled(
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
            vec![
                common::mono(row.id),
                common::mono(common::optional(row.user_id)),
                common::mono(common::optional(row.provider_id)),
                common::cell(common::optional(row.subject)),
                common::muted(common::optional(row.created_at)),
                ui::compact_action(
                    row_theme.clone(),
                    t("common.delete"),
                    common::stack(
                        row_theme.clone(),
                        vec![
                            common::muted(t("coauth.upstream_links.delete_description")),
                            form,
                        ],
                    ),
                ),
            ]
        },
    );
    common::page(
        "coauth.upstream_links.title",
        "coauth.upstream_links.subtitle",
        vec![common::refresh(theme, data)],
        vec![body],
    )
}
