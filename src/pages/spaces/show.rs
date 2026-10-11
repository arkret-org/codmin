//! Space hierarchy admin view
//!
//! Per-Space tree showing the immediate parent (at most one) and the
//! direct children. Backed by the `GET /_coland/admin/spaces` snapshot.
//!
//! The visual is intentionally lightweight: a single column with an
//! "ancestor" row, the centered Space row, and an indented list of
//! children. Each neighbouring node links into its own hierarchy view
//! so the operator can walk up/down the tree click-by-click without
//! a server-side recursion query.

use dioxus::prelude::*;

use crate::api::spaces;
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::empty_state::EmptyState;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::router::Route;
use crate::types::spaces::SpaceHierarchyNode;
use crate::utils::i18n::t;

#[component]
pub fn SpaceShow(space_id: String) -> Element {
    let space_id_for_fetch = space_id.clone();
    let mut data = use_resource(move || {
        let id = space_id_for_fetch.clone();
        async move { spaces::get_space(&id).await }
    });
    let header_space_id = space_id.clone();

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: format!("{} · {}", t("spaces.hierarchy_title"), header_space_id),
                description: t("spaces.hierarchy_subtitle"),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| data.restart(),
                    {t("common.refresh")}
                }
            }

            match &*data.read() {
                Some(Ok(tree)) => {
                    let parent_count = if tree.parent.is_some() { 1 } else { 0 };
                    let child_count = tree.children.len();
                    let depth_label = depth_summary_label(parent_count, child_count);
                    let center_name = tree.name.clone();
                    let center_id = tree.space_id.clone();
                    rsx! {
                        if parent_count == 0 && child_count == 0 {
                            EmptyState {
                                icon_name: "git-branch".to_string(),
                                title: t("spaces.hierarchy_empty_title"),
                                description: t("spaces.hierarchy_empty_subtitle"),
                            }
                        } else {
                            p { class: "text-xs text-muted-foreground", "{depth_label}" }
                            div { class: "rounded-md border p-4 space-y-2",
                                if let Some(parent) = tree.parent.as_ref() {
                                    {
                                        let label = t("spaces.parent_label");
                                        rsx! { HierarchyRow { node: parent.clone(), kind_label: label, indent: 0 } }
                                    }
                                }
                                div { class: "flex items-center gap-2 pl-4",
                                    span { class: "text-xs font-semibold text-foreground", {t("spaces.this_label")} }
                                    span { class: "font-mono text-xs", "{center_id}" }
                                    span { class: "text-sm text-muted-foreground", "·" }
                                    span { class: "text-sm", "{center_name}" }
                                }
                                if !tree.children.is_empty() {
                                    div { class: "pl-8 border-l-2 border-muted ml-4 space-y-1",
                                        p { class: "text-xs uppercase tracking-wide text-muted-foreground",
                                            {format!("{} ({})", t("spaces.children_label"), child_count)}
                                        }
                                        for child in tree.children.iter() {
                                            {
                                                let child_label = t("spaces.child_label");
                                                rsx! { HierarchyRow { node: child.clone(), kind_label: child_label, indent: 2 } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        on_retry: move |_| data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}

#[component]
fn HierarchyRow(node: SpaceHierarchyNode, kind_label: String, indent: u32) -> Element {
    let pad_class = if indent == 0 { "pl-0" } else { "pl-4" };
    let id = node.space_id.clone();
    let id_for_link = id.clone();
    let name = node.name.clone();
    let members = node.member_count;
    rsx! {
        div { class: "flex items-center gap-2 {pad_class}",
            span { class: "text-xs font-semibold text-muted-foreground", "{kind_label}" }
            span { class: "font-mono text-xs", "{id}" }
            span { class: "text-sm text-muted-foreground", "·" }
            span { class: "text-sm", "{name}" }
            span { class: "text-xs text-muted-foreground", "({members} members)" }
            Link {
                to: Route::SpaceShow { space_id: id_for_link },
                class: "ml-auto text-xs text-primary hover:underline".to_string(),
                "open"
            }
        }
    }
}

/// Build the human summary "1 parent · 4 children" label for the
/// hierarchy header. Pure helper so the cardinality logic is
/// unit-testable.
pub(crate) fn depth_summary_label(parent_count: usize, child_count: usize) -> String {
    let parent_phrase = match parent_count {
        0 => "no parent".to_string(),
        1 => "1 parent".to_string(),
        n => format!("{n} parents"),
    };
    let child_phrase = match child_count {
        0 => "no children".to_string(),
        1 => "1 child".to_string(),
        n => format!("{n} children"),
    };
    format!("{parent_phrase} · {child_phrase}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_summary_handles_singular_and_plural() {
        assert_eq!(depth_summary_label(0, 0), "no parent · no children");
        assert_eq!(depth_summary_label(1, 0), "1 parent · no children");
        assert_eq!(depth_summary_label(1, 1), "1 parent · 1 child");
        assert_eq!(depth_summary_label(1, 5), "1 parent · 5 children");
    }
}
