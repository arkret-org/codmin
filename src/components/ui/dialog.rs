//! `ConfirmDialog` rendering is delegated to yoface.
//!
//! This adapter preserves codmin's existing prop names while translating them
//! to the yoface modal API:
//! - `description` -> yoface `message`
//! - `confirm_text`/`cancel_text` -> yoface `confirm_label`/`cancel_label`
//! - `destructive: bool` -> yoface `variant: ButtonVariant::{Destructive,Primary}`

use dioxus::prelude::*;
use yoface::ui::button::ButtonVariant;

#[component]
pub fn ConfirmDialog(
    open: bool,
    title: String,
    description: String,
    #[props(default = "Confirm".to_string())] confirm_text: String,
    #[props(default = "Cancel".to_string())] cancel_text: String,
    #[props(default = false)] destructive: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let variant = if destructive {
        ButtonVariant::Destructive
    } else {
        ButtonVariant::Primary
    };
    rsx! {
        yoface::ui::modal::ConfirmDialog {
            open,
            title,
            message: description,
            confirm_label: confirm_text,
            cancel_label: cancel_text,
            variant,
            on_confirm: move |_| on_confirm.call(()),
            on_cancel: move |_| on_cancel.call(()),
        }
    }
}
