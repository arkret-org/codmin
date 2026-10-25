use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, operation_id: String) -> impl Widget + use<> {
    station::read(scope, theme, t("nav.federation"), move || {
        let id = operation_id.clone();
        async move {
            Document::default().section(
                "Federation operation",
                &crate::api::federation::get_federation_operation(&id).await?,
            )
        }
    })
}
