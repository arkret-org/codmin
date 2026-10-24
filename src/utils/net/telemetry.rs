//! Minimal opt-in browser error telemetry.
//!
//! This module is intentionally tiny: when the deployment exposes a
//! same-origin telemetry endpoint value in `/config.json` AND the operator
//! has flipped the local `codmin_telemetry_opt_in` flag in localStorage,
//! browser-side errors (HTTP errors with a status, JSON parse failures,
//! capability denials) are POSTed to that endpoint as a structured
//! payload — no PII, no message bodies, only:
//!
//! * `errcode` (e.g. `capability_denied`)
//! * `status` HTTP status code
//! * `request_id` from the `X-Arkret-Request-Id` header
//! * `path` (the request path, query string stripped)
//! * `ts` ISO-8601 timestamp
//!
//! Operators can correlate `request_id` with coland / coauth structured
//! logs to find the corresponding server-side log line. The
//! `request_id` is also preserved in error UI metadata so the operator
//! can paste it into a support ticket without opening DevTools.
//!
//! Sending failures are silently swallowed; telemetry MUST never break
//! the UI.
//!
//! Error storms (e.g. coland briefly returning 5xx on every API call) are
//! bounded by a client-side token-bucket rate limiter: at most
//! [`MAX_EVENTS_PER_WINDOW`] events are sent per [`WINDOW_MS`]; overflow is
//! dropped so telemetry never amplifies an outage into a request flood.

use std::cell::RefCell;
use std::collections::VecDeque;

use serde::Serialize;

use crate::utils::net::error::HttpError;
use crate::utils::storage;

/// Sliding rate-limit window length, in milliseconds.
const WINDOW_MS: f64 = 5_000.0;
/// Maximum telemetry events emitted within any [`WINDOW_MS`] window.
const MAX_EVENTS_PER_WINDOW: usize = 10;

thread_local! {
    /// Timestamps (ms since epoch) of recently sent telemetry events, used
    /// for the sliding-window rate limit. WASM is single-threaded, so a
    /// thread-local `RefCell` is sufficient and lock-free.
    static SEND_TIMES: RefCell<VecDeque<f64>> = const { RefCell::new(VecDeque::new()) };
}

/// Token-bucket gate: returns `true` (and records the send) when another
/// telemetry event is allowed inside the current window, `false` when the
/// window is saturated and the event must be dropped.
fn allow_send() -> bool {
    let now = js_sys::Date::now();
    SEND_TIMES.with(|cell| {
        let mut times = cell.borrow_mut();
        while times.front().is_some_and(|t| now - *t > WINDOW_MS) {
            times.pop_front();
        }
        if times.len() >= MAX_EVENTS_PER_WINDOW {
            return false;
        }
        times.push_back(now);
        true
    })
}

/// localStorage key for the operator opt-in flag.
const OPT_IN_KEY: &str = "codmin_telemetry_opt_in";
/// localStorage key for the runtime telemetry endpoint URL. Populated
/// at boot from `/config.json#telemetry_endpoint`. May be unset.
const ENDPOINT_KEY: &str = "codmin_telemetry_endpoint";

/// Set the endpoint URL. CSP keeps `connect-src` on `'self'`, so only
/// same-origin absolute paths are accepted.
pub fn set_endpoint(url: &str) {
    let endpoint = url.trim();
    if endpoint.is_empty()
        || !endpoint.starts_with('/')
        || endpoint.starts_with("//")
        || endpoint.contains("://")
    {
        storage::remove_item(ENDPOINT_KEY);
    } else {
        storage::set_item(ENDPOINT_KEY, endpoint);
    }
}

/// Returns `true` when the operator opted into telemetry AND a runtime
/// endpoint is configured.
pub fn is_enabled() -> bool {
    storage::get_item(OPT_IN_KEY).as_deref() == Some("1")
        && storage::get_item(ENDPOINT_KEY)
            .map(|s| !s.is_empty())
            .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize)]
struct ErrorEvent<'a> {
    errcode: &'a str,
    status: u16,
    request_id: Option<&'a str>,
    path: String,
    ts: String,
}

