//! Timeline marker helpers.
//!
//! Markers are owned by a sequence and stored in the project file as integer
//! ticks, so navigation and CSV import/export never accumulate float drift.
//! The CSV format is deliberately tick-based and header-stable:
//!
//! ```text
//! name,kind,start_ticks,duration_ticks,color,note
//! ```
//!
//! `kind` is one of `comment`, `chapter`, `flash_cue`, `web_link`, or
//! `segmentation`. A `duration_ticks` of `0` means a point marker.

use crate::project::{Marker, MarkerKind};
use crate::time::Tick;

/// Column order of the marker CSV format. Changing it is a breaking change.
pub const MARKER_CSV_HEADER: &str = "name,kind,start_ticks,duration_ticks,color,note";

/// A parsed marker row, before the engine assigns an id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedMarker {
    pub name: String,
    pub kind: MarkerKind,
    pub start_ticks: Tick,
    pub duration_ticks: Tick,
    pub color: String,
    pub note: String,
}

/// First marker whose start is strictly after `from`, in timeline order.
pub fn next_marker(markers: &[Marker], from: Tick) -> Option<&Marker> {
    markers
        .iter()
        .filter(|marker| marker.start_ticks.0 > from.0)
        .min_by_key(|marker| (marker.start_ticks.0, marker.id))
}

/// Last marker whose start is strictly before `from`, in timeline order.
pub fn previous_marker(markers: &[Marker], from: Tick) -> Option<&Marker> {
    markers
        .iter()
        .filter(|marker| marker.start_ticks.0 < from.0)
        .max_by_key(|marker| (marker.start_ticks.0, marker.id))
}

/// Range marker containing `tick`, if any (end exclusive).
pub fn marker_at(markers: &[Marker], tick: Tick) -> Option<&Marker> {
    markers.iter().find(|marker| marker.contains(tick))
}

/// Serialize markers to the documented CSV format.
pub fn export_markers_csv(markers: &[Marker]) -> String {
    let mut out = String::from(MARKER_CSV_HEADER);
    out.push('\n');
    for marker in markers {
        let row = [
            marker.name.as_str(),
            marker.kind.id(),
            &marker.start_ticks.0.to_string(),
            &marker.duration_ticks.0.to_string(),
            marker.color.as_str(),
            marker.note.as_str(),
        ];
        out.push_str(&row.map(escape_field).join(","));
        out.push('\n');
    }
    out
}

/// Parse the documented CSV format. The header row is optional and ignored
/// when present. Unknown kinds, malformed numbers, and negative timing are
/// reported with the 1-based row number so import failures are actionable.
pub fn parse_markers_csv(csv: &str) -> Result<Vec<ParsedMarker>, String> {
    let records = split_records(csv);
    let mut parsed = Vec::new();
    let mut saw_header = false;
    for (index, record) in records.iter().enumerate() {
        if record.iter().all(|field| field.trim().is_empty()) {
            continue;
        }
        if !saw_header && is_header(record) {
            saw_header = true;
            continue;
        }
        saw_header = true;
        let row = index + 1;
        if record.len() < 4 {
            return Err(format!(
                "Marker CSV row {row} needs at least name,kind,start_ticks,duration_ticks"
            ));
        }
        let name = record[0].trim();
        let kind = MarkerKind::parse(&record[1]).ok_or_else(|| {
            format!(
                "Marker CSV row {row} has unknown kind `{}` (expected comment, chapter, flash_cue, web_link, or segmentation)",
                record[1].trim()
            )
        })?;
        let start_ticks = parse_ticks(&record[2], row, "start_ticks")?;
        let duration_ticks = parse_ticks(&record[3], row, "duration_ticks")?;
        if start_ticks.0 < 0 {
            return Err(format!("Marker CSV row {row} starts before zero"));
        }
        if duration_ticks.0 < 0 {
            return Err(format!("Marker CSV row {row} has a negative duration"));
        }
        if start_ticks.0.checked_add(duration_ticks.0).is_none() {
            return Err(format!("Marker CSV row {row} overflows the timebase"));
        }
        parsed.push(ParsedMarker {
            name: if name.is_empty() {
                format!("Marker {row}")
            } else {
                name.to_string()
            },
            kind,
            start_ticks,
            duration_ticks,
            color: record.get(4).map(|value| value.trim().to_string()).unwrap_or_default(),
            note: record.get(5).map(|value| value.trim().to_string()).unwrap_or_default(),
        });
    }
    Ok(parsed)
}

