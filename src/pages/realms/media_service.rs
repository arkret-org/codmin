use glory::prelude::*;

use crate::components::ui::{self, View};
use crate::pages::{presentation as p, station};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    if station::placeholder(&realm_id) {
        return station::selection("Realm");
    }
    let id = realm_id.clone();
    let data = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::media_service::get_realm_media_service(&id).await }
    });
    ui::resource_view(scope, theme.clone(), data, move |_, cell| {
        let foci: Vec<View> = cell
            .foci
            .iter()
            .enumerate()
            .map(|(index, focus)| {
                use arkret_models_collaboration::objects::media::MediaBackendKind;
                let backend = match focus.focus_kind {
                    MediaBackendKind::Livekit => "livekit",
                    MediaBackendKind::Mediasoup => "mediasoup",
                    MediaBackendKind::Janus => "janus",
                    MediaBackendKind::ArkretNative => "arkret_native",
                    MediaBackendKind::MoqRelay => "moq_relay",
                };
                p::card(
                    theme.clone(),
                    &t("media_service.focus_index").replace("{n}", &(index + 1).to_string()),
                    vec![
                        ("media_service.focus_id", focus.focus_id.to_string()),
                        ("media_service.focus_backend", backend.into()),
                        (
                            "media_service.focus_region",
                            p::optional(focus.region.as_ref()),
                        ),
                        (
                            "media_service.focus_token_endpoint",
                            focus.token_endpoint.clone(),
                        ),
                        ("media_service.focus_connect_url", focus.connect_url.clone()),
                        (
                            "media_service.focus_health_endpoint",
                            p::optional(focus.health_endpoint.as_ref()),
                        ),
                        (
                            "media_service.focus_cascade_group",
                            p::optional(focus.cascade_group.as_ref()),
                        ),
                        (
                            "media_service.focus_capabilities",
                            if focus.capabilities.is_empty() {
                                "-".into()
                            } else {
                                focus
                                    .capabilities
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            },
                        ),
                    ],
                )
            })
            .collect();
        p::stack(vec![
            p::breadcrumb(
                theme.clone(),
                "nav.realms",
                &station::path("/realms", &realm_id),
                t("media_service.title"),
            ),
            ui::heading(t("media_service.title")),
            p::plain(format!("{}: {}", t("nav.realms"), realm_id)),
            p::description("media_service.readonly_note"),
            Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("media_service.epoch_title"))
                    .description(t("media_service.subtitle"))
                    .fill(ui::details(vec![(
                        t("media_service.service_id"),
                        p::optional(cell.service_id.as_ref()),
                    )])),
            ),
            Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("media_service.foci_label"))
                    .fill(if foci.is_empty() {
                        vec![p::plain(t("media_service.empty"))]
                    } else {
                        foci
                    }),
            ),
        ])
    })
}
