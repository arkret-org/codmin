use glory::{Scope, Widget};

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("nav.audit"),
        vec![
            ("Action", ""),
            ("Actor ID", ""),
            ("Realm ID", ""),
            ("Event kind", ""),
            ("Since (UTC / RFC3339)", ""),
            ("Until (UTC / RFC3339)", ""),
        ],
        |q| async move {
            let optional = |i| {
                let s = q.field(i).trim();
                (!s.is_empty()).then(|| s.to_owned())
            };
            let filter = crate::api::audit::AuditFilter {
                action: optional(0),
                actor_id: optional(1),
                realm_id: optional(2),
                kind: optional(3),
                since: optional(4),
                until: optional(5),
            };
            let response = crate::api::audit::list_audit_entries(q.cursor(), 25, filter).await?;
            Document::default()
                .section("Audit entries", &response)
                .map(|doc| doc.next(response.next_cursor))
        },
    )
}
