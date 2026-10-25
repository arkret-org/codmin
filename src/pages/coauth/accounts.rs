use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth_account_detail.account_id",
    "coauth_account_detail.handle",
    "coauth_accounts.display_name",
    "common.status",
    "coauth_account_detail.primary_principal_id",
    "common.actions",
];
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let search = scope.cage(String::new());
    let draft = scope.cage(String::new());
    let cursors = scope.cage(vec![None::<String>]);
    let data = ui::resource(scope, move || {
        let cursor = cursors.get().last().cloned().flatten();
        let search = search.get();
        async move { coauth::list_accounts_cursor(cursor.as_deref(), 25, &search).await }
    });
    let filter = Box::new(
        view().class("filter-panel").fill((
            text()
                .class("text-muted")
                .content(t("coauth_accounts.filters_hint")),
            view().class("filter-row").fill(
                sapid::text_field()
                    .theme(theme.clone())
                    .class("account-search")
                    .name(t("coauth_accounts.search_placeholder"))
                    .value(draft)
                    .on_value_change(move |value| {
                        draft.revise(|old| *old = value.clone());
                        cursors.revise(|old| *old = vec![None]);
                        search.revise(|old| *old = value);
                    }),
            ),
        )),
    );
    let current = data.clone();
    let row_theme = theme.clone();
    let body = common::resource_body(theme.clone(), data.clone(), move |_, page| {
        let count = page.data.len();
        let count_label = page.total.map_or_else(
            || t("coauth_accounts.count_unknown"),
            |count| t("coauth_accounts.count_matching").replace("{count}", &count.to_string()),
        );
        let rows = page
            .data
            .into_iter()
            .map(|account| {
                let status = account.lifecycle_label().to_owned();
                let kind = if account.is_deactivated || account.is_locked {
                    "danger"
                } else {
                    "success"
                };
                vec![
                    common::mono(account.id.clone()),
                    common::cell(common::optional(account.username)),
                    common::cell(common::optional(account.display_name)),
                    common::badge(row_theme.clone(), status, kind),
                    common::mono(
                        account
                            .primary_principal_id
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "—".into()),
                    ),
                    Box::new(
                        link()
                            .class("table-link")
                            .target(format!(
                                "/coauth/accounts/{}",
                                urlencoding::encode(&account.id)
                            ))
                            .label(t("common.open")),
                    ) as ui::View,
                ]
            })
            .collect();
        let previous = current.clone();
        let following = current.clone();
        let next = page.next_cursor;
        Box::new(
            view().class("coauth-list").fill((
                text().class("list-summary text-muted").content(format!(
                    "{}{}{}{}{}",
                    t("coauth_accounts.loaded_prefix"),
                    count,
                    t("coauth_accounts.loaded_suffix"),
                    count_label,
                    t("coauth_accounts.sentence_end")
                )),
                view().class("table-panel").fill((
                    common::table(row_theme.clone(), HEADERS, rows, "coauth_accounts.empty"),
                    view().class("table-pagination").fill((
                        text().class("text-muted").content(format!(
                            "{}{}{}",
                            t("coauth_accounts.page_prefix"),
                            cursors.get().len(),
                            if next.is_some() {
                                t("coauth_accounts.page_more")
                            } else {
                                t("coauth_accounts.page_last")
                            }
                        )),
                        view().class("pagination-actions").fill((
                            sapid::button()
                                .theme(row_theme.clone())
                                .size(sapid::Size::Small)
                                .variant(sapid::ButtonVariant::Secondary)
                                .label(t("common.previous"))
                                .enabled(cursors.get().len() > 1)
                                .on_press(move || {
                                    if previous.data.get_untracked().is_none()
                                        || cursors.get_untracked().len() <= 1
                                    {
                                        return;
                                    }
                                    previous.data.revise(|value| *value = None);
                                    cursors.revise(|value| {
                                        value.pop();
                                    });
                                }),
                            sapid::button()
                                .theme(row_theme.clone())
                                .size(sapid::Size::Small)
                                .variant(sapid::ButtonVariant::Secondary)
                                .label(t("common.next"))
                                .enabled(next.is_some())
                                .on_press(move || {
                                    if following.data.get_untracked().is_none() {
                                        return;
                                    }
                                    if let Some(next) = next.clone() {
                                        following.data.revise(|value| *value = None);
                                        cursors.revise(|value| value.push(Some(next)));
                                    }
                                }),
                        )),
                    )),
                )),
            )),
        )
    });
    common::page(
        "coauth_accounts.title",
        "coauth_accounts.description",
        vec![common::refresh(theme, data)],
        vec![filter, body],
    )
}
