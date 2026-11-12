// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Pure, deterministic content-planning analyzers for the three flagship Task Sheet recipes.
//!
//! These helpers evaluate caller-supplied transcripts (`SpeechSegment` / `SpeechWord`),
//! optional measured energy hints, and human-owned constraints (`must_include`, `must_not`,
//! duration windows) so CLI/MCP agents compute reproducible cut lists without inventing quality
//! metrics.

use serde::{Deserialize, Serialize};

use crate::speech::{SpeechSegment, SpeechWord};
use crate::tasksheet::types::SheetMustNot;
use crate::time::{Tick, TICKS_PER_SECOND};

/// Configuration for Recipe 1 (`podcast-to-clips`) candidate selection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PodcastClipRankParams {
    #[serde(default = "default_clip_target_s")]
    pub target_duration_s: f64,
    #[serde(default = "default_clip_tolerance_s")]
    pub tolerance_s: f64,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default)]
    pub must_include: Vec<String>,
    #[serde(default)]
    pub must_not: Option<SheetMustNot>,
    /// Optional per-segment measured RMS/energy values aligned with `segments`.
    #[serde(default)]
    pub segment_energy_hints: Vec<f64>,
}

fn default_clip_target_s() -> f64 {
    45.0
}

fn default_clip_tolerance_s() -> f64 {
    15.0
}

fn default_top_k() -> usize {
    3
}

impl Default for PodcastClipRankParams {
    fn default() -> Self {
        Self {
            target_duration_s: default_clip_target_s(),
            tolerance_s: default_clip_tolerance_s(),
            top_k: default_top_k(),
            must_include: Vec::new(),
            must_not: None,
            segment_energy_hints: Vec::new(),
        }
    }
}

/// Measured criteria requiring human judgment review (Recipe 1 acceptance.human).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodcastReviewFlag {
    pub criterion: String,
    pub target_tick: Tick,
    pub end_tick: Tick,
    pub prompt: String,
}

/// Ranked vertical-clip candidate strictly aligned to word boundaries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PodcastClipCandidate {
    pub rank: usize,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub duration_s: f64,
    pub text: String,
    pub word_count: usize,
    pub word_boundary_aligned: bool,
    pub speech_energy_score: f64,
    pub energy_source: String,
    pub topic_match_score: f64,
    pub sentence_completeness_score: f64,
    pub duration_fit_score: f64,
    pub composite_score: f64,
    pub matched_topics: Vec<String>,
    pub review_flags: Vec<PodcastReviewFlag>,
}

/// Output from Recipe 1 transcript ranking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PodcastRankReport {
    pub candidates: Vec<PodcastClipCandidate>,
    pub rejected_must_not_segments: usize,
    pub energy_source: String,
}

/// Configuration for Recipe 3 (`webinar-recap`) analysis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebinarAnalyzeParams {
    #[serde(default = "default_reel_target_s")]
    pub reel_target_s: f64,
    #[serde(default = "default_reel_tolerance_s")]
    pub reel_tolerance_s: f64,
    #[serde(default = "default_chapter_gap_s")]
    pub chapter_gap_s: f64,
    #[serde(default)]
    pub must_include: Vec<String>,
    #[serde(default)]
    pub must_not: Option<SheetMustNot>,
}

fn default_reel_target_s() -> f64 {
    120.0
}

fn default_reel_tolerance_s() -> f64 {
    30.0
}

fn default_chapter_gap_s() -> f64 {
    2.5
}

impl Default for WebinarAnalyzeParams {
    fn default() -> Self {
        Self {
            reel_target_s: default_reel_target_s(),
            reel_tolerance_s: default_reel_tolerance_s(),
            chapter_gap_s: default_chapter_gap_s(),
            must_include: Vec::new(),
            must_not: None,
        }
    }
}

/// Derived topical chapter for a webinar timeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebinarChapter {
    pub index: usize,
    pub title: String,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub segment_ids: Vec<u64>,
    pub matched_topics: Vec<String>,
}

