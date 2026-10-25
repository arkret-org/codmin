use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let data = ui::resource(scope, coauth::list_notification_templates);
    let table_theme = theme.clone();
    let body = common::resource_body(theme.clone(), data.clone(), move |_, rows| {
        common::table(
            table_theme.clone(),
            &[
                "coauth.notification_templates.key",
                "coauth.notification_templates.description",
            ],
            rows.into_iter()
                .map(|row| vec![common::cell(row.key), common::muted(row.description)])
                .collect(),
            "coauth.notification_templates.no_templates",
        )
    });
    common::page(
        "coauth.notification_templates.title",
        "coauth.notification_templates.subtitle",
        vec![common::refresh(theme, data)],
        vec![body],
    )
}
