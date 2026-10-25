use glory::prelude::*;

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    col().class("station-page").fill((
        station::read(scope, theme.clone(), t("server_status.title"), || async {
            Document::default().section(
                "Server information",
                &crate::api::server::get_server_info().await?,
            )
        }),
        super::dashboard::description(scope, theme.clone()),
        station::read(scope, theme, t("server_status.probe_title"), || async {
            Document::default().section(
                "Admin endpoint reachability (not a component health roll-up)",
                &crate::api::server::get_server_status().await?,
            )
        }),
    ))
}
