use std::cell::Cell;
use std::rc::Rc;

use gloo_net::http::{Headers, Request, RequestBuilder};
use serde::Serialize;
use serde::de::DeserializeOwned;
use web_sys::{AbortController, RequestCredentials};

use crate::utils::net::error::{AdminErrorEnvelope, HttpError, display_error};
use crate::utils::net::perf;
use crate::utils::security::crypto::random_token;

pub const HEADER_REQUEST_ID: &str = "X-Arkret-Request-Id";
pub const HEADER_IDEMPOTENCY_KEY: &str = "Idempotency-Key";
pub const HEADER_ARKRET_OPERATION: &str = "Arkret-Operation";
const REQUEST_TIMEOUT_MS: u32 = 30_000;

/// Deserialization target for mutation endpoints whose response body is
/// irrelevant (only success/failure matters). Tolerates `{}`, `null`, or any
/// JSON shape without allocating a `serde_json::Value` tree, and works with the
/// 204 fast-path in [`raw_fetch`] (which feeds `"{}"` / `"null"`).
#[derive(Debug, Default)]
pub struct NoBody;

impl<'de> serde::Deserialize<'de> for NoBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(NoBody)
    }
}

/// Sentinel for bodyless requests. Fixes `B = ()` so callers write
/// `api_client(url, method, NO_BODY)` instead of a `None::<()>` turbofish.
pub const NO_BODY: Option<()> = None;

const SENSITIVE_QUERY_KEYS: &[&str] = &[
    "access_token",
    "api_key",
    "apikey",
    "auth",
    "authorization",
    "bearer",
    "client_secret",
    "code",
    "id_token",
    "otp",
    "password",
    "refresh_token",
    "session",
    "session_token",
    "signature",
    "signature",
    "token",
];

/// Per-request correlation id (12 bytes / ~16 chars base64url) used in the
/// `X-Arkret-Request-Id` header so admin actions can be traced across
/// proxy + coauth + coland logs.
pub fn generate_request_id() -> String {
    random_token(12)
}

/// Idempotency key for mutating admin operations. The protocol requires
/// strong uniqueness so retries collapse on the server side; 16 random
/// bytes (~22 base64url chars) gives ~128 bits of entropy.
pub fn generate_idempotency_key() -> String {
    format!("codmin-{}", random_token(16))
}

pub fn json_body<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, HttpError> {
    serde_json::to_string(value)
        .map_err(|err| HttpError::message(format!("failed to serialize request body: {err}")))
}

pub async fn raw_fetch<T, F>(
    url: &str,
    method: &str,
    body: Option<String>,
    format_error: F,
    idempotency_key: Option<&str>,
) -> Result<T, HttpError>
where
    T: DeserializeOwned,
    F: Fn(u16, &str, Option<u64>) -> (String, Option<AdminErrorEnvelope>),
{
    let start_time = js_sys::Date::now();

    reject_query_credentials(url)?;

    let rid_value = generate_request_id();
    let rid = Some(rid_value.clone());
    let abort_controller = AbortController::new().map_err(|e| HttpError {
        message: format!("failed to create request abort controller: {e:?}"),
        status: 0,
        body: None,
        request_id: rid.clone(),
        retry_after_ms: None,
    })?;
    let abort_signal = abort_controller.signal();

    // S5 token hardening: every admin API call rides the httpOnly
    // session cookie. There is no `Authorization: Bearer <token>`
    // header from the SPA — the cookie is sent automatically by the
    // browser when `credentials: "include"` is set on the request.
    let normalized_method = method.to_ascii_uppercase();
    let mut builder: RequestBuilder = match normalized_method.as_str() {
        "POST" => Request::post(url),
        "PUT" => Request::put(url),
        "PATCH" => Request::patch(url),
        "DELETE" => Request::delete(url),
        "GET" => Request::get(url),
        _ => {
            return Err(HttpError {
                message: format!("unsupported HTTP method: {method}"),
                status: 0,
                body: None,
                request_id: rid.clone(),
                retry_after_ms: None,
            });
        }
    }
    .header("Accept", "application/json")
    .header(HEADER_REQUEST_ID, &rid_value)
    .credentials(RequestCredentials::Include)
    .abort_signal(Some(&abort_signal));

    if let Some(operation) = canonical_operation_for_request(url, &normalized_method) {
        builder = builder.header(HEADER_ARKRET_OPERATION, operation.as_str());
    }

    if is_mutation_method(&normalized_method) {
        let generated_key;
        let key = match idempotency_key {
            Some(key) => key,
            None => {
                generated_key = generate_idempotency_key();
                &generated_key
            }
        };
        builder = builder.header(HEADER_IDEMPOTENCY_KEY, key);
    }

    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }

    let request = if let Some(body) = body {
        builder.body(body).map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: rid.clone(),
            retry_after_ms: None,
        })?
    } else {
        builder.build().map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: rid.clone(),
            retry_after_ms: None,
        })?
    };

    let timed_out = Rc::new(Cell::new(false));
    let timed_out_for_timer = timed_out.clone();
    let timeout = gloo_timers::callback::Timeout::new(REQUEST_TIMEOUT_MS, move || {
        timed_out_for_timer.set(true);
        abort_controller.abort();
    });

    let response = request.send().await.map_err(|e| HttpError {
        message: fetch_error_message(&e.to_string(), timed_out.get()),
        status: 0,
        body: None,
        request_id: rid.clone(),
        retry_after_ms: None,
    })?;

    let status = response.status();
    let response_request_id = header_value(response.headers(), HEADER_REQUEST_ID);
    let response_rid = response_request_id.or(rid.clone());
    let retry_after_ms = retry_after_ms(response.headers());
    let duration_ms = js_sys::Date::now() - start_time;
    perf::record_api_call(duration_ms);

    if status == 204 {
        drop(timeout);
        let empty = serde_json::from_str::<T>("{}").or_else(|_| serde_json::from_str::<T>("null"));
        return empty.map_err(|e| HttpError {
            message: e.to_string(),
            status,
            body: None,
            request_id: response_rid.clone(),
            retry_after_ms,
        });
    }

    let text = response.text().await.map_err(|e| HttpError {
        message: fetch_error_message(&e.to_string(), timed_out.get()),
        status,
        body: None,
        request_id: response_rid.clone(),
        retry_after_ms,
    })?;
    drop(timeout);

    if status >= 400 {
        let (message, error_body) = format_error(status, &text, retry_after_ms);
        let err = HttpError {
            message,
            status,
            body: error_body,
            request_id: response_rid,
            retry_after_ms,
        };
        // Fire-and-forget opt-in telemetry. Internal no-op when
        // disabled / endpoint unset; never blocks the caller.
        crate::utils::net::telemetry::report_http_error(url, &err);
        return Err(err);
    }

    serde_json::from_str(&text).map_err(|e| HttpError {
        message: format!("JSON parse error: {e}"),
        status,
        body: None,
        request_id: response_rid,
        retry_after_ms,
    })
}

