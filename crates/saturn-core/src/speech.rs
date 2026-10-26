use crate::project::{Clip, Sequence, SubtitleItem};
use crate::timeline_edits::{cut_track_clips_against_ranges, merge_time_ranges};
use crate::time::Tick;
use serde::{Deserialize, Serialize};

/// A single transcribed word with precise timecode boundaries.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpeechWord {
    pub word: String,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    #[serde(default = "default_word_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub is_filler: bool,
}

fn default_word_confidence() -> f64 {
    0.95
}

/// A transcribed speech sentence or segment with timestamp and word breakdown.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpeechSegment {
    pub id: u64,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub text: String,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default = "default_word_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub words: Vec<SpeechWord>,
}

/// Options configuring speech transcription.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TranscriptionOptions {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default = "default_model_name")]
    pub model: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default = "default_detect_fillers")]
    pub detect_fillers: bool,
}

fn default_model_name() -> String {
    "whisper-base-en".to_string()
}

fn default_detect_fillers() -> bool {
    true
}

impl Default for TranscriptionOptions {
    fn default() -> Self {
        Self {
            language: Some("en".to_string()),
            model: default_model_name(),
            prompt: None,
            detect_fillers: true,
        }
    }
}

/// A detected silence time interval with millisecond duration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilenceInterval {
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub duration_ms: i64,
}

/// Parameters configuring audio silence detection.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SilenceDetectOptions {
    /// Silence threshold in dBFS (e.g. -35.0 dBFS). Any level below this is silence.
    #[serde(default = "default_threshold_db")]
    pub threshold_db: f64,
    /// Minimum silence duration in milliseconds to qualify for a pause cut (e.g. 400ms).
    #[serde(default = "default_min_duration_ms")]
    pub min_duration_ms: u64,
    /// Padding in milliseconds to leave on edges so speech isn't clipped abruptly (e.g. 50ms).
    #[serde(default = "default_padding_ms")]
    pub padding_ms: u64,
}

fn default_threshold_db() -> f64 {
    -35.0
}

fn default_min_duration_ms() -> u64 {
    400
}

fn default_padding_ms() -> u64 {
    50
}

impl Default for SilenceDetectOptions {
    fn default() -> Self {
        Self {
            threshold_db: default_threshold_db(),
            min_duration_ms: default_min_duration_ms(),
            padding_ms: default_padding_ms(),
        }
    }
}

/// Summary report returned after a multi-range ripple delete operation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RippleDeleteReport {
    pub deleted_ranges_count: usize,
    pub total_duration_removed_ticks: Tick,
    pub tracks_affected: usize,
    pub clips_modified: usize,
    pub subtitles_adjusted: usize,
}

/// List of standard hesitation and verbal filler words in speech.
pub const STANDARD_FILLER_WORDS: &[&str] = &[
    "um",
    "uh",
    "er",
    "ah",
    "hmm",
    "like",
    "you know",
    "basically",
    "literally",
    "sort of",
    "kind of",
    "i mean",
    "right",
    "actually",
];

/// Tests whether a given word string is considered a verbal hesitation filler.
pub fn is_filler_word(raw: &str, custom_fillers: Option<&[&str]>) -> bool {
    let clean = raw
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ')
        .to_lowercase();
    if clean.is_empty() {
        return false;
    }

    if let Some(custom) = custom_fillers {
        if custom.iter().any(|&f| f.eq_ignore_ascii_case(&clean)) {
            return true;
        }
    }

    STANDARD_FILLER_WORDS
        .iter()
        .any(|&f| f.eq_ignore_ascii_case(&clean))
}

/// Detects and returns all filler words from speech segments.
pub fn detect_filler_words(
    segments: &[SpeechSegment],
    custom_fillers: Option<&[&str]>,
) -> Vec<SpeechWord> {
    let mut fillers = Vec::new();
    for seg in segments {
        for word in &seg.words {
            if word.is_filler || is_filler_word(&word.word, custom_fillers) {
                let mut w = word.clone();
                w.is_filler = true;
                fillers.push(w);
            }
        }
    }
    fillers
}

