//! coauth admin OAuth2 sessions and personal sessions.

use serde::Serialize;

use super::pagination::{get_jsonapi_cursor_page, map_single_resource};
use crate::api::client::{NO_BODY, api_client};
use crate::types::CursorPage;
use crate::utils::net::error::HttpError;

const OAUTH2_SESSIONS_PATH: &str = "/_coauth/admin/oauth-sessions";
const PERSONAL_SESSIONS_PATH: &str = "/_coauth/admin/personal-sessions";

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CoauthOAuth2Session {
    pub id: String,
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub scope: Option<String>,
    pub human_name: Option<String>,
    pub created_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CoauthPersonalSessionView {
    pub id: String,
    pub user_id: Option<String>,
    pub name: Option<String>,
    pub created_at: Option<String>,
    pub last_active_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CoauthPersonalSessionOneShot {
    #[serde(flatten)]
    pub session: CoauthPersonalSessionView,
    pub access_token: Option<String>,
}

pub async fn list_oauth2_sessions(
    after: Option<&str>,
    per_page: u64,
) -> Result<CursorPage<CoauthOAuth2Session>, HttpError> {
    get_jsonapi_cursor_page(OAUTH2_SESSIONS_PATH, after, per_page, map_oauth2_session).await
}

pub async fn finish_oauth2_session(id: &str) -> Result<(), HttpError> {
    let url = format!(
        "/_coauth/admin/oauth-sessions/{}/finish",
        urlencoding::encode(id)
    );
    api_client(&url, "POST", NO_BODY).await
}

pub async fn list_personal_sessions(
    after: Option<&str>,
    per_page: u64,
) -> Result<CursorPage<CoauthPersonalSessionView>, HttpError> {
    get_jsonapi_cursor_page(
        PERSONAL_SESSIONS_PATH,
        after,
        per_page,
        map_personal_session,
    )
    .await
}

pub async fn create_personal_session(
    name: &str,
) -> Result<CoauthPersonalSessionOneShot, HttpError> {
    let body = serde_json::json!({ "name": name });
    let resp: coauth_admin_types::SingleOutcome<coauth_admin_types::PersonalSession> =
        api_client(PERSONAL_SESSIONS_PATH, "POST", Some(&body)).await?;
    Ok(map_single_resource(resp, map_personal_session_oneshot))
}

pub async fn revoke_personal_session(id: &str) -> Result<(), HttpError> {
    let url = format!(
        "/_coauth/admin/personal-sessions/{}/revoke",
        urlencoding::encode(id)
    );
    api_client(&url, "POST", NO_BODY).await
}

pub async fn regenerate_personal_session(
    id: &str,
) -> Result<CoauthPersonalSessionOneShot, HttpError> {
    let url = format!(
        "/_coauth/admin/personal-sessions/{}/regenerate",
        urlencoding::encode(id)
    );
    let resp: coauth_admin_types::SingleOutcome<coauth_admin_types::PersonalSession> =
        api_client(&url, "POST", NO_BODY).await?;
    Ok(map_single_resource(resp, map_personal_session_oneshot))
}

fn map_oauth2_session(
    resource: coauth_admin_types::SingleResource<coauth_admin_types::OAuthSession>,
) -> CoauthOAuth2Session {
    let attrs = resource.attributes;
    CoauthOAuth2Session {
        id: resource.id,
        user_id: attrs.user_id,
        client_id: Some(attrs.client_id).filter(|value| !value.is_empty()),
        scope: Some(attrs.scope).filter(|value| !value.is_empty()),
        human_name: attrs.human_name,
        created_at: Some(attrs.created_at.to_rfc3339()),
        finished_at: attrs.finished_at.map(|value| value.to_rfc3339()),
    }
}

fn map_personal_session(
    resource: coauth_admin_types::SingleResource<coauth_admin_types::PersonalSession>,
) -> CoauthPersonalSessionView {
    let attrs = resource.attributes;
    CoauthPersonalSessionView {
        id: resource.id,
        user_id: attrs
            .actor_user_id
            .is_empty()
            .then(|| attrs.owner_user_id.clone())
            .flatten()
            .or_else(|| Some(attrs.actor_user_id).filter(|value| !value.is_empty())),
        name: Some(attrs.human_name).filter(|value| !value.is_empty()),
        created_at: Some(attrs.created_at.to_rfc3339()),
        last_active_at: attrs.last_active_at.map(|value| value.to_rfc3339()),
    }
}

fn map_personal_session_oneshot(
    resource: coauth_admin_types::SingleResource<coauth_admin_types::PersonalSession>,
) -> CoauthPersonalSessionOneShot {
    let access_token = resource.attributes.access_token.clone();
    CoauthPersonalSessionOneShot {
        session: map_personal_session(resource),
        access_token,
    }
}

#[cfg(test)]
mod tests {
    use super::{CoauthPersonalSessionOneShot, CoauthPersonalSessionView};

    #[test]
    fn personal_session_row_drops_token_fields() {
        let row = CoauthPersonalSessionView {
            id: "session-1".to_owned(),
            user_id: Some("user-1".to_owned()),
            name: Some("ops key".to_owned()),
            created_at: Some("2026-06-07T00:00:00.000Z".to_owned()),
            last_active_at: None,
        };

        assert_eq!(row.id, "session-1");
        assert_eq!(row.user_id.as_deref(), Some("user-1"));
        assert_eq!(row.name.as_deref(), Some("ops key"));

        let serialized = serde_json::to_value(&row).expect("row serializes");
        assert!(serialized.get("token").is_none());
        assert!(serialized.get("access_token").is_none());
    }

    #[test]
    fn personal_session_one_shot_uses_current_access_token_field() {
        let response = CoauthPersonalSessionOneShot {
            session: CoauthPersonalSessionView {
                id: "session-1".to_owned(),
                user_id: None,
                name: Some("ops key".to_owned()),
                created_at: None,
                last_active_at: None,
            },
            access_token: Some("secret-access-token".to_owned()),
        };

        assert_eq!(response.session.name.as_deref(), Some("ops key"));
        assert_eq!(
            response.access_token.as_deref(),
            Some("secret-access-token")
        );
    }
}
