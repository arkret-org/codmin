use glory::prelude::*;

use crate::components::ui;
use crate::pages::station;
use crate::types::{RealmLinkDirection, RealmLinkList};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    if station::placeholder(&realm_id) {
        return station::selection("Realm");
    }
    let out_id = realm_id.clone();
    let outbound = ui::resource(scope, move || {
        let id = out_id.clone();
        async move { crate::api::realm_links::list_realm_links(&id, RealmLinkDirection::Outbound).await }
    });
    let in_id = realm_id.clone();
    let inbound = ui::resource(scope, move || {
        let id = in_id.clone();
        async move { crate::api::realm_links::list_realm_links(&id, RealmLinkDirection::Inbound).await }
    });
    let out = outbound.data;
    let incoming = inbound.data;
    let edges = scope.bond(move || {
        let mut rows = edge_rows(&incoming.get(), true);
        rows.extend(edge_rows(&out.get(), false));
        rows
    });
    let graph_empty = scope.bond(move || edges.get().is_empty());
    let state = scope.bond(move || {
        let describe = |result: Option<std::result::Result<RealmLinkList, HttpError>>| match result
        {
            None => "loading".to_owned(),
            Some(Ok(list)) => format!("{} links", list.links.len()),
            Some(Err(_)) => "unavailable; see the error below".to_owned(),
        };
        format!(
            "Inbound: {}; outbound: {}",
            describe(incoming.get()),
            describe(out.get())
        )
    });
    let graph_theme = theme.clone();
    let out_theme = theme.clone();
    let in_theme = theme.clone();
    let graph = sapid::card()
        .theme(theme.clone())
        .title(t("realm_links.graph_title"))
        .fill((
            text().content(format!("Selected Realm: {realm_id}")),
            text().content(state),
            when(graph_empty, || text().content(t("realm_links.graph_empty"))),
            each(
                edges,
                |edge| edge.key.clone(),
                move |edge, _| {
                    row()
                        .class("realm-link-edge")
                        .style(
                            Style::new()
                                .display(Display::Flex)
                                .flex_direction(FlexDirection::Row)
                                .flex_wrap(glory::style::FlexWrap::Wrap)
                                .gap(12.0),
                        )
                        .fill((
                            sapid::card()
                                .theme(graph_theme.clone())
                                .title("Source Realm")
                                .fill(
                                    sapid::anchor()
                                        .theme(graph_theme.clone())
                                        .href(
                                            edge.map(|edge| station::path("/realms", &edge.source)),
                                        )
                                        .label(edge.map(|edge| edge.source.clone())),
                                ),
                            col().class("realm-link-direction").fill((
                                text().content(edge.map(|edge| edge.direction.clone())),
                                text().content("→"),
                                text().content(edge.map(|edge| edge.kind.clone())),
                                text().content(edge.map(|edge| format!("Status: {}", edge.status))),
                                text().content(edge.map(|edge| edge.label.clone())),
                            )),
                            sapid::card()
                                .theme(graph_theme.clone())
                                .title("Target Realm")
                                .fill(
                                    sapid::anchor()
                                        .theme(graph_theme.clone())
                                        .href(
                                            edge.map(|edge| station::path("/realms", &edge.target)),
                                        )
                                        .label(edge.map(|edge| edge.target.clone())),
                                ),
                        ))
                },
            ),
        ));
    Box::new(col().class("station-page").fill((
        ui::heading(t("realm_links.title")),
        graph,
        ui::resource_view(scope, theme.clone(), outbound, move |_, list| {
            Box::new(
                sapid::card()
                    .theme(out_theme.clone())
                    .title(t("realm_links.outbound_title"))
                    .fill(station::value_view(
                        out_theme.clone(),
                        serde_json::to_value(list).unwrap_or_default(),
                    )),
            )
        }),
        ui::resource_view(scope, theme, inbound, move |_, list| {
            Box::new(
                sapid::card()
                    .theme(in_theme.clone())
                    .title(t("realm_links.inbound_title"))
                    .fill(station::value_view(
                        in_theme.clone(),
                        serde_json::to_value(list).unwrap_or_default(),
                    )),
            )
        }),
    )))
}

// Presentation rows preserve directed edges; they do not infer a hierarchy.
#[derive(Clone)]
struct Edge {
    key: String,
    source: String,
    target: String,
    direction: String,
    kind: String,
    status: String,
    label: String,
}
fn edge_rows(
    result: &Option<std::result::Result<RealmLinkList, HttpError>>,
    inbound: bool,
) -> Vec<Edge> {
    let Some(Ok(list)) = result else {
        return Vec::new();
    };
    list.links
        .iter()
        .map(|row| {
            let kind = row.link_kind.as_str();
            let translated = t(&format!("realm_links.kind.{kind}"));
            Edge {
                key: format!(
                    "{}:{}:{}:{kind}",
                    if inbound { "in" } else { "out" },
                    row.realm_id,
                    row.target_realm_id
                ),
                source: row.realm_id.to_string(),
                target: row.target_realm_id.to_string(),
                direction: if inbound {
                    "Inbound → selected Realm"
                } else {
                    "Selected Realm → outbound"
                }
                .to_owned(),
                kind: if translated.starts_with("realm_links.kind.") {
                    kind.to_owned()
                } else {
                    translated
                },
                status: row.status.as_str().to_owned(),
                label: row.label.clone().unwrap_or_default(),
            }
        })
        .collect()
}
