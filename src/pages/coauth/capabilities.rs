use glory::prelude::*;

use crate::utils::i18n::t;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    crate::pages::capabilities::page_with_heading(
        scope,
        theme,
        t("authz_caps.title"),
        t("authz_caps.subtitle"),
    )
}