/// Coverage status of a human-specified `must_include` topic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebinarTopicCoverage {
    pub requirement: String,
    pub covered: bool,
    pub matching_segment_ids: Vec<u64>,
    pub first_tick: Option<Tick>,
}

/// Segment flagged for violating `must_not` rules (flagged in full edit, excluded from reel).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebinarFlaggedHit {
    pub segment_id: u64,
    pub category: String,
    pub term: String,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub excerpt: String,
}

/// Highlight selection entry for the summary reel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebinarHighlight {
    pub segment_id: u64,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub duration_s: f64,
    pub matched_topics: Vec<String>,
    pub score: f64,
    pub text: String,
}

/// Aggregate Recipe 3 webinar analysis report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebinarAnalysisReport {
    pub chapters: Vec<WebinarChapter>,
    pub topic_coverage: Vec<WebinarTopicCoverage>,
    pub missing_required_topics: Vec<String>,
    pub flagged_must_not: Vec<WebinarFlaggedHit>,
    pub highlights: Vec<WebinarHighlight>,
    pub reel_duration_s: f64,
    pub escalation_needed: bool,
}

fn normalize_topic_tokens(raw: &str) -> Vec<String> {
    let stripped = raw
        .trim()
        .strip_prefix("topic:")
        .unwrap_or(raw.trim());
    stripped
        .split(|c: char| !c.is_alphanumeric())
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| s.len() >= 2)
        .collect()
}

fn text_matches_requirement(text_lower: &str, requirement: &str) -> bool {
    let tokens = normalize_topic_tokens(requirement);
    if tokens.is_empty() {
        return false;
    }
    let matched = tokens
        .iter()
        .filter(|tok| text_lower.contains(tok.as_str()))
        .count();
    matched == tokens.len() || (tokens.len() >= 2 && matched * 2 >= tokens.len() + 1)
}

fn find_must_not_hit<'a>(text: &str, must_not: &'a SheetMustNot) -> Option<(&'static str, &'a str)> {
    let lower = text.to_ascii_lowercase();
    for term in &must_not.words {
        let needle = term.trim().to_ascii_lowercase();
        if !needle.is_empty() && lower.contains(&needle) {
            return Some(("word", term.as_str()));
        }
    }
    for term in &must_not.people {
        let needle = term.trim().to_ascii_lowercase();
        if !needle.is_empty() && lower.contains(&needle) {
            return Some(("person", term.as_str()));
        }
    }
    for term in &must_not.content {
        let needle = term.trim().to_ascii_lowercase();
        if !needle.is_empty() && lower.contains(&needle) {
            return Some(("content", term.as_str()));
        }
    }
    None
}

fn segment_word_bounds(seg: &SpeechSegment) -> (Tick, Tick, bool) {
    if let (Some(first), Some(last)) = (seg.words.first(), seg.words.last()) {
        let start = if first.start_ticks.0 >= seg.start_ticks.0 {
            first.start_ticks
        } else {
            seg.start_ticks
        };
        let end = if last.end_ticks.0 <= seg.end_ticks.0 && last.end_ticks.0 > start.0 {
            last.end_ticks
        } else {
            seg.end_ticks
        };
        (start, end, true)
    } else {
        (seg.start_ticks, seg.end_ticks, false)
    }
}

fn sentence_completeness(text: &str) -> f64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    let starts_upper = trimmed
        .chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit());
    let ends_punct = trimmed
        .chars()
        .last()
        .is_some_and(|c| matches!(c, '.' | '!' | '?'));
    match (starts_upper, ends_punct) {
        (true, true) => 1.0,
        (false, true) | (true, false) => 0.5,
        (false, false) => 0.0,
    }
}

fn word_count_for_span(segments: &[&SpeechSegment], text: &str) -> usize {
    let explicit: usize = segments.iter().map(|s| s.words.len()).sum();
    if explicit > 0 {
        explicit
    } else {
        text.split_whitespace().count()
    }
}

