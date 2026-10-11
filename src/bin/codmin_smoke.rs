//! Post-deploy E2E smoke test runner for codmin's coland admin surface.
//!
//! Hits the configured coland's admin health + each Stream H' endpoint
//! with an admin bearer token and prints a PASS/FAIL summary. Useful as
//! a one-shot ops sanity check after a deploy:
//!
//! Pass the admin bearer via the environment, never on the command line
//! (CLI args leak into shell history, CI logs, and `/proc/<pid>/cmdline`):
//!
//! ```ignore
//! CODMIN_SMOKE_TOKEN=<BEARER> \
//! cargo run --bin codmin-smoke -- \
//!     --base-url https://coland.example.com \
//!     --realm-id ak:realm:AXtI0JHl403DNDFAoBki57-5cESsvwc5LMyLnrl7o-A8
//! ```
//!
//! A `--token` flag exists for ad-hoc local use only; prefer the env var
//! in any automated / shared context.
//!
//! Exit code is non-zero when any check fails so this can be wired into
//! a CI gate or deploy webhook. The binary intentionally does NOT import
//! the rest of the codmin crate (which is wasm32-only); each request
//! body shape is reproduced as a small inline literal so the smoke
//! runner stays a self-contained native binary.

use std::env;
use std::process::ExitCode;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue};

const DEFAULT_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone)]
struct Args {
    base_url: String,
    token: Option<String>,
    realm_id: String,
    timeout_secs: u64,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut base_url = env::var("CODMIN_SMOKE_BASE_URL").ok();
        let mut token = env::var("CODMIN_SMOKE_TOKEN").ok();
        let mut realm_id = env::var("CODMIN_SMOKE_REALM_ID").ok();
        let mut timeout_secs = DEFAULT_TIMEOUT_SECS;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--base-url" | "-u" => {
                    base_url = Some(
                        args.next()
                            .ok_or_else(|| "--base-url requires a value".to_string())?,
                    );
                }
                "--token" | "-t" => {
                    token = Some(
                        args.next()
                            .ok_or_else(|| "--token requires a value".to_string())?,
                    );
                }
                "--realm-id" | "-r" => {
                    realm_id = Some(
                        args.next()
                            .ok_or_else(|| "--realm-id requires a value".to_string())?,
                    );
                }
                "--timeout" => {
                    let raw = args
                        .next()
                        .ok_or_else(|| "--timeout requires a value".to_string())?;
                    timeout_secs = raw.parse().map_err(|e| format!("invalid --timeout: {e}"))?;
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }

        let base_url = base_url
            .ok_or_else(|| "--base-url (or CODMIN_SMOKE_BASE_URL env) is required".to_string())?;
        let realm_id = realm_id
            .ok_or_else(|| "--realm-id (or CODMIN_SMOKE_REALM_ID env) is required".to_string())?;

        Ok(Args {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            realm_id,
            timeout_secs,
        })
    }
}

