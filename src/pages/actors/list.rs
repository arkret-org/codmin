use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::types::{AccountStatus, AdminActorExt};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.actors"),
        vec![("actors.search_placeholder", "")],
        |q| async move {
            let response = crate::api::actors::list_actors(q.cursor(), 20, q.field(0)).await?;
            let rows = response.actors.clone();
            Ok(Document::default()
                .section("Actors", &response.actors)?
                .next(response.next_cursor)
                .view(move |_, theme| {
                    p::table(
                        theme.clone(),
                        &[
                            "actors.id",
                            "actors.handle",
                            "actors.display_name",
                            "actors.status",
                            "actors.is_admin",
                            "actors.created_at",
                        ],
                        rows.iter()
                            .map(|actor| {
                                vec![
                                    p::link(
                                        theme.clone(),
                                        &actor.id,
                                        station::path("/actors", &actor.id),
                                    ),
                                    p::plain(p::optional(actor.handle.as_ref())),
                                    p::plain(p::optional(actor.display_name.as_ref())),
                                    p::badge(
                                        theme.clone(),
                                        actor.status_display(),
                                        match actor.status {
                                            Some(AccountStatus::Active) => "success",
                                            Some(
                                                AccountStatus::Suspended
                                                | AccountStatus::Deactivated
                                                | AccountStatus::ErasurePending,
                                            ) => "danger",
                                            _ => "neutral",
                                        },
                                    ),
                                    p::plain(
                                        actor.is_admin.map(p::yes).unwrap_or_else(|| "-".into()),
                                    ),
                                    p::plain(p::optional(
                                        actor.created_at.map(|value| value.to_rfc3339()),
                                    )),
                                ]
                            })
                            .collect(),
                        "common.no_results",
                    )
                }))
        },
    )
}
