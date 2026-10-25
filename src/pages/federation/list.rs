use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.federation"),
        vec![("Operation or Realm ID", "")],
        |q| async move {
            let response =
                crate::api::federation::list_federation_operations(q.cursor(), 20, q.field(0))
                    .await?;
            let mut doc = Document::default()
                .section("Federation operations", &response.data)?
                .next(response.next_cursor);
            for row in response.data {
                let id = row.operation_id.to_string();
                doc = doc.link(id.clone(), station::path("/federation", &id));
            }
            Ok(doc)
        },
    )
}
