//! Realm organization relationship and principal-control surface.

use arkret_models_collaboration::governance::realm_governance::{
    RealmOrganizationLifecyclePhase, RealmOrganizationRelationshipList,
    RealmOrganizationRelationshipView,
};
use arkret_models_collaboration::{
    RealmOrganizationControlScope, RealmOrganizationIssuerRole, RealmOrganizationRelationship,
};
use coauth_admin_types::organization_admin::{
    OrganizationBootstrapAuthorization, OrganizationControlView, OrganizationDelegation,
    OrganizationDelegationStatus,
};
use dioxus::prelude::*;

use crate::api::realm_organization;
use crate::components::selection_required::{is_placeholder_resource_id, selection_required_state};
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::*;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::{BreadcrumbItem, Breadcrumbs, PageHeader};
use crate::components::ui::table::*;
use crate::router::Route;
use crate::utils::i18n::t;

#[component]
pub fn OrganizationPage(realm_id: String) -> Element {
    if is_placeholder_resource_id(&realm_id) {
        return selection_required_state("Realm");
    }

    let realm_id_for_load = realm_id.clone();
    let mut data = use_resource(move || {
        let id = realm_id_for_load.clone();
        async move { realm_organization::get_realm_organization_admin_view(&id).await }
    });

    let breadcrumbs = vec![
        BreadcrumbItem {
            label: t("nav.realms"),
            route: Some(Route::RealmList {}),
        },
        BreadcrumbItem {
            label: realm_id.clone(),
            route: Some(Route::RealmShow {
                realm_id: realm_id.clone(),
            }),
        },
        BreadcrumbItem {
            label: t("realm_organization.title"),
            route: None,
        },
    ];

    rsx! {
        div { class: "space-y-6",
            Breadcrumbs { items: breadcrumbs }
            PageHeader {
                title: t("realm_organization.title"),
                description: t("realm_organization.subtitle"),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| data.restart(),
                    {t("common.refresh")}
                }
            }

            match &*data.read() {
                Some(Ok(view)) => rsx! {
                    {verified_relationship_card(&view.relationships)}
                    {principal_control_card(&view.controls, &view.unavailable_control_ids)}
                },
                Some(Err(error)) => rsx! {
                    ErrorBanner { message: error.message.clone(), on_retry: move |_| data.restart() }
                },
                None => rsx! { PageSkeleton {} },
            }

            SecurityOperationsCard {}
        }
    }
}

