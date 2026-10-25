//! Shared presentation for the Station's typed administrative reads.
use std::collections::BTreeSet;
use std::future::Future;
use std::rc::Rc;
use std::result::Result;

use glory::prelude::*;
use serde::Serialize;
use serde_json::Value;

use crate::components::ui::{self, View};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

#[derive(Clone, Default)]
pub struct Query {
    pub fields: Vec<String>,
    pub page: u64,
    pub cursors: Vec<Option<String>>,
}
impl Query {
    pub fn field(&self, index: usize) -> &str {
        self.fields.get(index).map(String::as_str).unwrap_or("")
    }
    pub fn cursor(&self) -> Option<&str> {
        self.cursors.last().and_then(Option::as_deref)
    }
}

type DocumentView = Rc<dyn Fn(&Scope, sapid::Theme) -> View>;

#[derive(Clone, Default)]
pub struct Document {
    pub sections: Vec<(String, Value)>,
    pub next_cursor: Option<String>,
    pub links: Vec<(String, String)>,
    pub numeric: Option<NumericPagination>,
    pub view: Option<DocumentView>,
}
#[derive(Clone, Copy)]
pub struct NumericPagination {
    pub page: u64,
    pub per_page: u64,
    pub total: Option<u64>,
}
impl NumericPagination {
    fn last_page(self) -> Option<u64> {
        self.total
            .map(|total| total.div_ceil(self.per_page.max(1)).max(1))
    }
}
impl Document {
    pub fn view(mut self, render: impl Fn(&Scope, sapid::Theme) -> View + 'static) -> Self {
        self.view = Some(Rc::new(render));
        self
    }
    pub fn section(
        mut self,
        title: impl Into<String>,
        value: &impl Serialize,
    ) -> Result<Self, HttpError> {
        self.sections.push((
            title.into(),
            serde_json::to_value(value).map_err(|e| HttpError::message(e.to_string()))?,
        ));
        Ok(self)
    }
    pub fn next(mut self, value: Option<String>) -> Self {
        self.next_cursor = value.filter(|value| !value.is_empty());
        self
    }
    pub fn link(mut self, title: impl Into<String>, href: impl Into<String>) -> Self {
        self.links.push((title.into(), href.into()));
        self
    }
    pub fn numeric(mut self, page: u64, per_page: u64, total: Option<u64>) -> Self {
        self.numeric = Some(NumericPagination {
            page,
            per_page: per_page.max(1),
            total,
        });
        self
    }
}

fn jump_to(
    query: Cage<Query>,
    data: Cage<Option<Result<Document, HttpError>>>,
    raw: &str,
) -> Result<bool, String> {
    let target = raw
        .trim()
        .parse::<u64>()
        .map_err(|_| "Enter a positive page number".to_owned())?;
    let numeric = data.with_untracked(|value| {
        value
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .and_then(|document| document.numeric)
    });
    let Some(numeric) = numeric else {
        return Ok(false);
    };
    let last = numeric
        .last_page()
        .ok_or_else(|| "Direct page selection requires a known total".to_owned())?;
    if !(1..=last).contains(&target) {
        return Err(format!("Page must be between 1 and {last}"));
    }
    if query.with_untracked(|query| query.page == target) {
        return Ok(false);
    }
    data.revise(|value| *value = None);
    query.revise(|query| {
        query.page = target;
        query.cursors = vec![None];
    });
    Ok(true)
}

pub fn path(base: &str, id: &str) -> String {
    format!("{base}/{}", urlencoding::encode(id))
}

