//! The Modal family: the rendering implementation has been migrated to yoface
//! (`yoface::ui::modal`). The local hand-written `fixed inset-0 z-50 …`
//! overlay + panel implementation has been deleted.
//!
//! To avoid renaming props one by one across ~20 call sites (among them
//! `destructive: bool` → `variant` is a type change, and some are dynamic
//! expressions, which makes blind edits error-prone), a **thin adapter** layer
//! is kept here that passes through / translates codmin's existing prop names
//! to yoface's new API:
//!   * `DialogActions.confirm_text`/`cancel_text` → yoface `confirm_label`/`cancel_label`
//!   * `DialogActions.destructive: bool` → yoface `variant: ButtonVariant::{Destructive,Primary}`
//!   * `Modal.max_width: String` → yoface `Modal.class`
//!
//! `ModalOverlay`'s signature matches, so it is re-exported directly.
use dioxus::prelude::*;
use yoface::ui::button::ButtonVariant;
pub use yoface::ui::modal::ModalOverlay;

/// Adapter: the local `max_width: String` (e.g. `"max-w-lg"`) maps to yoface's
/// `Modal.class` (passed through to the container, overriding the default
/// max-width). The remaining props pass through under the same names.
#[component]
pub fn Modal(
    open: bool,
    #[props(default)] title: String,
    #[props(default = "max-w-md".to_string())] max_width: String,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        yoface::ui::modal::Modal {
            open,
            title,
            class: max_width,
            on_close: move |_| on_close.call(()),
            {children}
        }
    }
}

/// Adapter: `confirm_text`/`cancel_text`/`destructive` → yoface's new prop
/// names.
#[component]
pub fn DialogActions(
    confirm_text: String,
    cancel_text: String,
    #[props(default = false)] destructive: bool,
    #[props(default = false)] confirm_loading: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let variant = if destructive {
        ButtonVariant::Destructive
    } else {
        ButtonVariant::Primary
    };
    rsx! {
        yoface::ui::modal::DialogActions {
            confirm_label: confirm_text,
            cancel_label: cancel_text,
            variant,
            confirm_loading,
            on_confirm: move |_| on_confirm.call(()),
            on_cancel: move |_| on_cancel.call(()),
        }
    }
}
