use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, space_id: String) -> impl Widget + use<> {
    station::read(scope, theme, t("nav.spaces"), move || {
        let id = space_id.clone();
        async move {
            let response = crate::api::spaces::get_space(&id).await?;
            let mut doc = Document::default().section("Space hierarchy", &response)?;
            if let Some(parent) = response.parent {
                doc = doc.link(parent.name, station::path("/spaces", &parent.space_id));
            }
            for child in response.children {
                doc = doc.link(child.name, station::path("/spaces", &child.space_id));
            }
            Ok(doc)
        }
    })
}
