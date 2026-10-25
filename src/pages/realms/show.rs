use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    station::read(scope, theme, t("nav.realms"), move || {
        let id = realm_id.clone();
        async move {
            let mut doc =
                Document::default().section("Realm", &crate::api::realms::get_realm(&id).await?)?;
            for (name, suffix) in [
                ("Organization relationships", "organization"),
                ("Current authority and commit chain", "authority"),
                ("Realm links", "links"),
                ("Media service", "media-service"),
            ] {
                doc = doc.link(name, format!("{}/{suffix}", station::path("/realms", &id)));
            }
            Ok(doc)
        }
    })
}
