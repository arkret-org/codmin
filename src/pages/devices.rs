use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::types::devices::AdminDeviceExt;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("devices.title"),
        vec![("devices.search", "")],
        |q| async move {
            let response = crate::api::devices::list_devices(q.cursor(), 25, q.field(0)).await?;
            let rows = response.devices.clone();
            Ok(Document::default()
                .section("Devices", &response.devices)?
                .next(response.next_cursor)
                .view(move |_, theme| {
                    p::table(
                        theme,
                        &[
                            "devices.id",
                            "devices.actor_id",
                            "devices.display_name",
                            "devices.verification_status",
                            "devices.col_created",
                            "devices.col_updated",
                            "devices.col_revoked",
                        ],
                        rows.iter()
                            .map(|row| {
                                vec![
                                    p::mono(&row.id),
                                    p::plain(p::optional(row.actor_id.as_ref())),
                                    p::plain(p::optional(row.display_name.as_ref())),
                                    p::plain(row.verification_label().unwrap_or("-")),
                                    p::plain(
                                        row.created_at_display().unwrap_or_else(|| "-".into()),
                                    ),
                                    p::plain(
                                        row.updated_at_display().unwrap_or_else(|| "-".into()),
                                    ),
                                    p::plain(
                                        row.revoked_at_display().unwrap_or_else(|| "-".into()),
                                    ),
                                ]
                            })
                            .collect(),
                        "devices.no_devices",
                    )
                }))
        },
    )
}
