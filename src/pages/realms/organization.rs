use arkret_models_collaboration::governance::realm_governance::RealmOrganizationLifecyclePhase;
use arkret_models_collaboration::{
    RealmOrganizationControlScope, RealmOrganizationIssuerRole, RealmOrganizationRelationship,
};
use coauth_admin_types::organization_admin::{
    OrganizationBootstrapAuthorization, OrganizationControlView, OrganizationDelegation,
    OrganizationDelegationStatus,
};
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
        async move { crate::api::realm_organization::get_realm_organization_admin_view(&id).await }
    });
    ui::resource_view(scope, theme.clone(), data, move |_, view| {
        let mut verified: Vec<View> = Vec::new();
        if !view.relationships.declared_organization_hint_ids.is_empty() {
            verified.push(Box::new(
                sapid::alert("declared-only")
                    .theme(theme.clone())
                    .severity(sapid::AlertSeverity::Warning)
                    .title(t("realm_organization.declared_only_title"))
                    .content(
                        view.relationships
                            .declared_organization_hint_ids
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(
                                "
",
                            ),
                    ),
            ));
        }
        verified.push(p::table_with_key(
            theme.clone(),
            "organization-relationships",
            &[
                "realm_organization.col_org",
                "realm_organization.col_relationship",
                "realm_organization.col_status",
                "realm_organization.col_scopes",
                "realm_organization.col_issued",
                "realm_organization.col_expires",
                "realm_organization.col_statement",
            ],
            view.relationships
                .relationships
                .iter()
                .map(|row| {
                    let (key, severity) = match row.lifecycle_phase {
                        RealmOrganizationLifecyclePhase::VerifiedActive => {
                            ("realm_organization.lifecycle_active", "success")
                        }
                        RealmOrganizationLifecyclePhase::RevokedOrExpired => {
                            ("realm_organization.lifecycle_revoked_or_expired", "danger")
                        }
                    };
                    vec![
                        p::mono(&row.organization_id),
                        p::plain(relationship_label(row.relationship)),
                        p::badge(theme.clone(), t(key), severity),
                        p::plain(
                            row.control_scopes
                                .iter()
                                .map(|v| scope_label(*v))
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                        p::plain(row.issued_at.to_rfc3339()),
                        p::plain(
                            row.expires_at
                                .map(|v| v.to_rfc3339())
                                .unwrap_or_else(|| t("realm_organization.no_expiry")),
                        ),
                        p::stack(vec![
                            p::mono(&row.statement_id),
                            p::plain(issuer_role_label(row.issuer_role)),
                        ]),
                    ]
                })
                .collect(),
            "realm_organization.verified_empty",
        ));
        let mut controls: Vec<View> = Vec::new();
        if !view.unavailable_control_ids.is_empty() {
            controls.push(p::card(
                theme.clone(),
                "realm_organization.control_unavailable_title",
                view.unavailable_control_ids
                    .iter()
                    .map(|id| ("realm_organization.col_org", id.clone()))
                    .collect(),
            ));
        }
        let mut rows = Vec::new();
        for control in &view.controls {
            if control.delegations.is_empty() {
                rows.push(control_row(theme.clone(), control, None));
            } else {
                for delegation in &control.delegations {
                    rows.push(control_row(theme.clone(), control, Some(delegation)));
                }
            }
        }
        controls.push(p::table_with_key(
            theme.clone(),
            "organization-controls",
            &[
                "realm_organization.col_org",
                "realm_organization.col_control_stream",
                "realm_organization.col_pcr_realm",
                "realm_organization.col_role",
                "realm_organization.col_delegation",
                "realm_organization.col_pcr",
                "realm_organization.col_executed_by",
                "realm_organization.col_expires",
                "realm_organization.col_delegation_status",
            ],
            rows,
            "realm_organization.control_empty",
        ));
        p::stack(vec![
            p::breadcrumb(
                theme.clone(),
                "nav.realms",
                &station::path("/realms", &realm_id),
                t("realm_organization.title"),
            ),
            ui::heading(t("realm_organization.title")),
            p::description("realm_organization.subtitle"),
            Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("realm_organization.verified_title"))
                    .description(t("realm_organization.verified_subtitle"))
                    .fill(verified),
            ),
            Box::new(
                sapid::card()
                    .theme(theme.clone())
                    .title(t("realm_organization.control_title"))
                    .description(t("realm_organization.control_subtitle"))
                    .fill(controls),
            ),
            operations(theme.clone()),
        ])
    })
}
fn control_row(
    theme: sapid::Theme,
    view: &OrganizationControlView,
    delegation: Option<&OrganizationDelegation>,
) -> Vec<View> {
    let control = &view.control;
    let executed = delegation
        .map(|v| v.created_by.to_string())
        .or_else(|| control.executed_by.as_ref().map(ToString::to_string))
        .unwrap_or_else(|| "-".into());
    vec![
        p::mono(&control.organization_id),
        p::mono(&control.control_stream_ref),
        p::mono(&control.principal_control_realm_id),
        p::plain(
            delegation
                .map(|v| issuer_role_label(v.issuer_role))
                .unwrap_or_else(|| "-".into()),
        ),
        p::stack(vec![
            p::mono(
                delegation
                    .map(|v| v.delegation_ref.clone())
                    .unwrap_or_else(|| "-".into()),
            ),
            p::plain(
                delegation
                    .map(|v| {
                        v.covered_control_scopes
                            .iter()
                            .map(|s| scope_label(*s))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default(),
            ),
        ]),
        p::stack(vec![
            p::plain(bootstrap_authorization_label(
                control.bootstrap_authorization,
            )),
            p::mono(p::optional(control.bootstrap_delegation_ref.as_ref())),
        ]),
        p::stack(vec![
            p::mono(&executed),
            p::plain(if executed != "-" {
                t("realm_organization.executor_note")
            } else {
                String::new()
            }),
        ]),
        p::plain(
            delegation
                .and_then(|v| v.valid_until)
                .map(|v| v.to_rfc3339())
                .unwrap_or_else(|| t("realm_organization.no_expiry")),
        ),
        delegation_badge(theme, delegation),
    ]
}
fn delegation_badge(theme: sapid::Theme, delegation: Option<&OrganizationDelegation>) -> View {
    let now = chrono::Utc::now();
    let (key, severity) = match delegation {
        None => ("realm_organization.delegation_none", "neutral"),
        Some(v) if v.status == OrganizationDelegationStatus::Revoked || v.revoked_at.is_some() => {
            ("realm_organization.delegation_revoked", "danger")
        }
        Some(v) if v.valid_until.is_some_and(|until| until <= now) => {
            ("realm_organization.delegation_expired", "danger")
        }
        Some(v)
            if v.valid_until
                .is_some_and(|until| until <= now + chrono::Duration::days(7)) =>
        {
            ("realm_organization.delegation_expiring", "warning")
        }
        Some(_) => ("realm_organization.delegation_live", "success"),
    };
    p::badge(theme, t(key), severity)
}
fn operations(theme: sapid::Theme) -> View {
    Box::new(
        sapid::card()
            .theme(theme.clone())
            .title(t("realm_organization.ops_title"))
            .description(t("realm_organization.ops_subtitle"))
            .fill((
                view()
                    .class("organization-impact-notice")
                    .style(
                        Style {
                            background: Some(theme.canvas),
                            ..Style::new()
                        }
                        .display(Display::Flex)
                        .flex_direction(FlexDirection::Column)
                        .gap(4.0)
                        .padding(Edges {
                            top: 8.0,
                            right: 12.0,
                            bottom: 8.0,
                            left: 12.0,
                        })
                        .border_width(Edges::all(1.0))
                        .border_color(theme.border)
                        .border_style(glory::style::BorderStyle::Solid)
                        .radius(6.0),
                    )
                    .fill((
                        text()
                            .style(Style {
                                font_size: Some(14.0),
                                font_weight: Some(600.0),
                                ..Style::new()
                            })
                            .content(t("realm_organization.impact_title")),
                        sapid::text_list()
                            .theme(theme.clone())
                            .size(sapid::Size::Small)
                            .items(
                                [
                                    "realm_organization.impact_official_badge",
                                    "realm_organization.impact_governance_policy",
                                    "realm_organization.impact_directory_listing",
                                    "realm_organization.impact_durability_policy",
                                ]
                                .into_iter()
                                .map(|key| {
                                    sapid::text_list_item().fill(
                                        sapid::text()
                                            .theme(theme.clone())
                                            .font_size(12.0)
                                            .line_height(18.0)
                                            .content(t(key)),
                                    )
                                }),
                            ),
                    )),
                sapid::group().theme(theme.clone()).fill((
                    sapid::button()
                        .theme(theme.clone())
                        .enabled(false)
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("realm_organization.op_revoke_delegation")),
                    sapid::button()
                        .theme(theme.clone())
                        .enabled(false)
                        .variant(sapid::ButtonVariant::Secondary)
                        .label(t("realm_organization.op_renew_delegation")),
                    sapid::button()
                        .theme(theme)
                        .enabled(false)
                        .variant(sapid::ButtonVariant::Danger)
                        .label(t("realm_organization.op_revoke_relationship")),
                )),
                p::description("realm_organization.ops_pending_note"),
            )),
    )
}
fn bootstrap_authorization_label(value: OrganizationBootstrapAuthorization) -> String {
    match value {
        OrganizationBootstrapAuthorization::DidControllerProof => "did_controller_proof",
        OrganizationBootstrapAuthorization::DelegatedGovernance => "delegated_governance",
    }
    .to_string()
}

fn relationship_label(value: RealmOrganizationRelationship) -> String {
    let key = match value {
        RealmOrganizationRelationship::Owner => "realm_organization.rel_owner",
        RealmOrganizationRelationship::Governance => "realm_organization.rel_governance",
        RealmOrganizationRelationship::Sponsor => "realm_organization.rel_sponsor",
        RealmOrganizationRelationship::DirectoryCertifier => {
            "realm_organization.rel_directory_certifier"
        }
    };
    t(key)
}

fn issuer_role_label(value: RealmOrganizationIssuerRole) -> String {
    let key = match value {
        RealmOrganizationIssuerRole::Organization => "realm_organization.role_organization",
        RealmOrganizationIssuerRole::GovernanceService => {
            "realm_organization.role_governance_service"
        }
        RealmOrganizationIssuerRole::AccountAuthority => {
            "realm_organization.role_account_authority"
        }
        RealmOrganizationIssuerRole::ThresholdQuorum => "realm_organization.role_threshold_quorum",
    };
    t(key)
}

fn scope_label(value: RealmOrganizationControlScope) -> String {
    match value {
        RealmOrganizationControlScope::OfficialBadge => "official_badge",
        RealmOrganizationControlScope::RealmAdmin => "realm_admin",
        RealmOrganizationControlScope::RealmAuthority => "realm_authority",
        RealmOrganizationControlScope::ModerationPolicy => "moderation_policy",
        RealmOrganizationControlScope::RetentionPolicy => "retention_policy",
        RealmOrganizationControlScope::DirectoryListing => "directory_listing",
        RealmOrganizationControlScope::PlaintextVisibleService => "plaintext_visible_service",
    }
    .to_string()
}