/// Ranks contiguous transcript spans into word-boundary-aligned clip candidates (Recipe 1).
pub fn rank_podcast_clips(
    segments: &[SpeechSegment],
    params: &PodcastClipRankParams,
) -> Result<PodcastRankReport, String> {
    if segments.is_empty() {
        return Err("Transcript contains zero speech segments".into());
    }
    if params.target_duration_s <= 0.0 || !params.target_duration_s.is_finite() {
        return Err("target_duration_s must be positive".into());
    }
    let top_k = params.top_k.max(1);
    let tolerance = params.tolerance_s.max(0.0);
    let min_s = (params.target_duration_s - tolerance).max(1.0);
    let max_s = params.target_duration_s + tolerance;
    let has_hints = !params.segment_energy_hints.is_empty()
        && params.segment_energy_hints.len() == segments.len();
    let energy_source = if has_hints {
        "measured_segment_energy".to_string()
    } else {
        "speech_rate_proxy".to_string()
    };

    let mut ordered: Vec<(usize, &SpeechSegment)> = segments.iter().enumerate().collect();
    ordered.sort_by_key(|(_, s)| s.start_ticks);

    let mut rejected_must_not_segments = 0usize;
    let mut raw_candidates: Vec<(f64, PodcastClipCandidate)> = Vec::new();

    for i in 0..ordered.len() {
        let mut span_segments: Vec<&SpeechSegment> = Vec::new();
        let mut span_energies: Vec<f64> = Vec::new();
        let mut contains_banned = false;

        for j in i..ordered.len() {
            let (orig_idx, seg) = ordered[j];
            if let Some(ref must_not) = params.must_not {
                if find_must_not_hit(&seg.text, must_not).is_some() {
                    if j == i {
                        rejected_must_not_segments += 1;
                    }
                    contains_banned = true;
                    break;
                }
            }
            span_segments.push(seg);
            if has_hints {
                span_energies.push(params.segment_energy_hints[orig_idx].clamp(0.0, 1.0));
            }

            let first = span_segments.first().expect("non-empty");
            let last = span_segments.last().expect("non-empty");
            let (start_ticks, _, first_aligned) = segment_word_bounds(first);
            let (_, end_ticks, last_aligned) = segment_word_bounds(last);
            if end_ticks.0 <= start_ticks.0 {
                continue;
            }
            let duration_s = (end_ticks.0 - start_ticks.0) as f64 / TICKS_PER_SECOND as f64;
            if duration_s > max_s {
                break;
            }
            // Accept exact range hits, OR single-segment complete utterances when clip pool is short.
            if duration_s < min_s && span_segments.len() == ordered.len() - i {
                // Only keep short trailing tail if no full-window candidates exist.
            } else if duration_s < min_s {
                continue;
            }

            let combined_text: String = span_segments
                .iter()
                .map(|s| s.text.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let text_lower = combined_text.to_ascii_lowercase();
            let word_count = word_count_for_span(&span_segments, &combined_text);

            let speech_energy_score = if has_hints {
                let sum: f64 = span_energies.iter().sum();
                (sum / span_energies.len().max(1) as f64).clamp(0.0, 1.0)
            } else {
                let words_per_s = word_count as f64 / duration_s.max(0.1);
                (words_per_s / 3.2).clamp(0.0, 1.0)
            };

            let mut matched_topics = Vec::new();
            for req in &params.must_include {
                if text_matches_requirement(&text_lower, req) {
                    matched_topics.push(req.clone());
                }
            }
            let topic_match_score = if params.must_include.is_empty() {
                0.5
            } else {
                matched_topics.len() as f64 / params.must_include.len() as f64
            };
            let sentence_completeness_score = sentence_completeness(&combined_text);
            let duration_fit_score = (1.0
                - ((duration_s - params.target_duration_s).abs()
                    / params.target_duration_s.max(1.0)))
            .clamp(0.0, 1.0);

            let composite_score = 0.35 * topic_match_score
                + 0.30 * sentence_completeness_score
                + 0.25 * speech_energy_score
                + 0.10 * duration_fit_score;

            let review_flags = vec![
                PodcastReviewFlag {
                    criterion: "hook".into(),
                    target_tick: start_ticks,
                    end_tick: end_ticks,
                    prompt: "Human check: verify opening 2 seconds deliver a compelling hook.".into(),
                },
                PodcastReviewFlag {
                    criterion: "tone".into(),
                    target_tick: start_ticks,
                    end_tick: end_ticks,
                    prompt: "Human check: confirm editorial tone fits social channel audience.".into(),
                },
                PodcastReviewFlag {
                    criterion: "ending".into(),
                    target_tick: start_ticks,
                    end_tick: end_ticks,
                    prompt: "Human check: confirm closing line lands cleanly without trailing breath.".into(),
                },
            ];

            raw_candidates.push((
                composite_score,
                PodcastClipCandidate {
                    rank: 0,
                    start_ticks,
                    end_ticks,
                    duration_s,
                    text: combined_text,
                    word_count,
                    word_boundary_aligned: first_aligned && last_aligned,
                    speech_energy_score,
                    energy_source: energy_source.clone(),
                    topic_match_score,
                    sentence_completeness_score,
                    duration_fit_score,
                    composite_score,
                    matched_topics,
                    review_flags,
                },
            ));
        }

        if contains_banned {
            continue;
        }
    }

    // Fallback when transcript total duration is shorter than min_s: emit best single non-banned span.
    if raw_candidates.is_empty() && rejected_must_not_segments < ordered.len() {
        for (orig_idx, seg) in &ordered {
            if let Some(ref must_not) = params.must_not {
                if find_must_not_hit(&seg.text, must_not).is_some() {
                    continue;
                }
            }
            let (start_ticks, end_ticks, aligned) = segment_word_bounds(seg);
            if end_ticks.0 <= start_ticks.0 {
                continue;
            }
            let duration_s = (end_ticks.0 - start_ticks.0) as f64 / TICKS_PER_SECOND as f64;
            let text_lower = seg.text.to_ascii_lowercase();
            let word_count = word_count_for_span(&[seg], &seg.text);
            let speech_energy_score = if has_hints {
                params.segment_energy_hints[*orig_idx].clamp(0.0, 1.0)
            } else {
                ((word_count as f64 / duration_s.max(0.1)) / 3.2).clamp(0.0, 1.0)
            };
            let matched_topics: Vec<String> = params
                .must_include
                .iter()
                .filter(|r| text_matches_requirement(&text_lower, r))
                .cloned()
                .collect();
            let topic_match_score = if params.must_include.is_empty() {
                0.5
            } else {
                matched_topics.len() as f64 / params.must_include.len() as f64
            };
            let sentence_completeness_score = sentence_completeness(&seg.text);
            let duration_fit_score = (duration_s / params.target_duration_s.max(1.0)).clamp(0.0, 1.0);
            let composite_score = 0.35 * topic_match_score
                + 0.30 * sentence_completeness_score
                + 0.25 * speech_energy_score
                + 0.10 * duration_fit_score;

            raw_candidates.push((
                composite_score,
                PodcastClipCandidate {
                    rank: 0,
                    start_ticks,
                    end_ticks,
                    duration_s,
                    text: seg.text.clone(),
                    word_count,
                    word_boundary_aligned: aligned,
                    speech_energy_score,
                    energy_source: energy_source.clone(),
                    topic_match_score,
                    sentence_completeness_score,
                    duration_fit_score,
                    composite_score,
                    matched_topics,
                    review_flags: vec![PodcastReviewFlag {
                        criterion: "hook".into(),
                        target_tick: start_ticks,
                        end_tick: end_ticks,
                        prompt: "Human check: short source span; verify hook, tone, and ending fit.".into(),
                    }],
                },
            ));
        }
    }

    raw_candidates.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.start_ticks.cmp(&b.1.start_ticks))
    });

    // Select non-overlapping top-K candidates.
    let mut selected = Vec::new();
    for (_, mut candidate) in raw_candidates {
        let overlaps = selected.iter().any(|existing: &PodcastClipCandidate| {
            candidate.start_ticks < existing.end_ticks && candidate.end_ticks > existing.start_ticks
        });
        if !overlaps {
            candidate.rank = selected.len() + 1;
            selected.push(candidate);
            if selected.len() >= top_k {
                break;
            }
        }
    }

    Ok(PodcastRankReport {
        candidates: selected,
        rejected_must_not_segments,
        energy_source,
    })
}

