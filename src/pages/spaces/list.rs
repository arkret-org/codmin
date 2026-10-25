use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::types::spaces::{SpaceHealth, SpaceHealthExt};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.spaces"),
        vec![("spaces.search_placeholder", "")],
        |q| async move {
            let response = crate::api::spaces::list_spaces(q.cursor(), 25, q.field(0)).await?;
            let rows = response.data.clone();
            let total = response.total;
            Ok(Document::default()
                .section("Spaces", &response.data)?
                .next(response.next_cursor)
                .view(move |_, theme| {
                    p::stack(vec![
                        p::plain(format!(
                            "Showing {} (server total: {})",
                            rows.len(),
                            total.map(|v| v.to_string()).unwrap_or_else(|| "?".into())
                        )),
                        p::table(
                            theme.clone(),
                            &[
                                "spaces.id",
                                "spaces.name",
                                "spaces.members",
                                "spaces.health",
                                "spaces.created_at",
                                "common.actions",
                            ],
                            rows.iter()
                                .map(|row| {
                                    vec![
                                        p::mono(&row.id),
                                        p::plain(&row.name),
                                        p::plain(row.member_count),
                                        p::badge(
                                            theme.clone(),
                                            t(row.health.label()),
                                            match row.health {
                                                SpaceHealth::Active => "success",
                                                SpaceHealth::Archived => "neutral",
                                                SpaceHealth::Tombstoned => "danger",
                                            },
                                        ),
                                        p::plain(row.created_at.to_rfc3339()),
                                        p::link(
                                            theme.clone(),
                                            t("spaces.open_detail"),
                                            station::path("/spaces", &row.id),
                                        ),
                                    ]
                                })
                                .collect(),
                            "spaces.empty_title",
                        ),
                    ])
                }))
        },
    )
}
