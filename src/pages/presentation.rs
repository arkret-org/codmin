//! Explicit presentation primitives shared by typed administrative pages.
use glory::prelude::*;

use crate::components::ui::{self, View};
use crate::utils::i18n::t;

pub fn plain(value: impl ToString) -> View {
    Box::new(text().content(value.to_string()))
}
pub fn mono(value: impl ToString) -> View {
    Box::new(text().class("font-mono text-xs").content(value.to_string()))
}
pub fn link(theme: sapid::Theme, label: impl ToString, href: impl Into<String>) -> View {
    Box::new(
        sapid::anchor()
            .theme(theme)
            .class("text-primary hover:underline")
            .label(label.to_string())
            .href(href.into()),
    )
}
pub fn badge(theme: sapid::Theme, label: impl ToString, severity: &str) -> View {
    Box::new(
        sapid::badge()
            .theme(theme)
            .class(format!("admin-status status-{severity}"))
            .content(label.to_string()),
    )
}
pub fn optional(value: Option<impl ToString>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".into())
}
pub fn yes(value: bool) -> String {
    t(if value { "common.yes" } else { "common.no" })
}
pub fn table(theme: sapid::Theme, headers: &[&str], rows: Vec<Vec<View>>, empty: &str) -> View {
    let key = format!(
        "table-{}",
        headers
            .first()
            .copied()
            .unwrap_or("empty")
            .chars()
            .filter(|value| value.is_ascii_alphanumeric() || *value == '-')
            .collect::<String>()
    );
    table_with_key(theme, key, headers, rows, empty)
}

/// Tables shown together must have distinct keys, even when their headers match.
pub fn table_with_key(
    theme: sapid::Theme,
    key: impl Into<String>,
    headers: &[&str],
    rows: Vec<Vec<View>>,
    empty: &str,
) -> View {
    let columns = headers.len();
    let head = sapid::table_head().row(sapid::table_row("head").cells(
        headers.iter().enumerate().map(|(index, label)| {
            sapid::table_header_cell(format!("header-{index}")).text(t(label))
        }),
    ));
    let rows = if rows.is_empty() {
        vec![
            sapid::table_row("empty").cell(
                sapid::table_cell("empty")
                    .column_span(columns as u16)
                    .text(t(empty)),
            ),
        ]
    } else {
        rows.into_iter()
            .enumerate()
            .map(|(index, cells)| {
                sapid::table_row(format!("row-{index}")).cells(cells.into_iter().enumerate().map(
                    |(column, view)| {
                        let mut view = Some(view);
                        sapid::table_cell(format!("cell-{column}"))
                            .content(move || view.take().unwrap_or_else(|| plain("")))
                    },
                ))
            })
            .collect()
    };
    Box::new(
        view()
            .class("admin-table-scroll")
            .style(
                Style::new()
                    .width(Length::Percent(100.0))
                    .min_width(Length::Px(0.0))
                    .overflow_x(Overflow::Auto),
            )
            .fill(
                sapid::table(key)
                    .theme(theme)
                    .class("admin-table")
                    .head(head)
                    .body(sapid::table_body().rows(rows)),
            ),
    )
}
pub fn card(theme: sapid::Theme, title: &str, fields: Vec<(&str, String)>) -> View {
    Box::new(
        sapid::card().theme(theme).title(t(title)).fill(ui::details(
            fields
                .into_iter()
                .map(|(label, value)| (t(label), value))
                .collect(),
        )),
    )
}
pub fn grid(children: Vec<View>) -> View {
    Box::new(view().class("detail-grid").fill(children))
}
pub fn stack(children: Vec<View>) -> View {
    Box::new(col().class("page-sections").fill(children))
}
pub fn description(key: &str) -> View {
    Box::new(
        text()
            .class("text-muted-foreground page-description")
            .content(t(key)),
    )
}
pub fn bytes(value: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{size:.1} {}", units[unit])
    }
}
pub fn metric(theme: sapid::Theme, label: &str, value: impl ToString) -> View {
    Box::new(sapid::card().theme(theme).class("stat-card").fill((
        text().class("stat-label").content(t(label)),
        text().class("stat-value").content(value.to_string()),
    )))
}
pub fn breadcrumb(theme: sapid::Theme, label: &str, href: &str, current: impl ToString) -> View {
    Box::new(row().class("breadcrumbs").fill((
        link(theme, t(label), href.to_owned()),
        plain("/"),
        plain(current),
    )))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use glory::render::Props;

    use super::*;

    #[test]
    fn same_first_column_tables_with_distinct_keys_render_in_one_scope() {
        let app = App::memory();
        let theme = sapid::Theme::default();
        let headers = ["realm_organization.col_org", "common.status"];
        app.mount(col().fill((
            table_with_key(
                theme.clone(),
                "organization-relationships",
                &headers,
                vec![vec![plain("Engineering relationship"), plain("Verified")]],
                "common.empty",
            ),
            table_with_key(
                theme,
                "organization-controls",
                &headers,
                vec![vec![plain("Engineering control"), plain("Active")]],
                "common.empty",
            ),
        )))
        .expect("separate table keys must prevent duplicate semantic IDs");
        app.flush().unwrap();
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        let mut tables = 0;
        let mut contents = Vec::new();
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            let descriptor = node.descriptor.view();
            if descriptor
                .classes
                .iter()
                .any(|class| class == "sapid-table")
            {
                tables += 1;
            }
            if let Props::Text(value) = descriptor.props {
                contents.push(value.content.clone());
            }
            pending.extend(node.children.iter().copied());
        }
        assert_eq!(tables, 2);
        assert!(
            contents
                .iter()
                .any(|value| value == "Engineering relationship")
        );
        assert!(contents.iter().any(|value| value == "Engineering control"));
    }
}
