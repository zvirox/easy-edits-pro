// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Script-to-take synchronization (Packet 2.1): fountain-style script line
//! extraction plus order-preserving alignment of each line against
//! transcribed takes, ranking takes by how much of the script they realize.
//!
//! The engine command `speech.script_sync_align` receives **already
//! transcribed** takes as parameters — the core stays inference-free while
//! `saturn-perception` produces the transcripts. All match scores are
//! order-preserving token-overlap **heuristics** (plan §4.4), labelled as
//! such in every doc comment and the engine result payload.

use crate::speech::{SpeechSegment, SpeechWord};
use crate::time::Tick;
use serde::{Deserialize, Serialize};

/// One transcript candidate ("take") for script synchronization: the media
/// item id plus its already-transcribed speech segments.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScriptTake {
    pub media_id: u64,
    pub segments: Vec<SpeechSegment>,
}

/// A single script line located inside one take's transcript.
///
/// `match_score` is an **order-preserving token-overlap heuristic** (0..1),
/// not a probability: 1.0 means every script token was found in order, and
/// lower values indicate paraphrase, dropped words or transcription noise.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScriptLineMatch {
    pub line_index: usize,
    pub line_text: String,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub match_score: f64,
}

/// Script-sync alignment for one take.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TakeAlignment {
    pub media_id: u64,
    /// Fraction of script lines that matched this take's transcript.
    pub coverage: f64,
    /// Mean match score across matched lines (heuristic, see above).
    pub mean_match_score: f64,
    pub matched_lines: Vec<ScriptLineMatch>,
}

/// Result of aligning a script against one or more transcribed takes,
/// ranking takes by how much of the script they realize.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScriptSyncReport {
    /// Number of usable dialogue lines extracted from the script.
    pub lines_count: usize,
    pub takes: Vec<TakeAlignment>,
    /// Take with the highest line coverage (ties broken by mean score).
    pub best_take_media_id: Option<u64>,
}

/// Minimum heuristic score for a script line to count as located. 0.6 (not
/// 0.5) because half-overlap on common words ("this is…") matched the wrong
/// location in testing; 0.6 keeps recall while cutting spurious hits.
pub const SCRIPT_LINE_MATCH_FLOOR: f64 = 0.6;

/// Lowercases and strips everything but letters/digits; `None` when nothing
/// survives (punctuation-only tokens cannot match).
fn normalize_script_token(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn tokens_of(text: &str) -> Vec<String> {
    text.split_whitespace().filter_map(normalize_script_token).collect()
}

/// Extracts the dialogue lines of a Fountain-style script.
///
/// A **heuristic** parser (plan §4.4): it drops blank lines, scene headings
/// (`INT.`/`EXT.`/`EST.`/`I/E` prefixed), all-caps lines (character cues,
/// slug fragments, sound calls), forced transitions (`> ...` or `... TO:`),
/// and standalone parentheticals. Lines are trimmed and returned verbatim —
/// capitalization inside dialogue is preserved for display.
pub fn parse_script_lines(script_text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in script_text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('(') && line.ends_with(')') {
            continue; // parenthetical
        }
        if line.starts_with('>') {
            continue; // forced transition
        }
        let upper = line.to_uppercase();
        if upper != line {
            lines.push(line.to_owned());
            continue;
        }
        // All-caps from here on: headings, cues, transitions.
        let head = upper
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('.');
        if matches!(head, "INT" | "EXT" | "EST" | "I" | "E") {
            continue; // scene heading
        }
        if upper.ends_with("TO:") {
            continue; // un-forced transition ("CUT TO:", "FADE OUT.")
        }
        // Remaining all-caps lines are character cues or sound calls —
        // dropped; a fully-caps *dialogue* line would be too (rare in
        // practice; the parser says so honestly).
    }
    lines
}

