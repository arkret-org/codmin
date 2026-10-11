use dioxus::prelude::*;
// The label and search-input components live in yoface; codmin keeps no
// local adapter for either.
//   * `yoface::ui::label::LabelFor` takes `r#for` + `class` + children.
//   * `yoface::ui::input::SearchInput` names its label prop `aria_label`.
pub use yoface::ui::input::SearchInput;
pub use yoface::ui::label::LabelFor;

/// `Input` — migrated to yoface (`yoface::ui::input::Input`, rendered with
/// css_module).
///
/// This thin **adapter** is kept locally: codmin's call sites pass `value` /
/// `placeholder` / `disabled` / `oninput` (not `Option`) / and a set of HTML
/// constraints (`min_length` / `max_length` / `pattern` / `aria_describedby`
/// and so on) as named props. The adapter passes them through as attributes of
/// yoface's `Input` (`value` / `placeholder` and the like are all global input
/// attributes), and wraps `oninput` into yoface's `Option<EventHandler>`. An
/// unknown number of call sites, all needing zero changes.
#[component]
pub fn Input(
    #[props(default)] id: String,
    #[props(default)] name: String,
    #[props(default = "text".to_string())] r#type: String,
    #[props(default)] class: String,
    #[props(default)] placeholder: String,
    #[props(default)] value: String,
    #[props(default)] aria_label: String,
    #[props(default)] disabled: bool,
    #[props(default)] required: bool,
    /// HTML `minlength` constraint. `None` omits the attribute so
    /// callers that don't care preserve the prior wire shape.
    #[props(default)]
    min_length: Option<u32>,
    /// HTML `maxlength` constraint.
    #[props(default)]
    max_length: Option<u32>,
    /// HTML `pattern` regex for client-side validation. The pattern
    /// is anchored by the browser so callers should not add `^`/`$`.
    #[props(default)]
    pattern: Option<String>,
    /// Optional ARIA description (e.g. inline error message id). When
    /// `pattern` validation fails the browser surfaces this via the
    /// standard `:invalid` pseudoclass; the description hint lets
    /// screen-reader users locate the message.
    #[props(default)]
    aria_describedby: String,
    #[props(default)] oninput: EventHandler<FormEvent>,
) -> Element {
    let resolved_name = if name.is_empty() { id.clone() } else { name };
    rsx! {
        yoface::ui::input::Input {
            id,
            name: resolved_name,
            r#type,
            class: "{class}",
            placeholder,
            value,
            aria_label,
            aria_describedby,
            disabled,
            required,
            minlength: min_length.map(|n| n.to_string()),
            maxlength: max_length.map(|n| n.to_string()),
            pattern: pattern.clone(),
            oninput: move |evt| oninput.call(evt),
        }
    }
}
