use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("service_routes.title"),
        vec![],
        |q| async move {
            let response = crate::api::service_routes::list_service_routes(q.cursor(), 25).await?;
            let mut doc = Document::default()
                .section("Service routes", &response.routes)?
                .next(response.next_cursor);
            for row in response.routes {
                let id = row.service_id.to_string();
                doc = doc.link(
                    format!("{id} / {}", row.service_kind),
                    format!(
                        "{}/{}",
                        station::path("/service-routes", &id),
                        urlencoding::encode(&row.service_kind)
                    ),
                );
            }
            Ok(doc)
        },
    )
}
