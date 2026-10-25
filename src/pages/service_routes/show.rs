use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(
    scope: &Scope,
    theme: sapid::Theme,
    service_id: String,
    service_kind: String,
) -> impl Widget + use<> {
    station::read(scope, theme, t("service_routes.title"), move || {
        let id = service_id.clone();
        let kind = service_kind.clone();
        async move {
            Document::default().section(
                "Service route, cache and quarantine",
                &crate::api::service_routes::get_service_route(&id, &kind).await?,
            )
        }
    })
}