/// Preserve structured data as fields and tables, including nested contract fields.
pub fn value_view(theme: sapid::Theme, value: Value) -> View {
    let mut value = Some(value);
    Box::new(widget(move |scope| {
        match value.take().ok_or(glory::ui::Error::Gone)? {
            Value::Object(fields) => {
                let mut simple = Vec::new();
                let mut nested = Vec::new();
                for (key, value) in fields {
                    if value.is_object() || value.is_array() {
                        nested.push((key, value));
                    } else {
                        simple.push((key.replace('_', " "), scalar(&value)));
                    }
                }
                ui::details(simple).show_in(scope)?;
                for (key, value) in nested {
                    ui::heading(key.replace('_', " ")).show_in(scope)?;
                    value_view(theme.clone(), value).show_in(scope)?;
                }
            }
            Value::Array(values) if values.is_empty() => {
                text().content(t("common.no_results")).show_in(scope)?;
            }
            Value::Array(values) => {
                let (headers, rows) = tabular(&values);
                ui::table(theme.clone(), headers, rows).show_in(scope)?;
            }
            value => text().content(scalar(&value)).show_in(scope)?,
        }
        Ok(())
    }))
}
fn readable(value: &Value) -> String {
    match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| format!("{}: {}", key.replace('_', " "), readable(value)))
            .collect::<Vec<_>>()
            .join("; "),
        Value::Array(values) => values.iter().map(readable).collect::<Vec<_>>().join("; "),
        value => scalar(value),
    }
}
fn tabular(values: &[Value]) -> (Vec<String>, Vec<Vec<String>>) {
    let keys: Vec<_> = values
        .iter()
        .filter_map(Value::as_object)
        .flat_map(|row| row.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if keys.is_empty() {
        return (
            vec!["Value".to_owned()],
            values.iter().map(|value| vec![readable(value)]).collect(),
        );
    }
    let headers = keys.iter().map(|key| key.replace('_', " ")).collect();
    let rows = values
        .iter()
        .map(|row| {
            keys.iter()
                .map(|key| row.get(key).map(readable).unwrap_or_else(|| "—".into()))
                .collect()
        })
        .collect();
    (headers, rows)
}
fn export_section(name: &str, section: Value) {
    match section {
        Value::Array(rows) => {
            let (headers, rows) = tabular(&rows);
            let borrowed = headers.iter().map(String::as_str).collect::<Vec<_>>();
            let csv = crate::utils::fmt::csv::build_csv(&borrowed, &rows);
            crate::utils::fmt::csv::export_to_csv(&format!("{}.csv", name.replace(' ', "-")), &csv);
        }
        Value::Object(fields) => {
            for (field, value) in fields {
                if value.is_array() {
                    export_section(&format!("{name}-{field}"), value);
                }
            }
        }
        _ => {}
    }
}
fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "—".into(),
        Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

pub fn render(theme: sapid::Theme, document: Document) -> View {
    if let Some(render) = document.view.as_ref() {
        let render = render.clone();
        return Box::new(widget(move |scope| {
            render(scope, theme.clone()).show_in(scope)
        }));
    }
    let mut document = Some(document);
    Box::new(widget(move |scope| {
        let document = document.take().ok_or(glory::ui::Error::Gone)?;
        for (title, value) in document.sections {
            sapid::card()
                .theme(theme.clone())
                .title(title)
                .fill(value_view(theme.clone(), value))
                .show_in(scope)?;
        }
        for (label, href) in document.links {
            link().label(label).target(href).show_in(scope)?;
        }
        Ok(())
    }))
}

pub fn read<F, Fut>(scope: &Scope, theme: sapid::Theme, title: String, loader: F) -> View
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<Document, HttpError>> + 'static,
{
    let data = ui::resource(scope, loader);
    let reload = data.clone();
    let body_theme = theme.clone();
    Box::new(
        col().class("station-page").fill((
            row().class("page-header").fill((
                ui::heading(title),
                sapid::button()
                    .theme(theme.clone())
                    .variant(sapid::ButtonVariant::Secondary)
                    .label(t("common.refresh"))
                    .on_press(move || reload.reload()),
            )),
            ui::resource_content(scope, theme, data, move |_, document| {
                render(body_theme.clone(), document)
            }),
        )),
    )
}

pub fn browse<F, Fut>(
    scope: &Scope,
    theme: sapid::Theme,
    title: String,
    filters: Vec<(&str, &str)>,
    loader: F,
) -> View
where
    F: Fn(Query) -> Fut + 'static,
    Fut: Future<Output = Result<Document, HttpError>> + 'static,
{
    let page_description = description(&title);
    browse_with_description(scope, theme, title, page_description, filters, loader)
}

pub fn browse_with_description<F, Fut>(
    scope: &Scope,
    theme: sapid::Theme,
    title: String,
    page_description: String,
    filters: Vec<(&str, &str)>,
    loader: F,
) -> View
where
    F: Fn(Query) -> Fut + 'static,
    Fut: Future<Output = Result<Document, HttpError>> + 'static,
{
    let has_filters = !filters.is_empty();
    let storage_key = if title == t("nav.realms") {
        Some("codmin.realms.autorefresh")
    } else if title == t("nav.devices") {
        Some("codmin.devices.autorefresh")
    } else {
        None
    };
    let refresh_key = format!(
        "station-refresh-{}",
        filters
            .first()
            .map(|(name, _)| *name)
            .unwrap_or("inventory")
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase()
    );
    let query = scope.cage(Query {
        fields: filters.iter().map(|(_, value)| value.to_string()).collect(),
        page: 1,
        cursors: vec![None],
    });
    let drafts: Vec<_> = filters
        .iter()
        .map(|(_, value)| scope.cage(value.to_string()))
        .collect();
    let data = ui::resource(scope, move || loader(query.get()));
    let refresh = data.clone();
    let next_data = data.clone();
    let previous_data = data.clone();
    let export_data = data.clone();
    let render_theme = theme.clone();
    let fields = filters
        .into_iter()
        .zip(drafts.iter().copied())
        .map(|((label, _), value)| {
            col().fill((
                text().class("field-label").content(t(label)),
                sapid::text_field()
                    .theme(theme.clone())
                    .name(t(label))
                    .value(value)
                    .on_value_change(move |next| value.revise(|value| *value = next)),
            ))
        })
        .collect::<Vec<_>>();
    let stored = storage_key
        .and_then(crate::utils::storage::get_item)
        .filter(|value| matches!(value.as_str(), "off" | "30s" | "1m" | "5m"))
        .unwrap_or_else(|| "off".into());
    let refresh_interval = scope.cage(Some(stored));
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        let automatic = data.clone();
        let active = Rc::new(RefCell::new(None::<glory::core::TaskToken>));
        let weak = scope.downgrade();
        scope.effect(move || {
            if let Some(token) = active.borrow_mut().take() {
                token.cancel();
            }
            let milliseconds = match refresh_interval.get().as_deref() {
                Some("30s") => 30_000,
                Some("1m") => 60_000,
                Some("5m") => 300_000,
                _ => return,
            };
            let Some(scope) = weak.upgrade() else {
                return;
            };
            let reload = automatic.clone();
            if let Ok(token) = scope.owner().spawn(async move {
                loop {
                    gloo_timers::future::TimeoutFuture::new(milliseconds).await;
                    reload.reload();
                }
            }) {
                *active.borrow_mut() = Some(token);
            }
        });
    }
    let previous_ready = data.data;
    let previous_enabled =
        scope.bond(move || query.get().page > 1 && previous_ready.with(Option::is_some));
    let next_enabled = data.data.map(|data| {
        data.as_ref().is_some_and(|result| {
            result.as_ref().is_ok_and(|value| {
                match value.numeric.and_then(NumericPagination::last_page) {
                    Some(last) => value.numeric.is_some_and(|numeric| numeric.page < last),
                    None => value.next_cursor.is_some(),
                }
            })
        })
    });
    let page_data = data.data;
    let jump = scope.cage(String::new());
    let jump_error = scope.cage(String::new());
    let numeric_available = page_data.map(|value| {
        value.as_ref().is_some_and(|result| {
            result
                .as_ref()
                .is_ok_and(|document| document.numeric.is_some())
        })
    });
    let numeric_status = page_data.map(|value| {
        value
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .and_then(|document| document.numeric)
            .map_or_else(String::new, |numeric| {
                match (numeric.total, numeric.last_page()) {
                    (Some(total), Some(last)) => format!(
                        "Total: {total}; page {} of {last}; {} per page",
                        numeric.page, numeric.per_page
                    ),
                    _ => format!(
                        "Page {}; total unavailable; {} per page",
                        numeric.page, numeric.per_page
                    ),
                }
            })
    });
    let jump_enabled = page_data.map(|value| {
        value.as_ref().is_some_and(|value| {
            value.as_ref().is_ok_and(|document| {
                document
                    .numeric
                    .and_then(NumericPagination::last_page)
                    .is_some()
            })
        })
    });
    let jump_theme = theme.clone();
    let numeric_controls = when(numeric_available, move || {
        row().class("numeric-pagination").fill((
            text().content(numeric_status),
            sapid::text_field()
                .theme(jump_theme.clone())
                .name("Page number")
                .value(jump)
                .on_value_change(move |value| jump.revise(|old| *old = value)),
            sapid::button()
                .theme(jump_theme.clone())
                .label("Go to page")
                .enabled(jump_enabled)
                .on_press(
                    move || match jump_to(query, page_data, &jump.get_untracked()) {
                        Ok(_) => jump_error.revise(String::clear),
                        Err(error) => jump_error.revise(|value| *value = error),
                    },
                ),
            text().content(jump_error),
        ))
    });
    let filter_action: View = if has_filters {
        Box::new(
            sapid::button()
                .theme(theme.clone())
                .label("Apply filters")
                .on_press(move || {
                    query.revise(|query| {
                        query.fields = drafts.iter().map(|value| value.get()).collect();
                        query.page = 1;
                        query.cursors = vec![None];
                    });
                }),
        )
    } else {
        Box::new(group())
    };
    Box::new(
        col().class("station-page").fill((
            col().class("page-header").fill((
                ui::heading(title),
                text().class("page-description").content(page_description),
            )),
            row().class("list-toolbar").fill((
                row().class("filter-fields").fill(fields),
                row().class("toolbar-actions").fill((
                    filter_action,
                    sapid::button()
                        .theme(theme.clone())
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("common.refresh"))
                        .on_press(move || refresh.reload()),
                    sapid::button()
                        .theme(theme.clone())
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("common.export_csv"))
                        .on_press(move || {
                            if let Some(Ok(document)) = export_data.data.get_untracked() {
                                for (name, section) in document.sections {
                                    export_section(&name, section);
                                }
                            }
                        }),
                    sapid::native_select(refresh_key)
                        .theme(theme.clone())
                        .label(t("common.auto_refresh"))
                        .value(refresh_interval)
                        .options([
                            sapid::select_option("off", "Off"),
                            sapid::select_option("30s", "30s"),
                            sapid::select_option("1m", "1m"),
                            sapid::select_option("5m", "5m"),
                        ])
                        .on_value_change(move |value| {
                            let value = value
                                .filter(|value| {
                                    matches!(value.as_str(), "off" | "30s" | "1m" | "5m")
                                })
                                .unwrap_or_else(|| "off".into());
                            if let Some(key) = storage_key {
                                crate::utils::storage::set_item(key, &value);
                            }
                            refresh_interval.revise(|current| *current = Some(value));
                        }),
                )),
            )),
            ui::resource_content(scope, theme.clone(), data.clone(), move |_, document| {
                render(render_theme.clone(), document)
            }),
            row().class("table-pagination").fill((
                numeric_controls,
                row().class("pagination-actions").fill((
                    sapid::button()
                        .theme(theme.clone())
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("common.previous"))
                        .enabled(previous_enabled)
                        .on_press(move || {
                            if query.with_untracked(|query| query.page <= 1)
                                || !previous_data.data.with_untracked(Option::is_some)
                            {
                                return;
                            }
                            previous_data.data.revise(|value| *value = None);
                            query.revise(|query| {
                                query.page -= 1;
                                if query.cursors.len() > 1 {
                                    query.cursors.pop();
                                }
                            });
                        }),
                    text().content(query.map(|query| format!("Page {}", query.page))),
                    sapid::button()
                        .theme(theme)
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("common.next"))
                        .enabled(next_enabled)
                        .on_press(move || {
                            let next = next_data.data.with_untracked(|value| {
                                let document = value.as_ref()?.as_ref().ok()?;
                                if let Some(numeric) = document.numeric {
                                    if numeric.last_page().is_some_and(|last| numeric.page >= last)
                                    {
                                        return None;
                                    }
                                    if numeric.last_page().is_none()
                                        && document.next_cursor.is_none()
                                    {
                                        return None;
                                    }
                                    Some((None, true))
                                } else {
                                    document.next_cursor.clone().map(|next| (Some(next), false))
                                }
                            });
                            if let Some((next, numeric)) = next {
                                next_data.data.revise(|value| *value = None);
                                query.revise(|query| {
                                    query.page += 1;
                                    if numeric {
                                        query.cursors = vec![None];
                                    } else {
                                        query.cursors.push(next);
                                    }
                                });
                            }
                        }),
                )),
            )),
        )),
    )
}

