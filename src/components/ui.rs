//! Shared semantic widgets and owner-bound asynchronous admin operations.
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

use glory::prelude::*;
use glory::ui::Mounted;

use crate::utils::destructive_reason::destructive_reason_error;
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

pub type View = Box<dyn Widget>;

pub struct Resource<T: Clone + 'static> {
    pub data: Cage<Option<std::result::Result<T, HttpError>>>,
    pub revision: Cage<u64>,
}
impl<T: Clone + 'static> Clone for Resource<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            revision: self.revision,
        }
    }
}
impl<T: Clone + 'static> Resource<T> {
    pub fn reload(&self) {
        self.revision.revise(|n| *n = n.wrapping_add(1));
    }
}

pub fn resource<T, F, Fut>(scope: &Scope, loader: F) -> Resource<T>
where
    T: Clone + 'static,
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = std::result::Result<T, HttpError>> + 'static,
{
    let data = scope.cage(None);
    let revision = scope.cage(0u64);
    let generation = Rc::new(Cell::new(0u64));
    let active = Rc::new(RefCell::new(None::<glory::core::TaskToken>));
    let weak = scope.downgrade();
    scope.effect(move || {
        let _ = revision.get();
        if let Some(task) = active.borrow_mut().take() {
            task.cancel();
        }
        let Some(scope) = weak.upgrade() else {
            return;
        };
        let serial = generation.get().wrapping_add(1);
        generation.set(serial);
        data.revise(|value| *value = None);
        let future = loader();
        let generation = generation.clone();
        let weak = scope.downgrade();
        match scope.owner().spawn(async move {
            let result = future.await;
            if weak.upgrade().is_some() && generation.get() == serial {
                data.revise(|value| *value = Some(result));
            }
        }) {
            Ok(token) => *active.borrow_mut() = Some(token),
            Err(error) => {
                data.revise(|value| *value = Some(Err(HttpError::message(error.to_string()))))
            }
        }
    });
    Resource { data, revision }
}

/// Rebuild only this owned region; fields outside it keep their identity.
pub fn reactive<T: Clone + 'static>(
    source: Cage<T>,
    render: impl Fn(&Scope, T) -> View + 'static,
) -> View {
    let render = Rc::new(render);
    Box::new(
        view()
            .class("admin-reactive-region")
            .style(Style::new().min_width(Length::Px(0.0)))
            .fill(widget(move |scope| {
                let mounted = Rc::new(RefCell::new(None::<Mounted>));
                scope.retain(mounted.clone())?;
                let weak = scope.downgrade();
                let render = render.clone();
                scope.effect(move || {
                    let value = source.get();
                    let Some(parent) = weak.upgrade() else {
                        return;
                    };
                    mounted.borrow_mut().take();
                    let render = render.clone();
                    let next = parent.mount_with(|| {
                        widget(move |child| render(child, value.clone()).build(child))
                    });
                    match next {
                        Ok(next) => *mounted.borrow_mut() = Some(next),
                        Err(error) => log::error!("admin region render failed: {error}"),
                    }
                });
                Ok(())
            })),
    )
}

pub fn resource_view<T: Clone + 'static>(
    _scope: &Scope,
    theme: sapid::Theme,
    source: Resource<T>,
    render: impl Fn(&Scope, T) -> View + 'static,
) -> View {
    let refresh = source.clone();
    let refresh_theme = theme.clone();
    let body = resource_content(_scope, theme, source, render);
    Box::new(
        col().class("resource-panel").fill((
            sapid::button()
                .theme(refresh_theme)
                .label(t("common.refresh"))
                .on_press(move || refresh.reload()),
            body,
        )),
    )
}

pub fn resource_content<T: Clone + 'static>(
    _scope: &Scope,
    theme: sapid::Theme,
    source: Resource<T>,
    render: impl Fn(&Scope, T) -> View + 'static,
) -> View {
    reactive(source.data, move |scope, result| match result {
        None => Box::new(text().class("loading").content(t("common.loading"))),
        Some(Ok(value)) => render(scope, value),
        Some(Err(error)) => {
            let reload = source.clone();
            Box::new(
                col().class("resource-error").fill((
                    sapid::alert("resource-error")
                        .theme(theme.clone())
                        .severity(sapid::AlertSeverity::Danger)
                        .content(error_text(&error)),
                    sapid::button()
                        .theme(theme.clone())
                        .label(t("common.retry"))
                        .on_press(move || reload.reload()),
                )),
            )
        }
    })
}

