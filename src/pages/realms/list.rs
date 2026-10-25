use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::types::realms::AdminRealmItemExt;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(scope, theme, t("nav.realms"), vec![], |q| async move {
        let response = crate::api::realms::list_realms(q.cursor(), 20).await?;
        let rows = response.data.clone();
        Ok(Document::default()
            .section("Realms", &response.data)?
            .next(response.next_cursor)
            .view(move |_, theme| {
                p::table(
                    theme.clone(),
                    &[
                        "realms.id",
                        "realms.name",
                        "realms.type",
                        "realms.members",
                        "realms.encrypted",
                        "realms.status",
                        "realms.created_at",
                    ],
                    rows.iter()
                        .map(|row| {
                            vec![
                                p::link(theme.clone(), &row.id, station::path("/realms", &row.id)),
                                p::plain(&row.title),
                                p::plain(row.type_label()),
                                p::plain(row.member_count),
                                p::plain(if row.is_encrypted { "🔒" } else { "" }),
                                p::plain(t(if row.is_blocked {
                                    "realms.blocked"
                                } else {
                                    "realms.active"
                                })),
                                p::plain(row.created_at_display().unwrap_or_else(|| "-".into())),
                            ]
                        })
                        .collect(),
                    "common.no_results",
                )
            }))
    })
}
