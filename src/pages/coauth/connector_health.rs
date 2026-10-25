use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let data = ui::resource(scope, coauth::get_connector_health);
    let outer = theme.clone();
    let body = ui::resource_view(scope, theme.clone(), data, move |_, rows| {
        let children = rows
            .into_iter()
            .map(|row| {
                Box::new(
                    sapid::card()
                        .theme(theme.clone())
                        .title(format!(
                            "{} · {}",
                            row.provider,
                            if row.is_healthy() { "OK" } else { "Error" }
                        ))
                        .fill(common::record(&row)),
                ) as ui::View
            })
            .collect();
        common::stack(theme.clone(), children)
    });
    common::stack(
        outer,
        vec![common::heading("coauth.connector_health.title"), body],
    )
}
