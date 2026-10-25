use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth.audit_log.id",
    "coauth.audit_log.operation",
    "coauth.audit_log.actor",
    "coauth.audit_log.target_type",
    "coauth.audit_log.target_id",
    "coauth.audit_log.col_detail",
    "coauth.audit_log.timestamp",
    "coauth.audit_log.signature_status",
];
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let filter = scope.cage((String::new(), String::new()));
    let actor = scope.cage(String::new());
    let target = scope.cage(String::new());
    let data = ui::resource(scope, move || {
        let (actor, target) = filter.get();
        coauth::list_audit_feed(
            25,
            coauth::AuditFeedFilter {
                actor_user_id: (!actor.trim().is_empty()).then(|| actor.trim().into()),
                target_type: (!target.trim().is_empty()).then(|| target.trim().into()),
            },
        )
    });
    let filters = Box::new(
        view().class("filter-panel").fill((
            view().class("filter-grid").fill((
                view().class("filter-field").fill((
                    text()
                        .class("filter-label")
                        .content(t("coauth.audit_log.filter_actor")),
                    sapid::text_field()
                        .theme(theme.clone())
                        .name(t("coauth.audit_log.filter_actor"))
                        .value(actor)
                        .on_value_change(move |value| actor.revise(|old| *old = value)),
                )),
                view().class("filter-field").fill((
                    text()
                        .class("filter-label")
                        .content(t("coauth.audit_log.filter_target_type")),
                    sapid::text_field()
                        .theme(theme.clone())
                        .name(t("coauth.audit_log.filter_target_type"))
                        .value(target)
                        .on_value_change(move |value| target.revise(|old| *old = value)),
                )),
            )),
            view().class("filter-actions").fill((
                sapid::button()
                    .theme(theme.clone())
                    .label(t("coauth.audit_log.filter_apply"))
                    .on_press(move || {
                        filter.revise(|old| *old = (actor.get_untracked(), target.get_untracked()))
                    }),
                sapid::button()
                    .theme(theme.clone())
                    .variant(sapid::ButtonVariant::Secondary)
                    .label(t("coauth.audit_log.filter_reset"))
                    .on_press(move || {
                        actor.revise(String::clear);
                        target.revise(String::clear);
                        filter.revise(|old| *old = (String::new(), String::new()));
                    }),
            )),
        )),
    );
    let table_theme = theme.clone();
    let body = common::resource_body(theme.clone(), data.clone(), move |_, rows| {
        common::table(
            table_theme.clone(),
            HEADERS,
            rows.into_iter()
                .map(|row| {
                    vec![
                        common::mono(row.id),
                        common::cell(row.operation),
                        common::mono(common::optional(row.admin_user_id)),
                        common::cell(row.resource_kind),
                        common::mono(row.resource_id),
                        Box::new(
                            text().class("audit-details").content(
                                row.details
                                    .map(|value| serde_json::to_string(&value).unwrap_or_default())
                                    .unwrap_or_else(|| "—".into()),
                            ),
                        ),
                        common::muted(row.created_at.to_rfc3339()),
                        common::cell(format!("{:?}", row.signature_status)),
                    ]
                })
                .collect(),
            "coauth.audit_log.no_entries",
        )
    });
    common::page(
        "coauth.audit_log.title",
        "coauth.audit_log.subtitle",
        vec![common::refresh(theme, data)],
        vec![filters, body],
    )
}