/// Flags filler words within a slice of speech segments.
pub fn flag_filler_words_in_segments(
    segments: &mut [SpeechSegment],
    custom_fillers: Option<&[&str]>,
) {
    for segment in segments {
        for word in &mut segment.words {
            if is_filler_word(&word.word, custom_fillers) {
                word.is_filler = true;
            }
        }
    }
}

/// Extracts all detected filler word intervals from speech segments.
pub fn extract_filler_intervals(
    segments: &[SpeechSegment],
    custom_fillers: Option<&[&str]>,
) -> Vec<SilenceInterval> {
    let mut intervals = Vec::new();
    for seg in segments {
        for word in &seg.words {
            if word.is_filler || is_filler_word(&word.word, custom_fillers) {
                let dur_ms = (word.end_ticks.0 - word.start_ticks.0).max(0)
                    / (crate::time::TICKS_PER_SECOND / 1000);
                intervals.push(SilenceInterval {
                    start_ticks: word.start_ticks,
                    end_ticks: word.end_ticks,
                    duration_ms: dur_ms,
                });
            }
        }
    }
    intervals
}

/// Converts speech segments directly into `SubtitleItem` instances for the sequence subtitle track.
pub fn speech_segments_to_subtitles(segments: &[SpeechSegment]) -> Vec<SubtitleItem> {
    let mut subtitles = Vec::with_capacity(segments.len());
    let mut id = 1u64;
    for seg in segments {
        let clean_text = seg.text.trim();
        if clean_text.is_empty() || seg.end_ticks.0 <= seg.start_ticks.0 {
            continue;
        }
        subtitles.push(SubtitleItem {
            id,
            start_ticks: seg.start_ticks,
            end_ticks: seg.end_ticks,
            text: clean_text.to_string(),
            speaker: None,
            style_id: None,
        });
        id = id.saturating_add(1);
    }
    subtitles
}

/// Analyzes an audio waveform peak profile (0..255 peak amplitude values) to identify
/// silence intervals that exceed `options.min_duration_ms` below `options.threshold_db`.
pub fn detect_silence_from_peaks(
    peaks: &[u8],
    total_duration: Tick,
    options: &SilenceDetectOptions,
) -> Vec<SilenceInterval> {
    if peaks.is_empty() || total_duration.0 <= 0 {
        return Vec::new();
    }

    // Convert dBFS threshold to 0..255 linear amplitude threshold:
    // amp = 255.0 * 10^(dBFS / 20)
    let linear_threshold =
        (255.0 * 10.0f64.powf(options.threshold_db / 20.0)).clamp(1.0, 254.0) as u8;

    let num_bins = peaks.len();
    let ticks_per_bin = total_duration.0 / num_bins as i64;
    if ticks_per_bin <= 0 {
        return Vec::new();
    }

    let min_ticks = (options.min_duration_ms as i64) * (crate::time::TICKS_PER_SECOND / 1000);
    let pad_ticks = (options.padding_ms as i64) * (crate::time::TICKS_PER_SECOND / 1000);

    let mut intervals = Vec::new();
    let mut in_silence = false;
    let mut silence_start_bin = 0;

    for (i, &amp) in peaks.iter().enumerate() {
        let is_silent = amp <= linear_threshold;
        if is_silent && !in_silence {
            in_silence = true;
            silence_start_bin = i;
        } else if !is_silent && in_silence {
            in_silence = false;
            let start_tick = Tick(silence_start_bin as i64 * ticks_per_bin);
            let end_tick = Tick(i as i64 * ticks_per_bin);
            let raw_dur = end_tick.0 - start_tick.0;

            if raw_dur >= min_ticks {
                // Apply speech edge padding
                let effective_start = Tick((start_tick.0 + pad_ticks).min(end_tick.0));
                let effective_end = Tick((end_tick.0 - pad_ticks).max(effective_start.0));
                let eff_dur = effective_end.0 - effective_start.0;
                if eff_dur > 0 {
                    intervals.push(SilenceInterval {
                        start_ticks: effective_start,
                        end_ticks: effective_end,
                        duration_ms: eff_dur / (crate::time::TICKS_PER_SECOND / 1000),
                    });
                }
            }
        }
    }

    // Handle trailing silence at the end of the timeline
    if in_silence {
        let start_tick = Tick(silence_start_bin as i64 * ticks_per_bin);
        let end_tick = total_duration;
        let raw_dur = end_tick.0 - start_tick.0;
        if raw_dur >= min_ticks {
            let effective_start = Tick((start_tick.0 + pad_ticks).min(end_tick.0));
            let effective_end = end_tick;
            let eff_dur = effective_end.0 - effective_start.0;
            if eff_dur > 0 {
                intervals.push(SilenceInterval {
                    start_ticks: effective_start,
                    end_ticks: effective_end,
                    duration_ms: eff_dur / (crate::time::TICKS_PER_SECOND / 1000),
                });
            }
        }
    }

    intervals
}

