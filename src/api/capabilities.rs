//! Deployment-local operator review of the Station's accepted grant rows.
//! The operator's Station credential must pass admin.read and the deployment
//! allowlist. These rows never replace the standard effective-grants read used
//! for authorization or grant authoring; no Coauth mirror is consulted.

use arkret_wire::{ActorId, RealmId};
use coland_contracts::admin::{AdminCapabilityList, CapabilityGrantState};

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::utils::net::error::HttpError;

#[derive(Clone, Debug, Default)]
pub struct CapabilityFilters {
    pub realm_id: Option<RealmId>,
    pub subject_actor_id: Option<ActorId>,
    pub state: Option<CapabilityGrantState>,
}

impl CapabilityFilters {
    pub fn parse(realm: &str, actor: &str, state: &str) -> Result<Self, String> {
        let realm_id = (!realm.trim().is_empty())
            .then(|| {
                RealmId::new(realm.trim().to_owned())
                    .map_err(|error| format!("Invalid RealmId: {error}"))
            })
            .transpose()?;
        let subject_actor_id = (!actor.trim().is_empty())
            .then(|| {
                let actor: ActorId = serde_json::from_str(actor.trim()).map_err(|error| {
                    format!("A complete Account or Service ActorId is required: {error}")
                })?;
                actor
                    .validate()
                    .map_err(|error| format!("Invalid ActorId: {error}"))?;
                Ok::<_, String>(actor)
            })
            .transpose()?;
        let state = CapabilityGrantState::from_wire(state)
            .ok_or_else(|| "Unknown grant lifecycle filter".to_owned())?;
        Ok(Self {
            realm_id,
            subject_actor_id,
            state: Some(state),
        })
    }
}

pub(crate) fn query_params(
    filters: &CapabilityFilters,
    cursor: Option<&str>,
    limit: u64,
) -> Result<Vec<(String, String)>, String> {
    let mut params = vec![("limit".to_owned(), limit.max(1).to_string())];
    if let Some(cursor) = cursor.filter(|value| !value.is_empty()) {
        params.push(("cursor".to_owned(), cursor.to_owned()));
    }
    if let Some(realm) = &filters.realm_id {
        params.push(("filter[realm_id]".to_owned(), realm.to_string()));
    }
    if let Some(actor) = &filters.subject_actor_id {
        params.push((
            "filter[subject_actor_id]".to_owned(),
            arkret_canonical::canonical_json_string(actor).map_err(|error| error.to_string())?,
        ));
    }
    if let Some(state) = &filters.state {
        params.push(("filter[state]".to_owned(), state.as_str().to_owned()));
    }
    Ok(params)
}

pub(crate) fn verify_page(
    page: &AdminCapabilityList,
    filters: &CapabilityFilters,
) -> Result<(), String> {
    for (key, value) in query_params(filters, None, 25)? {
        if let Some(key) = key
            .strip_prefix("filter[")
            .and_then(|key| key.strip_suffix(']'))
            && page.filters.get(key) != Some(&value)
        {
            return Err(format!(
                "Station did not confirm the requested {key} filter"
            ));
        }
    }
    if page.has_more
        != page
            .next_cursor
            .as_ref()
            .is_some_and(|cursor| !cursor.is_empty())
    {
        return Err("Station returned inconsistent pagination".to_owned());
    }
    for row in &page.capabilities {
        if row.selector.grant_id != row.value.id
            || row
                .value
                .realm_id
                .as_ref()
                .is_some_and(|realm| realm != row.source_stream_ref.realm_id())
        {
            return Err("Station returned an inconsistent accepted grant identity".to_owned());
        }
        if filters
            .realm_id
            .as_ref()
            .is_some_and(|realm| realm != row.source_stream_ref.realm_id())
        {
            return Err("Station returned a grant from another Realm".to_owned());
        }
        if let Some(state) = filters.state {
            use arkret_models_collaboration::governance::grant_constraint::CapabilityGrantStatus;
            let matches = match state {
                CapabilityGrantState::All => true,
                CapabilityGrantState::Active => row.value.status == CapabilityGrantStatus::Active,
                CapabilityGrantState::Revoked => row.value.status == CapabilityGrantStatus::Revoked,
                CapabilityGrantState::Relinquished => {
                    row.value.status == CapabilityGrantStatus::Relinquished
                }
                _ => false,
            };
            if !matches {
                return Err("Station returned a grant outside the requested lifecycle".to_owned());
            }
        }
        if let Some(actor) = &filters.subject_actor_id
            && !matches!(&row.value.subject, arkret_models_collaboration::governance::grant_constraint::CapabilitySubject::Actor(subject) if subject==actor)
        {
            return Err("Station returned a grant for another complete Actor".to_owned());
        }
    }
    Ok(())
}

pub async fn list_capabilities(
    cursor: Option<&str>,
    limit: u64,
    filters: &CapabilityFilters,
) -> Result<AdminCapabilityList, HttpError> {
    let params = query_params(filters, cursor, limit).map_err(|message| HttpError {
        message,
        status: 400,
        body: None,
        request_id: None,
        retry_after_ms: None,
    })?;
    let borrowed: Vec<_> = params
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let url = build_url("/_coland/admin/capabilities", &borrowed)?;
    let page: AdminCapabilityList = api_client(&url, "GET", NO_BODY).await?;
    verify_page(&page, filters).map_err(|message| HttpError {
        message,
        status: 502,
        body: None,
        request_id: None,
        retry_after_ms: None,
    })?;
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_keep_full_accounts_and_service_branches() {
        for actor in [
            r#"{"kind":"account","account_id":{"principal_id":"ak:did_core:web:person.example","station_id":"ak:did_core:web:one.example"}}"#,
            r#"{"kind":"account","account_id":{"principal_id":"ak:did_core:web:person.example","station_id":"ak:did_core:web:two.example"}}"#,
            r#"{"kind":"service","service_id":"ak:did_core:web:service.example"}"#,
        ] {
            let filters = CapabilityFilters::parse("", actor, "all").unwrap();
            let params = query_params(&filters, Some("grant-cursor"), 25).unwrap();
            let (_, encoded) = params
                .iter()
                .find(|(key, _)| key == "filter[subject_actor_id]")
                .unwrap();
            assert_eq!(
                serde_json::from_str::<ActorId>(encoded).unwrap(),
                filters.subject_actor_id.unwrap()
            );
            assert!(!params.iter().any(|(key, _)| key == "filter[subject]"));
        }
    }

    #[test]
    fn filters_reject_principal_only_and_incomplete_account_identity() {
        for actor in [
            "ak:did_core:web:person.example",
            r#"{"kind":"account","account_id":{"principal_id":"ak:did_core:web:person.example"}}"#,
        ] {
            assert!(CapabilityFilters::parse("", actor, "all").is_err());
        }
        assert!(CapabilityFilters::parse("not-a-realm", "", "all").is_err());
        assert!(CapabilityFilters::parse("", "", "paused").is_err());
    }

    #[test]
    fn accepted_empty_page_requires_explicit_rows_and_applied_filter_echo() {
        let value = serde_json::json!({"total":0,"has_more":false,"filters":{}});
        assert!(serde_json::from_value::<AdminCapabilityList>(value).is_err());
        let page: AdminCapabilityList = serde_json::from_value(
            serde_json::json!({"capabilities":[],"total":0,"has_more":false,"filters":{}}),
        )
        .unwrap();
        assert!(verify_page(&page, &CapabilityFilters::default()).is_ok());
        assert!(verify_page(&page, &CapabilityFilters::parse("", "", "active").unwrap()).is_err());
    }
}