/// Locates a script line within a run of transcribed words.
///
/// Order-preserving greedy token matching over a bounded window: for each
/// candidate start word, script tokens are matched in transcript order;
/// among equally-complete matches the most compact span wins (a wide match
/// would smear the line across unrelated transcript). Returns
/// `(start_word_index, end_word_index, score)` or `None` under the floor.
/// `from_word` restricts the search to `take_tokens[from_word..]` so
/// callers can keep successive script lines monotonic.
pub fn align_script_line_to_words(
    line_tokens: &[String],
    take_tokens: &[&SpeechWord],
    from_word: usize,
) -> Option<(usize, usize, f64)> {
    if line_tokens.is_empty() || take_tokens.len() <= from_word {
        return None;
    }
    let window = line_tokens.len() * 3 + 4;
    let mut best: Option<(usize, usize, usize)> = None; // start, end, matched
    for start in from_word..take_tokens.len() {
        let mut token_iter = line_tokens.iter();
        let mut wanted = token_iter.next()?;
        let mut matched = 0_usize;
        let mut end = start;
        for (offset, word) in take_tokens[start..].iter().take(window).enumerate() {
            let text = normalize_script_token(&word.word);
            if text.as_deref() == Some(wanted.as_str()) {
                matched += 1;
                end = start + offset;
                match token_iter.next() {
                    Some(next) => wanted = next,
                    None => break,
                }
            }
        }
        if matched == 0 {
            continue;
        }
        let candidate = (start, end, matched);
        let better = match best {
            None => true,
            Some(prior) => {
                // More complete wins; ties prefer the compact span, then
                // the earlier start (deterministic).
                candidate.2 != prior.2 && candidate.2 > prior.2
                    || candidate.2 == prior.2
                        && ((candidate.1 - candidate.0).cmp(&(prior.1 - prior.0))
                            .then(candidate.0.cmp(&prior.0)))
                            == std::cmp::Ordering::Less
            }
        };
        if better {
            best = Some(candidate);
        }
    }
    let (start, end, matched) = best?;
    let score = matched as f64 / line_tokens.len() as f64;
    if score >= SCRIPT_LINE_MATCH_FLOOR {
        Some((start, end, score))
    } else {
        None
    }
}

