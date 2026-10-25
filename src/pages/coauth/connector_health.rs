use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let data = ui::resource(scope, coauth::get_connector_health);
    let card_theme = theme.clone();
    let body = common::resource_body(theme.clone(), data.clone(), move |_, connectors| {
        let cards = connectors
            .into_iter()
            .map(|connector| {
                let healthy = connector.is_healthy();
                let mut copy = vec![
                    Box::new(
                        text()
                            .class("status-card-title")
                            .content(connector.provider),
                    ) as ui::View,
                    common::muted(connector.account_id),
                ];
                if !healthy && let Some(error) = connector.error {
                    copy.push(Box::new(text().class("text-danger").content(error)));
                }
                Box::new(
                    sapid::card()
                        .theme(card_theme.clone())
                        .class("coauth-connector-card")
                        .fill(view().class("status-card-body").fill((
                            view().class("status-card-copy").fill(copy),
                            common::badge(
                                card_theme.clone(),
                                if healthy { "OK" } else { "Error" },
                                if healthy { "success" } else { "danger" },
                            ),
                        ))),
                ) as ui::View
            })
            .collect::<Vec<_>>();
        Box::new(view().class("coauth-card-grid").fill(cards))
    });
    common::page(
        "coauth.connector_health.title",
        "coauth.connector_health.subtitle",
        vec![common::refresh(theme, data)],
        vec![body],
    )
}