pub fn heading(title: impl Into<String>) -> View {
    Box::new(
        text()
            .kind(TextKind::Heading(1))
            .class("admin-page-title")
            .style(Style {
                font_size: Some(24.0),
                line_height: Some(32.0),
                font_weight: Some(700.0),
                margin: Some(Edges::all(0.0)),
                ..Style::new()
            })
            .content(title.into()),
    )
}

pub fn auth_frame(max_width: f64, children: Vec<View>) -> View {
    Box::new(
        view()
            .role(Role::Main)
            .class("auth-page")
            .style(Style {
                display: Some(Display::Flex),
                flex_direction: Some(FlexDirection::Column),
                width: Some(Length::Percent(100.0)),
                max_width: Some(Limit::None),
                align: Some(Align::Center),
                justify: Some(Justify::Center),
                padding: Some(Edges::all(16.0)),
                margin: Some(Edges::all(0.0)),
                ..Style::new()
            })
            .fill(
                col()
                    .class("auth-content")
                    .style(
                        Style::new()
                            .display(Display::Flex)
                            .flex_direction(FlexDirection::Column)
                            .width(Length::Percent(100.0))
                            .max_width(Limit::Px(max_width))
                            .gap(24.0),
                    )
                    .fill(children),
            ),
    )
}

pub fn auth_brand(theme: sapid::Theme, title: String, subtitle: String, warning: bool) -> View {
    use glory::render::{
        GraphicPoint as Point, GraphicProps, GraphicStroke, PathCommand as Path, StrokeCap,
    };
    let ink = if warning { theme.danger } else { theme.accent };
    let commands = if warning {
        vec![
            Path::Move(Point::new(12., 3.)),
            Path::Line(Point::new(22., 21.)),
            Path::Line(Point::new(2., 21.)),
            Path::Close,
            Path::Move(Point::new(12., 8.)),
            Path::Line(Point::new(12., 14.)),
            Path::Move(Point::new(12., 17.)),
            Path::Line(Point::new(12., 18.)),
        ]
    } else {
        vec![
            Path::Move(Point::new(12., 3.)),
            Path::Line(Point::new(21., 7.)),
            Path::Line(Point::new(20., 14.)),
            Path::Cubic {
                first: Point::new(19., 18.),
                second: Point::new(15., 21.),
                end: Point::new(12., 22.),
            },
            Path::Cubic {
                first: Point::new(9., 21.),
                second: Point::new(5., 18.),
                end: Point::new(4., 14.),
            },
            Path::Line(Point::new(3., 7.)),
            Path::Close,
        ]
    };
    Box::new(
        col()
            .class("auth-brand")
            .style(Style {
                display: Some(Display::Flex),
                flex_direction: Some(FlexDirection::Column),
                align: Some(Align::Center),
                gap: Some(8.0),
                ..Style::new()
            })
            .fill((
                view()
                    .class("auth-brand-icon")
                    .style(Style {
                        display: Some(Display::Flex),
                        align: Some(Align::Center),
                        justify: Some(Justify::Center),
                        width: Some(Length::Px(64.0)),
                        height: Some(Length::Px(64.0)),
                        radius: Some(32.0),
                        background: Some(if warning {
                            theme.danger_soft
                        } else {
                            theme.accent_soft
                        }),
                        ..Style::new()
                    })
                    .fill(
                        glory::ui::graphic()
                            .style(
                                Style::new()
                                    .width(Length::Px(32.0))
                                    .height(Length::Px(32.0)),
                            )
                            .drawing(GraphicProps {
                                width: 24.,
                                height: 24.,
                                commands,
                                stroke: Some(GraphicStroke {
                                    color: ink,
                                    width: 2.,
                                    cap: StrokeCap::Round,
                                }),
                                ..GraphicProps::default()
                            }),
                    ),
                sapid::title(1)
                    .theme(theme.clone())
                    .class("auth-title")
                    .font_size(24.0)
                    .line_height(32.0)
                    .weight(700.0)
                    .content(title),
                sapid::text()
                    .theme(theme)
                    .class("auth-subtitle")
                    .font_size(14.0)
                    .line_height(20.0)
                    .variant(sapid::TextVariant::Muted)
                    .content(subtitle),
            )),
    )
}

