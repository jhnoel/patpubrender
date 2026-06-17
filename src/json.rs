//! Minimal JSON string escaping shared by the biblio sidecar (tier 1) and the
//! bulk-ingest manifest writer (tier 3). Kept dependency-free so the default
//! build pulls in no serializer.

/// Encode `value` as a quoted, escaped JSON string literal.
pub(crate) fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
