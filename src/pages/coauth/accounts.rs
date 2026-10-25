use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Field, View};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let search = scope.cage(String::new());
    let cursors = scope.cage(vec![None::<String>]);
    let data = ui::resource(scope, move || {
        let cursor = cursors.get().last().cloned().flatten();
        let search = search.get();
        async move { coauth::list_accounts_cursor(cursor.as_deref(), 25, &search).await }
    });
    let search_form = ui::action_form(
        scope,
        theme.clone(),
        t("coauth_accounts.search_placeholder"),
        vec![Field::new(t("coauth_accounts.search_placeholder"), "")],
        None,
        move |values| {
            cursors.revise(|v| *v = vec![None]);
            search.revise(|s| *s = values[0].clone());
            async { Ok(()) }
        },
        || {},
    );
    let current = data.clone();
    let outer = theme.clone();
    let body = ui::resource_view(scope, theme.clone(), data, move |_, page| {
        let mut children: Vec<View> = vec![ui::details(vec![
            ("Loaded".into(), page.data.len().to_string()),
            (
                "Total".into(),
                page.total.map_or_else(|| "—".into(), |n| n.to_string()),
            ),
        ])];
        for account in page.data {
            let href = format!("/coauth/accounts/{}", urlencoding::encode(&account.id));
            children.push(Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(
                        account
                            .display_name
                            .clone()
                            .unwrap_or_else(|| account.id.clone()),
                    )
                    .fill((
                        common::record(&account),
                        link().target(href).label(t("common.open")),
                    )),
            ));
        }
        if children.len() == 1 {
            children.push(Box::new(text().content(t("coauth_accounts.empty"))));
        }
        let previous_data = current.clone();
        let next_data = current.clone();
        let next = page.next_cursor;
        children.push(Box::new(
            sapid::group().theme(theme.clone()).fill((
                sapid::button()
                    .theme(theme.clone())
                    .label(t("common.previous"))
                    .enabled(cursors.get().len() > 1)
                    .on_press(move || {
                        if previous_data.data.get_untracked().is_none()
                            || cursors.get_untracked().len() <= 1
                        {
                            return;
                        }
                        previous_data.data.revise(|value| *value = None);
                        cursors.revise(|value| {
                            value.pop();
                        });
                    }),
                sapid::button()
                    .theme(theme.clone())
                    .label(t("common.next"))
                    .enabled(next.is_some())
                    .on_press(move || {
                        if next_data.data.get_untracked().is_none() {
                            return;
                        }
                        if let Some(next) = next.clone() {
                            next_data.data.revise(|value| *value = None);
                            cursors.revise(|v| v.push(Some(next)));
                        }
                    }),
            )),
        ));
        common::stack(theme.clone(), children)
    });
    common::stack(
        outer,
        vec![common::heading("coauth_accounts.title"), search_form, body],
    )
}
