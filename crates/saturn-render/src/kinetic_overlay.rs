// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Bridge from kinetic layout to the existing graphics overlay (Packet 6.2).

use saturn_core::TICKS_PER_SECOND;
use saturn_core::kinetic_typography::ClipKineticTypography;
use saturn_core::project::Graphic;
use saturn_text::kinetic::{
    FaceBox, KineticLayoutOptions, TimedWord, layout_kinetic_words,
};
use serde_json::json;

fn layout_options(config: &ClipKineticTypography) -> KineticLayoutOptions {
    KineticLayoutOptions {
        ticks_per_second: TICKS_PER_SECOND,
        stiffness: config.style.stiffness,
        damping: config.style.damping,
        lanes: config.style.lanes,
        ..Default::default()
    }
}

/// Build the graphics visible at `playhead_tick`. The caller passes them to
/// `render_graphics_overlay`, so kinetic words use the same renderer as every
/// other timeline graphic.
pub fn kinetic_graphics_at(
    config: &ClipKineticTypography,
    playhead_tick: i64,
    width: u32,
    height: u32,
) -> Result<Vec<Graphic>, String> {
    let words: Vec<TimedWord> = config
        .words
        .iter()
        .map(|word| TimedWord {
            text_start_tick: word.start_ticks.0,
            text_end_tick: word.end_ticks.0,
        })
        .collect();
    let faces: Vec<FaceBox> = config
        .faces
        .iter()
        .map(|face| FaceBox {
            x: face.x,
            y: face.y,
            width: face.width,
            height: face.height,
        })
        .collect();
    let positioned = layout_kinetic_words(&words, &faces, playhead_tick, &layout_options(config));
    positioned
        .into_iter()
        .map(|word| {
            let source = &config.words[word.index];
            let size = config.style.font_size * word.scale;
            serde_json::from_value(json!({
                "id": (word.index as u64) + 1,
                "name": source.text,
                "kind": "text",
                "start_ticks": playhead_tick,
                "duration_ticks": 1,
                "visible": true,
                "transform": {
                    "x": word.x * width as f32,
                    "y": word.y * height as f32,
                    "width": size * (source.text.chars().count().max(1) as f32) * 0.6,
                    "height": size,
                    "opacity": word.opacity,
                },
                "text": {
                    "content": source.text,
                    "font_size": size,
                },
            }))
            .map_err(|error| format!("could not build kinetic graphic: {error}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use saturn_core::kinetic_typography::{KineticFaceBox, KineticStyle, KineticWord};
    use saturn_core::{Tick, audio_edit::audio_clip};

    fn config(faces: Vec<KineticFaceBox>) -> ClipKineticTypography {
        let mut clip = audio_clip(1, Tick(0), Tick(200));
        clip.id = 1;
        ClipKineticTypography {
            clip_id: 1,
            words: vec![KineticWord {
                text: "Hello".into(),
                start_ticks: Tick(0),
                end_ticks: Tick(200),
            }],
            faces,
            style: KineticStyle::default(),
            start_ticks: clip.timeline_start,
            duration_ticks: clip.duration,
        }
    }

    #[test]
    fn visible_word_becomes_an_overlay_graphic() {
        let graphics = kinetic_graphics_at(&config(vec![]), 100, 1920, 1080).expect("graphics");
        assert_eq!(graphics.len(), 1);
        assert_eq!(graphics[0].text.as_ref().map(|text| text.content.as_str()), Some("Hello"));
        assert!(graphics[0].transform.opacity >= 0.0);
    }

    #[test]
    fn words_outside_their_timing_produce_no_graphics() {
        let graphics = kinetic_graphics_at(&config(vec![]), TICKS_PER_SECOND, 1920, 1080)
            .expect("graphics");
        assert!(graphics.is_empty());
    }
}
