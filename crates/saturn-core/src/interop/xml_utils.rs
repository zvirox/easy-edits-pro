pub(super) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(super) fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let key = format!("{}=\"", attr);
    let start_idx = tag.find(&key)? + key.len();
    let rest = &tag[start_idx..];
    let end_idx = rest.find('"')?;
    Some(rest[..end_idx].to_string())
}

pub(super) fn extract_tag_content(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = text.find(&open)? + open.len();
    let rest = &text[start..];
    let end = rest.find(&close)?;
    Some(rest[..end].trim().to_string())
}

pub(super) fn parse_seconds_attr(val: &str) -> i64 {
    let num_str = val.trim_end_matches('s').trim();
    if let Ok(secs) = num_str.parse::<f64>() {
        (secs * 1000.0) as i64
    } else {
        0
    }
}
// =========================================================================
// 4. OpenTimelineIO (.otio) Export & Import (Pixar / ASWF Standard)
// =========================================================================
