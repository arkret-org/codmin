use glory::prelude::*;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    crate::pages::capabilities::page(scope, theme)
}
