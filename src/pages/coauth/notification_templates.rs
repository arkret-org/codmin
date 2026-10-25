use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let data = ui::resource(scope, coauth::list_notification_templates);
    let outer = theme.clone();
    let body = ui::resource_view(scope, theme.clone(), data, move |_, rows| {
        ui::table(
            theme.clone(),
            vec!["Key".into(), "Description".into()],
            rows.into_iter()
                .map(|row| vec![row.key, row.description])
                .collect(),
        )
    });
    common::stack(
        outer,
        vec![common::heading("coauth.notification_templates.title"), body],
    )
}
