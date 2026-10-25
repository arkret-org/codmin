use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.capabilities"),
        vec![
            ("Realm ID", ""),
            ("Complete Account or Service ActorId (JSON)", ""),
            ("State: all / active / revoked / relinquished", "all"),
        ],
        |q| async move {
            let filters = crate::api::capabilities::CapabilityFilters::parse(
                q.field(0),
                q.field(1),
                q.field(2),
            )
            .map_err(crate::utils::net::error::HttpError::message)?;
            let response =
                crate::api::capabilities::list_capabilities(q.cursor(), 25, &filters).await?;
            Document::default()
                .section("Accepted capability grants (read only)", &response)
                .map(|doc| doc.next(response.next_cursor))
        },
    )
}
