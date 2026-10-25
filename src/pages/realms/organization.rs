use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    if station::placeholder(&realm_id) {
        return station::selection("Realm");
    }
    station::read(scope, theme, t("realm_organization.title"), move || {
        let id = realm_id.clone();
        async move {
            let response =
                crate::api::realm_organization::get_realm_organization_admin_view(&id).await?;
            Document::default()
                .section(
                    "Verified Realm organization relationships",
                    &response.relationships,
                )?
                .section("Principal control audit", &response.controls)?
                .section(
                    "Organizations managed by another Account Authority",
                    &response.unavailable_control_ids,
                )
        }
    })
}
