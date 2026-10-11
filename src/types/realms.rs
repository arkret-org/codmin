//! Realm admin surface — contract-authoritative types plus thin display
//! helpers.
//!
//! The row is the shared `coland_contracts::admin::AdminRealmItem`; codmin
//! keeps no wire mirror. The object carrying encryption / join-rule /
//! history-access / realm-class boundary fields is a Realm. Space
//! containers are represented separately by [`crate::types::spaces::AdminSpaceRow`].

pub use coland_contracts::admin::{AdminRealmItem, RealmClass};
use serde::Serialize;

/// Display helpers for the shared [`AdminRealmItem`].
pub trait AdminRealmItemExt {
    fn discoverability_label(&self) -> Option<String>;
    fn join_rule_label(&self) -> Option<String>;
    /// Wire class string, `-` when the server reported no recognised class.
    fn type_label(&self) -> &'static str;
    /// RFC3339 rendering of the optional creation timestamp.
    fn created_at_display(&self) -> Option<String>;
}

impl AdminRealmItemExt for AdminRealmItem {
    fn discoverability_label(&self) -> Option<String> {
        self.discoverability.as_ref().map(wire_label)
    }

    fn join_rule_label(&self) -> Option<String> {
        self.default_join_rule.as_ref().map(wire_label)
    }

    fn type_label(&self) -> &'static str {
        self.realm_class.map(RealmClass::as_str).unwrap_or("-")
    }

    fn created_at_display(&self) -> Option<String> {
        self.created_at.map(|ts| ts.to_rfc3339())
    }
}

fn wire_label<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "-".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realm_class_and_discoverability_render_from_the_shared_row() {
        let realm: AdminRealmItem = serde_json::from_value(serde_json::json!({
            "kind": "realm",
            "id": "ak:realm:AUwZORI60DxE1STM676ap_m1sIzBcRRCjoewWDPyAjx-",
            "strand": {},
            "strand_id": "ak:strand:AbZyWjHz86aXMxPVsi5SUW9QgFqny2SpKW_jkeUm61xz",
            "realm_id": "ak:realm:AUwZORI60DxE1STM676ap_m1sIzBcRRCjoewWDPyAjx-",
            "title": "Realm",
            "topic": null,
            "category": null,
            "realm_class": "principal_control",
            "discoverability": "invite_only",
            "default_join_rule": "invite",
            "tags": [],
            "public": false,
            "member_count": 2,
            "members": [],
            "created_by": null,
            "history_access": "since_join",
            "is_encrypted": true,
            "is_blocked": false,
            "plaintext_visible_services": [],
            "deleted": false,
            "created_at": "2026-08-14T00:00:00.000Z",
            "updated_at": null,
        }))
        .expect("shared realm row should deserialize");

        assert_eq!(realm.type_label(), "principal_control");
        assert_eq!(
            realm.discoverability_label().as_deref(),
            Some("invite_only")
        );
        assert!(realm.created_at_display().is_some());
    }

    #[test]
    fn unknown_class_renders_as_dash_rather_than_guessing() {
        let realm = AdminRealmItem {
            kind: "realm".to_owned(),
            id: "ak:realm:AWASxhHncyxoEbHwACOwwtlRkQ3MT2gAAiT3ZBg5sMbb".to_owned(),
            strand: serde_json::json!({}),
            strand_id: "ak:strand:AYzCUBVUFQVBJnnZypYi4P4yj_gqjnlbjCrh1wq0Kx6Z".to_owned(),
            realm_id: "ak:realm:AWASxhHncyxoEbHwACOwwtlRkQ3MT2gAAiT3ZBg5sMbb".to_owned(),
            title: "Realm".to_owned(),
            topic: None,
            category: None,
            realm_class: None,
            discoverability: None,
            default_join_rule: None,
            tags: Vec::new(),
            public: false,
            member_count: 0,
            members: Vec::new(),
            created_by: None,
            history_access: None,
            is_encrypted: false,
            is_blocked: false,
            plaintext_visible_services: Vec::new(),
            deleted: false,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(realm.type_label(), "-");
        assert!(realm.discoverability_label().is_none());
        assert!(realm.created_at_display().is_none());
    }
}
