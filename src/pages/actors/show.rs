use glory::prelude::*;

use crate::components::ui::{self, Danger, View};
use crate::pages::station;
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
            ui::heading(
                actor
                    .display_name
                    .clone()
                    .unwrap_or_else(|| actor.id.clone()),
            ),
            station::value_view(
                theme.clone(),
                serde_json::to_value(&actor).unwrap_or_default(),
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
            children.push(ui::action_form(
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
            ));
        } else {
            children.push(Box::new(text().content(t("actors.already_deactivated"))));
        }
        Box::new(col().class("station-page").fill(children))
    })
}