fn error_text(error: &HttpError) -> String {
    let mut parts = vec![error.message.clone()];
    if error.status != 0 {
        parts.push(format!("HTTP {}", error.status));
    }
    if let Some(body) = error.body.as_ref() {
        parts.push(body.errcode.clone());
    }
    if let Some(id) = error.request_id.as_ref() {
        parts.push(format!("request_id: {id}"));
    }
    if let Some(delay) = error.retry_after_ms {
        parts.push(format!("retry_after_ms: {delay}"));
    }
    parts.join(" · ")
}

pub fn details(values: Vec<(String, String)>) -> View {
    Box::new(
        col()
            .class("admin-details")
            .style(
                Style::new()
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Column)
                    .gap(12.0)
                    .min_width(Length::Px(0.0)),
            )
            .fill(
                values
                    .into_iter()
                    .map(|(label, value)| {
                        row()
                            .class("admin-detail")
                            .style(Style {
                                display: Some(Display::Flex),
                                flex_direction: Some(FlexDirection::Row),
                                gap: Some(16.0),
                                align: Some(Align::Start),
                                ..Style::new()
                            })
                            .fill((
                                text()
                                    .class("detail-label")
                                    .style(Style {
                                        width: Some(Length::Px(180.0)),
                                        shrink: Some(0.0),
                                        font_size: Some(14.0),
                                        line_height: Some(20.0),
                                        ..Style::new()
                                    })
                                    .content(label),
                                text()
                                    .class("detail-value")
                                    .style(Style {
                                        min_width: Some(Length::Px(0.0)),
                                        grow: Some(1.0),
                                        font_size: Some(14.0),
                                        line_height: Some(20.0),
                                        white_space: Some(WhiteSpace::PreWrap),
                                        ..Style::new()
                                    })
                                    .content(value),
                            ))
                    })
                    .collect::<Vec<_>>(),
            ),
    )
}

pub fn table(theme: sapid::Theme, headers: Vec<String>, rows: Vec<Vec<String>>) -> View {
    table_content(
        theme,
        headers,
        rows.into_iter().map(|cells| (cells, None)).collect(),
    )
}

/// Headers include the trailing actions column, keeping all row operations aligned.
pub fn table_with_actions(
    theme: sapid::Theme,
    headers: Vec<String>,
    rows: Vec<(Vec<String>, View)>,
) -> View {
    table_content(
        theme,
        headers,
        rows.into_iter()
            .map(|(cells, actions)| (cells, Some(actions)))
            .collect(),
    )
}

pub fn table_widgets(theme: sapid::Theme, headers: Vec<String>, rows: Vec<Vec<View>>) -> View {
    let mut content = Some((theme, headers, rows));
    Box::new(widget(move |scope| {
        let Some((theme, headers, rows)) = content.take() else {
            return Ok(());
        };
        let head =
            sapid::table_head().row(sapid::table_row("head").cells(
                headers.into_iter().enumerate().map(|(index, label)| {
                    sapid::table_header_cell(format!("h{index}")).text(label)
                }),
            ));
        let body = sapid::table_body().rows(rows.into_iter().enumerate().map(|(index, cells)| {
            sapid::table_row(format!("r{index}")).cells(cells.into_iter().enumerate().map(
                |(column, cell)| {
                    let mut cell = Some(cell);
                    sapid::table_cell(format!("c{column}"))
                        .content(move || cell.take().unwrap_or_else(|| Box::new(group())))
                },
            ))
        }));
        view()
            .class("admin-table-scroll")
            .style(
                Style::new()
                    .overflow_x(Overflow::Auto)
                    .min_width(Length::Px(0.0)),
            )
            .fill(
                sapid::table(format!("admin-table-{}", scope.id().slot()))
                    .theme(theme)
                    .class("admin-table")
                    .head(head)
                    .body(body),
            )
            .show_in(scope)
    }))
}