/// Reduce a concrete request URL to a low-cardinality, PII-free path
/// template for telemetry.
///
/// 1. Strips the query string + fragment (cursor tokens / filter values count as PII).
/// 2. Templates inline id segments to `{id}`. Our REST paths embed actor / account / realm / handle
///    ids directly in the path (e.g. `/_coland/admin/accounts/01HXY.../dids/did:web:x`), and those
///    ids — ULIDs, numeric ids, DIDs — are identifying. A segment is treated as an id when it
///    contains a `:` (DID / `ak:` ref), starts with a digit (ULID / numeric id), or is an overly
///    long opaque token. Static words like `api`, `v1`, `accounts` are preserved.
fn redact_path(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let path = path.split('#').next().unwrap_or(path);
    path.split('/')
        .map(|seg| {
            let is_id = seg.contains(':')
                || seg.chars().next().is_some_and(|c| c.is_ascii_digit())
                || seg.len() > 24;
            if is_id { "{id}" } else { seg }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Fire-and-forget POST of an error event. No-op when telemetry is
/// disabled, the URL is unset, or the user has not opted in.
///
/// Callers MUST NOT include the operator's input value, the server
/// error body, or anything that could carry credentials. Only the
/// fields documented on [`ErrorEvent`] are emitted.
pub fn report_http_error(path: &str, error: &HttpError) {
    if !is_enabled() {
        return;
    }
    let endpoint = match storage::get_item(ENDPOINT_KEY) {
        Some(e) if !e.is_empty() => e,
        _ => return,
    };
    // Bound error storms: drop the event when the window is saturated.
    if !allow_send() {
        return;
    }

    let errcode = error
        .body
        .as_ref()
        .map(|b| b.code.as_str())
        .unwrap_or("codmin.http_status");
    let event = ErrorEvent {
        errcode,
        status: error.status,
        request_id: error.request_id.as_deref(),
        path: redact_path(path),
        ts: iso_now(),
    };
    let body = match serde_json::to_string(&event) {
        Ok(b) => b,
        Err(_) => return,
    };

    // Fire-and-forget. We do not await — telemetry MUST never block.
    wasm_bindgen_futures::spawn_local(async move {
        // Telemetry MUST never break the UI: if the request builder
        // fails, drop the event silently rather than panicking.
        if let Ok(req) = gloo_net::http::Request::post(&endpoint)
            .header("Content-Type", "application/json")
            .body(body)
        {
            let _ = req.send().await;
        }
    });
}

/// Fire-and-forget reporter for browser-side errors that do NOT originate
/// from an HTTP envelope (panics, decoder failures, UI assertion misses).
///
/// Posts to the configured same-origin telemetry endpoint. No-op when
/// telemetry is disabled or the endpoint is unset.
///
/// `code` is a stable wire-style identifier (e.g. `ui.error_banner.shown`).
/// `context` is a short human-readable hint, MUST NOT include operator
/// input, secrets, or PII — keep it to component names and error kinds.
pub fn report_error(code: &str, context: &str) {
    if !is_enabled() {
        return;
    }
    let endpoint = match storage::get_item(ENDPOINT_KEY) {
        Some(e) if !e.is_empty() => e,
        _ => return,
    };
    // Bound error storms: drop the event when the window is saturated.
    if !allow_send() {
        return;
    }

    let payload = serde_json::json!({
        "code": code,
        "context": context,
        "ts": iso_now(),
    });
    let body = match serde_json::to_string(&payload) {
        Ok(b) => b,
        Err(_) => return,
    };

    // Fire-and-forget. We do not await — telemetry MUST never block.
    wasm_bindgen_futures::spawn_local(async move {
        // Telemetry MUST never break the UI: drop on builder failure.
        if let Ok(req) = gloo_net::http::Request::post(&endpoint)
            .header("Content-Type", "application/json")
            .body(body)
        {
            let _ = req.send().await;
        }
    });
}

fn iso_now() -> String {
    // js_sys::Date::to_iso_string returns a JsString; convert to native.
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_path_strips_query_and_fragment() {
        assert_eq!(redact_path("/_arkret/self/x?cursor=c1"), "/_arkret/self/x");
        assert_eq!(redact_path("/_arkret/self/x"), "/_arkret/self/x");
        assert_eq!(redact_path("/_arkret/self/x?q=1#frag"), "/_arkret/self/x");
    }

    #[test]
    fn redact_path_templates_inline_ids() {
        // ULID account id + DID segment → both templated; static words
        // (_coland / admin / accounts / dids) preserved.
        assert_eq!(
            redact_path("/_coland/admin/accounts/01HXYABCDEF/dids/did:web:alice.example"),
            "/_coland/admin/accounts/{id}/dids/{id}"
        );
        // Numeric id templated.
        assert_eq!(
            redact_path("/_coland/admin/accounts/12345/deactivate"),
            "/_coland/admin/accounts/{id}/deactivate"
        );
        // No ids → unchanged.
        assert_eq!(
            redact_path("/_coland/admin/server/info"),
            "/_coland/admin/server/info"
        );
    }
}