/// Performs a multi-range ripple deletion across all unlocked tracks in a sequence.
///
/// Cuts clips intersecting any deleted interval, removes the gap segments, and shifts
/// all subsequent clips on unlocked tracks backward so gaps are completely closed.
/// Also shifts sequence subtitles to preserve exact speech-to-subtitle synchronization.
pub fn ripple_delete_time_ranges(
    sequence: &mut Sequence,
    ranges: &[(Tick, Tick)],
) -> Result<RippleDeleteReport, String> {
    let merged = merge_time_ranges(ranges);
    if merged.is_empty() {
        return Ok(RippleDeleteReport {
            deleted_ranges_count: 0,
            total_duration_removed_ticks: Tick(0),
            tracks_affected: 0,
            clips_modified: 0,
            subtitles_adjusted: 0,
        });
    }

    let total_removed_ticks: i64 = merged.iter().map(|(s, e)| e.0 - s.0).sum();
    let mut tracks_affected = 0;
    let mut clips_modified = 0;

    // Process each unlocked track
    for track in &mut sequence.tracks {
        if track.locked {
            continue;
        }

        let (fragments, track_had_changes, modified) =
            cut_track_clips_against_ranges(std::mem::take(&mut track.clips), &merged);
        clips_modified += modified;

        // Shift fragments left based on all merged ranges that occurred before them
        let mut new_clips: Vec<Clip> = Vec::new();
        for mut frag in fragments {
            let mut shift = 0i64;
            for &(range_start, range_end) in &merged {
                let r_start = range_start.0;
                let r_end = range_end.0;
                let r_dur = r_end - r_start;

                if frag.timeline_start.0 >= r_end {
                    shift += r_dur;
                } else if frag.timeline_start.0 > r_start {
                    shift += frag.timeline_start.0 - r_start;
                }
            }

            frag.timeline_start = Tick((frag.timeline_start.0 - shift).max(0));
            new_clips.push(frag);
        }

        if track_had_changes {
            tracks_affected += 1;
        }

        // Re-sort clips by timeline start
        new_clips.sort_by_key(|c| c.timeline_start.0);
        track.clips = new_clips;
    }

    // Shift and trim caption cues so captions stay in sync with spoken video
    let mut subtitles_adjusted = 0usize;

    for track in &mut sequence.caption_tracks {
        let mut new_cues = Vec::with_capacity(track.cues.len());

        for sub in std::mem::take(&mut track.cues) {
            let sub_start = sub.start_ticks.0;
            let sub_end = sub.end_ticks.0;

            // Check if the cue lies completely inside a deleted range
            let inside_deleted = merged
                .iter()
                .any(|(s, e)| sub_start >= s.0 && sub_end <= e.0);
            if inside_deleted {
                subtitles_adjusted += 1;
                continue;
            }

            let mut shift_start = 0i64;
            let mut shift_end = 0i64;

            for &(range_start, range_end) in &merged {
                let r_start = range_start.0;
                let r_end = range_end.0;
                let r_dur = r_end - r_start;

                if sub_start >= r_end {
                    shift_start += r_dur;
                } else if sub_start > r_start {
                    shift_start += sub_start - r_start;
                }

                if sub_end >= r_end {
                    shift_end += r_dur;
                } else if sub_end > r_start {
                    shift_end += sub_end - r_start;
                }
            }

            let new_start = (sub_start - shift_start).max(0);
            let new_end = (sub_end - shift_end).max(new_start + 1);

            if new_end > new_start {
                if shift_start > 0 || shift_end > 0 {
                    subtitles_adjusted += 1;
                }
                new_cues.push(SubtitleItem {
                    id: sub.id,
                    start_ticks: Tick(new_start),
                    end_ticks: Tick(new_end),
                    text: sub.text,
                    speaker: sub.speaker,
                    style_id: sub.style_id,
                });
            }
        }

        track.cues = new_cues;
    }

    Ok(RippleDeleteReport {
        deleted_ranges_count: merged.len(),
        total_duration_removed_ticks: Tick(total_removed_ticks),
        tracks_affected,
        clips_modified,
        subtitles_adjusted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{CaptionTrack, Track, TrackKind};

    #[test]
    fn test_silence_detection_from_peaks() {
        // 100 peaks representing a 10-second sequence (100 ms per bin)
        // Bins 0..30 loud (200), bins 30..60 quiet (5), bins 60..100 loud (200)
        let mut peaks = vec![200u8; 100];
        for p in peaks.iter_mut().take(60).skip(30) {
            *p = 5; // -34 dBFS or lower
        }

        let total_duration = Tick::from_millis(10_000);
        let options = SilenceDetectOptions {
            threshold_db: -30.0,
            min_duration_ms: 1000,
            padding_ms: 100,
        };

        let silences = detect_silence_from_peaks(&peaks, total_duration, &options);
        assert_eq!(silences.len(), 1);
        let s = &silences[0];
        // 3000ms + 100ms padding = 3100ms, 6000ms - 100ms padding = 5900ms
        assert_eq!(s.start_ticks.as_millis(), 3100);
        assert_eq!(s.end_ticks.as_millis(), 5900);
        assert_eq!(s.duration_ms, 2800);
    }

    #[test]
    fn test_filler_word_detection() {
        assert!(is_filler_word("um", None));
        assert!(is_filler_word("Uh...", None));
        assert!(is_filler_word("LIKE", None));
        assert!(!is_filler_word("camera", None));
        assert!(is_filler_word("so basically", Some(&["so basically"])));

        let mut segs = vec![SpeechSegment {
            id: 1,
            start_ticks: Tick(0),
            end_ticks: Tick(100),
            text: "Um hello world".to_string(),
            speaker: None,
            confidence: 0.95,
            words: vec![
                SpeechWord {
                    word: "Um".to_string(),
                    start_ticks: Tick(0),
                    end_ticks: Tick(20),
                    confidence: 0.98,
                    is_filler: false,
                },
                SpeechWord {
                    word: "hello".to_string(),
                    start_ticks: Tick(25),
                    end_ticks: Tick(60),
                    confidence: 0.99,
                    is_filler: false,
                },
            ],
        }];

        flag_filler_words_in_segments(&mut segs, None);
        assert!(segs[0].words[0].is_filler);
        assert!(!segs[0].words[1].is_filler);

        let fillers = extract_filler_intervals(&segs, None);
        assert_eq!(fillers.len(), 1);
        assert_eq!(fillers[0].start_ticks, Tick(0));
        assert_eq!(fillers[0].end_ticks, Tick(20));
    }

    #[test]
    fn test_multi_range_ripple_deletion() {
        // A sequence with 1 video track and 1 clip from 0 to 10s (10,000 ms)
        let clip = Clip {
            media_id: 1,
            timeline_start: Tick::from_millis(0),
            source_in: Tick::from_millis(0),
            duration: Tick::from_millis(10_000),
            speed: 1.0,
            group_id: None,
            color: Default::default(),
            effects: Vec::new(),
            keyframes: Vec::new(),
            multicam_id: None,
            multicam_angle_id: None,
            tracking: None,
            node_graph: None,
            speed_ramp: None,
            motion_blur: None,
            compound_sequence_id: None,
                label: String::new(),
        };

        let mut seq = Sequence {
            id: 1,
            name: "Main".to_string(),
            track_groups: Vec::new(),
            tracks: vec![Track {
                id: 1,
                group_id: None,
                kind: TrackKind::Video,
                name: "V1".to_string(),
                gain_db: 0.0,
                pan: 0.0,
                muted: false,
                solo: false,
                locked: false,
                audio_processing: Default::default(),
                bus: Default::default(),
                automation: Default::default(),
                clips: vec![clip],
                transitions: Vec::new(),
            }],
            mark_in: None,
            mark_out: None,
            graphics: Vec::new(),
            caption_tracks: vec![CaptionTrack {
                id: 1,
                name: "Captions".into(),
                language: "en".into(),
                visible: true,
                cues: vec![
                    SubtitleItem {
                        id: 1,
                        start_ticks: Tick::from_millis(1000),
                        end_ticks: Tick::from_millis(2500),
                        text: "Opening speech".to_string(),
                        speaker: None,
                        style_id: None,
                    },
                    SubtitleItem {
                        id: 2,
                        start_ticks: Tick::from_millis(4000),
                        end_ticks: Tick::from_millis(4500),
                        text: "Um...".to_string(), // In silence / filler range
                        speaker: None,
                        style_id: None,
                    },
                    SubtitleItem {
                        id: 3,
                        start_ticks: Tick::from_millis(8000),
                        end_ticks: Tick::from_millis(9500),
                        text: "Closing remarks".to_string(),
                        speaker: None,
                        style_id: None,
                    },
                ],
                styles: Vec::new(),
            }],
            markers: Vec::new(),
            multicam_clips: Vec::new(),
            buses: Vec::new(),
            node_graph: None,
        };

        // Two silence cuts: 3s to 5s (2000ms), and 7s to 8s (1000ms)
        let ranges = vec![
            (Tick::from_millis(3000), Tick::from_millis(5000)),
            (Tick::from_millis(7000), Tick::from_millis(8000)),
        ];

        let report = ripple_delete_time_ranges(&mut seq, &ranges).expect("ripple delete succeeds");
        assert_eq!(report.deleted_ranges_count, 2);
        assert_eq!(report.total_duration_removed_ticks.as_millis(), 3000);
        assert_eq!(report.tracks_affected, 1);

        // Sequence tracks should now have 3 segments:
        // 1: 0ms..3000ms (dur 3000ms)
        // 2: 3000ms..5000ms (dur 2000ms, source_in: 5000ms)
        // 3: 5000ms..7000ms (dur 2000ms, source_in: 8000ms)
        let clips = &seq.tracks[0].clips;
        assert_eq!(clips.len(), 3);

        assert_eq!(clips[0].timeline_start.as_millis(), 0);
        assert_eq!(clips[0].duration.as_millis(), 3000);
        assert_eq!(clips[0].source_in.as_millis(), 0);

        assert_eq!(clips[1].timeline_start.as_millis(), 3000);
        assert_eq!(clips[1].duration.as_millis(), 2000);
        assert_eq!(clips[1].source_in.as_millis(), 5000);

        assert_eq!(clips[2].timeline_start.as_millis(), 5000);
        assert_eq!(clips[2].duration.as_millis(), 2000);
        assert_eq!(clips[2].source_in.as_millis(), 8000);

        // Total sequence duration should now be 7000ms (10000 - 3000)
        assert_eq!(seq.duration().as_millis(), 7000);

        // Cue 2 (4000..4500) was inside the first deleted range (3000..5000) and removed!
        // Cue 3 (8000..9500) was shifted left by 3000ms -> now 5000..6500!
        let cues = &seq.caption_tracks[0].cues;
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Opening speech");
        assert_eq!(cues[1].text, "Closing remarks");
        assert_eq!(cues[1].start_ticks.as_millis(), 5000);
        assert_eq!(cues[1].end_ticks.as_millis(), 6500);
    }

}
