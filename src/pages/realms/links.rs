//! Realm link-graph admin page.
//!
//! Renders the outbound (`this Realm → others`) and inbound
//! (`others → this Realm`) `ak.realm.link` rows for a single Realm.
//! Each row is a card with a chip for the `link_kind`
//! (`governed_by`, `discoverable_from`, `mirror_of`, …) and the
//! target Realm identifier.
use dioxus::prelude::*;

use crate::api::realm_links;
use crate::components::selection_required::{is_placeholder_resource_id, selection_required_state};
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::*;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::types::{RealmLinkDirection, RealmLinkKind, RealmLinkList, RealmLinkView};
use crate::utils::fmt::date::format_iso_datetime;
use crate::utils::i18n::t;

/// Map a free-form `link_kind` to the badge variant used to colour the
/// chip. Unknown kinds fall back to the neutral `Secondary` variant so
/// the surface keeps rendering when coland introduces a new link
/// shape.
fn link_kind_variant(link_kind: &RealmLinkKind) -> BadgeVariant {
    match link_kind.as_str() {
        "governed_by" => BadgeVariant::Default,
        "discoverable_from" => BadgeVariant::Secondary,
        "mirror_of" => BadgeVariant::Outline,
        _ => BadgeVariant::Secondary,
    }
}

/// Look up the i18n label for a `link_kind`, falling back to the raw
/// wire string so unknown kinds remain debuggable.
fn link_kind_label(link_kind: &RealmLinkKind) -> String {
    let wire = link_kind.as_str();
    let key = format!("realm_links.kind.{}", wire);
    let translated = t(&key);
    if translated == key {
        wire.to_string()
    } else {
        translated
    }
}

#[component]
pub fn LinksPage(realm_id: String) -> Element {
    if is_placeholder_resource_id(&realm_id) {
        return selection_required_state("Realm");
    }

    let id_outbound = realm_id.clone();
    let id_inbound = realm_id.clone();

    let mut outbound = use_resource(move || {
        let id = id_outbound.clone();
        async move { realm_links::list_realm_links(&id, RealmLinkDirection::Outbound).await }
    });
    let mut inbound = use_resource(move || {
        let id = id_inbound.clone();
        async move { realm_links::list_realm_links(&id, RealmLinkDirection::Inbound).await }
    });

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("realm_links.title"),
                description: format!("{}: {}", t("realm_links.realm"), realm_id),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        outbound.restart();
                        inbound.restart();
                    },
                    {t("common.refresh")}
                }
            }

            // Outbound — this Realm points at others.
            Card {
                CardHeader {
                    CardTitle { {t("realm_links.outbound_title")} }
                    CardDescription { {t("realm_links.outbound_subtitle")} }
                }
                CardContent {
                    {render_link_list(&outbound.read(), move |_| outbound.restart(), true)}
                }
            }

            // Inbound — others point at this Realm.
            Card {
                CardHeader {
                    CardTitle { {t("realm_links.inbound_title")} }
                    CardDescription { {t("realm_links.inbound_subtitle")} }
                }
                CardContent {
                    {render_link_list(&inbound.read(), move |_| inbound.restart(), false)}
                }
            }

            Card {
                CardHeader {
                    CardTitle { {t("realm_links.graph_title")} }
                    CardDescription { {t("realm_links.graph_subtitle")} }
                }
                CardContent {
                    {render_link_graph(&realm_id, &outbound.read(), &inbound.read())}
                }
            }
        }
    }
}

fn render_link_list<F>(
    data: &Option<Result<RealmLinkList, crate::utils::net::error::HttpError>>,
    retry: F,
    outbound: bool,
) -> Element
where
    F: FnMut(MouseEvent) + 'static + Copy,
{
    match data {
        Some(Ok(resp)) => {
            if resp.links.is_empty() {
                let empty_key = if outbound {
                    "realm_links.outbound_empty"
                } else {
                    "realm_links.inbound_empty"
                };
                rsx! {
                    p { class: "text-sm text-muted-foreground py-4 text-center",
                        {t(empty_key)}
                    }
                }
            } else {
                rsx! {
                    div { class: "space-y-2",
                        for row in resp.links.iter() {
                            {render_link_row(row, outbound)}
                        }
                    }
                }
            }
        }
        Some(Err(e)) => rsx! {
            ErrorBanner { message: e.message.clone(), on_retry: retry }
        },
        None => rsx! { PageSkeleton {} },
    }
}

fn render_link_row(row: &RealmLinkView, outbound: bool) -> Element {
    let other_realm = if outbound {
        row.target_realm_id.to_string()
    } else {
        row.realm_id.to_string()
    };
    let label = link_kind_label(&row.link_kind);
    let variant = link_kind_variant(&row.link_kind);
    let updated = format_iso_datetime(&row.updated_at.to_rfc3339());
    let display_name = row.label.clone();
    let status = row.status.as_str().to_string();

    rsx! {
        div {
            key: "{other_realm}-{label}",
            class: "flex flex-wrap items-center gap-2 rounded-md border border-border bg-card px-3 py-2 text-sm",
            Badge {
                variant: variant,
                class: "text-xs".to_string(),
                "{label}"
            }
            span { class: "font-mono text-xs break-all", "{other_realm}" }
            if let Some(name) = display_name.as_ref() {
                span { class: "text-xs text-muted-foreground", "({name})" }
            }
            span { class: "text-xs text-muted-foreground", "{status}" }
            span { class: "ml-auto text-xs text-muted-foreground", "{updated}" }
        }
    }
}

