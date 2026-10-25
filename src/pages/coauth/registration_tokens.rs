use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger, Field};
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth.registration_tokens.id",
    "coauth.registration_tokens.token",
    "coauth.registration_tokens.uses_allowed",
    "coauth.registration_tokens.uses_completed",
    "coauth.registration_tokens.uses_pending",
    "coauth.registration_tokens.expires_at",
    "coauth.registration_tokens.revoked",
    "common.actions",
];
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let created = scope.cage(String::new());
    let row_theme = theme.clone();
    let (body, data) = common::cursor(
        scope,
        theme.clone(),
        HEADERS,
        "coauth.registration_tokens.no_tokens",
        |cursor| async move { coauth::list_registration_tokens(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let action = if !row.is_revoked {
                let id = row.id.clone();
                let form = ui::action_form_controlled(
                    scope,
                    &mutations,
                    row.id.clone(),
                    row_theme.clone(),
                    t("coauth.registration_tokens.revoke"),
                    vec![],
                    Some(Danger::new(common::suffix(&id)).without_reason()),
                    move |_| {
                        let id = id.clone();
                        async move { coauth::revoke_registration_token(&id).await }
                    },
                    move || data.reload(),
                );
                ui::compact_action(
                    row_theme.clone(),
                    t("coauth.registration_tokens.revoke"),
                    common::stack(
                        row_theme.clone(),
                        vec![
                            common::muted(t("coauth.registration_tokens.revoke_confirm")),
                            form,
                        ],
                    ),
                )
            } else {
                common::muted("—")
            };
            vec![
                common::mono(row.id),
                common::mono(common::optional(row.token)),
                common::cell(
                    row.uses_allowed
                        .map_or_else(|| "Unlimited".into(), |n| n.to_string()),
                ),
                common::cell(row.uses_completed.to_string()),
                common::cell(row.uses_pending.to_string()),
                common::muted(common::optional(row.expires_at)),
                common::badge(
                    row_theme.clone(),
                    t(if row.is_revoked {
                        "common.yes"
                    } else {
                        "common.no"
                    }),
                    if row.is_revoked { "danger" } else { "neutral" },
                ),
                action,
            ]
        },
    );
    let changed = data.clone();
    let create = ui::action_form(
        scope,
        theme.clone(),
        t("common.create"),
        vec![Field::new(t("coauth.registration_tokens.uses_allowed"), "")],
        None,
        move |values| async move {
            let value = values[0].trim();
            let limit = if value.is_empty() {
                None
            } else {
                Some(
                    value
                        .parse::<u64>()
                        .ok()
                        .filter(|n| *n > 0 && *n <= u32::MAX as u64)
                        .ok_or_else(|| {
                            crate::utils::net::error::HttpError::message(
                                "Usage limit must be a positive 32-bit integer",
                            )
                        })?,
                )
            };
            let response = coauth::create_registration_token(limit).await?;
            created.revise(|v| *v = response.token.unwrap_or_default());
            Ok(())
        },
        move || changed.reload(),
    );
    let reveal_theme = theme.clone();
    let reveal = ui::reactive(created, move |_, value| {
        if value.is_empty() {
            return Box::new(group());
        }
        common::panel(
            reveal_theme.clone(),
            t("coauth.registration_tokens.token"),
            Box::new(
                view().class("token-reveal").fill((
                    text().class("mono").content(value),
                    sapid::button()
                        .theme(reveal_theme.clone())
                        .label(t("common.close"))
                        .on_press(move || created.revise(String::clear)),
                )),
            ),
        )
    });
    common::page(
        "coauth.registration_tokens.title",
        "coauth.registration_tokens.subtitle",
        vec![
            common::refresh(theme.clone(), data),
            ui::compact_action(theme, t("coauth.registration_tokens.create"), create),
        ],
        vec![reveal, body],
    )
}