/// Aligns a script against transcribed take(s) and ranks the takes.
///
/// Pure analysis — no checkpoint, no document access. Line matching is the
/// order-preserving heuristic above; `coverage` counts matched lines, and
/// the best take maximizes (coverage, mean match score).
pub fn sync_script(script_text: &str, takes: &[ScriptTake]) -> ScriptSyncReport {
    let lines = parse_script_lines(script_text);
    let line_tokens: Vec<Vec<String>> = lines.iter().map(|l| tokens_of(l)).collect();

    let mut alignments = Vec::with_capacity(takes.len());
    for take in takes {
        // Flatten this take's transcript into one word stream with ticks.
        let mut take_words: Vec<&SpeechWord> = Vec::new();
        for segment in &take.segments {
            for word in &segment.words {
                if normalize_script_token(&word.word).is_some() {
                    take_words.push(word);
                }
            }
        }
        let mut matched_lines = Vec::new();
        // Script reads in order; require matches to advance monotonically
        // through the transcript so a later line cannot lock onto an
        // earlier line's words.
        let mut cursor = 0_usize;
        for (line_index, (line, tokens)) in lines.iter().zip(&line_tokens).enumerate() {
            if tokens.is_empty() {
                continue;
            }
            if let Some((start, end, score)) =
                align_script_line_to_words(tokens, &take_words, cursor)
            {
                cursor = end + 1;
                matched_lines.push(ScriptLineMatch {
                    line_index,
                    line_text: line.clone(),
                    start_ticks: take_words[start].start_ticks,
                    end_ticks: take_words[end].end_ticks,
                    match_score: score,
                });
            }
        }
        let coverage = if lines.is_empty() {
            0.0
        } else {
            matched_lines.len() as f64 / lines.len() as f64
        };
        let mean_match_score = if matched_lines.is_empty() {
            0.0
        } else {
            matched_lines.iter().map(|m| m.match_score).sum::<f64>()
                / matched_lines.len() as f64
        };
        alignments.push(TakeAlignment {
            media_id: take.media_id,
            coverage,
            mean_match_score,
            matched_lines,
        });
    }

    let best_take_media_id = alignments
        .iter()
        .filter(|a| !a.matched_lines.is_empty())
        .max_by(|a, b| {
            a.coverage
                .partial_cmp(&b.coverage)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.mean_match_score
                        .partial_cmp(&b.mean_match_score)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(b.media_id.cmp(&a.media_id)) // deterministic: smallest id wins ties
        })
        .map(|a| a.media_id);

    ScriptSyncReport {
        lines_count: lines.len(),
        takes: alignments,
        best_take_media_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(word: &str, start_ms: i64, end_ms: i64) -> SpeechWord {
        SpeechWord {
            word: word.to_owned(),
            start_ticks: Tick::from_millis(start_ms),
            end_ticks: Tick::from_millis(end_ms),
            confidence: 0.9,
            is_filler: false,
        }
    }

    fn take(media_id: u64, words: &[(&str, i64, i64)]) -> ScriptTake {
        let speech_words: Vec<SpeechWord> = words
            .iter()
            .map(|(w, s, e)| word(w, *s, *e))
            .collect();
        let start = speech_words.first().map(|w| w.start_ticks).unwrap_or(Tick(0));
        let end = speech_words.last().map(|w| w.end_ticks).unwrap_or(Tick(0));
        let text = speech_words.iter().map(|w| w.word.clone()).collect::<Vec<_>>().join(" ");
        ScriptTake {
            media_id,
            segments: vec![SpeechSegment {
                id: 1,
                start_ticks: start,
                end_ticks: end,
                text,
                speaker: None,
                confidence: 0.9,
                words: speech_words,
            }],
        }
    }

    #[test]
    fn script_parser_keeps_dialogue_and_drops_markup() {
        let script = "\n\
            INT. GARAGE - NIGHT\n\
            \n\
            MAYA\n\
            (quietly)\n\
            We shouldn't be here.\n\
            \n\
            > CUT TO:\n\
            \n\
            EXT. STREET - DAWN\n\
            The street is empty.\n";
        let lines = parse_script_lines(script);
        assert_eq!(
            lines,
            vec!["We shouldn't be here.".to_owned(), "The street is empty.".to_owned()]
        );
    }

    #[test]
    fn script_line_aligns_to_words_in_order() {
        // Take words at 0.5 s per word covering both lines back to back.
        let words: Vec<(&str, i64, i64)> = ("we shouldnt be here the street is empty")
            .split(' ')
            .enumerate()
            .map(|(i, w)| (w, (i as i64) * 500, (i as i64) * 500 + 400))
            .collect();
        let tt = take(7, &words);
        let take_tokens: Vec<&SpeechWord> =
            tt.segments[0].words.iter().collect();
        let line_tokens = tokens_of("We shouldn't be here.");
        let (start, end, score) =
            align_script_line_to_words(&line_tokens, &take_tokens, 0).expect("line located");
        assert_eq!((start, end), (0, 3));
        assert!((score - 1.0).abs() < 1e-9);
        assert_eq!(take_tokens[start].start_ticks, Tick::from_millis(0));
        assert_eq!(
            take_tokens[end].end_ticks,
            Tick::from_millis(1900) // word 3 spans 1500..1900
        );
    }

    #[test]
    fn sync_script_ranks_takes_by_coverage() {
        let script = "INT. LAB - DAY\n\nThis is the good version.\n\nThis is take two.\n";
        // Take 1 realizes every line; take 2 misses the second line.
        let full: Vec<(&str, i64, i64)> = ("this is the good version this is take two")
            .split(' ')
            .enumerate()
            .map(|(i, w)| (w, (i as i64) * 400, (i as i64) * 400 + 300))
            .collect();
        let partial: Vec<(&str, i64, i64)> = ("this is the good version").split(' ')
            .enumerate()
            .map(|(i, w)| (w, (i as i64) * 400, (i as i64) * 400 + 300))
            .collect();
        let report = sync_script(
            script,
            &[take(2, &partial), take(1, &full)],
        );
        assert_eq!(report.lines_count, 2);
        assert_eq!(report.best_take_media_id, Some(1));
        let best = report.takes.iter().find(|a| a.media_id == 1).unwrap();
        assert!((best.coverage - 1.0).abs() < 1e-9);
        let other = report.takes.iter().find(|a| a.media_id == 2).unwrap();
        assert!((other.coverage - 0.5).abs() < 1e-9);
        assert_eq!(best.matched_lines.len(), 2);
        // First line of take 1 starts at word 0, second line at word 5.
        assert_eq!(best.matched_lines[0].start_ticks, Tick::from_millis(0));
        assert_eq!(best.matched_lines[1].start_ticks, Tick::from_millis(2000));
    }

    #[test]
    fn sync_script_reports_no_match_honestly() {
        let report = sync_script("Completely unrelated dialogue.", &[take(
            4,
            &[("hello", 0, 300)],
        )]);
        assert_eq!(report.lines_count, 1);
        assert_eq!(report.best_take_media_id, None);
        let alignment = &report.takes[0];
        assert!((alignment.coverage).abs() < 1e-9);
        assert!(alignment.matched_lines.is_empty());
    }
}
