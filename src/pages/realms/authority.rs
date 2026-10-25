use glory::prelude::*;

use crate::components::ui;
use crate::pages::{presentation as p, station};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme, realm_id: String) -> impl Widget + use<> {
    if station::placeholder(&realm_id) {
        return station::selection("Realm");
    }
    let id = realm_id.clone();
    let data = ui::resource(scope, move || {
        let id = id.clone();
        async move { crate::api::authority::get_realm_authority_bundle(&id).await }
    });
    ui::resource_view(scope, theme.clone(), data, move |_, bundle| {
        p::stack(vec![
            ui::heading(t("realm_authority.title").replace("{realm_id}", &realm_id)),
            p::description("realm_authority.description"),
            p::card(
                theme.clone(),
                "realm_authority.current_station_title",
                vec![
                    (
                        "realm_authority.station_label",
                        bundle.current_service_id.to_string(),
                    ),
                    (
                        "realm_authority.generation_label",
                        bundle.current_generation.to_string(),
                    ),
                    (
                        "realm_authority.last_handoff_label",
                        p::optional(bundle.current_assertion.last_handoff_ref.as_ref()),
                    ),
                    (
                        "realm_authority.assertion_nonce_label",
                        bundle.current_assertion.nonce.as_str().into(),
                    ),
                    (
                        "realm_authority.assertion_expires_label",
                        bundle.current_assertion.expires_at.to_rfc3339(),
                    ),
                ],
            ),
            Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("realm_authority.stream_head_title"))
                    .fill((
                        p::description("realm_authority.stream_head_note"),
                        ui::details(vec![
                            (
                                t("realm_authority.stream_position_label"),
                                bundle.realm_stream_head.stream_position.to_string(),
                            ),
                            (
                                t("realm_authority.head_commit_label"),
                                bundle.realm_stream_head.commit_id.to_string(),
                            ),
                            (
                                t("realm_authority.genesis_event_label"),
                                bundle.genesis_event.event_id.to_string(),
                            ),
                            (
                                t("realm_authority.genesis_commit_label"),
                                bundle.genesis_commit.commit_id.to_string(),
                            ),
                        ]),
                    )),
            ),
            Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("realm_authority.handoff_chain_title"))
                    .fill(p::table(
                        theme.clone(),
                        &[
                            "realm_authority.col_generations",
                            "realm_authority.col_from_station",
                            "realm_authority.col_to_station",
                            "realm_authority.col_change_commit",
                            "realm_authority.col_snapshot",
                            "realm_authority.col_signatures",
                        ],
                        bundle
                            .authority_transitions
                            .iter()
                            .map(|transition| {
                                let handoff = &transition.handoff;
                                vec![
                                    p::plain(format!(
                                        "{} → {}",
                                        handoff.from_generation, handoff.to_generation
                                    )),
                                    p::mono(&handoff.from_service_id),
                                    p::mono(&handoff.to_service_id),
                                    p::mono(&handoff.change_commit_id),
                                    p::mono(&handoff.snapshot_ref),
                                    p::stack(vec![
                                        p::mono(
                                            &handoff.old_authority_signature.verification_method,
                                        ),
                                        p::mono(
                                            &handoff
                                                .new_authority_acceptance_signature
                                                .verification_method,
                                        ),
                                    ]),
                                ]
                            })
                            .collect(),
                        "realm_authority.no_handoff_title",
                    )),
            ),
        ])
    })
}
