use crate::project::SubtitleItem;
use crate::time::Tick;

/// Converts a time tick to an SRT formatted timestamp `HH:MM:SS,mmm`.
pub fn format_timestamp_srt(tick: Tick) -> String {
    let total_ms = tick.as_millis().max(0);
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let secs = total_secs % 60;
    let total_mins = total_secs / 60;
    let mins = total_mins % 60;
    let hours = total_mins / 60;
    format!("{hours:02}:{mins:02}:{secs:02},{ms:03}")
}

/// Converts a time tick to a WebVTT formatted timestamp `HH:MM:SS.mmm`.
pub fn format_timestamp_vtt(tick: Tick) -> String {
    let total_ms = tick.as_millis().max(0);
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let secs = total_secs % 60;
    let total_mins = total_secs / 60;
    let mins = total_mins % 60;
    let hours = total_mins / 60;
    format!("{hours:02}:{mins:02}:{secs:02}.{ms:03}")
}

/// Parses an SRT or WebVTT timestamp into `Tick`.
/// Supports `HH:MM:SS,mmm`, `HH:MM:SS.mmm`, `MM:SS.mmm`, and `MM:SS,mmm`.
pub fn parse_timestamp(s: &str) -> Result<Tick, String> {
    let clean = s.trim();
    let (time_part, ms_part) = if let Some((t, m)) = clean.split_once(',') {
        (t, m)
    } else if let Some((t, m)) = clean.split_once('.') {
        (t, m)
    } else {
        (clean, "000")
    };

    let ms: i64 = ms_part
        .chars()
        .take(3)
        .collect::<String>()
        .parse::<i64>()
        .map_err(|_| format!("Invalid millisecond in timestamp `{s}`"))?;
    // Pad millisecond if less than 3 digits (e.g. ".5" -> 500, ".50" -> 500)
    let ms = match ms_part.len() {
        1 => ms * 100,
        2 => ms * 10,
        _ => ms,
    };

    let time_tokens: Vec<&str> = time_part.split(':').collect();
    let (hours, mins, secs): (i64, i64, i64) = match time_tokens.as_slice() {
        [h, m, sec] => (
            h.parse()
                .map_err(|_| format!("Invalid hour in timestamp `{s}`"))?,
            m.parse()
                .map_err(|_| format!("Invalid minute in timestamp `{s}`"))?,
            sec.parse()
                .map_err(|_| format!("Invalid second in timestamp `{s}`"))?,
        ),
        [m, sec] => (
            0,
            m.parse()
                .map_err(|_| format!("Invalid minute in timestamp `{s}`"))?,
            sec.parse()
                .map_err(|_| format!("Invalid second in timestamp `{s}`"))?,
        ),
        [sec] => (
            0,
            0,
            sec.parse()
                .map_err(|_| format!("Invalid second in timestamp `{s}`"))?,
        ),
        _ => return Err(format!("Unrecognized time format `{s}`")),
    };

    let total_ms = (hours * 3600 + mins * 60 + secs) * 1000 + ms;
    Ok(Tick::from_millis(total_ms))
}

/// Parses SubRip (.srt) subtitle text content into a list of `SubtitleItem`s.
pub fn parse_srt(content: &str) -> Result<Vec<SubtitleItem>, String> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let blocks = normalized.split("\n\n");
    let mut items = Vec::new();
    let mut auto_id = 1u64;

    for block in blocks {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }

        // Find the line with "-->"
        let arrow_idx = lines.iter().position(|l| l.contains("-->"));
        let Some(arrow_idx) = arrow_idx else {
            continue;
        };

        let time_line = lines[arrow_idx];
        let (start_str, end_raw) = time_line
            .split_once("-->")
            .ok_or_else(|| format!("Invalid timing line: {time_line}"))?;

        // End timestamp may have trailing positioning/settings like X1:0 Y1:0
        let end_str = end_raw.split_whitespace().next().unwrap_or(end_raw);

        let start_ticks = parse_timestamp(start_str)?;
        let end_ticks = parse_timestamp(end_str)?;

        if end_ticks.0 <= start_ticks.0 {
            // Skip invalid non-positive duration cues gracefully
            continue;
        }

        // Content lines are everything after arrow_idx
        let text_lines = &lines[(arrow_idx + 1)..];
        let text = text_lines.join("\n");

        if !text.is_empty() {
            items.push(SubtitleItem {
                id: auto_id,
                start_ticks,
                end_ticks,
                text,
                speaker: None,
                style_id: None,
            });
            auto_id = auto_id.saturating_add(1);
        }
    }

    Ok(items)
}

/// Strips WebVTT styling tags like `<c.yellow>`, `<b>`, `</b>`, `<v Speaker>`, etc.
fn clean_vtt_text(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut in_tag = false;
    for c in raw.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result.trim().to_string()
}

