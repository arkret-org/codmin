use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.spaces"),
        vec![("Search", "")],
        |q| async move {
            let response = crate::api::spaces::list_spaces(q.cursor(), 20, q.field(0)).await?;
            let mut doc = Document::default()
                .section("Spaces", &response.data)?
                .next(response.next_cursor);
            for row in response.data {
                doc = doc.link(row.name.clone(), station::path("/spaces", &row.id));
            }
            Ok(doc)
        },
    )
}
