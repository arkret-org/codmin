use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger, Field};
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth.upstream_providers.id",
    "coauth.upstream_providers.issuer",
    "coauth.upstream_providers.enabled",
    "coauth.upstream_providers.created_at",
    "common.actions",
];
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let row_theme = theme.clone();
    let (body, data) = common::cursor(
        scope,
        theme.clone(),
        HEADERS,
        "coauth.upstream_providers.no_providers",
        |cursor| async move { coauth::list_upstream_providers(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let id = row.id.clone();
            let changed = data.clone();
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
                move || changed.reload(),
            );
            let id = row.id.clone();
            let enable = !row.is_enabled;
            let toggle_label = t(if enable {
                "common.enable"
            } else {
                "common.disable"
            });
            let toggle = ui::action_form_controlled(
                scope,
                &mutations,
                row.id.clone(),
                row_theme.clone(),
                toggle_label.clone(),
                vec![],
                Some(Danger::new(common::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move { coauth::toggle_upstream_provider(&id, enable).await }
                },
                move || data.reload(),
            );
            vec![
                common::mono(row.id),
                common::cell(common::optional(row.oidc_issuer_uri)),
                common::badge(
                    row_theme.clone(),
                    t(if row.is_enabled {
                        "common.enabled"
                    } else {
                        "common.disabled"
                    }),
                    if row.is_enabled { "success" } else { "neutral" },
                ),
                common::muted(common::optional(row.created_at)),
                common::group(vec![
                    ui::compact_action(row_theme.clone(), toggle_label, toggle),
                    ui::compact_action(
                        row_theme.clone(),
                        t("common.delete"),
                        common::stack(
                            row_theme.clone(),
                            vec![
                                common::muted(t("coauth.upstream_providers.delete_confirm")),
                                delete,
                            ],
                        ),
                    ),
                ]),
            ]
        },
    );
    let changed = data.clone();
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
        move || changed.reload(),
    );
    common::page(
        "coauth.upstream_providers.title",
        "coauth.upstream_providers.subtitle",
        vec![
            common::refresh(theme.clone(), data),
            ui::compact_action(theme, t("coauth.upstream_providers.create"), create),
        ],
        vec![body],
    )
}
