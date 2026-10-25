use glory::prelude::*;

use crate::components::ui::{self, Danger, Field, View};
use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("handles.title"),
        vec![("handles.search", "")],
        |q| async move {
            let response = crate::api::handles::list_handles(q.page, 25, q.field(0)).await?;
            let rows = response.data.clone();
            Ok(Document::default()
                .section("Handles", &response.data)?
                .next(response.next_cursor)
                .numeric(q.page, 25, response.total)
                .view(move |scope, theme| {
                    p::table(
                        theme.clone(),
                        &[
                            "handles.canonical_uri",
                            "handles.aliases",
                            "handles.issuer",
                            "handles.expires_at",
                            "handles.last_reassignment",
                            "handles.status",
                            "common.actions",
                        ],
                        rows.iter()
                            .map(|row| {
                                let status = row.status.as_deref().unwrap_or("active");
                                vec![
                                    p::link(
                                        theme.clone(),
                                        &row.canonical_uri,
                                        station::path("/handles", &row.id),
                                    ),
                                    p::plain(row.aliases.join(", ")),
                                    p::mono(&row.issuer_id),
                                    p::plain(p::optional(row.expires_at.as_ref())),
                                    p::plain(p::optional(row.last_reassignment_at.as_ref())),
                                    p::badge(
                                        theme.clone(),
                                        status,
                                        if status == "revoked" {
                                            "danger"
                                        } else {
                                            "success"
                                        },
                                    ),
                                    handle_actions(scope, theme.clone(), row, || {
                                        crate::router::navigate(&crate::router::current_path())
                                    }),
                                ]
                            })
                            .collect(),
                        "handles.empty",
                    )
                }))
        },
    )
}
pub fn show_page(scope: &Scope, theme: sapid::Theme, handle_id: String) -> impl Widget + use<> {
    let id = handle_id.clone();
    let handle = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::handles::get_handle(&id).await }
    });
    let id = handle_id.clone();
    let audit = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::handles::get_handle_audit(&id).await }
    });
    let reload_handle = handle.clone();
    let reload_audit = audit.clone();
    let th = theme.clone();
    let ta = theme.clone();
    col().class("station-page").fill((
        ui::heading(t("handles.title")),
        p::description("handles.subtitle"),
        ui::resource_view(scope, theme.clone(), handle, move |scope, row| {
            let h = reload_handle.clone();
            let a = reload_audit.clone();
            let actions = handle_actions(scope, th.clone(), &row, move || {
                h.reload();
                a.reload();
            });
            p::stack(vec![
                p::breadcrumb(th.clone(), "handles.title", "/handles", &row.canonical_uri),
                Box::new(
                    sapid::card()
                        .theme(th.clone())
                        .title(row.canonical_uri.clone())
                        .description(format!(
                            "{}: {}",
                            t("handles.aliases"),
                            if row.aliases.is_empty() {
                                "-".into()
                            } else {
                                row.aliases.join(", ")
                            }
                        ))
                        .fill((
                            p::badge(
                                th.clone(),
                                row.status.as_deref().unwrap_or("active"),
                                if row.status.as_deref() == Some("revoked") {
                                    "danger"
                                } else {
                                    "success"
                                },
                            ),
                            ui::details(vec![
                                (t("handles.issuer"), row.issuer_id.to_string()),
                                (t("handles.subject"), row.subject_id.to_string()),
                                (
                                    t("handles.assigned_at"),
                                    p::optional(row.assigned_at.as_ref()),
                                ),
                                (
                                    t("handles.expires_at"),
                                    p::optional(row.expires_at.as_ref()),
                                ),
                                (
                                    t("handles.last_reassignment"),
                                    p::optional(row.last_reassignment_at.as_ref()),
                                ),
                            ]),
                            actions,
                        )),
                ),
            ])
        }),
        ui::resource_view(scope, theme, audit, move |_, entries| {
            Box::new(
                sapid::card()
                    .theme(ta.clone())
                    .title(t("handles.audit_title"))
                    .description(t("handles.audit_subtitle"))
                    .fill(p::table(
                        ta.clone(),
                        &[
                            "handles.audit_when",
                            "handles.audit_action",
                            "handles.audit_actor",
                            "handles.audit_reason",
                        ],
                        entries
                            .data
                            .iter()
                            .map(|row| {
                                vec![
                                    p::plain(p::optional(row.timestamp.as_ref())),
                                    p::plain(&row.action),
                                    p::mono(p::optional(row.actor_id.as_ref())),
                                    p::plain(p::optional(row.reason.as_ref())),
                                ]
                            })
                            .collect(),
                        "handles.audit_empty",
                    )),
            )
        }),
    ))
}
fn handle_actions(
    scope: &Scope,
    theme: sapid::Theme,
    row: &crate::types::AdminHandleRecord,
    on_success: impl Fn() + 'static,
) -> View {
    let id = row.id.clone();
    let notify = std::rc::Rc::new(on_success);
    let revoke = notify.clone();
    let reassign = notify.clone();
    if row.status.as_deref() == Some("revoked") {
        return p::plain("-");
    }
    let rid = id.clone();
    let reason_id = id.clone();
    p::stack(vec![
        ui::compact_action(
            theme.clone(),
            t("handles.revoke"),
            ui::action_form(
                scope,
                theme.clone(),
                t("handles.revoke"),
                vec![],
                Some(Danger::new(station::suffix(&id)).without_reason()),
                move |_| {
                    let id = rid.clone();
                    async move { crate::api::handles::revoke_handle(&id).await }
                },
                move || revoke(),
            ),
        ),
        ui::compact_action(
            theme.clone(),
            t("handles.reassign"),
            ui::action_form(
                scope,
                theme,
                t("handles.reassign"),
                vec![Field::new(t("handles.subject"), "")],
                Some(Danger::new(station::suffix(&id))),
                move |values| {
                    let id = reason_id.clone();
                    async move {
                        let new_subject_id = arkret_identifiers::DidCoreId::new(values[0].trim())
                            .map_err(|e| HttpError::message(e.to_string()))?;
                        crate::api::handles::reassign_handle(
                            &id,
                            &crate::types::AdminHandleReassignBody {
                                new_subject_id,
                                reason: values[1].clone(),
                            },
                        )
                        .await
                        .map(|_| ())
                    }
                },
                move || reassign(),
            ),
        ),
    ])
}
