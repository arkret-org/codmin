use std::future::Future;

use glory::prelude::*;
use serde::Serialize;

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
pub fn stack(theme: sapid::Theme, children: Vec<View>) -> View {
    Box::new(sapid::stack().theme(theme).gap(16.0).fill(children))
}
pub fn record(value: &impl Serialize) -> View {
    let value = serde_json::to_value(value).unwrap_or_default();
    let mut fields = Vec::new();
    flatten("", &value, &mut fields);
    ui::details(fields)
}
fn flatten(prefix: &str, value: &serde_json::Value, fields: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) if !map.is_empty() => {
            for (key, value) in map {
                let label = if prefix.is_empty() {
                    key.replace('_', " ")
                } else {
                    format!("{prefix} / {}", key.replace('_', " "))
                };
                flatten(&label, value, fields);
            }
        }
        serde_json::Value::Array(values) if !values.is_empty() => {
            for (index, value) in values.iter().enumerate() {
                flatten(&format!("{prefix} [{}]", index + 1), value, fields);
            }
        }
        _ => fields.push((
            prefix.into(),
            match value {
                serde_json::Value::String(v) => v.clone(),
                serde_json::Value::Null => "—".into(),
                _ => value.to_string(),
            },
        )),
    }
}
pub fn cursor<T, F, Fut, R>(
    scope: &Scope,
    theme: sapid::Theme,
    fetch: F,
    render: R,
) -> (View, Resource<CursorPage<T>>)
where
    T: Clone + 'static,
    F: Fn(Option<String>) -> Fut + 'static,
    Fut: Future<Output = std::result::Result<CursorPage<T>, HttpError>> + 'static,
    R: Fn(&Scope, T, Resource<CursorPage<T>>, ui::MutationController) -> View + 'static,
{
    let mutations = ui::MutationController::new(scope);
    let cursors = scope.cage(vec![None::<String>]);
    let data = ui::resource(scope, move || {
        fetch(cursors.get().last().cloned().flatten())
    });
    let data_rows = data.clone();
    let view_theme = theme.clone();
    let body = ui::resource_view(scope, theme.clone(), data.clone(), move |scope, page| {
        let mut children: Vec<View> = Vec::new();
        children.push(ui::details(vec![
            (
                t("coauth_accounts.page_prefix"),
                cursors.get().len().to_string(),
            ),
            ("Loaded".into(), page.data.len().to_string()),
            (
                "Total".into(),
                page.total.map_or_else(|| "—".into(), |n| n.to_string()),
            ),
        ]));
        if page.data.is_empty() {
            children.push(Box::new(text().content(t("common.no_results"))));
        }
        for row in page.data {
            children.push(render(scope, row, data_rows.clone(), mutations.clone()));
        }
        let previous_data = data_rows.clone();
        let next_data = data_rows.clone();
        let next = page.next_cursor;
        children.push(Box::new(
            sapid::group().theme(view_theme.clone()).fill((
                sapid::button()
                    .theme(view_theme.clone())
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
                    .theme(view_theme.clone())
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
        stack(view_theme.clone(), children)
    });
    (body, data)
}
pub fn heading(title: &str) -> View {
    ui::heading(t(title))
}
