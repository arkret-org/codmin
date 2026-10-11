use arkret_wire::event_kind_str;
use coland_contracts::admin::policy::{
    AdminPolicyDocument, AdminPolicyDocumentPage, AdminPolicyPayload, PolicyEffect,
    UpsertPolicyDocumentRequestBody,
};
use serde_json::{Value, json};

use crate::api::client::{NO_BODY, NoBody, api_client, build_url};
use crate::types::policy::{
    AdminPolicy, AdminPolicyListOutcome, CreatePolicyRequest, PolicyGuardrailSummary,
    PolicyObligation, PolicyRuleSet, PolicySafetySummary,
};
use crate::utils::net::error::HttpError;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolicyListFilter {
    pub scope: String,
    pub subject_ref: String,
    pub include_inactive: bool,
}

impl PolicyListFilter {
    fn into_query(self) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if !self.scope.trim().is_empty() {
            query.push(("scope", self.scope.trim().to_owned()));
        }
        if !self.subject_ref.trim().is_empty() {
            query.push(("subject_ref", self.subject_ref.trim().to_owned()));
        }
        if self.include_inactive {
            query.push(("include_inactive", "true".to_owned()));
        }
        query
    }
}

pub async fn list_policies(filter: PolicyListFilter) -> Result<AdminPolicyListOutcome, HttpError> {
    let owned_query = filter.into_query();
    let query = owned_query
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect::<Vec<_>>();
    let url = build_url("/_coland/self/policies", &query)?;
    let resp: AdminPolicyDocumentPage = api_client(&url, "GET", NO_BODY).await?;
    Ok(AdminPolicyListOutcome {
        data: resp
            .policies
            .into_iter()
            .map(policy_from_document)
            .collect(),
        total: None,
        next_cursor: resp.next_cursor,
    })
}

pub async fn create_policy(req: &CreatePolicyRequest) -> Result<AdminPolicy, HttpError> {
    if request_targets_pin_policy(req) {
        return Err(pin_policy_unavailable_error());
    }
    require_operator_policy_request(req)?;
    let body = upsert_body(None, req)?;
    let resp: AdminPolicyDocument =
        api_client("/_coland/self/policies", "POST", Some(&body)).await?;
    Ok(policy_from_document(resp))
}

pub async fn update_policy(id: &str, req: &CreatePolicyRequest) -> Result<AdminPolicy, HttpError> {
    if request_targets_pin_policy(req) {
        return Err(pin_policy_unavailable_error());
    }
    require_operator_policy_request(req)?;
    let body = upsert_body(Some(id.to_string()), req)?;
    let resp: AdminPolicyDocument =
        api_client("/_coland/self/policies", "POST", Some(&body)).await?;
    Ok(policy_from_document(resp))
}

pub async fn delete_policy(id: &str) -> Result<(), HttpError> {
    let url = format!("/_coland/self/policies/{}", urlencoding::encode(id));
    let _: NoBody = api_client(&url, "DELETE", NO_BODY).await?;
    Ok(())
}

fn upsert_body(
    policy_id: Option<String>,
    req: &CreatePolicyRequest,
) -> Result<UpsertPolicyDocumentRequestBody, HttpError> {
    let (effect, actions, resource, obligations) = payload_parts(req)?;
    Ok(UpsertPolicyDocumentRequestBody {
        policy_id,
        scope: req
            .scope
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "*".to_string()),
        subject_ref: req
            .subject_ref
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "*".to_string()),
        policy_kind: req
            .policy_kind
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "*".to_string()),
        effect,
        actions,
        resource,
        obligations,
        active: req.is_enabled,
    })
}

fn payload_parts(
    req: &CreatePolicyRequest,
) -> Result<(PolicyEffect, Vec<String>, Value, Vec<Value>), HttpError> {
    let rules = req.rules.clone().unwrap_or_default();
    let rules_value = rules.as_value();
    let effect = match rules_value.get("effect").and_then(Value::as_str) {
        // The wire spelling lives in the SDK type's serde impl (spec
        // four-value `policy_effect` closed set); decision-only values such
        // as `soft_deny` / `hard_deny` are rejected here.
        Some(value) => serde_json::from_value::<PolicyEffect>(Value::String(value.to_owned()))
            .map_err(|_| {
                HttpError::message(format!("unsupported policy effect from server: {value}"))
            })?,
        None => PolicyEffect::Allow,
    };
    let actions = rules_value
        .get("actions")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| vec!["*".to_string()]);
    let obligations = rules_value
        .get("obligations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let resource = json!({
        "name": req.name,
        "description": req.description,
        "priority": req.priority,
        "rules": rules.into_value(),
    });
    Ok((effect, actions, resource, obligations))
}