fn canonical_operation_for_request(
    url: &str,
    method: &str,
) -> Option<arkret_wire::ServiceOperationId> {
    let relative_path = url.split(['?', '#']).next().unwrap_or(url);
    let path = if relative_path.starts_with('/') {
        relative_path.to_owned()
    } else {
        web_sys::Url::new(url).ok()?.pathname()
    };
    arkret_wire::ServiceOperationId::from_http_request(method, &path)
}

pub fn format_admin_error(
    status: u16,
    text: &str,
    retry_after_ms: Option<u64>,
) -> (String, Option<AdminErrorEnvelope>) {
    let mut error_body: Option<AdminErrorEnvelope> = AdminErrorEnvelope::from_wire(text);
    if let Some(ref mut body) = error_body
        && body.retry_after_ms.is_none()
    {
        body.retry_after_ms = retry_after_ms;
    }
    let mut message = if let Some(ref eb) = error_body {
        display_error(&eb.errcode, status, eb.error.as_deref().unwrap_or(""))
    } else {
        // Non-envelope body (opaque proxy error, HTML 5xx, etc.) — synthesize
        // a local, non-wire code so it is never mistaken for a registry code.
        display_error("codmin.http_status", status, text)
    };
    let retry_after_ms = error_body
        .as_ref()
        .and_then(|body| body.retry_after_ms)
        .or(retry_after_ms);
    if let Some(retry_after_ms) = retry_after_ms {
        message.push_str(&format!(" Retry after {}s.", retry_after_ms / 1000));
    }
    (message, error_body)
}

async fn api_client_raw<T: DeserializeOwned>(
    url: &str,
    method: &str,
    body: Option<String>,
) -> Result<T, HttpError> {
    api_client_raw_with_idempotency(url, method, body, None).await
}

async fn api_client_raw_with_idempotency<T: DeserializeOwned>(
    url: &str,
    method: &str,
    body: Option<String>,
    idempotency_key: Option<String>,
) -> Result<T, HttpError> {
    // Spend a refresh round-trip ahead of the request when the cached
    // access token is within ~60s of expiry, so the happy path stays a
    // single call instead of failing with 401 and replaying.
    crate::api::auth::refresh_if_expiring_soon().await;

    let idempotency_key = idempotency_key.or_else(|| {
        if is_mutation_method(method) {
            Some(generate_idempotency_key())
        } else {
            None
        }
    });

    let result = raw_fetch::<T, _>(
        url,
        method,
        body.clone(),
        format_admin_error,
        idempotency_key.as_deref(),
    )
    .await;

    if let Err(ref err) = result
        && err.status == 401
        && crate::api::auth::handle_unauthorized().await
    {
        return raw_fetch::<T, _>(
            url,
            method,
            body,
            format_admin_error,
            idempotency_key.as_deref(),
        )
        .await;
    }

    result
}

