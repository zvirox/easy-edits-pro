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

/// Lexically sanitize a media path parsed from an untrusted interchange
/// document (xmeml `pathurl`, FCPXML `src`, OTIO `target_url`, EDL clip
/// names). Percent-decodes first, strips the `file://` scheme, drops `.`
/// components, and collapses interior `..` components the way the OS would
/// resolve them - so an import can never store a parent-escaping `..` path.
///
/// A *relative* path that still climbs above its root (leading `..` after
/// collapsing) is rejected outright and the caller stores an unlinked,
/// empty-path asset. Absolute paths pass through structurally unchanged:
/// adopting absolute media references is the app's normal behavior for
/// user-chosen projects; per-item confirmation is a product-level concern.
pub(super) fn sanitize_untrusted_media_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }
    // Decode %XX escapes BEFORE scanning for traversal so `%2e%2e` cannot
    // smuggle a parent reference past the component check.
    let decoded = percent_decode(trimmed);
    let without_scheme = match decoded.strip_prefix("file://localhost") {
        Some(rest) => rest.to_string(),
        None => decoded
            .strip_prefix("file://")
            .map(str::to_string)
            .unwrap_or(decoded),
    };
    let is_absolute = without_scheme.starts_with('/');
    let mut components: Vec<&str> = Vec::new();
    for component in without_scheme.split(['/', '\\']) {
        match component {
            "" | "." => {}
            ".." => match components.pop() {
                Some(_) => {}
                // Climb above the root: absolute paths clamp at `/`
                // (matching real path resolution); relative paths escape
                // their base and are rejected.
                None if is_absolute => {}
                None => return None,
            },
            other => components.push(other),
        }
    }
    if is_absolute {
        Some(format!("/{}", components.join("/")))
    } else if components.is_empty() {
        None
    } else {
        Some(components.join("/"))
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(hex) = bytes.get(i + 1..i + 3) {
                if let Ok(value) = u8::from_str_radix(&String::from_utf8_lossy(hex), 16) {
                    out.push(value);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
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
