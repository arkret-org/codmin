use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.invite_tokens"),
        vec![],
        |q| async move {
            let response = crate::api::invite_tokens::list_invite_tokens(q.page, 25).await?;
            Document::default()
                .section("Invite tokens", &response.data)
                .map(|doc| {
                    doc.next(response.next_cursor)
                        .numeric(q.page, 25, response.total)
                })
        },
    )
}