/// Admin API call. `body` is serialized to JSON internally; pass
/// [`NO_BODY`] for a bodyless GET / DELETE / POST.
pub async fn api_client<T: DeserializeOwned, B: Serialize>(
    url: &str,
    method: &str,
    body: Option<B>,
) -> Result<T, HttpError> {
    let body = match body {
        Some(value) => Some(json_body(&value)?),
        None => None,
    };
    api_client_raw(url, method, body).await
}

pub fn build_url(path: &str, parameters: &[(&str, &str)]) -> Result<String, HttpError> {
    reject_query_credentials(path)?;
    for (key, _) in parameters {
        reject_query_key(key)?;
    }

    let mut url = path.to_string();

    let query_params: Vec<String> = parameters
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect();

    if !query_params.is_empty() {
        url.push('?');
        url.push_str(&query_params.join("&"));
    }

    Ok(url)
}

fn is_mutation_method(method: &str) -> bool {
    matches!(
        method.to_ascii_uppercase().as_str(),
        "POST" | "PUT" | "PATCH" | "DELETE"
    )
}

fn fetch_error_message(error: &str, timed_out: bool) -> String {
    if timed_out {
        format!("request timed out after {REQUEST_TIMEOUT_MS}ms")
    } else {
        error.to_string()
    }
}

fn header_value(headers: Headers, name: &str) -> Option<String> {
    headers.get(name).filter(|value| !value.trim().is_empty())
}

fn retry_after_ms(headers: Headers) -> Option<u64> {
    let value = header_value(headers, "Retry-After")?;
    let trimmed = value.trim();
    if let Some(ms) = trimmed
        .parse::<u64>()
        .ok()
        .and_then(|seconds| seconds.checked_mul(1000))
    {
        return Some(ms);
    }

    let target_ms = js_sys::Date::parse(trimmed);
    if target_ms.is_nan() {
        return None;
    }
    let wait_ms = target_ms - js_sys::Date::now();
    Some(wait_ms.max(0.0) as u64)
}

fn reject_query_credentials(url: &str) -> Result<(), HttpError> {
    let Some(query) = url
        .split_once('?')
        .map(|(_, rest)| rest.split('#').next().unwrap_or(rest))
    else {
        return Ok(());
    };
    for pair in query.split('&') {
        let key = pair.split_once('=').map(|(key, _)| key).unwrap_or(pair);
        reject_query_key(key)?;
    }
    Ok(())
}

fn reject_query_key(key: &str) -> Result<(), HttpError> {
    let decoded = urlencoding::decode(key).unwrap_or_else(|_| key.into());
    if is_sensitive_query_key(&decoded) {
        return Err(HttpError::message(
            "query string authentication material is not allowed",
        ));
    }
    Ok(())
}

fn is_sensitive_query_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_QUERY_KEYS
        .iter()
        .any(|sensitive| key.contains(sensitive))
}

#[cfg(test)]
mod tests {
    use super::{
        build_url, canonical_operation_for_request, format_admin_error, is_mutation_method,
    };

    #[test]
    fn canonical_arkret_request_selects_exact_operation() {
        let operation =
            canonical_operation_for_request("/_arkret/describe?service_kind=station", "GET")
                .expect("registered describe operation");
        assert_eq!(operation.as_str(), "ak.server.read.describe.v1");
        assert!(canonical_operation_for_request("/_coland/admin/server/info", "GET").is_none());
    }

    #[test]
    fn build_url_rejects_query_credentials() {
        assert!(build_url("/_coland/admin/actors", &[("access_token", "secret")]).is_err());
        assert!(build_url("/_coland/admin/actors?token=secret", &[]).is_err());
        assert!(build_url("/_coland/admin/actors?client_secret=secret", &[]).is_err());
        assert!(build_url("/_coland/admin/actors?oauth_code=secret", &[]).is_err());
    }

    #[test]
    fn build_url_appends_query_params() {
        assert_eq!(
            build_url("/_coland/admin/actors", &[("cursor", "c1")]).unwrap(),
            "/_coland/admin/actors?cursor=c1"
        );
    }

    #[test]
    fn mutation_methods_get_idempotency_keys() {
        assert!(is_mutation_method("POST"));
        assert!(is_mutation_method("delete"));
        assert!(!is_mutation_method("GET"));
    }

    #[test]
    fn admin_error_preserves_retry_after() {
        // Canonical Problem Details body; retry_after falls back to the Retry-After
        // header when absent in the body.
        let (_, body) = format_admin_error(
            429,
            r#"{"type":"https://arkret.org/problems/rate_limited","title":"Rate limited","status":429,"detail":"slow down","instance":"r"}"#,
            Some(2000),
        );

        assert_eq!(body.unwrap().retry_after_ms, Some(2000));
    }

    #[test]
    fn admin_error_reads_canonical_code() {
        let (_, body) = format_admin_error(
            403,
            r#"{"type":"https://arkret.org/problems/capability_denied","title":"Capability denied","status":403,"detail":"no","instance":"r"}"#,
            None,
        );
        assert_eq!(body.unwrap().errcode, "capability_denied");
    }
}
