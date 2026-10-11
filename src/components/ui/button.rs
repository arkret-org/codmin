//! `Button` — migrated to yoface (`yoface::ui::button::Button`).
//!
//! This thin **adapter** is kept locally: codmin's call sites use
//! `ButtonVariant::Default` (yoface renamed it to `Primary`), and they are
//! used to passing `class` / `disabled` / `type` / `onclick` as named props
//! (not `Option`). The adapter maps the local enum onto yoface's and passes
//! those named props through as yoface attributes, with rendering going
//! through yoface's css_module + the Soft Orbit tokens. As a result, the ~140
//! call sites need zero changes.
use dioxus::prelude::*;
use yoface::ui::button::{Button as YButton, ButtonSize as YSize, ButtonVariant as YVariant};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
}

impl ButtonVariant {
    fn to_yoface(self) -> YVariant {
        match self {
            ButtonVariant::Default => YVariant::Primary,
            ButtonVariant::Destructive => YVariant::Destructive,
            ButtonVariant::Outline => YVariant::Outline,
            ButtonVariant::Secondary => YVariant::Secondary,
            ButtonVariant::Ghost => YVariant::Ghost,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonSize {
    #[default]
    Default,
    Sm,
}

impl ButtonSize {
    fn to_yoface(self) -> YSize {
        match self {
            ButtonSize::Default => YSize::Default,
            ButtonSize::Sm => YSize::Sm,
        }
    }
}

#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default)] class: String,
    #[props(default)] disabled: bool,
    #[props(default = "button".to_string())] r#type: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        YButton {
            variant: variant.to_yoface(),
            size: size.to_yoface(),
            class,
            disabled,
            r#type,
            onclick: move |evt| onclick.call(evt),
            {children}
        }
    }
}