/// Parses WebVTT (.vtt) subtitle text content into a list of `SubtitleItem`s.
pub fn parse_vtt(content: &str) -> Result<Vec<SubtitleItem>, String> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.lines().map(str::trim);

    // Verify or skip header
    let mut in_header = true;
    let mut cues_text = Vec::new();

    for line in lines.by_ref() {
        if in_header {
            if line.starts_with("WEBVTT") || line.starts_with("NOTE") || line.is_empty() {
                if line.is_empty() {
                    in_header = false;
                }
                continue;
            } else {
                in_header = false;
            }
        }
        cues_text.push(line);
    }

    let joined = cues_text.join("\n");
    let blocks = joined.split("\n\n");
    let mut items = Vec::new();
    let mut auto_id = 1u64;

    for block in blocks {
        let block_lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if block_lines.is_empty() {
            continue;
        }

        if block_lines[0].starts_with("NOTE") || block_lines[0].starts_with("STYLE") {
            continue;
        }

        let arrow_idx = block_lines.iter().position(|l| l.contains("-->"));
        let Some(arrow_idx) = arrow_idx else {
            continue;
        };

        let time_line = block_lines[arrow_idx];
        let (start_str, end_raw) = time_line
            .split_once("-->")
            .ok_or_else(|| format!("Invalid timing line: {time_line}"))?;

        let end_str = end_raw.split_whitespace().next().unwrap_or(end_raw);

        let start_ticks = parse_timestamp(start_str)?;
        let end_ticks = parse_timestamp(end_str)?;

        if end_ticks.0 <= start_ticks.0 {
            continue;
        }

        let text_lines: Vec<String> = block_lines[(arrow_idx + 1)..]
            .iter()
            .map(|l| clean_vtt_text(l))
            .filter(|l| !l.is_empty())
            .collect();
        let text = text_lines.join("\n");

        if !text.is_empty() {
            items.push(SubtitleItem {
                id: auto_id,
                start_ticks,
                end_ticks,
                text,
                speaker: None,
                style_id: None,
            });
            auto_id = auto_id.saturating_add(1);
        }
    }

    Ok(items)
}

/// Auto-detects whether the content is WebVTT or SRT and parses accordingly.
pub fn detect_and_parse_subtitles(content: &str) -> Result<Vec<SubtitleItem>, String> {
    let trimmed = content.trim_start();
    if trimmed.starts_with("WEBVTT") {
        parse_vtt(content)
    } else {
        parse_srt(content)
    }
}

/// Serializes subtitles into a standard SubRip (.srt) file content string.
pub fn export_srt(subtitles: &[SubtitleItem]) -> String {
    let mut out = String::new();
    for (i, sub) in subtitles.iter().enumerate() {
        let index = i + 1;
        let start = format_timestamp_srt(sub.start_ticks);
        let end = format_timestamp_srt(sub.end_ticks);
        out.push_str(&format!("{index}\n{start} --> {end}\n{}\n\n", sub.text));
    }
    out
}

/// Serializes subtitles into a standard WebVTT (.vtt) file content string.
pub fn export_vtt(subtitles: &[SubtitleItem]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for (i, sub) in subtitles.iter().enumerate() {
        let index = i + 1;
        let start = format_timestamp_vtt(sub.start_ticks);
        let end = format_timestamp_vtt(sub.end_ticks);
        out.push_str(&format!("{index}\n{start} --> {end}\n{}\n\n", sub.text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_conversions() {
        let tick = Tick::from_millis(3661234); // 1h 1m 1s 234ms
        assert_eq!(format_timestamp_srt(tick), "01:01:01,234");
        assert_eq!(format_timestamp_vtt(tick), "01:01:01.234");

        let parsed_srt = parse_timestamp("01:01:01,234").unwrap();
        assert_eq!(parsed_srt, tick);

        let parsed_vtt = parse_timestamp("01:01:01.234").unwrap();
        assert_eq!(parsed_vtt, tick);

        let short_ts = parse_timestamp("02:15.500").unwrap();
        assert_eq!(short_ts, Tick::from_millis(135500));
    }

    #[test]
    fn test_parse_and_export_srt() {
        let raw_srt = r#"1
00:00:01,500 --> 00:00:04,000
Welcome to Project Saturn!

2
00:00:05,200 --> 00:00:08,750
Professional video editing.
Second line of caption.
"#;
        let subs = parse_srt(raw_srt).unwrap();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].start_ticks, Tick::from_millis(1500));
        assert_eq!(subs[0].end_ticks, Tick::from_millis(4000));
        assert_eq!(subs[0].text, "Welcome to Project Saturn!");

        assert_eq!(subs[1].start_ticks, Tick::from_millis(5200));
        assert_eq!(subs[1].end_ticks, Tick::from_millis(8750));
        assert_eq!(
            subs[1].text,
            "Professional video editing.\nSecond line of caption."
        );

        let exported = export_srt(&subs);
        let round_trip = parse_srt(&exported).unwrap();
        assert_eq!(round_trip.len(), 2);
        assert_eq!(round_trip[0].text, subs[0].text);
        assert_eq!(round_trip[1].text, subs[1].text);
    }

    #[test]
    fn test_parse_and_export_vtt() {
        let raw_vtt = r#"WEBVTT

1
00:00:02.000 --> 00:00:05.500 line:80%
<c.yellow>This is stylized caption.</c>

2
00:00:06.000 --> 00:00:10.000
<v Speaker 1>Clear speech text.</v>
"#;
        let subs = parse_vtt(raw_vtt).unwrap();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].start_ticks, Tick::from_millis(2000));
        assert_eq!(subs[0].end_ticks, Tick::from_millis(5500));
        assert_eq!(subs[0].text, "This is stylized caption.");

        assert_eq!(subs[1].start_ticks, Tick::from_millis(6000));
        assert_eq!(subs[1].end_ticks, Tick::from_millis(10000));
        assert_eq!(subs[1].text, "Clear speech text.");

        let exported = export_vtt(&subs);
        assert!(exported.starts_with("WEBVTT\n\n"));
        let round_trip = parse_vtt(&exported).unwrap();
        assert_eq!(round_trip.len(), 2);
        assert_eq!(round_trip[0].text, subs[0].text);
    }
}