fn table_content(
    theme: sapid::Theme,
    headers: Vec<String>,
    rows: Vec<(Vec<String>, Option<View>)>,
) -> View {
    let mut content = Some((theme, headers, rows));
    Box::new(widget(move |scope| {
        let Some((theme, headers, rows)) = content.take() else {
            return Ok(());
        };
        let head = sapid::table_head().row(
            sapid::table_row("head").cells(
                headers
                    .into_iter()
                    .enumerate()
                    .map(|(i, label)| sapid::table_header_cell(format!("h{i}")).text(label)),
            ),
        );
        let body =
            sapid::table_body().rows(rows.into_iter().enumerate().map(|(i, (cells, actions))| {
                let mut row = sapid::table_row(format!("r{i}")).cells(
                    cells.into_iter().enumerate().map(|(j, value)| {
                        sapid::table_cell(format!("c{j}"))
                            .content(move || text().content(value.clone()))
                    }),
                );
                if let Some(actions) = actions {
                    let mut actions = Some(actions);
                    row = row.cell(
                        sapid::table_cell("actions")
                            .content(move || actions.take().unwrap_or_else(|| Box::new(group()))),
                    );
                }
                row
            }));
        view()
            .class("admin-table-scroll")
            .style(
                Style::new()
                    .overflow_x(Overflow::Auto)
                    .min_width(Length::Px(0.0)),
            )
            .fill(
                sapid::table(format!("admin-table-{}", scope.id().slot()))
                    .theme(theme)
                    .class("admin-table")
                    .head(head)
                    .body(body)
                    .striped(false),
            )
            .show_in(scope)
    }))
}

/// Keep mounted editors stable while giving dense tables a compact action trigger.
pub fn compact_action(theme: sapid::Theme, label: impl Into<String>, content: View) -> View {
    let label = label.into();
    let mut content = Some(content);
    Box::new(widget(move |scope| {
        let Some(content) = content.take() else {
            return Ok(());
        };
        let mut content = Some(content);
        sapid::collapsible(format!("action-{}", scope.id().slot()))
            .theme(theme.clone())
            .class("admin-compact-action")
            .label(label.clone())
            .content_policy(sapid::ContentPolicy::KeepMounted)
            .content(move || content.take().unwrap_or_else(|| Box::new(group())))
            .show_in(scope)
    }))
}

#[derive(Clone)]
pub struct Field {
    pub label: String,
    pub initial: String,
    pub multiline: bool,
}
impl Field {
    pub fn new(label: impl Into<String>, initial: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            initial: initial.into(),
            multiline: false,
        }
    }
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }
}

#[derive(Clone)]
pub struct Danger {
    pub phrase: String,
    pub required_reason: bool,
    pub show_reason: bool,
}

/// A mutation belongs to the page, while its editor may belong to a refreshed row.
#[derive(Clone)]
pub struct MutationController {
    owner: glory::ui::WeakScope,
    states: Rc<RefCell<std::collections::BTreeMap<String, MutationState>>>,
}
#[derive(Clone)]
struct MutationState {
    owner: glory::ui::WeakScope,
    busy: Cage<bool>,
    message: Cage<String>,
}
impl MutationController {
    pub fn new(page: &Scope) -> Self {
        Self {
            owner: page.downgrade(),
            states: Rc::default(),
        }
    }
    fn state(&self, key: String) -> MutationState {
        let mut states = self.states.borrow_mut();
        states
            .entry(key)
            .or_insert_with(|| {
                let owner = self.owner.upgrade().expect("mutation page is alive");
                MutationState {
                    owner: self.owner.clone(),
                    busy: owner.cage(false),
                    message: owner.cage(String::new()),
                }
            })
            .clone()
    }
}
impl Danger {
    pub fn new(phrase: impl Into<String>) -> Self {
        Self {
            phrase: phrase.into(),
            required_reason: true,
            show_reason: true,
        }
    }
    pub fn optional_reason(mut self) -> Self {
        self.required_reason = false;
        self
    }
    pub fn without_reason(mut self) -> Self {
        self.required_reason = false;
        self.show_reason = false;
        self
    }
}