/// Validates and aligns a `SpeechWord` slice lookup (utility exposed for tests).
pub fn find_word_boundary(words: &[SpeechWord], target: Tick) -> Option<Tick> {
    words
        .iter()
        .min_by_key(|w| (w.start_ticks.0 - target.0).abs())
        .map(|w| w.start_ticks)
}

/// Derives chapters, topic coverage, `must_not` flags, and summary highlights for Recipe 3.
pub fn analyze_webinar_recap(
    segments: &[SpeechSegment],
    params: &WebinarAnalyzeParams,
) -> Result<WebinarAnalysisReport, String> {
    if segments.is_empty() {
        return Err("Webinar transcript contains zero speech segments".into());
    }

    let mut ordered: Vec<&SpeechSegment> = segments.iter().collect();
    ordered.sort_by_key(|s| s.start_ticks);

    // 1. Flag `must_not` violations so full edit can mark them and summary reel excludes them.
    let mut flagged_must_not = Vec::new();
    let mut flagged_segment_ids = std::collections::HashSet::new();
    if let Some(ref must_not) = params.must_not {
        for seg in &ordered {
            if let Some((category, term)) = find_must_not_hit(&seg.text, must_not) {
                flagged_segment_ids.insert(seg.id);
                flagged_must_not.push(WebinarFlaggedHit {
                    segment_id: seg.id,
                    category: category.into(),
                    term: term.into(),
                    start_ticks: seg.start_ticks,
                    end_ticks: seg.end_ticks,
                    excerpt: seg.text.chars().take(120).collect(),
                });
            }
        }
    }

    // 2. Check `must_include` topic coverage.
    let mut topic_coverage = Vec::new();
    let mut missing_required_topics = Vec::new();
    for req in &params.must_include {
        let mut matching_ids = Vec::new();
        let mut first_tick = None;
        for seg in &ordered {
            let lower = seg.text.to_ascii_lowercase();
            if text_matches_requirement(&lower, req) {
                matching_ids.push(seg.id);
                if first_tick.is_none() {
                    first_tick = Some(seg.start_ticks);
                }
            }
        }
        let covered = !matching_ids.is_empty();
        if !covered {
            missing_required_topics.push(req.clone());
        }
        topic_coverage.push(WebinarTopicCoverage {
            requirement: req.clone(),
            covered,
            matching_segment_ids: matching_ids,
            first_tick,
        });
    }

    // 3. Build topical chapters on pause gaps or topic transitions.
    let gap_threshold_ticks = (params.chapter_gap_s.max(0.5) * TICKS_PER_SECOND as f64) as i64;
    let mut chapters: Vec<WebinarChapter> = Vec::new();
    let mut cur_ids: Vec<u64> = Vec::new();
    let mut cur_topics: Vec<String> = Vec::new();
    let mut cur_start = ordered[0].start_ticks;
    let mut cur_end = ordered[0].end_ticks;
    let mut cur_title = chapter_title_from_text(&ordered[0].text, 1);

    for (idx, seg) in ordered.iter().enumerate() {
        let seg_topics: Vec<String> = params
            .must_include
            .iter()
            .filter(|req| text_matches_requirement(&seg.text.to_ascii_lowercase(), req))
            .cloned()
            .collect();
        let gap = if idx > 0 {
            seg.start_ticks.0.saturating_sub(cur_end.0)
        } else {
            0
        };
        let topic_shift = idx > 0
            && !seg_topics.is_empty()
            && !cur_topics.is_empty()
            && seg_topics.iter().all(|t| !cur_topics.contains(t));

        if idx > 0 && (gap >= gap_threshold_ticks || topic_shift) {
            let chapter_index = chapters.len() + 1;
            chapters.push(WebinarChapter {
                index: chapter_index,
                title: cur_title.clone(),
                start_ticks: cur_start,
                end_ticks: cur_end,
                segment_ids: std::mem::take(&mut cur_ids),
                matched_topics: std::mem::take(&mut cur_topics),
            });
            cur_start = seg.start_ticks;
            cur_title = seg_topics
                .first()
                .map(|t| humanize_topic(t))
                .unwrap_or_else(|| chapter_title_from_text(&seg.text, chapter_index + 1));
        }

        cur_ids.push(seg.id);
        for t in seg_topics {
            if !cur_topics.contains(&t) {
                cur_topics.push(t);
            }
        }
        cur_end = cur_end.max(seg.end_ticks);
    }
    if !cur_ids.is_empty() {
        let chapter_index = chapters.len() + 1;
        chapters.push(WebinarChapter {
            index: chapter_index,
            title: cur_title,
            start_ticks: cur_start,
            end_ticks: cur_end,
            segment_ids: cur_ids,
            matched_topics: cur_topics,
        });
    }

    // 4. Select highlight segments for summary reel (never including `must_not` hits).
    let max_reel_s = params.reel_target_s.max(5.0) + params.reel_tolerance_s.max(0.0);
    let mut scored: Vec<(f64, WebinarHighlight)> = Vec::new();
    for seg in &ordered {
        if flagged_segment_ids.contains(&seg.id) {
            continue;
        }
        let (start_ticks, end_ticks, _) = segment_word_bounds(seg);
        if end_ticks.0 <= start_ticks.0 {
            continue;
        }
        let duration_s = (end_ticks.0 - start_ticks.0) as f64 / TICKS_PER_SECOND as f64;
        let lower = seg.text.to_ascii_lowercase();
        let matched_topics: Vec<String> = params
            .must_include
            .iter()
            .filter(|req| text_matches_requirement(&lower, req))
            .cloned()
            .collect();
        let topic_score = if params.must_include.is_empty() {
            0.5
        } else {
            matched_topics.len() as f64 / params.must_include.len() as f64
        };
        let completeness = sentence_completeness(&seg.text);
        let score = 0.65 * topic_score + 0.35 * completeness;
        scored.push((
            score,
            WebinarHighlight {
                segment_id: seg.id,
                start_ticks,
                end_ticks,
                duration_s,
                matched_topics,
                score,
                text: seg.text.clone(),
            },
        ));
    }

    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.start_ticks.cmp(&b.1.start_ticks))
    });

    let mut highlights = Vec::new();
    let mut accumulated_s = 0.0;
    for (_, hl) in scored {
        if accumulated_s + hl.duration_s <= max_reel_s || highlights.is_empty() {
            accumulated_s += hl.duration_s;
            highlights.push(hl);
        }
    }
    highlights.sort_by_key(|h| h.start_ticks);

    let escalation_needed = !missing_required_topics.is_empty();

    Ok(WebinarAnalysisReport {
        chapters,
        topic_coverage,
        missing_required_topics,
        flagged_must_not,
        highlights,
        reel_duration_s: accumulated_s,
        escalation_needed,
    })
}

fn humanize_topic(raw: &str) -> String {
    let tokens = normalize_topic_tokens(raw);
    if tokens.is_empty() {
        return "Topic Chapter".into();
    }
    tokens
        .into_iter()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn chapter_title_from_text(text: &str, index: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().take(5).collect();
    if words.is_empty() {
        format!("Chapter {index}")
    } else {
        words.join(" ")
    }
}
