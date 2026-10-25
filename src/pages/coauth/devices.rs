use glory::prelude::*;

use super::common;
use crate::api::coauth_devices;
use crate::components::ui::{self, Danger};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, account_id: String) -> impl Widget + use<> {
    let mutations = ui::MutationController::new(scope);
    let account = account_id.clone();
    let data = ui::resource(scope, move || {
        let account = account.clone();
        async move { coauth_devices::list_account_devices(&account).await }
    });
    let mutation = data.clone();
    let row_theme = theme.clone();
    let body = ui::resource_view(scope, theme.clone(), data, move |scope, rows| {
        let mut children = vec![];
        for row in rows {
            let mut parts = vec![common::record(&row)];
            if row.is_revocable() {
                let id = row.id.clone();
                let account = account_id.clone();
                let mutation = mutation.clone();
                parts.push(ui::action_form_controlled(
                    scope,
                    &mutations,
                    row.id.clone(),
                    row_theme.clone(),
                    t("coauth_devices.revoke"),
                    vec![],
                    Some(Danger::new(common::suffix(&id))),
                    move |values| {
                        let id = id.clone();
                        let account = account.clone();
                        async move {
                            coauth_devices::revoke_account_device(&account, &id, &values[0]).await
                        }
                    },
                    move || mutation.reload(),
                ));
            }
            children.push(Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(row.id.clone())
                    .fill(common::stack(row_theme.clone(), parts)),
            ) as ui::View);
        }
        if children.is_empty() {
            children.push(Box::new(text().content(t("coauth_devices.empty_title"))));
        }
        common::stack(row_theme.clone(), children)
    });
    common::stack(theme, vec![common::heading("coauth_devices.title"), body])
}
