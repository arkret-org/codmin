use glory::prelude::*;

use crate::components::ui::{self, Danger, Field, View};
use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("handles.title"),
        vec![("Search", "")],
        |q| async move {
            let response = crate::api::handles::list_handles(q.page, 25, q.field(0)).await?;
            let mut doc = Document::default()
                .section("Handles", &response.data)?
                .next(response.next_cursor)
                .numeric(q.page, 25, response.total);
            for row in response.data {
                doc = doc.link(row.canonical_uri, station::path("/handles", &row.id));
            }
            Ok(doc)
        },
    )
}
pub fn show_page(scope: &Scope, theme: sapid::Theme, handle_id: String) -> impl Widget + use<> {
    let id = handle_id.clone();
    let handle = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::handles::get_handle(&id).await }
    });
    let audit_id = handle_id.clone();
    let audit = ui::resource(scope, move || {
        let id = audit_id.clone();
        async move { crate::api::handles::get_handle_audit(&id).await }
    });
    let reload_handle = handle.clone();
    let reload_audit = audit.clone();
    let action_theme = theme.clone();
    let audit_theme = theme.clone();
    col().class("station-page").fill((
        ui::heading(t("handles.title")),
        ui::resource_view(scope, theme.clone(), handle, move |scope, row| {
            let mut children: Vec<View> = vec![station::value_view(
                action_theme.clone(),
                serde_json::to_value(&row).unwrap_or_default(),
            )];
            let id = row.id.clone();
            let h = reload_handle.clone();
            let a = reload_audit.clone();
            children.push(ui::action_form(
                scope,
                action_theme.clone(),
                t("handles.revoke"),
                vec![],
                Some(Danger::new(station::suffix(&id)).without_reason()),
                move |_| {
                    let id = id.clone();
                    async move { crate::api::handles::revoke_handle(&id).await }
                },
                move || {
                    h.reload();
                    a.reload();
                },
            ));
            let id = row.id;
            let h = reload_handle.clone();
            let a = reload_audit.clone();
            children.push(ui::action_form(
                scope,
                action_theme.clone(),
                t("handles.reassign"),
                vec![Field::new("New subject DID Core ID", "")],
                Some(Danger::new(station::suffix(&id))),
                move |values| {
                    let id = id.clone();
                    async move {
                        let new_subject_id = arkret_identifiers::DidCoreId::new(values[0].trim())
                            .map_err(|e| {
                            HttpError::message(format!("Invalid subject ID: {e}"))
                        })?;
                        let request = crate::types::AdminHandleReassignBody {
                            new_subject_id,
                            reason: values[1].clone(),
                        };
                        crate::api::handles::reassign_handle(&id, &request)
                            .await
                            .map(|_| ())
                    }
                },
                move || {
                    h.reload();
                    a.reload();
                },
            ));
            Box::new(col().fill(children))
        }),
        ui::resource_view(scope, theme, audit, move |_, entries| {
            Box::new(
                sapid::card()
                    .theme(audit_theme.clone())
                    .title("Handle audit history")
                    .fill(station::value_view(
                        audit_theme.clone(),
                        serde_json::to_value(entries.data).unwrap_or_default(),
                    )),
            )
        }),
    ))
}
