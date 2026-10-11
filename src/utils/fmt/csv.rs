//! Client-side CSV export helper for admin list pages.
//!
//! Why this exists: operators routinely want to spreadsheet a snapshot
//! of a codmin list (agents / devices / spaces) for offline review or
//! to attach to an incident ticket. Round-tripping through the coland
//! admin API for an export endpoint is heavyweight for what's
//! ultimately UI sugar, and it would force coland to grow a
//! resource-specific CSV serializer for every list. This helper keeps
//! the export client-side: take the same rows already rendered into
//! the table, run them through a single safe CSV writer, and trigger
//! a browser download.
//!
//! Safety: we always quote every field, escape embedded quotes, and
//! prefix CSV-injection-prone leading characters (`= + - @ \t`) with a
//! single quote so Excel / LibreOffice don't interpret the cell as a
//! formula. This is the same hardening Synapse-admin applies.

/// Build a CSV string from a header row + body rows.
///
/// * `headers` is a list of column names.
/// * `rows` is a list of rows where each row is a list of cells. Cells are padded with empty
///   strings up to `headers.len()` and truncated beyond it, so a malformed row never produces a
///   ragged CSV.
///
/// Pure function — no I/O. Call [`trigger_csv_download`] separately
/// when running in the browser.
pub fn build_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let cols = headers.len();
    let mut out = String::new();
    out.push_str(&format_csv_row(
        &headers.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    ));
    out.push('\n');
    for row in rows {
        let mut normalized = row.clone();
        if normalized.len() < cols {
            normalized.resize(cols, String::new());
        }
        if normalized.len() > cols {
            normalized.truncate(cols);
        }
        out.push_str(&format_csv_row(&normalized));
        out.push('\n');
    }
    out
}

fn format_csv_row(cells: &[String]) -> String {
    cells
        .iter()
        .map(|c| escape_csv_field(c))
        .collect::<Vec<_>>()
        .join(",")
}

/// Quote every field and prefix CSV-injection-prone leading
/// characters with a single quote. Embedded `"` is doubled per
/// RFC 4180.
fn escape_csv_field(s: &str) -> String {
    let needs_prefix = s
        .chars()
        .next()
        .map(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'))
        .unwrap_or(false);
    let mut body = String::with_capacity(s.len() + 2);
    if needs_prefix {
        body.push('\'');
    }
    body.push_str(s);
    let escaped = body.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

/// Browser-side download trigger. Wraps the CSV string in a Blob,
/// creates an object URL, clicks a synthetic `<a download>` and
/// revokes the URL.
///
/// Gated on `target_arch = "wasm32"` so host-side `cargo check` and
/// unit tests don't pull in `web-sys::Document` paths.
#[cfg(target_arch = "wasm32")]
pub fn export_to_csv(filename: &str, csv: &str) {
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            log::warn!("codmin.csv.export: window unavailable");
            return;
        }
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(csv));
    let bag = BlobPropertyBag::new();
    bag.set_type("text/csv;charset=utf-8");
    let blob = match Blob::new_with_str_sequence_and_options(&array, &bag) {
        Ok(b) => b,
        Err(_) => {
            log::warn!("codmin.csv.export: blob construction failed");
            return;
        }
    };
    let url = match Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(_) => return,
    };
    let anchor = match document.create_element("a").and_then(|el| {
        el.dyn_into::<HtmlAnchorElement>()
            .map_err(|_| JsValue::NULL)
    }) {
        Ok(a) => a,
        Err(_) => {
            let _ = Url::revoke_object_url(&url);
            return;
        }
    };
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.set_attribute("style", "display:none").ok();
    if let Some(body) = document.body() {
        body.append_child(&anchor).ok();
        anchor.click();
        body.remove_child(&anchor).ok();
    } else {
        anchor.click();
    }
    let _ = Url::revoke_object_url(&url);
}

/// Host-side fallback so `cargo check --offline` on a non-wasm
/// target compiles cleanly.
#[cfg(not(target_arch = "wasm32"))]
pub fn export_to_csv(_filename: &str, _csv: &str) {
    // no-op
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_csv_emits_header_then_rows() {
        let csv = build_csv(
            &["id", "name"],
            &[
                vec!["a1".into(), "Alice".into()],
                vec!["b2".into(), "Bob".into()],
            ],
        );
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), "\"id\",\"name\"");
        assert_eq!(lines.next().unwrap(), "\"a1\",\"Alice\"");
        assert_eq!(lines.next().unwrap(), "\"b2\",\"Bob\"");
    }

    #[test]
    fn escapes_embedded_double_quotes() {
        let csv = build_csv(&["x"], &[vec!["he said \"hi\"".into()]]);
        assert!(csv.contains("\"he said \"\"hi\"\"\""));
    }

    #[test]
    fn quotes_csv_injection_prefixes() {
        for prefix in ["=", "+", "-", "@", "\t"] {
            let cell = format!("{prefix}cmd|calc");
            let csv = build_csv(&["x"], &[vec![cell]]);
            // single-quote prefix protects against formula execution
            assert!(csv.contains("\"\'"), "missing apostrophe prefix in {csv}");
        }
    }

    #[test]
    fn pads_short_rows_to_header_width() {
        let csv = build_csv(&["a", "b", "c"], &[vec!["1".into()]]);
        assert!(csv.contains("\"1\",\"\",\"\""));
    }

    #[test]
    fn truncates_oversized_rows_to_header_width() {
        let csv = build_csv(&["a", "b"], &[vec!["1".into(), "2".into(), "3".into()]]);
        let line = csv.lines().nth(1).unwrap();
        assert_eq!(line, "\"1\",\"2\"");
    }

    #[test]
    fn empty_rows_produces_header_only() {
        let csv = build_csv(&["x"], &[]);
        assert_eq!(csv.trim(), "\"x\"");
    }
}
