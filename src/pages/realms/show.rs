use glory::{Scope, Widget};

use crate::components::ui;
use crate::pages::{presentation as p, station};
use crate::types::realms::AdminRealmItemExt;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    let id = realm_id.clone();
    let data = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::realms::get_realm(&id).await }
    });
    ui::resource_view(scope, theme.clone(), data, move |_, realm| {
        let technical = ui::technical_details(theme.clone(), "realm-technical", &realm);
        p::stack(vec![
            p::breadcrumb(theme.clone(), "nav.realms", "/realms", &realm.title),
            ui::heading(&realm.title),
            p::badge(theme.clone(), realm.type_label(), "neutral"),
            p::grid(vec![
                p::card(
                    theme.clone(),
                    "detail.current_state",
                    vec![
                        ("realms.members", realm.member_count.to_string()),
                        (
                            "realms.status",
                            t(if realm.is_blocked {
                                "realms.blocked"
                            } else {
                                "realms.active"
                            }),
                        ),
                        ("realms.encrypted", p::yes(realm.is_encrypted)),
                    ],
                ),
                Box::new(
                    sapid::card()
                        .theme(theme.clone())
                        .title(t("detail.authority_sync"))
                        .fill(p::stack(vec![
                            p::description("detail.realm_status_not_reported"),
                            p::link(
                                theme.clone(),
                                t("detail.open_authority"),
                                format!("{}/authority", station::path("/realms", &realm.id)),
                            ),
                        ])),
                ),
                p::card(
                    theme.clone(),
                    "realms.overview",
                    vec![
                        ("realms.id", realm.id.clone()),
                        ("realms.type", realm.type_label().into()),
                        (
                            "realms.discoverability",
                            realm.discoverability_label().unwrap_or_else(|| "-".into()),
                        ),
                        ("realms.creator", p::optional(realm.created_by.as_ref())),
                        ("realms.members", realm.member_count.to_string()),
                        ("realms.encrypted", p::yes(realm.is_encrypted)),
                        (
                            "realms.join_rule",
                            realm.join_rule_label().unwrap_or_else(|| "-".into()),
                        ),
                        (
                            "realms.status",
                            t(if realm.is_blocked {
                                "realms.blocked"
                            } else {
                                "realms.active"
                            }),
                        ),
                        (
                            "realms.created_at",
                            realm.created_at_display().unwrap_or_else(|| "-".into()),
                        ),
                        ("realms.topic", p::optional(realm.topic.as_ref())),
                    ],
                ),
                Box::new(
                    sapid::card()
                        .theme(theme.clone())
                        .title(t("realms.actions"))
                        .fill(vec![
                            p::link(
                                theme.clone(),
                                t("realm_organization.title"),
                                format!("{}/organization", station::path("/realms", &realm.id)),
                            ),
                            p::link(
                                theme.clone(),
                                t("realm_authority.title").replace("{realm_id}", &realm.id),
                                format!("{}/authority", station::path("/realms", &realm.id)),
                            ),
                            p::link(
                                theme.clone(),
                                t("realm_links.title"),
                                format!("{}/links", station::path("/realms", &realm.id)),
                            ),
                            p::link(
                                theme.clone(),
                                t("media_service.title"),
                                format!("{}/media-service", station::path("/realms", &realm.id)),
                            ),
                        ]),
                ),
            ]),
            p::description("detail.signed_readonly"),
            technical,
        ])
    })
}
