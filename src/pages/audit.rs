use glory::prelude::*;

use crate::components::ui;
use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::types::audit::AdminAuditEntryExt;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("audit.title"),
        vec![
            ("audit.filter_action", ""),
            ("audit.filter_actor", ""),
            ("audit.filter_realm", ""),
            ("audit.filter_event_kind", ""),
            ("audit.filter_since", ""),
            ("audit.filter_until", ""),
        ],
        |q| async move {
            let optional = |i| {
                let value = q.field(i).trim();
                (!value.is_empty()).then(|| value.to_owned())
            };
            let response = crate::api::audit::list_audit_entries(
                q.cursor(),
                25,
                crate::api::audit::AuditFilter {
                    action: optional(0),
                    actor_id: optional(1),
                    realm_id: optional(2),
                    kind: optional(3),
                    since: optional(4),
                    until: optional(5),
                },
            )
            .await?;
            let rows = response.entries.clone();
            Ok(Document::default()
                .section("Audit", &response.entries)?
                .next(response.next_cursor)
                .view(move |_, theme| {
                    p::table(
                        theme.clone(),
                        &[
                            "audit.id",
                            "audit.action",
                            "audit.actor_id",
                            "audit.realm_id",
                            "audit.outcome",
                            "executed_by",
                            "authz_ref",
                            "audit.timestamp",
                        ],
                        rows.iter()
                            .map(|row| {
                                let payload = row
                                    .payload
                                    .as_ref()
                                    .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                                    .unwrap_or_else(|| t("audit.no_details"));
                                vec![
                                    ui::compact_action(
                                        theme.clone(),
                                        row.id.clone(),
                                        Box::new(col().class("audit-payload").fill((
                                            p::plain(t("audit.details")),
                                            p::mono(payload),
                                        ))),
                                    ),
                                    p::plain(&row.action),
                                    p::plain(p::optional(row.actor_id.as_ref())),
                                    p::mono(p::optional(row.realm_id.as_ref())),
                                    p::plain(p::optional(row.outcome.as_ref())),
                                    p::mono(row.executed_by().unwrap_or_else(|| "-".into())),
                                    p::mono(row.authorization_ref().unwrap_or_else(|| "-".into())),
                                    p::plain(p::optional(row.created_at.map(|v| v.to_rfc3339()))),
                                ]
                            })
                            .collect(),
                        "audit.no_entries",
                    )
                }))
        },
    )
}
