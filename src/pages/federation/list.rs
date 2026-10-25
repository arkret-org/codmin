use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::types::federation::AdminFederationOperationExt;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("federation.title"),
        vec![("federation.search", "")],
        |q| async move {
            let response =
                crate::api::federation::list_federation_operations(q.cursor(), 25, q.field(0))
                    .await?;
            let rows = response.data.clone();
            Ok(Document::default()
                .section("Federation operations", &response.data)?
                .next(response.next_cursor)
                .view(move |_, theme| {
                    p::table(
                        theme.clone(),
                        &[
                            "federation.operation_id",
                            "federation.realm_id",
                            "federation.operation_kind",
                            "federation.canonical_kind",
                            "federation.created_at",
                        ],
                        rows.iter()
                            .map(|row| {
                                vec![
                                    p::link(
                                        theme.clone(),
                                        row.operation_id.as_str(),
                                        station::path("/federation", row.operation_id.as_str()),
                                    ),
                                    p::plain(row.realm_id.as_str()),
                                    p::badge(theme.clone(), row.operation_kind_label(), "neutral"),
                                    p::plain(row.canonical_kind_label()),
                                    p::plain(row.created_at_display()),
                                ]
                            })
                            .collect(),
                        "federation.no_operations",
                    )
                }))
        },
    )
}
