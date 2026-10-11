use dioxus::prelude::*;

use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;
use crate::utils::net::telemetry;

/// Render an [`ErrorBanner`] for a cached resource that resolved to an error.
///
/// Pages hold their server reads as `Option<Result<T, HttpError>>`: `None`
/// while the read is still in flight, `Some(Err(..))` once it failed. Both
/// dashboard-style pages render exactly this banner for the failed case and
/// nothing at all otherwise.
pub fn resource_error<T>(result: &Option<Result<T, HttpError>>) -> Element {
    match result.as_ref().and_then(|value| value.as_ref().err()) {
        Some(error) => rsx! {
            ErrorBanner {
                message: error.message.clone(),
                errcode: error.body.as_ref().map(|body| body.errcode.clone()),
                request_id: error.request_id.clone(),
                retry_after_ms: error.retry_after_ms,
            }
        },
        None => rsx! {},
    }
}

/// Shared inline error banner used on list/detail pages.
///
/// Migrated to yoface: rendering is handed to
/// `yoface::ui::error_banner::ErrorBanner` (a purely presentational css_module
/// component). This thin wrapper is kept locally as an **adapter**, taking on
/// the two business couplings the yoface shared library deliberately stripped
/// out (see the integration notes in yoface README §3.2):
///   * i18n —— the `common.error` / `common.retry` copy and the `error.<errcode>` localization are
///     resolved on the codmin side, then passed into the yoface component via `error_label` /
///     `retry_label` / `detail`;
///   * telemetry —— when the banner mounts, report analytics by `errcode` in a fire-and-forget
///     fashion.
///
/// The outward signature is exactly the same as before the migration, so the
/// ~50 call sites need zero changes.
#[component]
pub fn ErrorBanner(
    message: String,
    #[props(default)] errcode: Option<String>,
    #[props(default)] request_id: Option<String>,
    #[props(default)] retry_after_ms: Option<u64>,
    #[props(default)] on_retry: Option<EventHandler<MouseEvent>>,
) -> Element {
    let retry_label = t("common.retry");
    let error_label = t("common.error");

    // Fire-and-forget telemetry ping when the banner mounts with a known
    // wire errcode. No-op when telemetry is
    // disabled / the user has not opted in (see `utils::net::telemetry`).
    {
        let telemetry_code = errcode.clone();
        use_effect(move || {
            if let Some(code) = telemetry_code.as_deref() {
                telemetry::report_error(code, "ui.error_banner.shown");
            }
        });
    }

    // When the server returned a known wire code, look up the
    // localized copy via `error.<errcode>`. Falls back to None (no detail
    // line) when the key is missing.
    let detail = errcode.as_deref().and_then(|c| {
        let key = format!("error.{}", c);
        let v = t(&key);
        if v == key { None } else { Some(v) }
    });

    rsx! {
        yoface::ui::error_banner::ErrorBanner {
            message,
            error_label,
            retry_label,
            detail,
            errcode,
            request_id,
            retry_after_ms,
            on_retry,
        }
    }
}