fn policy_from_document(doc: AdminPolicyDocument) -> AdminPolicy {
    let guardrails = policy_guardrails_from_payload(&doc.payload);
    let safety = policy_safety_from_document(&doc);
    // `resource` is permanently opaque operator data.  The console projects
    // only the typed payload members and never displays or interprets resource
    // keys (including keys which resemble evidence or audit records).
    let rules = Some(PolicyRuleSet::from(json!({
        "effect": doc.payload.effect,
        "actions": doc.payload.actions,
        "obligations": doc.payload.obligations,
    })));
    AdminPolicy {
        name: doc.policy_id.clone(),
        id: doc.policy_id,
        policy_kind: Some(doc.policy_kind).filter(|s| !s.is_empty()),
        description: None,
        scope: Some(doc.scope).filter(|s| !s.is_empty()),
        subject_ref: Some(doc.subject_ref).filter(|s| !s.is_empty()),
        rules,
        guardrails,
        is_enabled: doc.active,
        priority: 0,
        created_at: None,
        updated_at: Some(arkret_canonical::format_timestamp_canonical(doc.updated_at)),
        safety,
    }
}

pub fn request_targets_pin_policy(req: &CreatePolicyRequest) -> bool {
    req.policy_kind
        .as_deref()
        .is_some_and(text_targets_pin_policy)
        || req
            .rules
            .as_ref()
            .and_then(|rules| rules.as_value().get("actions"))
            .and_then(Value::as_array)
            .is_some_and(|actions| {
                actions
                    .iter()
                    .filter_map(Value::as_str)
                    .any(text_targets_pin_policy)
            })
}

fn pin_policy_unavailable_error() -> HttpError {
    HttpError::message(
        "pin policy editing is unavailable until coland exposes a standard pin policy summary API",
    )
}

fn management_policy_kind(value: &str) -> bool {
    serde_json::from_value::<arkret_wire::PolicyKind>(Value::String(value.to_owned())).is_ok_and(
        |kind| {
            matches!(
                kind,
                arkret_wire::PolicyKind::Agent | arkret_wire::PolicyKind::Applet
            )
        },
    )
}

fn require_operator_policy_request(req: &CreatePolicyRequest) -> Result<(), HttpError> {
    if req
        .policy_kind
        .as_deref()
        .is_some_and(management_policy_kind)
    {
        return Err(HttpError::message(
            "Realm Agent/Applet management permissions are unavailable in the server operator policy editor",
        ));
    }
    Ok(())
}

fn policy_safety_from_document(doc: &AdminPolicyDocument) -> PolicySafetySummary {
    if management_policy_kind(&doc.policy_kind) {
        return PolicySafetySummary {
            read_only: true,
            read_only_reason: Some(
                "Realm management permissions cannot be edited as server operator policies"
                    .to_owned(),
            ),
        };
    }
    if !policy_document_targets_pin(doc) {
        return PolicySafetySummary::default();
    }
    PolicySafetySummary {
        read_only: true,
        read_only_reason: Some(
            "pin policy summary standard surface unavailable; opaque resource is not inspected"
                .to_owned(),
        ),
    }
}

fn policy_document_targets_pin(doc: &AdminPolicyDocument) -> bool {
    text_targets_pin_policy(&doc.policy_kind)
        || text_targets_pin_policy(&doc.policy_id)
        || doc
            .payload
            .actions
            .iter()
            .any(|action| text_targets_pin_policy(action))
}

fn text_targets_pin_policy(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value == "ak.pin"
        || is_pin_action_or_namespace_pattern(&value)
        || value.contains(arkret_wire::ProfileId::PINNED_ITEMS_V1)
        || value.contains("pinned_items")
        || value.contains("pin_policy")
        || value.contains("pin.policy")
        || value.contains("pin.quota")
        || value.contains("pin-quota")
        || value.contains("pin_quota")
        || value.contains("pin policy")
}

fn pin_action_prefix() -> &'static str {
    event_kind_str::PIN_ADD
        .strip_suffix("add")
        .expect("generated pin-add kind ends in add")
}

fn is_pin_action_or_namespace_pattern(value: &str) -> bool {
    matches!(
        value,
        event_kind_str::PIN_ADD | event_kind_str::PIN_REMOVE | event_kind_str::PIN_REORDER
    ) || value
        .strip_prefix(pin_action_prefix())
        .is_some_and(|suffix| suffix == "*" || !suffix.is_empty())
}

