use std::future::Future;

use glory::prelude::*;

use crate::components::ui::{self, Resource, View};
use crate::types::CursorPage;
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

pub fn suffix(value: &str) -> String {
    value
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}
pub fn optional(value: Option<String>) -> String {
    value.unwrap_or_else(|| "—".into())
}
pub fn cell(value: impl Into<String>) -> View {
    let value: String = value.into();
    Box::new(text().content(value))
}
pub fn mono(value: impl Into<String>) -> View {
    let value: String = value.into();
    Box::new(text().class("mono table-id").content(value))
}
pub fn muted(value: impl Into<String>) -> View {
    let value: String = value.into();
    Box::new(text().class("text-muted").content(value))
}
pub fn stack(theme: sapid::Theme, children: Vec<View>) -> View {
    Box::new(
        sapid::stack()
            .theme(theme)
            .class("coauth-stack")
            .gap(16.0)
            .fill(children),
    )
}
pub fn group(children: Vec<View>) -> View {
    Box::new(view().class("coauth-row-actions").fill(children))
}
pub fn page(title: &str, description: &str, actions: Vec<View>, children: Vec<View>) -> View {
    page_heading(t(title), t(description), actions, children)
}
pub fn page_heading(
    title: String,
    description: String,
    actions: Vec<View>,
    children: Vec<View>,
) -> View {
    Box::new(view().class("admin-page coauth-page").fill((
        view().class("page-header").fill((
            view().class("page-heading").fill((
                ui::heading(title),
                text().class("page-description").content(description),
            )),
            view().class("page-actions").fill(actions),
        )),
        view().class("coauth-page-content").fill(children),
    )))
}
pub fn panel(theme: sapid::Theme, title: impl Into<String>, content: View) -> View {
    let title: String = title.into();
    Box::new(
        sapid::card()
            .theme(theme)
            .class("coauth-panel")
            .heading_level(2)
            .title(title)
            .fill(content),
    )
}
pub fn fields(values: Vec<(&str, String)>) -> View {
    ui::details(
        values
            .into_iter()
            .map(|(key, value)| (t(key), value))
            .collect(),
    )
}
pub fn attributes(values: Vec<(&str, String)>, class: &str) -> View {
    Box::new(
        view().class(format!("coauth-attributes {class}")).fill(
            values
                .into_iter()
                .map(|(key, value)| {
                    view().class("coauth-attribute").fill((
                        text().class("detail-label text-muted").content(t(key)),
                        text().class("attribute-value").content(value),
                    ))
                })
                .collect::<Vec<_>>(),
        ),
    )
}
pub fn badge(theme: sapid::Theme, label: impl Into<String>, kind: &str) -> View {
    let label: String = label.into();
    Box::new(
        sapid::badge()
            .theme(theme)
            .class(format!("status-badge status-{kind}"))
            .content(label),
    )
}
pub fn refresh<T: Clone + 'static>(theme: sapid::Theme, source: Resource<T>) -> View {
    Box::new(
        sapid::button()
            .theme(theme)
            .size(sapid::Size::Small)
            .variant(sapid::ButtonVariant::Secondary)
            .label(t("common.refresh"))
            .on_press(move || source.reload()),
    )
}
pub fn resource_body<T: Clone + 'static>(
    theme: sapid::Theme,
    source: Resource<T>,
    render: impl Fn(&Scope, T) -> View + 'static,
) -> View {
    ui::reactive(source.data, move |scope, result| match result {
        None => Box::new(
            view()
                .class("page-loading")
                .fill(text().content(t("common.loading"))),
        ),
        Some(Ok(value)) => render(scope, value),
        Some(Err(error)) => {
            let reload = source.clone();
            let mut message = error.message;
            if let Some(body) = error.body {
                message.push_str(&format!(" · {}", body.errcode));
            }
            if let Some(id) = error.request_id {
                message.push_str(&format!(" · request_id: {id}"));
            }
            Box::new(
                view().class("resource-error").fill((
                    sapid::alert("coauth-error")
                        .theme(theme.clone())
                        .severity(sapid::AlertSeverity::Danger)
                        .content(message),
                    sapid::button()
                        .theme(theme.clone())
                        .label(t("common.retry"))
                        .on_press(move || reload.reload()),
                )),
            )
        }
    })
}
pub fn table(theme: sapid::Theme, headers: &[&str], rows: Vec<Vec<View>>, empty_key: &str) -> View {
    if rows.is_empty() {
        return Box::new(
            view().class("table-panel-empty").fill((
                ui::table_widgets(
                    theme,
                    headers.iter().map(|key| t(key)).collect(),
                    Vec::new(),
                ),
                view()
                    .class("table-empty")
                    .fill(text().content(t(empty_key))),
            )),
        );
    }
    ui::table_widgets(theme, headers.iter().map(|key| t(key)).collect(), rows)
}
pub fn cursor<T, F, Fut, R>(
    scope: &Scope,
    theme: sapid::Theme,
    headers: &'static [&'static str],
    empty: &'static str,
    fetch: F,
    render: R,
) -> (View, Resource<CursorPage<T>>)
where
    T: Clone + 'static,
    F: Fn(Option<String>) -> Fut + 'static,
    Fut: Future<Output = std::result::Result<CursorPage<T>, HttpError>> + 'static,
    R: Fn(&Scope, T, Resource<CursorPage<T>>, ui::MutationController) -> Vec<View> + 'static,
{
    let mutations = ui::MutationController::new(scope);
    let cursors = scope.cage(vec![None::<String>]);
    let data = ui::resource(scope, move || {
        fetch(cursors.get().last().cloned().flatten())
    });
    let current = data.clone();
    let body = resource_body(theme.clone(), data.clone(), move |scope, page| {
        let rows = page
            .data
            .into_iter()
            .map(|row| render(scope, row, current.clone(), mutations.clone()))
            .collect();
        let previous_data = current.clone();
        let next_data = current.clone();
        let next = page.next_cursor;
        let paging = Box::new(
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
                        .theme(theme.clone())
                        .size(sapid::Size::Small)
                        .variant(sapid::ButtonVariant::Secondary)
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
                        .size(sapid::Size::Small)
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("common.next"))
                        .enabled(next.is_some())
                        .on_press(move || {
                            if next_data.data.get_untracked().is_none() {
                                return;
                            }
                            if let Some(next) = next.clone() {
                                next_data.data.revise(|value| *value = None);
                                cursors.revise(|value| value.push(Some(next)));
                            }
                        }),
                )),
            )),
        ) as View;
        Box::new(
            view()
                .class("table-panel")
                .fill((table(theme.clone(), headers, rows, empty), paging)),
        )
    });
    (Box::new(view().class("coauth-list").fill(body)), data)
}
