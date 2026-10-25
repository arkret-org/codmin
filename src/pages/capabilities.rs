use glory::{Scope, Widget};

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::types::capabilities::CapabilitySummaryExt;
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    page_with_heading(
        scope,
        theme,
        t("capabilities.title"),
        t("capabilities.subtitle"),
    )
}

pub fn page_with_heading(
    scope: &Scope,
    theme: sapid::Theme,
    title: String,
    description: String,
) -> impl Widget + use<> {
    station::browse_with_description(
        scope,
        theme,
        title,
        description,
        vec![
            ("capabilities.filter_realm", ""),
            ("capabilities.filter_actor", ""),
            ("authz_caps.status", "all"),
        ],
        |q| async move {
            let filters = crate::api::capabilities::CapabilityFilters::parse(
                q.field(0),
                q.field(1),
                q.field(2),
            )
            .map_err(crate::utils::net::error::HttpError::message)?;
            let response =
                crate::api::capabilities::list_capabilities(q.cursor(), 25, &filters).await?;
            let rows = response.capabilities.clone();
            Ok(Document::default().section("Capabilities",&response.capabilities)?.next(response.next_cursor).view(move |_,theme|p::table(theme.clone(),&["capabilities.id","capabilities.filter_realm","capabilities.revision","capabilities.grantor_id","capabilities.grantee_id","capabilities.capability","capabilities.scope","capabilities.granted_at","capabilities.expires_at","authz_caps.status"],rows.iter().map(|row|{
 use arkret_models_collaboration::governance::grant_constraint::CapabilityGrantStatus;
 let (status,severity)=match row.value.status{CapabilityGrantStatus::Active=>("capabilities.active","success"),CapabilityGrantStatus::Revoked=>("capabilities.revoked","danger"),CapabilityGrantStatus::Relinquished=>("capabilities.relinquished","danger")};
 vec![p::plain(row.selector.grant_id.to_string()),p::mono(row.source_stream_ref.realm_id()),p::mono(arkret_canonical::canonical_json_string(&row.revision).unwrap_or_else(|_|"unavailable".into())),p::plain(row.issuer_display()),p::plain(row.subject_display()),p::plain(row.actions_display()),p::plain(row.resource_display()),p::plain(row.value.issued_at.to_rfc3339()),p::plain(row.expiry_display()),p::badge(theme.clone(),t(status),severity)]
 }).collect(),"capabilities.no_capabilities")))
        },
    )
}
