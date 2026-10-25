use glory::prelude::*;

use crate::components::ui;
use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let stats = ui::resource(scope, crate::api::media::get_media_statistics);
    let actors = ui::resource(scope, crate::api::media::list_media_by_actor);
    let stat_theme = theme.clone();
    let actor_theme = theme.clone();
    col().class("station-page").fill((
        ui::resource_view(scope, theme.clone(), stats, move |_, stats| {
            Box::new(view().class("stats-grid").fill(vec![
                p::metric(stat_theme.clone(), "media.blobs", stats.total_blobs),
                p::metric(
                    stat_theme.clone(),
                    "media.stored",
                    p::bytes(stats.total_size),
                ),
                p::metric(stat_theme.clone(), "media.encrypted", stats.encrypted_count),
                p::metric(
                    stat_theme.clone(),
                    "media.quarantined",
                    stats.quarantined_count,
                ),
            ]))
        }),
        ui::resource_view(scope, theme.clone(), actors, move |_, response| {
            p::table(
                actor_theme.clone(),
                &[
                    "media.col_actor",
                    "media.col_display_name",
                    "media.blobs",
                    "media.stored",
                ],
                response
                    .data
                    .iter()
                    .map(|row| {
                        vec![
                            p::mono(&row.actor_id),
                            p::plain(p::optional(row.display_name.as_ref())),
                            p::plain(row.blob_count),
                            p::plain(p::bytes(row.total_size)),
                        ]
                    })
                    .collect(),
                "media.no_media",
            )
        }),
        station::browse(
            scope,
            theme,
            t("media.title"),
            vec![("media.search", "")],
            |q| async move {
                let response = crate::api::media::list_media(q.page, 25, q.field(0)).await?;
                let rows = response.data.clone();
                Ok(Document::default()
                    .section("Media", &response.data)?
                    .next(response.next_cursor)
                    .numeric(q.page, 25, response.total)
                    .view(move |_, theme| {
                        p::table(
                            theme,
                            &[
                                "media.col_filename",
                                "media.col_type",
                                "media.col_realm",
                                "media.col_uploaded_by",
                                "media.col_size",
                                "media.encrypted",
                                "media.col_created",
                            ],
                            rows.iter()
                                .map(|row| {
                                    vec![
                                        p::plain(p::optional(row.filename.as_ref())),
                                        p::plain(&row.media_type),
                                        p::mono(p::optional(row.realm_id.as_ref())),
                                        p::mono(&row.uploaded_by),
                                        p::plain(p::bytes(row.size_bytes)),
                                        p::plain(t(if row.encrypted {
                                            "media.encrypted_yes"
                                        } else {
                                            "media.encrypted_no"
                                        })),
                                        p::plain(row.created_at.to_rfc3339()),
                                    ]
                                })
                                .collect(),
                            "media.no_media",
                        )
                    }))
            },
        ),
    ))
}
