use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let data = ui::resource(scope, coauth::list_notification_channels);
    let card_theme = theme.clone();
    let body = common::resource_body(theme.clone(), data.clone(), move |_, channels| {
        let cards = channels
            .into_iter()
            .map(|channel| {
                Box::new(
                    sapid::card()
                        .theme(card_theme.clone())
                        .class("coauth-channel-card")
                        .fill(view().class("status-card-body").fill((
                            view().class("status-card-copy").fill((
                                text().class("status-card-title").content(channel.channel),
                                text().class("text-muted").content(t(if channel.configured {
                                    "coauth.notification_channels.configured"
                                } else {
                                    "coauth.notification_channels.unconfigured"
                                })),
                            )),
                            common::badge(
                                card_theme.clone(),
                                if channel.configured {
                                    "Configured"
                                } else {
                                    "Off"
                                },
                                if channel.configured {
                                    "success"
                                } else {
                                    "neutral"
                                },
                            ),
                        ))),
                ) as ui::View
            })
            .collect::<Vec<_>>();
        Box::new(view().class("coauth-card-grid").fill(cards))
    });
    common::page(
        "coauth.notification_channels.title",
        "coauth.notification_channels.subtitle",
        vec![common::refresh(theme, data)],
        vec![body],
    )
}
