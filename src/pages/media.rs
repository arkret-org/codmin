use glory::prelude::*;

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    col().class("station-page").fill((
        station::read(scope, theme.clone(), t("media.title"), || async {
            Document::default().section(
                "Media statistics",
                &crate::api::media::get_media_statistics().await?,
            )
        }),
        station::read(
            scope,
            theme.clone(),
            "Media usage by actor".to_owned(),
            || async {
                Document::default().section(
                    "Media usage by actor",
                    &crate::api::media::list_media_by_actor().await?,
                )
            },
        ),
        station::browse(
            scope,
            theme,
            t("media.title"),
            vec![("Hash, filename, media type, Realm or uploader", "")],
            |q| async move {
                let response = crate::api::media::list_media(q.page, 25, q.field(0)).await?;
                Document::default()
                    .section("Media objects", &response.data)
                    .map(|doc| {
                        doc.next(response.next_cursor)
                            .numeric(q.page, 25, response.total)
                    })
            },
        ),
    ))
}
