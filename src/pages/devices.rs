use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.devices"),
        vec![("Name or device ID", "")],
        |q| async move {
            let response = crate::api::devices::list_devices(q.cursor(), 20, q.field(0)).await?;
            Document::default()
                .section("Devices", &response)
                .map(|doc| doc.next(response.next_cursor))
        },
    )
}
