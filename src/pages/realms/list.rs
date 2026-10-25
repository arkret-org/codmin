use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(scope, theme, t("nav.realms"), vec![], |q| async move {
        let response = crate::api::realms::list_realms(q.cursor(), 20).await?;
        let mut doc = Document::default()
            .section("Realms", &response.data)?
            .next(response.next_cursor);
        for row in response.data {
            doc = doc.link(row.title.clone(), station::path("/realms", &row.id));
        }
        Ok(doc)
    })
}
