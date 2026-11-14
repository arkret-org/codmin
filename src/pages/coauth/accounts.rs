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
    let data = ui::resource_retaining(scope, move || {
        let cursor_history = cursors.get();
        let cursor = cursor_history.last().cloned().flatten();
        let search = search.get();
        async move {
            let page = coauth::list_accounts_cursor(cursor.as_deref(), 25, &search).await?;
            Ok((search, cursor_history, page))
        }
    });
    let accepted_query = scope.cage(String::new());
    let source = data.clone();
    scope.effect(move || {
        if let Some(Ok((query, ..))) = source.data.get() {
            accepted_query.revise(|value| *value = query);
        }
    });
    let submitted = data.clone();
    let cleared = data.clone();
    let loading = data.loading;
    let result_state = data.data;
    crate::utils::drafts::track(scope, move || {
        if !draft.get_untracked().is_empty() || cursors.get_untracked().len() > 1 {
            crate::utils::drafts::State::Unsaved
        } else {
            crate::utils::drafts::State::Clean
        }
    });
    let filter = Box::new(
        view().class("filter-panel").fill((
            text()
                .class("text-muted")
                .content(t("coauth_accounts.filters_hint")),
            view().class("filter-row").fill((
                sapid::text_field()
                    .theme(theme.clone())
                    .class("account-search")
                    .name(t("coauth_accounts.search_placeholder"))
                    .value(draft)
                    .on_value_change(move |value| {
                        draft.revise(|old| *old = value);
                    }),
                sapid::button()
                    .theme(theme.clone())
                    .label(t("common.search"))
                    .enabled(
                        scope.bond(move || !loading.get() || draft.get().trim() != search.get()),
                    )
                    .on_press(move || {
                        if loading.get_untracked()
                            && draft.get_untracked().trim() == search.get_untracked()
                        {
                            return;
                        }
                        submitted.loading.revise(|value| *value = true);
                        cursors.revise(|old| *old = vec![None]);
                        search.revise(|old| *old = draft.get_untracked().trim().to_owned());
                        submitted.reload();
                    }),
                sapid::button()
                    .theme(theme.clone())
                    .variant(sapid::ButtonVariant::Secondary)
                    .label(t("common.clear"))
                    .enabled(scope.bond(move || {
                        !draft.get().is_empty()
                            || !search.get().is_empty()
                            || cursors.get().len() > 1
                    }))
                    .on_press(move || {
                        draft.revise(String::clear);
                        cursors.revise(|old| *old = vec![None]);
                        search.revise(String::clear);
                        cleared.reload();
                    }),
            )),
            view()
                .role(Role::Status)
                .class("account-query-state text-muted")
                .fill(text().content(scope.bond(move || {
                    if !matches!(result_state.get(), Some(Ok(_))) {
                        return t("coauth_accounts.not_loaded");
                    }
                    let label = if accepted_query.get().is_empty() {
                        t("coauth_accounts.all_accounts")
                    } else {
                        accepted_query.get()
                    };
                    t(if loading.get() {
                        "coauth_accounts.updating_previous"
                    } else {
                        "coauth_accounts.showing_query"
                    })
                    .replace("{query}", &label)
                }))),
            view()
                .role(Role::Alert)
                .class("account-query-error")
                .fill(text().content(data.refresh_error)),
        )),
    );
    let current = data.clone();
    let row_theme = theme.clone();
    let body = common::resource_body(
        theme.clone(),
        data.clone(),
        move |scope, (_query, cursor_history, page)| {
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
            let page_number = cursor_history.len();
            let previous_cursors = cursor_history.clone();
            let following_cursors = cursor_history;
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
                                page_number,
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
                                    .enabled(scope.bond(move || page_number > 1 && !loading.get()))
                                    .on_press(move || {
                                        if previous.data.get_untracked().is_none()
                                            || previous.loading.get_untracked()
                                            || previous_cursors.len() <= 1
                                        {
                                            return;
                                        }
                                        previous.loading.revise(|value| *value = true);
                                        let mut history = previous_cursors.clone();
                                        history.pop();
                                        cursors.revise(|value| *value = history);
                                    }),
                                sapid::button()
                                    .theme(row_theme.clone())
                                    .size(sapid::Size::Small)
                                    .variant(sapid::ButtonVariant::Secondary)
                                    .label(t("common.next"))
                                    .enabled(scope.bond({
                                        let has_next = next.is_some();
                                        move || has_next && !loading.get()
                                    }))
                                    .on_press(move || {
                                        if following.data.get_untracked().is_none()
                                            || following.loading.get_untracked()
                                        {
                                            return;
                                        }
                                        if let Some(next) = next.clone() {
                                            following.loading.revise(|value| *value = true);
                                            let mut history = following_cursors.clone();
                                            history.push(Some(next));
                                            cursors.revise(|value| *value = history);
                                        }
                                    }),
                            )),
                        )),
                    )),
                )),
            )
        },
    );
    common::page(
        "coauth_accounts.title",
        "coauth_accounts.description",
        vec![common::refresh(theme, data)],
        vec![filter, body],
    )
}
