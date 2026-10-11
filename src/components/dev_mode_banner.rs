//! T1.4 — Development-mode warning banner.
//!
//! Polls the admin describe surface (`GET /_arkret/describe`) for the
//! `development_mode` field surfaced by coland. When the connected server
//! reports `development_mode == true`, the layout renders a red,
//! non-dismissible top banner so operators can never mistake a dev
//! deployment for production at a glance.
//!
//! The banner is intentionally cheap to render:
//!   - One `use_resource` shared at the layout level (one HTTP call per mount, not per page).
//!   - Renders nothing while the describe call is in flight or when the server reports
//!     `development_mode == false` (or omits the field — older coland builds predate T1.4).
//!   - No close button: the banner is the safety signal, hiding it defeats the purpose.

use dioxus::prelude::*;

use crate::api::server;
use crate::utils::i18n::t;

/// Returns true when the cached / freshly-fetched server describe
/// reports `development_mode == true`. Returns false for all other
/// shapes (not loaded yet, request failed, or a production server).
fn server_in_dev_mode(describe: &Option<crate::types::ServerDescribeDocument>) -> bool {
    describe
        .as_ref()
        .map(|d| d.development_mode)
        .unwrap_or(false)
}

#[component]
pub fn DevModeBanner() -> Element {
    let describe = use_resource(|| async { server::get_server_describe().await.ok() });
    let describe_data = describe.read().clone().flatten();

    if !server_in_dev_mode(&describe_data) {
        return rsx! {};
    }

    rsx! {
        div {
            class: "w-full bg-red-600 text-white text-center py-2 px-4 font-semibold text-sm shadow-md",
            role: "alert",
            "aria-live": "polite",
            span { class: "mr-2", "\u{26A0}" }
            "{t(\"server_status.dev_banner\")}"
        }
    }
}

/// Inline variant for embedding inside a page (e.g. dashboard). Same data
/// source as `DevModeBanner`, but rendered as a full-width card so it
/// reads as page-level content rather than a chrome strip.
#[component]
pub fn DevModeDashboardNotice() -> Element {
    let describe = use_resource(|| async { server::get_server_describe().await.ok() });
    let describe_data = describe.read().clone().flatten();

    if !server_in_dev_mode(&describe_data) {
        return rsx! {};
    }

    rsx! {
        div {
            class: "w-full rounded-md border border-red-600 bg-red-600/10 dark:bg-red-600/20 px-4 py-3",
            role: "alert",
            "aria-live": "polite",
            div { class: "flex items-start gap-3",
                span { class: "text-red-600 text-xl leading-none mt-0.5", "\u{26A0}" }
                div { class: "min-w-0",
                    p { class: "font-semibold text-red-700 dark:text-red-300",
                        "{t(\"server_status.dev_banner\")}"
                    }
                    p { class: "mt-1 text-sm text-red-700/90 dark:text-red-200/90",
                        "{t(\"server_status.dev_banner_dashboard\")}"
                    }
                }
            }
        }
    }
}