fn print_help() {
    println!(
        "codmin-smoke — coland admin endpoint smoke test\n\n\
USAGE:\n  CODMIN_SMOKE_TOKEN=<BEARER> codmin-smoke --base-url <URL> --realm-id <REALM_ID>\n\n\
OPTIONS:\n  -u, --base-url     coland base URL (e.g. https://coland.example.com)\n  -t, --token        admin bearer token. PREFER the env var $CODMIN_SMOKE_TOKEN\n                     — CLI args leak into shell history, CI logs, and the\n                     process table (/proc/<pid>/cmdline)\n  -r, --realm-id     Realm id to probe (Stream H' is per-Realm)\n      --timeout      per-request timeout in seconds (default {DEFAULT_TIMEOUT_SECS})\n  -h, --help         print this message\n"
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckOutcome {
    Pass,
    Fail,
}

impl CheckOutcome {
    fn label(&self) -> &'static str {
        match self {
            CheckOutcome::Pass => "PASS",
            CheckOutcome::Fail => "FAIL",
        }
    }
}

/// Pure helper: classify an HTTP response status into a `CheckOutcome`.
/// 2xx is PASS; every other status is FAIL. Pulled out as a pure function so
/// the deployment-gate policy can be unit-tested without sending requests.
pub(crate) fn classify_status(status: u16) -> CheckOutcome {
    if (200..300).contains(&status) {
        CheckOutcome::Pass
    } else {
        CheckOutcome::Fail
    }
}

/// Pure helper: build a per-Realm admin URL like
/// `<base>/_coland/admin/realms/<id>/<suffix>`. URL-encodes the Realm id.
pub(crate) fn build_realm_url(base_url: &str, realm_id: &str, suffix: &str) -> String {
    format!(
        "{}/_coland/admin/realms/{}/{}",
        base_url.trim_end_matches('/'),
        urlencoding_encode(realm_id),
        suffix.trim_start_matches('/'),
    )
}

/// Minimal URL-encoder for path segments. Native binary doesn't depend
/// on the `urlencoding` crate (which the wasm crate uses); this avoids a
/// duplicate dep gate just for path encoding. Encodes the characters
/// that conflict with path syntax (`/`, `?`, `#`) and unsafe / reserved
/// chars.
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b':' => {
                // Colon is allowed in URI path segments per RFC 3986
                // (pchar) and Arkret typed ids depend on it. Leaving
                // it un-encoded keeps the smoke output readable.
                out.push(':');
            }
            b => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
struct CheckResult {
    name: String,
    method: String,
    status: Option<u16>,
    outcome: CheckOutcome,
    detail: String,
}

fn run_check(client: &Client, name: &str, url: &str) -> CheckResult {
    match client.get(url).send() {
        Ok(resp) => {
            let status = resp.status();
            let outcome = classify_status(status.as_u16());
            let detail = match outcome {
                CheckOutcome::Pass => "ok".to_string(),
                CheckOutcome::Fail => format!(
                    "{} — {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("error")
                ),
            };
            CheckResult {
                name: name.to_string(),
                method: "GET".to_string(),
                status: Some(status.as_u16()),
                outcome,
                detail,
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            method: "GET".to_string(),
            status: None,
            outcome: CheckOutcome::Fail,
            detail: format!("transport error: {e}"),
        },
    }
}

fn run() -> ExitCode {
    let args = match Args::parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("codmin-smoke: {e}");
            eprintln!("run with --help for usage");
            return ExitCode::from(2);
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    if let Some(token) = &args.token {
        match HeaderValue::from_str(&format!("Bearer {token}")) {
            Ok(hv) => {
                headers.insert(AUTHORIZATION, hv);
            }
            Err(e) => {
                eprintln!("codmin-smoke: bad token: {e}");
                return ExitCode::from(2);
            }
        }
    }

    let client = match Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(args.timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("codmin-smoke: failed to build http client: {e}");
            return ExitCode::from(2);
        }
    };

    let base = &args.base_url;
    let realm_id = &args.realm_id;

    // Health probe + Realm-scoped admin read endpoints. Smoke checks
    // reachability and authentication only.
    let checks = vec![
        (
            "health",
            // coland's health probe is at the root `/health`, not under
            // the admin namespace (which has no `health` route).
            format!("{base}/health"),
        ),
        ("realms/links", build_realm_url(base, realm_id, "links")),
        (
            "realms/organizations",
            build_realm_url(base, realm_id, "organizations"),
        ),
    ];

    println!("codmin-smoke → {base} (realm={realm_id})");
    if args.token.is_none() {
        println!("  (no admin token supplied — auth-required endpoints will likely FAIL)");
    }
    println!();

    let mut results = Vec::with_capacity(checks.len());
    for (name, url) in checks {
        let result = run_check(&client, name, &url);
        let status_label = result
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "—".to_string());
        println!(
            "[{}] {:<6} {:<40} {:<5} {}",
            result.outcome.label(),
            result.method,
            result.name,
            status_label,
            result.detail,
        );
        results.push(result);
    }

    let pass = results
        .iter()
        .filter(|r| r.outcome == CheckOutcome::Pass)
        .count();
    let fail = results
        .iter()
        .filter(|r| r.outcome == CheckOutcome::Fail)
        .count();

    println!();
    println!(
        "Summary: {pass} pass / {fail} fail (total {})",
        results.len()
    );

    if fail > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    run()
}

#[cfg(test)]
mod tests {
    use super::{CheckOutcome, build_realm_url, classify_status, urlencoding_encode};

    #[test]
    fn classify_status_buckets_by_code() {
        assert_eq!(classify_status(200), CheckOutcome::Pass);
        assert_eq!(classify_status(204), CheckOutcome::Pass);
        assert_eq!(classify_status(299), CheckOutcome::Pass);
        // Every 4xx fails, including a missing deployment route or realm.
        assert_eq!(classify_status(400), CheckOutcome::Fail);
        assert_eq!(classify_status(401), CheckOutcome::Fail);
        assert_eq!(classify_status(403), CheckOutcome::Fail);
        assert_eq!(classify_status(404), CheckOutcome::Fail);
        // 5xx is always Fail.
        assert_eq!(classify_status(500), CheckOutcome::Fail);
        assert_eq!(classify_status(503), CheckOutcome::Fail);
    }

    #[test]
    fn build_realm_url_strips_trailing_base_slash() {
        let url = build_realm_url(
            "https://coland.example.com/",
            "ak:realm:AXtI0JHl403DNDFAoBki57-5cESsvwc5LMyLnrl7o-A8",
            "links",
        );
        assert_eq!(
            url,
            "https://coland.example.com/_coland/admin/realms/ak:realm:AXtI0JHl403DNDFAoBki57-5cESsvwc5LMyLnrl7o-A8/links"
        );
    }

    #[test]
    fn urlencoding_encode_preserves_colon_and_alnum() {
        // A canonical Realm id (typed prefix + 44-char Event token) is
        // the shape that matters — colons MUST stay unescaped or the
        // URL becomes unreadable in logs, and the base64url payload
        // charset (`-`, `_`, alnum) must survive untouched.
        assert_eq!(
            urlencoding_encode("ak:realm:AWUvBcFWRWKrAZRDsCI8hvY2qbeiJiDwRmeiHGgPL0_U"),
            "ak:realm:AWUvBcFWRWKrAZRDsCI8hvY2qbeiJiDwRmeiHGgPL0_U"
        );
        assert_eq!(urlencoding_encode("abc-123_~."), "abc-123_~.");
    }

    #[test]
    fn urlencoding_encode_escapes_unsafe_chars() {
        // Slashes and spaces must be percent-encoded so they don't
        // accidentally look like extra path components.
        assert_eq!(urlencoding_encode("a/b"), "a%2Fb");
        assert_eq!(urlencoding_encode("a b"), "a%20b");
        assert_eq!(urlencoding_encode("a?b"), "a%3Fb");
    }

    #[test]
    fn check_outcome_labels_match_summary() {
        assert_eq!(CheckOutcome::Pass.label(), "PASS");
        assert_eq!(CheckOutcome::Fail.label(), "FAIL");
    }
}