pub fn action_form<F, Fut>(
    scope: &Scope,
    theme: sapid::Theme,
    label: impl Into<String>,
    fields: Vec<Field>,
    danger: Option<Danger>,
    submit: F,
    on_success: impl Fn() + 'static,
) -> View
where
    F: Fn(Vec<String>) -> Fut + 'static,
    Fut: Future<Output = std::result::Result<(), HttpError>> + 'static,
{
    action_form_controlled(
        scope,
        &MutationController::new(scope),
        "action",
        theme,
        label,
        fields,
        danger,
        submit,
        on_success,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Row and page ownership stay explicit beside the typed form callbacks"
)]
pub fn action_form_controlled<F, Fut>(
    scope: &Scope,
    controller: &MutationController,
    key: impl Into<String>,
    theme: sapid::Theme,
    label: impl Into<String>,
    fields: Vec<Field>,
    danger: Option<Danger>,
    submit: F,
    on_success: impl Fn() + 'static,
) -> View
where
    F: Fn(Vec<String>) -> Fut + 'static,
    Fut: Future<Output = std::result::Result<(), HttpError>> + 'static,
{
    let state = controller.state(key.into());
    let values: Vec<_> = fields
        .iter()
        .map(|field| scope.cage(field.initial.clone()))
        .collect();
    let phrase = scope.cage(String::new());
    let reason = scope.cage(String::new());
    let busy = state.busy;
    let message = state.message;
    let label = label.into();
    let children: Vec<View> = fields
        .into_iter()
        .zip(values.iter().copied())
        .map(|(field, value)| {
            Box::new(
                col()
                    .class("admin-field")
                    .style(Style {
                        display: Some(Display::Flex),
                        flex_direction: Some(FlexDirection::Column),
                        width: Some(Length::Px(240.0)),
                        max_width: Some(Limit::Percent(100.0)),
                        grow: Some(1.0),
                        min_width: Some(Length::Px(0.0)),
                        gap: Some(6.0),
                        ..Style::new()
                    })
                    .fill((
                        text()
                            .style(Style {
                                font_size: Some(14.0),
                                font_weight: Some(500.0),
                                ..Style::new()
                            })
                            .content(field.label.clone()),
                        sapid::text_field()
                            .theme(theme.clone())
                            .name(field.label)
                            .value(value)
                            .multiline(field.multiline)
                            .on_value_change(move |new| value.revise(|old| *old = new)),
                    )),
            ) as View
        })
        .collect();
    let field_grid = row()
        .class("admin-form-fields")
        .style(Style {
            display: Some(Display::Flex),
            flex_direction: Some(FlexDirection::Row),
            flex_wrap: Some(glory::style::FlexWrap::Wrap),
            gap: Some(16.0),
            min_width: Some(Length::Px(0.0)),
            ..Style::new()
        })
        .fill(children);
    let mut children: Vec<View> = vec![Box::new(field_grid)];
    if let Some(danger) = danger.as_ref() {
        children.push(Box::new(text().content(format!(
            "{}{}{}",
            t("dangerous_action.type_prefix"),
            danger.phrase,
            t("dangerous_action.type_suffix")
        ))));
        children.push(Box::new(
            sapid::text_field()
                .theme(theme.clone())
                .name(t("dangerous_action.confirmation"))
                .value(phrase)
                .on_value_change(move |new| phrase.revise(|old| *old = new)),
        ));
        if danger.show_reason {
            let required = danger.required_reason;
            children.push(Box::new(
                sapid::text_field()
                    .theme(theme.clone())
                    .name(t("dangerous_action.reason_label"))
                    .value(reason)
                    .on_value_change(move |new| reason.revise(|old| *old = new)),
            ));
            children.push(Box::new(text().content(t("dangerous_action.reason_help"))));
            children.push(Box::new(text().class("reason-error").content(reason.map(
                move |value| {
                    destructive_reason_error(value, required)
                        .map(t)
                        .unwrap_or_default()
                },
            ))));
        }
    }
    let valid_danger = danger.clone();
    let enabled = scope.bond(move || {
        !busy.get()
            && valid_danger.as_ref().is_none_or(|gate| {
                phrase.get().trim() == gate.phrase.trim()
                    && (!gate.show_reason
                        || destructive_reason_error(&reason.get(), gate.required_reason).is_none())
            })
    });
    let on_success = Rc::new(on_success);
    let submit = Rc::new(submit);
    let weak = state.owner;
    let row_owner = scope.downgrade();
    let variant = if danger.is_some() {
        sapid::ButtonVariant::Danger
    } else {
        sapid::ButtonVariant::Primary
    };
    let submit_button = sapid::button()
        .theme(theme)
        .label(label)
        .variant(variant)
        .loading(busy)
        .enabled(enabled)
        .on_press(move || {
            if busy.get_untracked() {
                return;
            }
            if let Some(gate) = danger.as_ref() {
                if phrase.get_untracked().trim() != gate.phrase.trim() {
                    return;
                }
                if gate.show_reason
                    && let Some(key) =
                        destructive_reason_error(&reason.get_untracked(), gate.required_reason)
                {
                    message.revise(|value| *value = t(key));
                    return;
                }
            }
            let mut snapshot: Vec<_> = values.iter().map(|value| value.get_untracked()).collect();
            if danger.as_ref().is_some_and(|gate| gate.show_reason) {
                snapshot.push(reason.get_untracked().trim().to_string());
            }
            let Some(scope) = weak.upgrade() else {
                return;
            };
            busy.revise(|value| *value = true);
            message.revise(|value| value.clear());
            let future = submit(snapshot);
            let success = on_success.clone();
            let weak = weak.clone();
            let row_owner = row_owner.clone();
            let result = scope.owner().spawn(async move {
                let result = future.await;
                if weak.upgrade().is_none() {
                    return;
                }
                busy.revise(|value| *value = false);
                match result {
                    Ok(()) => {
                        if row_owner.upgrade().is_some() {
                            phrase.revise(|value| value.clear());
                            reason.revise(|value| value.clear());
                        }
                        message.revise(|value| *value = t("common.completed"));
                        success();
                    }
                    Err(error) => message.revise(|value| *value = error_text(&error)),
                }
            });
            if let Err(error) = result {
                busy.revise(|value| *value = false);
                message.revise(|value| *value = error.to_string());
            }
        });
    children.push(Box::new(
        row()
            .class("admin-form-actions")
            .style(Style {
                display: Some(Display::Flex),
                flex_direction: Some(FlexDirection::Row),
                gap: Some(8.0),
                align: Some(Align::Center),
                justify: Some(Justify::End),
                ..Style::new()
            })
            .fill(submit_button),
    ));
    children.push(Box::new(text().class("action-message").content(message)));
    Box::new(
        col()
            .class("admin-action-form")
            .style(
                Style::new()
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Column)
                    .gap(16.0)
                    .min_width(Length::Px(0.0)),
            )
            .fill(children),
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::task::{Poll, Waker};

    use glory::core::executor::Executor;
    use glory::render::memory::Memory;
    use glory::render::{Event, NodeId, Props};

    use super::*;

    #[test]
    fn refreshed_region_keeps_its_document_position_between_static_siblings() {
        let app = App::memory();
        let value = app.scope().cage("initial".to_string());
        app.mount(col().fill((
            text().content("Before"),
            reactive(value, |_, value| Box::new(text().content(value))),
            text().content("After"),
        )))
        .unwrap();
        let region = find(&app, |node| {
            node.classes
                .iter()
                .any(|class| class == "admin-reactive-region")
        });
        let after = find(
            &app,
            |node| matches!(node.props, Props::Text(text) if text.content == "After"),
        );
        for content in ["initial", "refreshed", "reloaded"] {
            value.revise(|value| *value = content.into());
            app.flush().unwrap();
            let tree = app.tree();
            let root = tree.node(tree.roots()[0]).unwrap();
            assert_eq!(root.children[1], region);
            assert_eq!(root.children[2], after);
            let mut stack = tree.roots().iter().copied().rev().collect::<Vec<_>>();
            let mut reading_order = Vec::new();
            while let Some(id) = stack.pop() {
                let node = tree.node(id).unwrap();
                if let Props::Text(text) = node.descriptor.view().props {
                    reading_order.push(text.content.clone());
                }
                stack.extend(node.children.iter().copied().rev());
            }
            assert_eq!(reading_order, ["Before", content, "After"]);
            assert_eq!(tree.node(region).unwrap().children.len(), 1);
        }
    }

    #[derive(Default)]
    struct Gate {
        value: Option<u64>,
        waker: Option<Waker>,
    }
    fn release(gate: &Rc<RefCell<Gate>>, value: u64) {
        let wake = {
            let mut gate = gate.borrow_mut();
            gate.value = Some(value);
            gate.waker.take()
        };
        if let Some(wake) = wake {
            wake.wake();
        }
    }

    #[test]
    fn resource_reload_cancels_old_request_and_has_no_self_subscription() {
        let host = Rc::new(Executor::new(32));
        let app = App::with_host(host.clone(), Memory::new);
        let requests = Rc::new(RefCell::new(Vec::new()));
        let seen = requests.clone();
        let source = resource(app.scope(), move || {
            let gate = Rc::new(RefCell::new(Gate::default()));
            seen.borrow_mut().push(gate.clone());
            std::future::poll_fn(move |context| {
                let mut gate = gate.borrow_mut();
                if let Some(value) = gate.value.take() {
                    Poll::Ready(Ok(value))
                } else {
                    gate.waker = Some(context.waker().clone());
                    Poll::Pending
                }
            })
        });
        app.flush().unwrap();
        assert!(host.poll_one());
        assert_eq!(requests.borrow().len(), 1);
        source.reload();
        app.flush().unwrap();
        assert_eq!(requests.borrow().len(), 2);
        assert_eq!(host.len(), 1);
        assert!(host.poll_one());
        release(&requests.borrow()[0], 10);
        release(&requests.borrow()[1], 20);
        while host.poll_one() {}
        app.flush().unwrap();
        assert_eq!(source.data.get_untracked().unwrap().unwrap(), 20);
        app.flush().unwrap();
        assert_eq!(requests.borrow().len(), 2);
        assert!(host.is_empty());
    }

    #[test]
    fn resource_tasks_retire_with_their_page_owner() {
        let host = Rc::new(Executor::new(32));
        let app = App::with_host(host.clone(), Memory::new);
        let page = app.scope().child().unwrap();
        let source = resource(&page, || {
            std::future::pending::<std::result::Result<u64, HttpError>>()
        });
        app.flush().unwrap();
        assert!(host.poll_one());
        assert_eq!(host.len(), 1);
        page.dispose();
        app.flush().unwrap();
        assert!(host.is_empty());
        assert!(!source.data.is_alive());
    }

    fn find(app: &App, predicate: impl Fn(&glory::render::DescriptorView<'_>) -> bool) -> NodeId {
        let tree = app.tree();
        let mut queue = tree.roots().to_vec();
        while let Some(id) = queue.pop() {
            let node = tree.node(id).unwrap();
            if predicate(&node.descriptor.view()) {
                return id;
            }
            queue.extend(node.children.iter().copied());
        }
        panic!("test control missing")
    }
    fn edit(app: &App, name: &str, value: &str) {
        let id = find(app, |node| node.name.as_deref() == Some(name));
        let revision = match app.tree().node(id).unwrap().descriptor.view().props {
            Props::Input(input) => input.revision,
            _ => panic!("input expected"),
        };
        app.dispatch(
            id,
            Event::ValueChanged {
                value: value.into(),
                composing: false,
                revision,
            },
        )
        .unwrap();
    }

    #[test]
    fn destructive_action_requires_phrase_and_reason_and_retains_them_after_failure() {
        let host = Rc::new(Executor::new(32));
        let app = App::with_host(host.clone(), Memory::new);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let submitted = calls.clone();
        let (gate, wake) = (
            Rc::new(Cell::new(false)),
            Rc::new(RefCell::new(None::<Waker>)),
        );
        let future_gate = gate.clone();
        let future_wake = wake.clone();
        app.mount(action_form(
            app.scope(),
            sapid::Theme::default(),
            "Delete",
            vec![Field::new("Subject", "alice")],
            Some(Danger::new("alice")),
            move |values| {
                submitted.borrow_mut().push(values);
                let gate = future_gate.clone();
                let wake = future_wake.clone();
                std::future::poll_fn(move |context| {
                    if gate.get() {
                        Poll::Ready(Err(HttpError::message("Please retry")))
                    } else {
                        *wake.borrow_mut() = Some(context.waker().clone());
                        Poll::Pending
                    }
                })
            },
            || panic!("failed mutation cannot announce success"),
        ))
        .unwrap();
        let button = find(
            &app,
            |node| matches!(node.props, Props::Button(button) if button.label == "Delete"),
        );
        app.dispatch(button, Event::Activate).unwrap();
        assert!(calls.borrow().is_empty());
        edit(&app, "Confirmation phrase", "alice");
        edit(&app, "Reason / ticket", "password=secret");
        app.dispatch(button, Event::Activate).unwrap();
        assert!(calls.borrow().is_empty());
        edit(&app, "Reason / ticket", "SEC-1234 recovery");
        app.dispatch(button, Event::Activate).unwrap();
        app.dispatch(button, Event::Activate).unwrap();
        assert_eq!(
            *calls.borrow(),
            vec![vec!["alice".to_string(), "SEC-1234 recovery".to_string()]]
        );
        assert!(host.poll_one());
        gate.set(true);
        wake.borrow_mut().take().unwrap().wake();
        while host.poll_one() {}
        app.flush().unwrap();
        let phrase = find(&app, |node| {
            node.name.as_deref() == Some("Confirmation phrase")
        });
        assert!(
            matches!(app.tree().node(phrase).unwrap().descriptor.view().props, Props::Input(input) if input.value == "alice")
        );
        let reason = find(&app, |node| node.name.as_deref() == Some("Reason / ticket"));
        assert!(
            matches!(app.tree().node(reason).unwrap().descriptor.view().props, Props::Input(input) if input.value == "SEC-1234 recovery")
        );
    }

    #[test]
    fn page_owned_mutation_survives_row_refresh_and_keeps_duplicate_guard() {
        let host = Rc::new(Executor::new(32));
        let app = App::with_host(host.clone(), Memory::new);
        let controller = MutationController::new(app.scope());
        let reveal = app.scope().cage(String::new());
        let successes = app.scope().cage(0usize);
        let requests = Rc::new(Cell::new(0usize));
        let gate = Rc::new(RefCell::new(Gate::default()));
        let mount = || {
            let controller = controller.clone();
            let requests = requests.clone();
            let gate = gate.clone();
            app.scope()
                .mount_with(|| {
                    widget(move |scope| {
                        let requests = requests.clone();
                        let gate = gate.clone();
                        action_form_controlled(
                            scope,
                            &controller,
                            "session-1",
                            sapid::Theme::default(),
                            "Regenerate",
                            vec![],
                            None,
                            move |_| {
                                requests.set(requests.get() + 1);
                                let gate = gate.clone();
                                async move {
                                    let value = std::future::poll_fn(move |context| {
                                        let mut gate = gate.borrow_mut();
                                        if let Some(value) = gate.value.take() {
                                            Poll::Ready(value)
                                        } else {
                                            gate.waker = Some(context.waker().clone());
                                            Poll::Pending
                                        }
                                    })
                                    .await;
                                    reveal.revise(|token| *token = format!("one-shot-{value}"));
                                    Ok(())
                                }
                            },
                            move || successes.revise(|count| *count += 1),
                        )
                        .build(scope)
                    })
                })
                .unwrap()
        };
        let first = mount();
        app.flush().unwrap();
        let button = find(
            &app,
            |node| matches!(node.props, Props::Button(button) if button.label == "Regenerate"),
        );
        app.dispatch(button, Event::Activate).unwrap();
        assert!(host.poll_one());
        drop(first);
        app.flush().unwrap();
        assert_eq!(host.len(), 1);
        let _second = mount();
        app.flush().unwrap();
        let button = find(
            &app,
            |node| matches!(node.props, Props::Button(button) if button.label == "Regenerate"),
        );
        assert!(
            matches!(app.tree().node(button).unwrap().descriptor.view().props, Props::Button(button) if !button.enabled)
        );
        app.dispatch(button, Event::Activate).unwrap();
        assert_eq!(requests.get(), 1);
        release(&gate, 42);
        while host.poll_one() {}
        app.flush().unwrap();
        assert_eq!(reveal.get_untracked(), "one-shot-42");
        assert_eq!(successes.get_untracked(), 1);
        assert!(
            matches!(app.tree().node(button).unwrap().descriptor.view().props, Props::Button(button) if button.enabled)
        );
        app.dispatch(button, Event::Activate).unwrap();
        assert!(host.poll_one());
        assert_eq!(requests.get(), 2);
        assert_eq!(host.len(), 1);
        app.scope().dispose();
        assert!(host.is_empty());
        assert!(!reveal.is_alive());
    }

    #[test]
    fn table_keeps_large_audit_details_without_flat_cell_budget_rejection() {
        let app = App::memory();
        let full = "audit-detail-".repeat(1024);
        app.mount(table(
            sapid::Theme::default(),
            vec!["Details".into()],
            vec![vec![full.clone()]],
        ))
        .unwrap();
        let text = find(
            &app,
            |node| matches!(node.props, Props::Text(text) if text.content == full),
        );
        assert!(app.tree().node(text).is_ok());
    }
}
