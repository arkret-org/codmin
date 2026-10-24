use std::fmt;
use std::sync::OnceLock;

use regex_lite::Regex;
/// Display-only projection parsed through the SDK canonical RFC 9457 Problem.
/// This view has no protocol serializer or alternative wire representation.
#[derive(Debug, Clone, Default)]
pub struct AdminProblemView {
    pub code: String,
    pub detail: Option<String>,
    pub retry_after_ms: Option<u64>,
}

impl AdminProblemView {
    /// Parse the canonical spec/SDK RFC 9457 Problem Details shape
    /// from an upstream (coland/coauth) error body. Returns `None` when the
    /// body is not a canonical envelope (e.g. an opaque HTML 502 from the
    /// proxy), so callers fall back to a status-only message.
    pub fn from_wire(text: &str) -> Option<Self> {
        let env: arkret_wire::Problem = serde_json::from_str(text).ok()?;
        let errcode = env.code().to_owned();
        let retry_after_ms = env.retry_after_ms();
        Some(AdminProblemView {
            code: errcode,
            detail: Some(env.detail).filter(|m| !m.is_empty()),
            retry_after_ms,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HttpError {
    pub message: String,
    pub status: u16,
    pub body: Option<AdminProblemView>,
    pub request_id: Option<String>,
    pub retry_after_ms: Option<u64>,
}

impl HttpError {
    /// Construct an `HttpError` from a message with no HTTP status
    /// (status 0) and no optional metadata. For client-side / encode
    /// failures that never reached the server.
    pub fn message(msg: impl Into<String>) -> Self {
        HttpError {
            message: msg.into(),
            status: 0,
            body: None,
            request_id: None,
            retry_after_ms: None,
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref rid) = self.request_id {
            write!(f, " (ref: {rid})")?;
        }
        if let Some(retry_after_ms) = self.retry_after_ms {
            write!(f, " (retry after: {}s)", retry_after_ms / 1000)?;
        }
        Ok(())
    }
}

impl std::error::Error for HttpError {}

pub fn display_error(errcode: &str, status: u16, message: &str) -> String {
    // Registered Circle reason codes come from the generated SDK type so
    // protocol renames fail at compile time. Local admin-only reasons remain
    // literals and use the same i18n fallback path.
    let circle_reason: Option<&'static str> = match errcode {
        _ if errcode == arkret_wire::ReasonCode::CIRCLE_REALM_MISMATCH => {
            Some("error.circle_realm_mismatch")
        }
        _ if errcode == arkret_wire::ReasonCode::CIRCLE_MEMBER_MUST_BE_REALM_MEMBER => {
            Some("error.circle_member_must_be_realm_member")
        }
        _ if errcode == arkret_wire::ReasonCode::CIRCLE_NOT_ACTIVE => {
            Some("error.circle_not_active")
        }
        "circle_already_terminal" => Some("error.circle_already_terminal"),
        "circle_capability_denied" => Some("error.circle_capability_denied"),
        "circle_scope_rotation_in_progress" => Some("error.circle_scope_rotation_in_progress"),
        _ => None,
    };
    if let Some(key) = circle_reason {
        let localised = crate::utils::i18n::t(key);
        let safe_message = redact_pii(message);
        if safe_message.is_empty() || localised.contains(&safe_message) {
            return format!("{errcode} ({status}): {localised}");
        }
        return format!("{errcode} ({status}): {localised}: {safe_message}");
    }

    // coland's error envelope carries bare snake_case errcodes straight
    // from the spec error-code-registry (there is no `ak.error.*`
    // prefix in the registry, and `schema` is registered as
    // `schema_violation`). Match those literal registry codes only.
    let safe_message = redact_pii(message);
    let fallback = match errcode {
        arkret_wire::ErrorCode::NOT_FOUND => "Resource not found",
        arkret_wire::ErrorCode::UNAUTHENTICATED => "Authentication required",
        arkret_wire::ErrorCode::CAPABILITY_DENIED => "Administrator capability denied",
        arkret_wire::ErrorCode::RATE_LIMITED => "Rate limited",
        arkret_wire::ErrorCode::TEMPORARILY_UNAVAILABLE => "Service temporarily unavailable",
        arkret_wire::ErrorCode::SCHEMA_VIOLATION => "Request validation failed",
        arkret_wire::ErrorCode::POLICY_DENIED | arkret_wire::ErrorCode::POLICY_VIOLATION => {
            "Required policy approval is missing"
        }
        arkret_wire::ErrorCode::AUTH_EXPIRED | arkret_wire::ErrorCode::SOFT_LOGGED_OUT => {
            "Session expired - sign in again"
        }
        arkret_wire::ErrorCode::DUPLICATE_CONFLICT | arkret_wire::ErrorCode::CONFLICT => {
            "Request conflicted with a previous mutation"
        }
        arkret_wire::ErrorCode::CAS_CONFLICT | arkret_wire::ErrorCode::FAILED_PRECONDITION => {
            "Precondition failed - refresh and retry"
        }
        _ => safe_message.as_str(),
    };
    if safe_message.is_empty() || fallback == safe_message {
        format!("{errcode} ({status}): {fallback}")
    } else {
        format!("{errcode} ({status}): {fallback}: {safe_message}")
    }
}

/// Strip obvious PII (emails, IP addresses, bearer tokens) from server
/// error text before it lands in a toast. Server messages are best-effort
/// human-readable and routinely include the offending input — emitting
/// the user's email or IP into the admin UI is a leak.
pub fn redact_pii(message: &str) -> String {
    static PATTERNS: OnceLock<[(Regex, &'static str); 4]> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            (
                // Email: covers most RFC 5322 atoms used in practice.
                Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").unwrap(),
                "[email]",
            ),
            (
                // IPv4 literal.
                Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
                "[ip]",
            ),
            (
                // Bearer / authorization tokens that occasionally leak
                // through server-side validation errors.
                Regex::new(r"(?i)bearer\s+[A-Za-z0-9._\-]+").unwrap(),
                "[token]",
            ),
            (
                // JWT-shaped triple (header.payload.signature). Distinct
                // from plain UUIDs/IDs which lack the dot structure, so
                // diagnostic identifiers stay readable.
                Regex::new(r"\b[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{4,}\b")
                    .unwrap(),
                "[jwt]",
            ),
        ]
    });

    let mut out = message.to_string();
    for (re, replacement) in patterns {
        out = re.replace_all(&out, *replacement).into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{display_error, redact_pii};

    #[test]
    fn redact_pii_strips_email_ip_token() {
        let raw = "denied for alice@example.com from 10.0.0.7 with Bearer abc.def-123";
        let safe = redact_pii(raw);
        assert!(!safe.contains("alice@example.com"));
        assert!(!safe.contains("10.0.0.7"));
        assert!(!safe.contains("abc.def-123"));
        assert!(safe.contains("[email]"));
        assert!(safe.contains("[ip]"));
        assert!(safe.contains("[token]"));
    }

    #[test]
    fn redact_pii_preserves_diagnostic_ids() {
        // UUIDs and short cursor tokens stay readable so admins can grep
        // logs.
        let raw = "request=ak:request:0198fe3a-91c0-7b2d-8f41-6c0a55d31e77 cursor=eyAB12";
        assert_eq!(redact_pii(raw), raw);
    }

    #[test]
    fn display_error_handles_protocol_codes() {
        let s = display_error("policy_denied", 412, "");
        assert!(s.contains("policy approval"));
        let s = display_error("auth_expired", 401, "");
        assert!(s.contains("Session expired"));
        let s = display_error("failed_precondition", 409, "");
        assert!(s.contains("Precondition failed"));
        // `schema_violation` is the registry code (not `schema`).
        let s = display_error("schema_violation", 422, "");
        assert!(s.contains("validation failed"));
    }

    #[test]
    fn display_error_redacts_raw_message() {
        let s = display_error("schema_violation", 400, "user bob@example.org rejected");
        assert!(!s.contains("bob@example.org"));
        assert!(s.contains("[email]"));
    }

    #[test]
    fn display_error_redacts_unregistered_code_message() {
        let s = display_error(
            "codmin.http_status",
            502,
            "proxy leaked alice@example.org from 203.0.113.9 with Bearer abc.def",
        );
        assert!(!s.contains("alice@example.org"));
        assert!(!s.contains("203.0.113.9"));
        assert!(!s.contains("abc.def"));
        assert!(s.contains("[email]"));
        assert!(s.contains("[ip]"));
        assert!(s.contains("[token]"));
    }

    #[test]
    fn from_wire_reads_canonical_envelope_code_and_message() {
        use super::AdminProblemView;
        let raw = r#"{"type":"https://arkret.org/problems/capability_denied","title":"Capability denied","status":403,"detail":"denied","instance":"req_1"}"#;
        let env = AdminProblemView::from_wire(raw).expect("parse");
        assert_eq!(env.code, "capability_denied");
        assert_eq!(env.detail.as_deref(), Some("denied"));
    }

    #[test]
    fn from_wire_reads_retry_after() {
        use super::AdminProblemView;
        let raw = r#"{"type":"https://arkret.org/problems/rate_limited","title":"Rate limited","status":429,"detail":"slow down","instance":"req_2","retry_after_ms":2000}"#;
        let env = AdminProblemView::from_wire(raw).expect("parse");
        assert_eq!(env.code, "rate_limited");
        assert_eq!(env.retry_after_ms, Some(2000));
    }

    #[test]
    fn from_wire_returns_none_for_non_envelope_body() {
        use super::AdminProblemView;
        assert!(AdminProblemView::from_wire("<html>502</html>").is_none());
    }
}
