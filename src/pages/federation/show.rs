use glory::{Scope, Widget};

use crate::components::ui;
use crate::pages::presentation as p;
use crate::types::federation::AdminFederationOperationExt;
pub fn page(scope: &Scope, theme: sapid::Theme, operation_id: String) -> impl Widget + use<> {
    let id = operation_id.clone();
    let data = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::federation::get_federation_operation(&id).await }
    });
    ui::resource_view(scope, theme.clone(), data, move |_, row| {
        p::stack(vec![
            p::breadcrumb(
                theme.clone(),
                "federation.title",
                "/federation",
                &operation_id,
            ),
            ui::heading(row.operation_id.to_string()),
            p::description("federation.operation_details"),
            p::card(
                theme.clone(),
                "federation.operation_info",
                vec![
                    ("federation.operation_id", row.operation_id.to_string()),
                    ("federation.realm_id", row.realm_id.to_string()),
                    ("federation.operation_kind", row.operation_kind_label()),
                    ("federation.canonical_kind", row.canonical_kind_label()),
                    ("federation.strand_id", p::optional(row.strand_id.as_ref())),
                    ("federation.track", p::optional(row.track.as_ref())),
                    ("federation.digest", p::optional(row.digest.as_ref())),
                    ("federation.created_at", row.created_at_display()),
                ],
            ),
        ])
    })
}
