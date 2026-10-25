use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.actors"),
        vec![("Search", "")],
        |q| async move {
            let response = crate::api::actors::list_actors(q.cursor(), 20, q.field(0)).await?;
            let mut doc = Document::default()
                .section("Actors", &response.actors)?
                .next(response.next_cursor);
            for row in response.actors {
                doc = doc.link(
                    row.display_name.as_deref().unwrap_or(&row.id),
                    station::path("/actors", &row.id),
                );
            }
            Ok(doc)
        },
    )
}
