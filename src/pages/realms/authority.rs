use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    station::read(
        scope,
        theme,
        t("realm_authority.title").replace("{realm_id}", &realm_id),
        move || {
            let id = realm_id.clone();
            async move {
                Document::default().section("Current governing Station, genesis, stream head and signed authority transitions",&crate::api::authority::get_realm_authority_bundle(&id).await?)
            }
        },
    )
}
