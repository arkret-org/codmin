use glory::prelude::*;

use crate::components::ui::{self, View};
use crate::pages::presentation as p;
use crate::utils::i18n::t;
pub fn page(
    scope: &Scope,
    theme: sapid::Theme,
    service_id: String,
    service_kind: String,
) -> impl Widget + use<> {
    let id = service_id.clone();
    let data = ui::resource(scope, move || {
        let id = id.clone();
        let kind = service_kind.clone();
        async move { crate::api::service_routes::get_service_route(&id, &kind).await }
    });
    ui::resource_view(scope, theme.clone(), data, move |_, detail| {
        let mut children: Vec<View> = vec![
            p::breadcrumb(
                theme.clone(),
                "service_routes.title",
                "/service-routes",
                &service_id,
            ),
            ui::heading(detail.service_id.to_string()),
            p::plain(format!(
                "{} · {}",
                detail.service_kind,
                t("service_routes.local_only")
            )),
        ];
        if !detail.known {
            children.push(p::card(
                theme.clone(),
                "service_routes.unknown",
                vec![("service_routes.unknown_detail", String::new())],
            ));
            return p::stack(children);
        }
        let mut fields = vec![
            ("service_routes.service_id", detail.service_id.to_string()),
            ("service_routes.kind", detail.service_kind.clone()),
            ("service_routes.authority", detail.authority.clone()),
            (
                "service_routes.observed_at",
                detail.observed_at.to_rfc3339(),
            ),
        ];
        if let Some(current) = detail.current_route {
            fields.extend([
                ("service_routes.did", current.did),
                ("service_routes.history_head", current.method_history_head),
                ("service_routes.version", current.version_id),
                ("service_routes.base_url", current.base_url),
            ]);
        } else {
            fields.push(("service_routes.current_title", t("service_routes.unknown")));
        }
        children.push(p::card(
            theme.clone(),
            "service_routes.current_title",
            fields,
        ));
        children.push(p::grid(vec![
            p::card(
                theme.clone(),
                "service_routes.method_state_title",
                match detail.method_state {
                    Some(state) => vec![
                        ("service_routes.version", state.version_id),
                        ("service_routes.history_head", state.method_history_head),
                        ("service_routes.verified_at", state.verified_at.to_rfc3339()),
                    ],
                    None => vec![("service_routes.unknown", String::new())],
                },
            ),
            p::card(
                theme.clone(),
                "service_routes.cache_title",
                match detail.cache {
                    Some(cache) => vec![
                        ("service_routes.verified_at", cache.verified_at.to_rfc3339()),
                        (
                            "service_routes.cache_expiry",
                            cache.cache_expires_at.to_rfc3339(),
                        ),
                        (
                            "service_routes.routable",
                            p::yes(cache.routable_at_observed_at),
                        ),
                    ],
                    None => vec![("service_routes.unknown", String::new())],
                },
            ),
        ]));
        let mut quarantine: Vec<View> = detail
            .quarantine
            .into_iter()
            .map(|item| {
                Box::new(
                    sapid::card()
                        .theme(theme.clone())
                        .class("quarantine-item")
                        .fill((
                            p::badge(theme.clone(), t("service_routes.quarantined"), "danger"),
                            p::mono(item.version_id),
                            p::mono(format!(
                                "{} → {}",
                                item.accepted_digest, item.conflicting_digest
                            )),
                            p::mono(
                                serde_json::to_string_pretty(&item.diagnostic)
                                    .unwrap_or_else(|_| "{}".into()),
                            ),
                        )),
                ) as View
            })
            .collect();
        if quarantine.is_empty() {
            quarantine.push(p::plain(t("service_routes.none")));
        } else if detail.quarantine_truncated {
            quarantine.push(p::description("service_routes.truncated"));
        }
        children.push(Box::new(
            sapid::card()
                .theme(theme.clone())
                .title(t("service_routes.quarantine_title"))
                .description(t("service_routes.private_diagnostics"))
                .fill(quarantine),
        ));
        p::stack(children)
    })
}