fn parse_ticks(value: &str, row: usize, field: &str) -> Result<Tick, String> {
    value
        .trim()
        .parse::<i64>()
        .map(Tick)
        .map_err(|_| format!("Marker CSV row {row} has a non-integer {field} `{}`", value.trim()))
}

fn is_header(record: &[String]) -> bool {
    record
        .first()
        .is_some_and(|first| first.trim().eq_ignore_ascii_case("name"))
        && record
            .get(1)
            .is_some_and(|second| second.trim().eq_ignore_ascii_case("kind"))
}

fn escape_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// RFC 4180-style record splitter: commas separate fields, quoted fields may
/// contain commas, escaped quotes, and newlines.
fn split_records(input: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted => {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            }
            '"' if field.is_empty() => quoted = true,
            ',' if !quoted => record.push(std::mem::take(&mut field)),
            '\r' if !quoted => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            other => field.push(other),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(id: u64, name: &str, start: i64, duration: i64) -> Marker {
        Marker {
            id,
            name: name.into(),
            kind: MarkerKind::Comment,
            start_ticks: Tick(start),
            duration_ticks: Tick(duration),
            color: String::new(),
            note: String::new(),
        }
    }

    #[test]
    fn navigation_skips_the_current_position() {
        let markers = vec![
            marker(1, "A", 100, 0),
            marker(2, "B", 300, 0),
            marker(3, "C", 500, 0),
        ];
        assert_eq!(next_marker(&markers, Tick(100)).unwrap().name, "B");
        assert_eq!(previous_marker(&markers, Tick(300)).unwrap().name, "A");
        assert!(previous_marker(&markers, Tick(100)).is_none());
        assert!(next_marker(&markers, Tick(500)).is_none());
    }

    #[test]
    fn range_containment_is_end_exclusive() {
        let markers = vec![marker(1, "Range", 100, 50)];
        assert!(markers[0].contains(Tick(100)));
        assert!(markers[0].contains(Tick(149)));
        assert!(!markers[0].contains(Tick(150)));
        assert!(!markers[0].contains(Tick(99)));
        assert!(marker_at(&markers, Tick(120)).is_some());
        assert!(marker_at(&markers, Tick(160)).is_none());
    }

    #[test]
    fn csv_round_trip_preserves_every_field() {
        let markers = vec![
            Marker {
                id: 7,
                name: "Café, \"opening\"".into(),
                kind: MarkerKind::Chapter,
                start_ticks: Tick(254_016_000_000),
                duration_ticks: Tick(0),
                color: "#d9468f".into(),
                note: "Line one\nline two".into(),
            },
            Marker {
                id: 8,
                name: "範囲".into(),
                kind: MarkerKind::Segmentation,
                start_ticks: Tick(42),
                duration_ticks: Tick(1000),
                color: String::new(),
                note: String::new(),
            },
        ];
        let csv = export_markers_csv(&markers);
        let parsed = parse_markers_csv(&csv).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "Café, \"opening\"");
        assert_eq!(parsed[0].kind, MarkerKind::Chapter);
        assert_eq!(parsed[0].start_ticks, Tick(254_016_000_000));
        assert_eq!(parsed[0].note, "Line one\nline two");
        assert_eq!(parsed[1].name, "範囲");
        assert_eq!(parsed[1].duration_ticks, Tick(1000));
    }

    #[test]
    fn csv_accepts_missing_header_and_reports_bad_rows() {
        let parsed = parse_markers_csv("First,comment,10,0,,\nSecond,chapter,20,5,,\n").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].kind, MarkerKind::Chapter);

        let error = parse_markers_csv("Bad,unknown_kind,10,0,,\n").unwrap_err();
        assert!(error.contains("row 1"), "error should cite the row: {error}");
        let error = parse_markers_csv("Bad,comment,ten,0,,\n").unwrap_err();
        assert!(error.contains("start_ticks"), "error should cite the field: {error}");
    }

    #[test]
    fn csv_ignores_trailing_newlines_and_blank_rows() {
        let csv = format!("{MARKER_CSV_HEADER}\nA,comment,0,0,,\n\n\n");
        let parsed = parse_markers_csv(&csv).unwrap();
        assert_eq!(parsed.len(), 1);
    }
}
