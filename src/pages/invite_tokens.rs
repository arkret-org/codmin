use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("invite_tokens.title"),
        vec![],
        |q| async move {
            let response = crate::api::invite_tokens::list_invite_tokens(q.page, 25).await?;
            let rows = response.data.clone();
            Ok(Document::default()
                .section("Invite tokens", &response.data)?
                .next(response.next_cursor)
                .numeric(q.page, 25, response.total)
                .view(move |_, theme| {
                    p::table(
                        theme,
                        &[
                            "invite_tokens.id",
                            "invite_tokens.invitee",
                            "invite_tokens.status",
                            "invite_tokens.uses_allowed",
                            "invite_tokens.uses_completed",
                            "invite_tokens.uses_pending",
                            "invite_tokens.expires_at",
                            "invite_tokens.realm_id",
                            "invite_tokens.created_at",
                        ],
                        rows.iter()
                            .map(|row| {
                                vec![
                                    p::plain(&row.id),
                                    p::mono(p::optional(row.invitee_id.as_ref())),
                                    p::plain(&row.status),
                                    p::plain(row.uses_allowed),
                                    p::plain(row.uses_completed),
                                    p::plain(row.uses_pending),
                                    p::plain(p::optional(
                                        row.expires_at.as_ref().map(chrono::DateTime::to_rfc3339),
                                    )),
                                    p::plain(&row.realm_id),
                                    p::plain(row.created_at.to_rfc3339()),
                                ]
                            })
                            .collect(),
                        "invite_tokens.no_tokens",
                    )
                }))
        },
    )
}
