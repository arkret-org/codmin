use dioxus::prelude::*;
// `PageHeader` has been migrated to yoface (the signature matches, and
// rendering goes through css_module + the Soft Orbit tokens). Re-exported
// directly, so call sites need zero changes.
pub use yoface::ui::page_header::PageHeader;

use crate::router::Route;

// The Breadcrumbs rendering has been migrated to yoface
// (`yoface::ui::page_header::Breadcrumbs`, `href: Option<String>`). Only a
// **thin adapter** layer is kept locally: codmin's call sites use the strongly
// typed `crate::router::Route` (which validates at compile time that the route
// exists), and the adapter converts each `Route` into the `href` string yoface
// expects via `Routable`'s `Display` (`route.to_string()` is the URL path),
// then delegates the rendering to yoface. The original hand-written
// `nav`/`Link`/utility-class implementation has been deleted.
#[derive(Debug, Clone, PartialEq)]
pub struct BreadcrumbItem {
    pub label: String,
    pub route: Option<Route>,
}

#[component]
pub fn Breadcrumbs(items: Vec<BreadcrumbItem>) -> Element {
    let mapped: Vec<yoface::ui::page_header::BreadcrumbItem> = items
        .into_iter()
        .map(|item| yoface::ui::page_header::BreadcrumbItem {
            label: item.label,
            href: item.route.map(|r| r.to_string()),
        })
        .collect();
    rsx! {
        yoface::ui::page_header::Breadcrumbs { items: mapped }
    }
}