fn description(title: &str) -> String {
    for (key, description) in [
        ("nav.actors", "actors.description"),
        ("nav.devices", "devices.subtitle"),
        ("nav.realms", "realms.description"),
        ("nav.spaces", "spaces.description"),
        ("nav.federation", "federation.subtitle"),
        ("federation.title", "federation.subtitle"),
        ("handles.title", "handles.description"),
        ("nav.invite_tokens", "invite_tokens.description"),
    ] {
        if title == t(key) {
            let value = t(description);
            return if value == description {
                String::new()
            } else {
                value
            };
        }
    }
    String::new()
}
pub fn selection(title: &str) -> View {
    Box::new(
        col().class("selection-required").fill((
            crate::appearance::icon("layers"),
            ui::heading(t("selection_required.title").replace("{kind}", title)),
            text()
                .class("page-description")
                .content(t("selection_required.description").replace("{kind}", title)),
        )),
    )
}
pub fn placeholder(id: &str) -> bool {
    id.trim().is_empty() || matches!(id, "selected" | "select" | "_" | ":realm_id")
}

pub fn suffix(id: &str) -> String {
    let mut chars = id.chars().rev().take(4).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use glory::core::executor::Executor;
    use glory::render::memory::Memory;
    use glory::render::{
        Capabilities, Event, Input, Mutation, NodeId, Props, Query as RenderQuery, QueryToken,
        Receipt, Render,
    };

    use super::*;

    struct QueuedRender {
        memory: Memory,
        events: Rc<RefCell<Vec<Input>>>,
    }
    impl Render for QueuedRender {
        fn style_profile(&self) -> glory::style::Profile {
            self.memory.style_profile()
        }
        fn capabilities(&self) -> Capabilities {
            self.memory.capabilities()
        }
        fn apply(&mut self, values: &[Mutation]) -> Result<Receipt, glory::render::Error> {
            self.memory.apply(values)
        }
        fn request(
            &mut self,
            query: RenderQuery,
            token: QueryToken,
        ) -> Result<(), glory::render::Error> {
            self.memory.request(query, token)
        }
        fn drain(&mut self, inputs: &mut Vec<Input>) {
            self.memory.drain(inputs);
            inputs.extend(self.events.borrow_mut().drain(..));
        }
    }
    fn button(app: &App, label: &str) -> NodeId {
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            if matches!(node.descriptor.view().props, Props::Button(value) if value.label == label)
            {
                return id;
            }
            pending.extend(node.children.iter().copied());
        }
        panic!("pagination control missing: {label}")
    }
    #[test]
    fn queued_double_next_and_previous_advance_only_once_per_loaded_page() {
        let host = Rc::new(Executor::new(32));
        let events = Rc::new(RefCell::new(Vec::new()));
        let input = events.clone();
        let app = App::with_host(host.clone(), move |runtime| QueuedRender {
            memory: Memory::new(runtime),
            events: input,
        });
        let requests = Rc::new(RefCell::new(Vec::new()));
        let seen = requests.clone();
        app.mount(browse(app.scope(), sapid::Theme::default(), "Pagination fixture".into(), vec![], move |query| {
            seen.borrow_mut().push(query.clone());
            async move { Ok(Document::default().next(Some(format!("cursor-{}", query.page + 1)))) }
        })).unwrap();
        while host.poll_one() {}
        app.flush().unwrap();
        let next = button(&app, &t("common.next"));
        events.borrow_mut().extend([
            Input::Event {
                node: next,
                event: Event::Activate,
            },
            Input::Event {
                node: next,
                event: Event::Activate,
            },
        ]);
        app.pump().unwrap();
        assert_eq!(requests.borrow().len(), 2);
        assert_eq!(requests.borrow().last().unwrap().page, 2);
        assert_eq!(requests.borrow().last().unwrap().cursor(), Some("cursor-2"));
        while host.poll_one() {}
        app.flush().unwrap();
        let previous = button(&app, &t("common.previous"));
        events.borrow_mut().extend([
            Input::Event {
                node: previous,
                event: Event::Activate,
            },
            Input::Event {
                node: previous,
                event: Event::Activate,
            },
        ]);
        app.pump().unwrap();
        assert_eq!(requests.borrow().len(), 3);
        assert_eq!(requests.borrow().last().unwrap().page, 1);
        assert_eq!(requests.borrow().last().unwrap().cursor(), None);
    }

    #[test]
    fn numeric_jump_validates_bounds_and_rejects_duplicates_and_opaque_cursors() {
        let app = App::memory();
        let query = app.scope().cage(Query {
            fields: vec![],
            page: 1,
            cursors: vec![None],
        });
        let data = app
            .scope()
            .cage(Some(Ok(Document::default().numeric(1, 25, Some(101)))));
        assert!(jump_to(query, data, "0").is_err());
        assert!(jump_to(query, data, "6").is_err());
        assert!(jump_to(query, data, "").is_err());
        assert!(jump_to(query, data, "two").is_err());
        assert!(!jump_to(query, data, "1").unwrap());
        assert!(jump_to(query, data, "5").unwrap());
        assert!(!jump_to(query, data, "5").unwrap());
        assert_eq!(query.get_untracked().page, 5);
        assert_eq!(query.get_untracked().cursors, [None]);
        assert!(data.get_untracked().is_none());
        data.revise(|value| {
            *value = Some(Ok(Document::default().next(Some("opaque-next".into()))))
        });
        assert!(!jump_to(query, data, "2").unwrap());
        assert_eq!(query.get_untracked().page, 5);
        data.revise(|value| *value = Some(Ok(Document::default().numeric(5, 25, None))));
        assert!(jump_to(query, data, "2").is_err());
        assert_eq!(query.get_untracked().page, 5);
    }
}
