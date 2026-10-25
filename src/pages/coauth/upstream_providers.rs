use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger, Field};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let row_theme = theme.clone();
    let (body, data) = common::cursor(
        scope,
        theme.clone(),
        |cursor| async move { coauth::list_upstream_providers(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let id = row.id.clone();
            let delete_data = data.clone();
            let delete = ui::action_form_controlled(
                scope,
                &mutations,
                row.id.clone(),
                row_theme.clone(),
                t("common.delete"),
                vec![],
                Some(Danger::new(common::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move { coauth::delete_upstream_provider(&id).await }
                },
                move || delete_data.reload(),
            );
            let id = row.id.clone();
            let enable = !row.is_enabled;
            let toggle = ui::action_form_controlled(
                scope,
                &mutations,
                row.id.clone(),
                row_theme.clone(),
                t(if enable {
                    "common.enable"
                } else {
                    "common.disable"
                }),
                vec![],
                Some(Danger::new(common::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move { coauth::toggle_upstream_provider(&id, enable).await }
                },
                move || data.reload(),
            );
            Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(row.id.clone())
                    .fill((common::record(&row), toggle, delete)),
            )
        },
    );
    let create = ui::action_form(
        scope,
        theme.clone(),
        t("common.create"),
        vec![
            Field::new(t("coauth.upstream_providers.issuer"), ""),
            Field::new(t("coauth.upstream_providers.client_id"), ""),
        ],
        None,
        |v| async move {
            if v[0].trim().is_empty() || v[1].trim().is_empty() {
                return Err(crate::utils::net::error::HttpError::message(
                    "Issuer and client ID are required",
                ));
            }
            coauth::create_upstream_provider(&coauth::CreateUpstreamProviderRequest {
                oidc_issuer_uri: v[0].trim().into(),
                client_id: v[1].trim().into(),
            })
            .await
            .map(|_| ())
        },
        move || data.reload(),
    );
    common::stack(
        theme,
        vec![
            common::heading("coauth.upstream_providers.title"),
            create,
            body,
        ],
    )
}
