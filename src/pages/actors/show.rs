use glory::prelude::*;

use crate::components::ui::{self, Danger, View};
use crate::pages::{presentation as p, station};
use crate::types::AdminActorExt;
use crate::utils::i18n::t;
use crate::utils::net::audit::{AdminAuditOutcome, emit_admin_audit};

pub fn page(scope: &Scope, theme: sapid::Theme, actor_id: String) -> impl Widget + use<> {
    let id = actor_id.clone();
    let data = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::actors::get_actor(&id).await }
    });
    let reload = data.clone();
    ui::resource_view(scope, theme.clone(), data, move |scope, actor| {
        let mut children: Vec<View> = vec![
            p::breadcrumb(theme.clone(), "nav.actors", "/actors", &actor.id),
            ui::heading(
                actor
                    .display_name
                    .clone()
                    .unwrap_or_else(|| actor.id.clone()),
            ),
            p::card(
                theme.clone(),
                "actors.overview",
                vec![
                    ("actors.id", actor.id.clone()),
                    ("actors.handle", p::optional(actor.handle.as_ref())),
                    (
                        "actors.display_name",
                        p::optional(actor.display_name.as_ref()),
                    ),
                    ("actors.status", actor.status_display().into()),
                    (
                        "actors.is_admin",
                        actor.is_admin.map(p::yes).unwrap_or_else(|| "-".into()),
                    ),
                    (
                        "actors.created_at",
                        p::optional(actor.created_at.map(|v| v.to_rfc3339())),
                    ),
                    (
                        "actors.last_active",
                        p::optional(actor.last_active_at.map(|v| v.to_rfc3339())),
                    ),
                    ("actors.device_count", p::optional(actor.device_count)),
                    ("actors.realm_count", p::optional(actor.realm_count)),
                ],
            ),
        ];
        if actor.is_deactivated() && actor.deactivation_federation_incomplete == Some(true) {
            children.push(Box::new(
                sapid::alert("federation-incomplete")
                    .theme(theme.clone())
                    .severity(sapid::AlertSeverity::Warning)
                    .content(t("actors.deactivation_federation_incomplete_detail")),
            ));
        }
        if !actor.is_deactivated() {
            let id = actor_id.clone();
            let success = reload.clone();
            children.push(Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("actors.actions"))
                    .fill(ui::compact_action(
                        theme.clone(),
                        t("actors.deactivate"),
                        ui::action_form(
                            scope,
                            theme.clone(),
                            t("actors.deactivate"),
                            vec![],
                            Some(Danger::new(station::suffix(&id)).without_reason()),
                            move |_| {
                                let id = id.clone();
                                async move {
                                    let result = crate::api::actors::deactivate_account(&id).await;
                                    match &result {
                                        Ok(()) => emit_admin_audit(
                                            "account",
                                            &id,
                                            "deactivate",
                                            AdminAuditOutcome::Accepted,
                                            None,
                                        ),
                                        Err(error) => emit_admin_audit(
                                            "account",
                                            &id,
                                            "deactivate",
                                            AdminAuditOutcome::Rejected,
                                            Some(&error.message),
                                        ),
                                    }
                                    result
                                }
                            },
                            move || success.reload(),
                        ),
                    )),
            ));
        } else {
            children.push(Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("actors.actions"))
                    .fill(text().content(t("actors.already_deactivated"))),
            ));
        }
        let actions = children.pop().expect("actor actions");
        let overview = children.remove(2);
        children.push(p::grid(vec![overview, actions]));
        p::stack(children)
    })
}