fn verified_relationship_card(panel: &RealmOrganizationRelationshipList) -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { {t("realm_organization.verified_title")} }
                CardDescription { {t("realm_organization.verified_subtitle")} }
            }
            CardContent {
                div { class: "space-y-4",
                    if !panel.declared_organization_hint_ids.is_empty() {
                        div {
                            class: "rounded-md border border-amber-600/60 bg-amber-600/5 px-3 py-2 text-sm",
                            p { class: "font-semibold mb-1", {t("realm_organization.declared_only_title")} }
                            ul { class: "list-disc pl-5 space-y-0.5",
                                for organization_id in panel.declared_organization_hint_ids.iter() {
                                    li { key: "{organization_id}", class: "font-mono text-xs", "{organization_id}" }
                                }
                            }
                        }
                    }

                    Table {
                        TableHeader {
                            TableRow {
                                TableHead { {t("realm_organization.col_org")} }
                                TableHead { {t("realm_organization.col_relationship")} }
                                TableHead { {t("realm_organization.col_status")} }
                                TableHead { {t("realm_organization.col_scopes")} }
                                TableHead { {t("realm_organization.col_issued")} }
                                TableHead { {t("realm_organization.col_expires")} }
                                TableHead { {t("realm_organization.col_statement")} }
                            }
                        }
                        TableBody {
                            if panel.relationships.is_empty() {
                                TableRow {
                                    TableCell {
                                        class: "text-center text-muted-foreground py-6".to_string(),
                                        colspan: 99,
                                        {t("realm_organization.verified_empty")}
                                    }
                                }
                            } else {
                                for row in panel.relationships.iter() {
                                    {relationship_row(row)}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn relationship_row(row: &RealmOrganizationRelationshipView) -> Element {
    let scopes = row
        .control_scopes
        .iter()
        .map(|scope| scope_label(*scope))
        .collect::<Vec<_>>()
        .join(", ");
    let organization_id = row.organization_id.to_string();
    let issued_at = row.issued_at.to_rfc3339();
    let expires_at = row
        .expires_at
        .map(|timestamp| timestamp.to_rfc3339())
        .unwrap_or_else(|| t("realm_organization.no_expiry"));
    rsx! {
        TableRow {
            key: "{row.statement_id}",
            TableCell { class: "font-mono text-xs".to_string(), "{organization_id}" }
            TableCell { class: "text-xs".to_string(), {relationship_label(row.relationship)} }
            TableCell { {relationship_lifecycle_badge(row.lifecycle_phase)} }
            TableCell { class: "text-xs max-w-[260px]".to_string(), "{scopes}" }
            TableCell { class: "text-xs".to_string(), "{issued_at}" }
            TableCell { class: "text-xs".to_string(), "{expires_at}" }
            TableCell { class: "font-mono text-xs".to_string(),
                div { class: "flex flex-col",
                    span { "{row.statement_id}" }
                    span { class: "text-muted-foreground/80 text-[10px]",
                        {issuer_role_label(row.issuer_role)}
                    }
                }
            }
        }
    }
}

fn relationship_lifecycle_badge(lifecycle: RealmOrganizationLifecyclePhase) -> Element {
    let (variant, key) = match lifecycle {
        RealmOrganizationLifecyclePhase::VerifiedActive => {
            (BadgeVariant::Success, "realm_organization.lifecycle_active")
        }
        RealmOrganizationLifecyclePhase::RevokedOrExpired => (
            BadgeVariant::Destructive,
            "realm_organization.lifecycle_revoked_or_expired",
        ),
    };
    rsx! { Badge { variant, {t(key)} } }
}

fn principal_control_card(
    controls: &[OrganizationControlView],
    unavailable_control_ids: &[String],
) -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { {t("realm_organization.control_title")} }
                CardDescription { {t("realm_organization.control_subtitle")} }
            }
            CardContent {
                div { class: "space-y-4",
                    if !unavailable_control_ids.is_empty() {
                        div {
                            class: "rounded-md border border-muted-foreground/40 bg-muted/30 px-3 py-2 text-sm",
                            p { class: "font-semibold mb-1", {t("realm_organization.control_unavailable_title")} }
                            ul { class: "list-disc pl-5 space-y-0.5",
                                for organization_id in unavailable_control_ids.iter() {
                                    li { key: "{organization_id}", class: "font-mono text-xs", "{organization_id}" }
                                }
                            }
                        }
                    }
                    Table {
                        TableHeader {
                            TableRow {
                                TableHead { {t("realm_organization.col_org")} }
                                TableHead { {t("realm_organization.col_control_stream")} }
                                TableHead { {t("realm_organization.col_pcr_realm")} }
                                TableHead { {t("realm_organization.col_role")} }
                                TableHead { {t("realm_organization.col_delegation")} }
                                TableHead { {t("realm_organization.col_pcr")} }
                                TableHead { {t("realm_organization.col_executed_by")} }
                                TableHead { {t("realm_organization.col_expires")} }
                                TableHead { {t("realm_organization.col_delegation_status")} }
                            }
                        }
                        TableBody {
                            if controls.is_empty() {
                                TableRow {
                                    TableCell {
                                        class: "text-center text-muted-foreground py-6".to_string(),
                                        colspan: 99,
                                        {t("realm_organization.control_empty")}
                                    }
                                }
                            } else {
                                for view in controls.iter() {
                                    if view.delegations.is_empty() {
                                        {principal_control_row(view, None)}
                                    } else {
                                        for delegation in view.delegations.iter() {
                                            {principal_control_row(view, Some(delegation))}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn principal_control_row(
    view: &OrganizationControlView,
    delegation: Option<&OrganizationDelegation>,
) -> Element {
    let dash = "-".to_string();
    let control = &view.control;
    let row_key = delegation
        .map(|row| row.delegation_ref.clone())
        .unwrap_or_else(|| format!("{}:control", control.organization_id));
    let control_stream = control.control_stream_ref.clone();
    let issuer_role = delegation
        .map(|row| issuer_role_label(row.issuer_role))
        .unwrap_or_else(|| dash.clone());
    let delegation_ref = delegation
        .map(|row| row.delegation_ref.clone())
        .unwrap_or_else(|| dash.clone());
    let scopes = delegation
        .map(|row| {
            row.covered_control_scopes
                .iter()
                .map(|scope| scope_label(*scope))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let executed_by = delegation
        .map(|row| row.created_by.to_string())
        .or_else(|| control.executed_by.as_ref().map(ToString::to_string))
        .unwrap_or_else(|| dash.clone());
    let expires_at = delegation
        .and_then(|row| row.valid_until)
        .map(|timestamp| timestamp.to_rfc3339())
        .unwrap_or_else(|| t("realm_organization.no_expiry"));
    let bootstrap = bootstrap_authorization_label(control.bootstrap_authorization);

    rsx! {
        TableRow {
            key: "{row_key}",
            TableCell { class: "font-mono text-xs".to_string(), "{control.organization_id}" }
            TableCell { class: "font-mono text-xs".to_string(), "{control_stream}" }
            TableCell { class: "font-mono text-xs".to_string(), "{control.principal_control_realm_id}" }
            TableCell { class: "text-xs".to_string(), "{issuer_role}" }
            TableCell { class: "font-mono text-xs max-w-[220px] truncate".to_string(),
                div { class: "flex flex-col",
                    span { "{delegation_ref}" }
                    if !scopes.is_empty() {
                        span { class: "text-muted-foreground/80 text-[10px]", "{scopes}" }
                    }
                }
            }
            TableCell { class: "text-xs".to_string(),
                div { class: "flex flex-col",
                    span { "{bootstrap}" }
                    if let Some(reference) = control.bootstrap_delegation_ref.as_ref() {
                        span { class: "font-mono text-muted-foreground/80 text-[10px]", "{reference}" }
                    }
                }
            }
            TableCell { class: "font-mono text-xs".to_string(),
                div { class: "flex flex-col",
                    span { "{executed_by}" }
                    if executed_by != "-" {
                        span { class: "text-muted-foreground/80 text-[10px]",
                            {t("realm_organization.executor_note")}
                        }
                    }
                }
            }
            TableCell { class: "text-xs".to_string(), "{expires_at}" }
            TableCell { {delegation_badge(delegation)} }
        }
    }
}

fn delegation_badge(delegation: Option<&OrganizationDelegation>) -> Element {
    let Some(delegation) = delegation else {
        return rsx! { Badge { variant: BadgeVariant::Outline, {t("realm_organization.delegation_none")} } };
    };
    let now = chrono::Utc::now();
    let (variant, key) = if delegation.status == OrganizationDelegationStatus::Revoked
        || delegation.revoked_at.is_some()
    {
        (
            BadgeVariant::Destructive,
            "realm_organization.delegation_revoked",
        )
    } else if delegation.valid_until.is_some_and(|until| until <= now) {
        (
            BadgeVariant::Destructive,
            "realm_organization.delegation_expired",
        )
    } else if delegation
        .valid_until
        .is_some_and(|until| until <= now + chrono::Duration::days(7))
    {
        (
            BadgeVariant::Outline,
            "realm_organization.delegation_expiring",
        )
    } else {
        (BadgeVariant::Success, "realm_organization.delegation_live")
    };
    rsx! { Badge { variant, {t(key)} } }
}

#[component]
fn SecurityOperationsCard() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { {t("realm_organization.ops_title")} }
                CardDescription { {t("realm_organization.ops_subtitle")} }
            }
            CardContent {
                div { class: "space-y-4",
                    div {
                        class: "rounded-md border border-border bg-muted/30 px-3 py-2 text-sm",
                        p { class: "font-semibold mb-1", {t("realm_organization.impact_title")} }
                        ul { class: "list-disc pl-5 space-y-0.5 text-xs",
                            li { {t("realm_organization.impact_official_badge")} }
                            li { {t("realm_organization.impact_governance_policy")} }
                            li { {t("realm_organization.impact_directory_listing")} }
                            li { {t("realm_organization.impact_durability_policy")} }
                        }
                    }
                    div { class: "flex flex-wrap gap-2",
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: true,
                            {t("realm_organization.op_revoke_delegation")}
                        }
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: true,
                            {t("realm_organization.op_renew_delegation")}
                        }
                        Button {
                            variant: ButtonVariant::Destructive,
                            disabled: true,
                            {t("realm_organization.op_revoke_relationship")}
                        }
                    }
                    p { class: "text-xs text-muted-foreground",
                        {t("realm_organization.ops_pending_note")}
                    }
                }
            }
        }
    }
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
