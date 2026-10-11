//! Visual badge that distinguishes "Principal Control" Realms
//! (those that back DID issuance and recovery) from
//! ordinary "Collaboration" Realms.
//!
//! Per AKP-0007 the classification is set at create time and is
//! immutable afterwards. Most Realm surfaces in codmin (member panels,
//! Realm links, audit drilldown) should render this badge
//! so operators can tell at a glance whether an action will touch
//! identity-bearing infrastructure or just collaboration scope.

use dioxus::prelude::*;

use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::types::RealmClass;
use crate::utils::i18n::t;

#[component]
pub fn RealmClassificationBadge(
    #[props(default)] class: String,
    realm_class: RealmClass,
) -> Element {
    let (variant, key) = match realm_class {
        RealmClass::PrincipalControl => (BadgeVariant::Destructive, "realm.principal_control"),
        RealmClass::Collaboration => (BadgeVariant::Secondary, "realm.collaboration"),
    };
    rsx! {
        Badge { variant, class: class.clone(), {t(key)} }
    }
}
