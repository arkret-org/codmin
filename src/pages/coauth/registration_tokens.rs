use glory::prelude::*;

use super::common;
use crate::api::coauth;
use crate::components::ui::{self, Danger, Field};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let created = scope.cage(String::new());
    let row_theme = theme.clone();
    let (body, data) = common::cursor(
        scope,
        theme.clone(),
        |cursor| async move { coauth::list_registration_tokens(cursor.as_deref(), 25).await },
        move |scope, row, data, mutations| {
            let mut children = vec![common::record(&row)];
            if !row.is_revoked {
                let id = row.id.clone();
                children.push(ui::action_form_controlled(
                    scope,
                    &mutations,
                    row.id.clone(),
                    row_theme.clone(),
                    t("common.revoke"),
                    vec![],
                    Some(Danger::new(common::suffix(&id)).without_reason()),
                    move |_| {
                        let id = id.clone();
                        async move { coauth::revoke_registration_token(&id).await }
                    },
                    move || data.reload(),
                ));
            }
            Box::new(
                sapid::card()
                    .theme(row_theme.clone())
                    .title(row.id.clone())
                    .fill(common::stack(row_theme.clone(), children)),
            )
        },
    );
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
        move || data.reload(),
    );
    common::stack(
        theme.clone(),
        vec![
            common::heading("coauth.registration_tokens.title"),
            create,
            Box::new(text().content(created)),
            Box::new(
                sapid::button()
                    .theme(theme)
                    .label(t("common.close"))
                    .on_press(move || created.revise(String::clear)),
            ),
            body,
        ],
    )
}
