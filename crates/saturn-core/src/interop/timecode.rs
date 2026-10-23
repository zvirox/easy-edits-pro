use crate::time::{FrameRate, Tick};

/// Formats a time `Tick` into a standard SMPTE non-drop timecode `HH:MM:SS:FF`.
pub fn ticks_to_timecode(tick: Tick, fps: FrameRate) -> String {
    let frame_dur = fps.frame_ticks().unwrap_or(10_584_000_000).max(1);
    let total_frames = (tick.0.max(0) / frame_dur) as u64;
    let nom_fps = (fps.numerator / fps.denominator.max(1)).max(1) as u64;

    let frames = total_frames % nom_fps;
    let total_secs = total_frames / nom_fps;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;

    format!("{hours:02}:{mins:02}:{secs:02}:{frames:02}")
}

/// Parses an SMPTE timecode `HH:MM:SS:FF` or `HH:MM:SS;FF` into a `Tick`.
pub fn timecode_to_ticks(tc: &str, fps: FrameRate) -> Result<Tick, String> {
    let clean = tc.trim();
    let tokens: Vec<&str> = clean
        .split(|c| c == ':' || c == ';')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.len() != 4 {
        return Err(format!(
            "Invalid timecode format `{tc}`, expected HH:MM:SS:FF"
        ));
    }

    let h: u64 = tokens[0]
        .parse()
        .map_err(|_| format!("Invalid hours in `{tc}`"))?;
    let m: u64 = tokens[1]
        .parse()
        .map_err(|_| format!("Invalid minutes in `{tc}`"))?;
    let s: u64 = tokens[2]
        .parse()
        .map_err(|_| format!("Invalid seconds in `{tc}`"))?;
    let f: u64 = tokens[3]
        .parse()
        .map_err(|_| format!("Invalid frames in `{tc}`"))?;

    let nom_fps = (fps.numerator / fps.denominator.max(1)).max(1) as u64;
    let total_frames = (h * 3600 + m * 60 + s) * nom_fps + f;
    let frame_dur = fps.frame_ticks().unwrap_or(10_584_000_000);

    Ok(Tick(total_frames as i64 * frame_dur))
}

// =========================================================================
// 1. CMX 3600 EDL (Edit Decision List) Export & Import
// =========================================================================
