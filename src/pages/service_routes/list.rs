use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    station::browse(
        scope,
        theme,
        t("service_routes.title"),
        vec![],
        |q| async move {
            let response = crate::api::service_routes::list_service_routes(q.cursor(), 25).await?;
            let rows = response.routes.clone();
            Ok(Document::default()
                .section("Service routes", &response.routes)?
                .next(response.next_cursor)
                .view(move |_, theme| {
                    p::table(
                        theme.clone(),
                        &[
                            "service_routes.service_id",
                            "service_routes.kind",
                            "service_routes.version",
                            "service_routes.cache_expiry",
                            "service_routes.status",
                        ],
                        rows.iter()
                            .map(|row| {
                                let id = row.service_id.to_string();
                                let href = format!(
                                    "{}/{}",
                                    station::path("/service-routes", &id),
                                    urlencoding::encode(&row.service_kind)
                                );
                                vec![
                                    p::link(theme.clone(), id, href),
                                    p::mono(&row.service_kind),
                                    p::mono(
                                        row.version_id
                                            .clone()
                                            .unwrap_or_else(|| t("service_routes.unknown")),
                                    ),
                                    p::mono(
                                        row.cache_expires_at
                                            .map(|value| value.to_rfc3339())
                                            .unwrap_or_else(|| t("service_routes.unknown")),
                                    ),
                                    p::badge(
                                        theme.clone(),
                                        t(if row.quarantined {
                                            "service_routes.quarantined"
                                        } else if row.known {
                                            "service_routes.known"
                                        } else {
                                            "service_routes.unknown"
                                        }),
                                        if row.quarantined { "danger" } else { "neutral" },
                                    ),
                                ]
                            })
                            .collect(),
                        "service_routes.empty",
                    )
                }))
        },
    )
}
