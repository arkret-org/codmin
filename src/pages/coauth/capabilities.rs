use dioxus::prelude::*;

use crate::pages::capabilities::CapabilityLedger;
use crate::utils::i18n::t;

#[component]
pub fn AuthzCapabilitiesPage() -> Element {
    rsx! { CapabilityLedger { title: t("authz_caps.title"), description: t("authz_caps.subtitle") } }
}