fn render_link_graph(
    realm_id: &str,
    outbound: &Option<Result<RealmLinkList, crate::utils::net::error::HttpError>>,
    inbound: &Option<Result<RealmLinkList, crate::utils::net::error::HttpError>>,
) -> Element {
    let outbound_rows = match outbound {
        Some(Ok(resp)) => resp.links.clone(),
        _ => Vec::new(),
    };
    let inbound_rows = match inbound {
        Some(Ok(resp)) => resp.links.clone(),
        _ => Vec::new(),
    };

    if outbound_rows.is_empty() && inbound_rows.is_empty() {
        return rsx! {
            p { class: "text-sm text-muted-foreground py-6 text-center",
                {t("realm_links.graph_empty")}
            }
        };
    }

    let outbound_count = outbound_rows.len().max(1);
    let inbound_count = inbound_rows.len().max(1);

    rsx! {
        div {
            class: "overflow-x-auto rounded-md border border-border bg-muted/20 p-3",
            role: "img",
            aria_label: format!("{} {}", t("realm_links.graph_title"), realm_id),
            svg {
                class: "min-w-[720px] w-full h-[360px]",
                view_box: "0 0 720 360",
                xmlns: "http://www.w3.org/2000/svg",
                defs {
                    marker {
                        id: "realm-link-arrow",
                        marker_width: "10",
                        marker_height: "10",
                        ref_x: "8",
                        ref_y: "3",
                        orient: "auto",
                        marker_units: "strokeWidth",
                        path { d: "M0,0 L0,6 L9,3 z", fill: "currentColor" }
                    }
                }
                for (idx, row) in inbound_rows.iter().enumerate() {
                    {
                        let y = lane_y(idx, inbound_count);
                        let label = link_kind_label(&row.link_kind);
                        let source = row.realm_id.to_string();
                        let display = compact_realm_label(&source);
                        rsx! {
                            line {
                                x1: "160",
                                y1: "{y}",
                                x2: "330",
                                y2: "180",
                                stroke: "currentColor",
                                stroke_width: "2",
                                stroke_opacity: "0.55",
                                marker_end: "url(#realm-link-arrow)"
                            }
                            {graph_node(90, y, &display, "bg-source")}
                            text {
                                x: "225",
                                y: "{y - 8}",
                                class: "fill-current text-[10px] text-muted-foreground",
                                text_anchor: "middle",
                                "{label}"
                            }
                        }
                    }
                }
                for (idx, row) in outbound_rows.iter().enumerate() {
                    {
                        let y = lane_y(idx, outbound_count);
                        let label = link_kind_label(&row.link_kind);
                        let target = row.target_realm_id.to_string();
                        let display = compact_realm_label(&target);
                        rsx! {
                            line {
                                x1: "390",
                                y1: "180",
                                x2: "560",
                                y2: "{y}",
                                stroke: "currentColor",
                                stroke_width: "2",
                                stroke_opacity: "0.55",
                                marker_end: "url(#realm-link-arrow)"
                            }
                            {graph_node(630, y, &display, "bg-target")}
                            text {
                                x: "495",
                                y: "{y - 8}",
                                class: "fill-current text-[10px] text-muted-foreground",
                                text_anchor: "middle",
                                "{label}"
                            }
                        }
                    }
                }
                {graph_node(360, 180, &compact_realm_label(realm_id), "bg-current")}
            }
        }
    }
}

fn lane_y(index: usize, total: usize) -> i32 {
    let step = 260 / (total + 1);
    50 + (step * (index + 1)) as i32
}

fn compact_realm_label(realm_id: &str) -> String {
    const MAX: usize = 26;
    if realm_id.chars().count() <= MAX {
        return realm_id.to_string();
    }
    let prefix: String = realm_id.chars().take(12).collect();
    let suffix: String = realm_id
        .chars()
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{prefix}...{suffix}")
}

fn graph_node(x: i32, y: i32, label: &str, tone: &str) -> Element {
    let class = match tone {
        "bg-current" => "fill-primary stroke-primary",
        "bg-source" => "fill-blue-600 stroke-blue-700",
        "bg-target" => "fill-green-600 stroke-green-700",
        _ => "fill-muted stroke-border",
    };
    let text_color = if tone == "bg-current" {
        "fill-primary-foreground"
    } else {
        "fill-white"
    };
    rsx! {
        g {
            rect {
                x: "{x - 70}",
                y: "{y - 22}",
                width: "140",
                height: "44",
                rx: "8",
                class: "{class}",
                stroke_width: "1"
            }
            text {
                x: "{x}",
                y: "{y + 4}",
                class: "{text_color} text-[11px] font-mono",
                text_anchor: "middle",
                "{label}"
            }
        }
    }
}