fn policy_guardrails_from_payload(payload: &AdminPolicyPayload) -> PolicyGuardrailSummary {
    PolicyGuardrailSummary {
        obligations: payload
            .obligations
            .iter()
            .cloned()
            .map(PolicyObligation::from)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use serde_json::json;

    use super::*;

    #[test]
    fn policy_list_filter_uses_supported_backend_parameters() {
        let query = PolicyListFilter {
            scope: " realm-a ".to_owned(),
            subject_ref: "did:web:alice.example".to_owned(),
            include_inactive: true,
        }
        .into_query();

        assert_eq!(
            query,
            vec![
                ("scope", "realm-a".to_owned()),
                ("subject_ref", "did:web:alice.example".to_owned()),
                ("include_inactive", "true".to_owned()),
            ]
        );
    }

    fn fixture_timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-14T00:00:00Z")
            .expect("fixture timestamp")
            .with_timezone(&Utc)
    }

    #[test]
    fn policy_guardrails_only_use_typed_top_level_obligations() {
        let payload = AdminPolicyPayload {
            effect: PolicyEffect::RequireReview,
            actions: vec!["ak.realm.policy.update".to_owned()],
            resource: json!({"opaque_operator_data": {"label": "not a typed guardrail"}}),
            obligations: vec![json!({"kind": "approval_required"})],
        };

        let guardrails = policy_guardrails_from_payload(&payload);
        assert_eq!(guardrails.obligations.len(), 1);
    }

    #[test]
    fn policy_from_document_never_interprets_or_displays_opaque_resource() {
        let doc = AdminPolicyDocument {
            policy_id: "ak:policy:9f18274d-cb85-75eb-9891-eff82acff4c0".to_owned(),
            owner: "did:web:admin.example".to_owned(),
            scope: "ak:realm:ASrEhTMQSRXph9wD98UfShBx6MNM7ISQiBIDT0Ayiozq".to_owned(),
            subject_ref: "did:web:admin.example".to_owned(),
            policy_kind: "ak.realm.policy.update".to_owned(),
            payload: AdminPolicyPayload {
                effect: PolicyEffect::Allow,
                actions: Vec::new(),
                resource: json!({
                    "name": "Realm policy",
                    "priority": 3,
                    "approval_evidence": {"secret": "must-not-surface"},
                    "audit_trail": [{"actor": "must-not-surface"}]
                }),
                obligations: Vec::new(),
            },
            active: true,
            updated_at: fixture_timestamp(),
        };

        let policy = policy_from_document(doc);
        assert_eq!(
            policy.name,
            "ak:policy:9f18274d-cb85-75eb-9891-eff82acff4c0"
        );
        assert_eq!(policy.priority, 0);
        assert_eq!(policy.description, None);
        assert_eq!(policy.subject_ref.as_deref(), Some("did:web:admin.example"));
        assert!(policy.guardrails.obligations.is_empty());
        let displayed = serde_json::to_string(&policy.rules).unwrap();
        assert!(!displayed.contains("Realm policy"));
        assert!(!displayed.contains("approval_evidence"));
        assert!(!displayed.contains("audit_trail"));
        assert!(!displayed.contains("must-not-surface"));
    }

    #[test]
    fn operator_store_cannot_author_realm_management_policies() {
        for kind in ["agent", "applet"] {
            let req = CreatePolicyRequest {
                policy_kind: Some(kind.to_owned()),
                ..Default::default()
            };
            assert!(require_operator_policy_request(&req).is_err());
        }
        let ordinary = CreatePolicyRequest {
            policy_kind: Some("access".to_owned()),
            ..Default::default()
        };
        assert!(require_operator_policy_request(&ordinary).is_ok());
    }

    #[test]
    fn management_operator_document_is_read_only() {
        let doc = AdminPolicyDocument {
            policy_id: "operator-rule".to_owned(),
            owner: "operator".to_owned(),
            scope: "*".to_owned(),
            subject_ref: "*".to_owned(),
            policy_kind: "applet".to_owned(),
            payload: AdminPolicyPayload {
                effect: PolicyEffect::Allow,
                actions: Vec::new(),
                resource: json!({"schema": "ak.schema.policy.v1"}),
                obligations: Vec::new(),
            },
            active: true,
            updated_at: fixture_timestamp(),
        };
        assert!(policy_from_document(doc).safety.read_only);
    }

    #[test]
    fn upsert_body_preserves_subject_ref_and_deny_effect() {
        let req = CreatePolicyRequest {
            name: "Targeted deny".to_owned(),
            scope: Some("ak:realm:AcTCbPKkRYcSVLkuEFQjnPuFXfCsFvSRZDAjCsho3_b-".to_owned()),
            subject_ref: Some("did:web:bob.example".to_owned()),
            policy_kind: Some("ak.message.send".to_owned()),
            rules: Some(
                json!({
                    "effect": "deny",
                    "actions": ["ak.message.send"]
                })
                .into(),
            ),
            is_enabled: true,
            ..Default::default()
        };

        let body = upsert_body(
            Some("ak:policy:f54a950e-b8e5-70af-92bd-4f27a495b754".to_owned()),
            &req,
        )
        .unwrap();
        let encoded = serde_json::to_value(body).unwrap();

        assert_eq!(encoded["subject_ref"], "did:web:bob.example");
        assert_eq!(encoded["effect"], "deny");
        assert_eq!(encoded["actions"], json!(["ak.message.send"]));
    }

    #[test]
    fn upsert_body_rejects_unknown_policy_effect() {
        // `soft_deny` is a Policy Server decision value, not a rule effect —
        // the spec `policy_effect` closed set rejects it.
        let req = CreatePolicyRequest {
            name: "Bad effect".to_owned(),
            rules: Some(json!({"effect": "soft_deny"}).into()),
            ..Default::default()
        };

        assert!(upsert_body(None, &req).is_err());
    }

    #[test]
    fn pin_policy_document_is_read_only_without_inspecting_resource() {
        let doc = AdminPolicyDocument {
            policy_id: "ak:policy:577e618f-3aed-75d9-9cb2-8fc1199b163a".to_owned(),
            owner: "did:web:admin.example".to_owned(),
            scope: "ak:realm:AcjPhXNC5Rr73hdU3Lep53Z0K69AczAn915GJwCtexkF".to_owned(),
            subject_ref: "*".to_owned(),
            policy_kind: "ak.profile.pinned_items.v1".to_owned(),
            payload: AdminPolicyPayload {
                effect: PolicyEffect::Allow,
                actions: vec![
                    event_kind_str::PIN_ADD.to_owned(),
                    event_kind_str::PIN_REORDER.to_owned(),
                ],
                resource: json!({
                    "name": "Realm pins",
                    "pin_scope": {"kind": "realm", "id": "ak:realm:AcjPhXNC5Rr73hdU3Lep53Z0K69AczAn915GJwCtexkF"},
                    "quota": {"max_pins_per_scope": 5, "period": "PT1H"},
                    "note_visibility": "encrypted",
                    "account_data_key": "ak.search.index_manifest.v1:secret-realm-key",
                    "search_index": {"shard_key": "super-secret-token"}
                }),
                obligations: Vec::new(),
            },
            active: true,
            updated_at: fixture_timestamp(),
        };

        let policy = policy_from_document(doc);
        assert!(policy.safety.read_only);
        assert_eq!(
            policy.safety.read_only_reason.as_deref(),
            Some(
                "pin policy summary standard surface unavailable; opaque resource is not inspected"
            )
        );

        let public_rules = serde_json::to_string(&policy.rules.unwrap()).unwrap();
        assert!(!public_rules.contains("secret-realm-key"));
        assert!(!public_rules.contains("super-secret-token"));
        assert!(!public_rules.contains("max_pins_per_scope"));
        assert!(!public_rules.contains("note_visibility"));
    }

    #[test]
    fn pin_policy_mutation_requests_fail_closed_before_network() {
        let pin_by_type = CreatePolicyRequest {
            policy_kind: Some(event_kind_str::PIN_ADD.to_owned()),
            ..Default::default()
        };
        assert!(request_targets_pin_policy(&pin_by_type));

        let pin_by_typed_actions = CreatePolicyRequest {
            rules: Some(json!({"actions": [event_kind_str::PIN_REORDER]}).into()),
            ..Default::default()
        };
        assert!(request_targets_pin_policy(&pin_by_typed_actions));

        let ordinary = CreatePolicyRequest {
            policy_kind: Some("ak.message.create".to_owned()),
            rules: Some(json!({"actions": ["ak.message.create"]}).into()),
            ..Default::default()
        };
        assert!(!request_targets_pin_policy(&ordinary));

        let ordinary_with_free_text = CreatePolicyRequest {
            policy_kind: Some("ak.message.create".to_owned()),
            rules: Some(
                json!({"resource": {"description": "mentions pin policy in prose"}}).into(),
            ),
            ..Default::default()
        };
        assert!(!request_targets_pin_policy(&ordinary_with_free_text));

        let resource_lookalike = CreatePolicyRequest {
            policy_kind: Some("ak.message.create".to_owned()),
            rules: Some(json!({"resource": {"pin_policy": {"max_pins": 1}}}).into()),
            ..Default::default()
        };
        assert!(!request_targets_pin_policy(&resource_lookalike));
    }
}
