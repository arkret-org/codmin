use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    if station::placeholder(&realm_id) {
        return station::selection("Realm");
    }
    station::read(scope, theme, t("media_service.title"), move || {
        let id = realm_id.clone();
        async move {
            Document::default().section(
                "Media service and foci",
                &crate::api::media_service::get_realm_media_service(&id).await?,
            )
        }
    })
}
