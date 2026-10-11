//! `Badge` — migrated to yoface (`yoface::ui::badge::Badge`).
//!
//! This thin **adapter** is kept locally: codmin's call sites use
//! `BadgeVariant::Default` (yoface renamed it `Primary`) and
//! `BadgeVariant::Success` (yoface has no equivalent semantic). The adapter
//! hands the 4 mappable variants over to yoface (rendered with css_module +
//! the Soft Orbit tokens); `Success`, having no such semantic in yoface, keeps
//! the local green-styled span implementation. The `class` named prop is
//! passed through as yoface attributes. The ~170 call sites need zero changes.
use dioxus::prelude::*;
use yoface::ui::badge::{Badge as YBadge, BadgeVariant as YVariant};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
    Success,
}

#[component]
pub fn Badge(
    #[props(default)] variant: BadgeVariant,
    #[props(default)] class: String,
    children: Element,
) -> Element {
    let yv = match variant {
        BadgeVariant::Default => Some(YVariant::Primary),
        BadgeVariant::Secondary => Some(YVariant::Secondary),
        BadgeVariant::Destructive => Some(YVariant::Destructive),
        BadgeVariant::Outline => Some(YVariant::Outline),
        // yoface has no success semantic; keep the local green span.
        BadgeVariant::Success => None,
    };

    match yv {
        Some(yv) => rsx! {
            YBadge { variant: yv, class, {children} }
        },
        None => rsx! {
            span {
                class: "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold bg-green-500/10 text-green-600 dark:text-green-400 {class}",
                {children}
            }
        },
    }
}
