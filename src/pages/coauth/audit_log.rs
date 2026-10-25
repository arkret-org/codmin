use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let filter = scope.cage((String::new(), String::new()));
    let data = ui::resource(scope, move || {
        let (actor, target) = filter.get();
        coauth::list_audit_feed(
            25,
            coauth::AuditFeedFilter {
                actor_user_id: (!actor.trim().is_empty()).then(|| actor.trim().to_owned()),
                target_type: (!target.trim().is_empty()).then(|| target.trim().to_owned()),
            },
        )
    });
    let actor = scope.cage(String::new());
    let target = scope.cage(String::new());
    let actor_label = t("coauth.audit_log.filter_actor");
    let target_label = t("coauth.audit_log.filter_target_type");
    let form = Box::new(
        sapid::stack().theme(theme.clone()).fill((
            text().content(actor_label.clone()),
            sapid::text_field()
                .theme(theme.clone())
                .name(actor_label)
                .value(actor)
                .on_value_change(move |value| actor.revise(|old| *old = value)),
            text().content(target_label.clone()),
            sapid::text_field()
                .theme(theme.clone())
                .name(target_label)
                .value(target)
                .on_value_change(move |value| target.revise(|old| *old = value)),
            sapid::button()
                .theme(theme.clone())
                .label(t("coauth.audit_log.filter_apply"))
                .on_press(move || {
                    filter.revise(|value| *value = (actor.get_untracked(), target.get_untracked()))
                }),
        )),
    );
    let reset = Box::new(
        sapid::button()
            .theme(theme.clone())
            .label(t("coauth.audit_log.filter_reset"))
            .on_press(move || {
                actor.revise(String::clear);
                target.revise(String::clear);
                filter.revise(|value| *value = (String::new(), String::new()));
            }),
    );
    let outer = theme.clone();
    let body = ui::resource_view(scope, theme.clone(), data, move |_, rows| {
        ui::table(
            theme.clone(),
            vec![
                "ID".into(),
                "Operation".into(),
                "Actor".into(),
                "Target type".into(),
                "Target ID".into(),
                "Details".into(),
                "Timestamp".into(),
                "Signature status".into(),
            ],
            rows.into_iter()
                .map(|r| {
                    vec![
                        r.id,
                        r.operation,
                        r.admin_user_id.unwrap_or_default(),
                        r.resource_kind,
                        r.resource_id,
                        r.details.map(|v| v.to_string()).unwrap_or_default(),
                        r.created_at.to_rfc3339(),
                        format!("{:?}", r.signature_status),
                    ]
                })
                .collect(),
        )
    });
    common::stack(
        outer,
        vec![common::heading("coauth.audit_log.title"), form, reset, body],
    )
}
