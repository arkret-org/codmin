use glory::prelude::*;

use crate::components::ui::{self, View};
use crate::pages::{presentation as p, station};
use crate::types::spaces::SpaceHierarchyNode;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, space_id: String) -> impl Widget + use<> {
    let id = space_id.clone();
    let data = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::spaces::get_space(&id).await }
    });
    ui::resource_view(scope, theme.clone(), data, move |_, tree| {
        let mut children: Vec<View> = vec![
            ui::heading(format!("{} · {}", t("spaces.hierarchy_title"), space_id)),
            p::description("spaces.hierarchy_subtitle"),
        ];
        if tree.parent.is_none() && tree.children.is_empty() {
            children.push(Box::new(
                sapid::empty_state()
                    .theme(theme.clone())
                    .title(t("spaces.hierarchy_empty_title"))
                    .description(t("spaces.hierarchy_empty_subtitle")),
            ));
        } else {
            children.push(p::plain(depth_summary_label(
                usize::from(tree.parent.is_some()),
                tree.children.len(),
            )));
            let mut nodes: Vec<View> = Vec::new();
            if let Some(parent) = tree.parent {
                nodes.push(hierarchy_row(
                    theme.clone(),
                    &parent,
                    "spaces.parent_label",
                    "hierarchy-parent",
                ));
            }
            nodes.push(Box::new(row().class("hierarchy-center").fill((
                p::plain(t("spaces.this_label")),
                p::mono(tree.space_id),
                p::plain("·"),
                p::plain(tree.name),
            ))));
            let mut sub: Vec<View> = vec![p::plain(format!(
                "{} ({})",
                t("spaces.children_label"),
                tree.children.len()
            ))];
            for child in tree.children {
                sub.push(hierarchy_row(
                    theme.clone(),
                    &child,
                    "spaces.child_label",
                    "hierarchy-child",
                ));
            }
            nodes.push(Box::new(col().class("hierarchy-children").fill(sub)));
            children.push(Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .class("hierarchy-tree")
                    .fill(nodes),
            ));
        }
        p::stack(children)
    })
}
fn hierarchy_row(theme: sapid::Theme, node: &SpaceHierarchyNode, label: &str, class: &str) -> View {
    Box::new(row().class(class.to_owned()).fill((
        p::plain(t(label)),
        p::mono(&node.space_id),
        p::plain("·"),
        p::plain(&node.name),
        p::plain(format!("({} members)", node.member_count)),
        p::link(theme, "open", station::path("/spaces", &node.space_id)),
    )))
}
pub(crate) fn depth_summary_label(parent: usize, children: usize) -> String {
    let parent = match parent {
        0 => "no parent".into(),
        1 => "1 parent".into(),
        n => format!("{n} parents"),
    };
    let children = match children {
        0 => "no children".into(),
        1 => "1 child".into(),
        n => format!("{n} children"),
    };
    format!("{parent} · {children}")
}
