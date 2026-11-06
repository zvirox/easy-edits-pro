// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub async fn ask_local_ai(
    prompt: Option<String>,
    model: Option<String>,
    messages: Option<Vec<ChatMessage>>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    let model_name = model.unwrap_or_else(|| "llama3.2".to_string());
    let mut chat_messages: Vec<Value> = Vec::new();

    if let Some(sys) = system_prompt {
        if !sys.trim().is_empty() {
            chat_messages.push(json!({ "role": "system", "content": sys.trim() }));
        }
    } else {
        chat_messages.push(json!({
            "role": "system",
            "content": "You are Saturn AI, an expert post-production and editing assistant embedded in Easy Edits Pro NLE. You provide clear, concise advice on color grading, audio mixing, pacing, visual effects, and keyboard shortcuts."
        }));
    }

    if let Some(history) = messages {
        for msg in history {
            chat_messages.push(json!({ "role": msg.role, "content": msg.content }));
        }
    } else if let Some(p) = prompt {
        chat_messages.push(json!({ "role": "user", "content": p.trim() }));
    } else {
        return Err("No prompt or chat messages provided".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?;

    let result = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&json!({
            "model": model_name,
            "messages": chat_messages,
            "stream": false
        }))
        .send()
        .await;

    match result {
        Ok(res) => {
            if let Ok(json_res) = res.json::<Value>().await {
                if let Some(content) = json_res.pointer("/message/content").and_then(Value::as_str)
                {
                    return Ok(content.to_string());
                }
            }
            Ok("Received empty response from local Ollama model.".to_string())
        }
        Err(_) => {
            // Provide helpful fallback assistant response if Ollama daemon is offline
            let last_user_prompt = chat_messages
                .iter()
                .rev()
                .find(|m| m["role"] == "user")
                .and_then(|m| m["content"].as_str())
                .unwrap_or("hello")
                .to_lowercase();

            let fallback_advice = if last_user_prompt.contains("color")
                || last_user_prompt.contains("grade")
            {
                "🎨 **Color Grading Recommendation:**\n• Apply an S-Curve in RGB Curves to boost contrast.\n• Use Teal & Orange 3D LUT at 65% opacity for a modern cinema look.\n• Isolate skin tones with HSL Qualifier (Hue 15°-35°) and apply +0.2 EV exposure lift."
            } else if last_user_prompt.contains("audio")
                || last_user_prompt.contains("loudness")
                || last_user_prompt.contains("ebu")
            {
                "🔊 **Fairlight Audio Master Bus:**\n• Target -23.0 LUFS integrated loudness for EBU R128 broadcast compliance (-14.0 LUFS for YouTube/Spotify).\n• Keep true peaks below -1.0 dBTP.\n• Enable the 3-Band Parametric EQ and Fast Compressor (ratio 3.5:1, attack 15ms)."
            } else if last_user_prompt.contains("multicam") || last_user_prompt.contains("angle") {
                "🎥 **Live Multi-Cam Cutting Tips:**\n• Switch between angles 1-9 during playback using number keys 1-9.\n• Green tally denotes Preview (PVW) and Red denotes Program (PGM).\n• Keep Audio Follow Video (AFV) locked to angle 1 (boom mic) for continuous dialogue."
            } else if last_user_prompt.contains("shortcut") {
                "⌨️ **Key Saturn Shortcuts:**\n• `B`: Razor blade cut at playhead\n• `Shift+Del`: Ripple delete clip or gap\n• `Ctrl+D`: Standard crossfade transition\n• `Ctrl+T`: Insert Title clip\n• `Space`: Toggle Play/Pause"
            } else {
                "🤖 **Saturn Local AI Assistant:**\nLocal Ollama daemon is currently not running at `http://127.0.0.1:11434`.\nTo connect full LLM models like `llama3.2`, `mistral`, or `qwen2.5`, start Ollama via `ollama run llama3.2`.\n\nIn the meantime, I can assist with timeline operations, broadcast standards, and color workflows!"
            };
            Ok(fallback_advice.to_string())
        }
    }
}

/// Run MODNet portrait matting on one decoded frame of a media asset and
/// composite the measured matte against the requested background.
///
/// Honest scope: the model is portrait-only (humans) — frames without a
/// person report low foreground coverage, which is the model being truthful.
/// The returned RGBA is a *preview* of the selected frame; the response also
/// carries the measured mean alpha so the UI cannot overstate the result.
#[tauri::command]
pub async fn run_background_matting(
    state: State<'_, AppState>,
    media_id: u64,
    at_tick: Option<i64>,
    threshold: Option<f32>,
    feather_radius: Option<f32>,
    background_mode: Option<String>,
) -> Result<serde_json::Value, String> {
    // Snapshot only what we need under the lock; never hold it across
    // inference (Packet 2.2 rule).
    let source = {
        let engine = state
            .0
            .lock()
            .map_err(|_| "Editor state is unavailable".to_owned())?;
        let asset = engine
            .document()
            .project
            .media
            .iter()
            .find(|asset| asset.id == media_id)
            .ok_or_else(|| format!("Media asset {media_id} was not found"))?;
        match &asset.kind {
            MediaKind::Video | MediaKind::Image => {}
            other => {
                return Err(format!("run_background_matting: {other:?} media has no frames"));
            }
        }
        if !asset.path.is_file() {
            return Err(format!(
                "run_background_matting: source file is missing: {}",
                asset.path.display()
            ));
        }
        asset.path.clone()
    };

    let options = saturn_render::MattingOptions {
        threshold: threshold.unwrap_or(0.5).clamp(0.0, 1.0),
        feather_radius: feather_radius.unwrap_or(2.0).clamp(0.0, 20.0).round() as u32,
        background_mode: saturn_render::background_mode_from_id(
            background_mode.as_deref().unwrap_or("transparent"),
        )?,
    };
    let at_tick = Tick(at_tick.unwrap_or(0).max(0));

    tauri::async_runtime::spawn_blocking(move || {
        let (width, height, rgb) = saturn_render::frame_extractor::decode_frame_rgb(
            &source,
            at_tick,
        )?;
        let frame = saturn_perception::shot_detect::SampledFrame {
            tick: at_tick,
            width,
            height,
            rgb,
        };
        let mut matting = saturn_perception::ModNetMatting::load()?;
        let measured = matting.matte_frame(&frame)?;
        let alpha_mean = measured.iter().sum::<f32>() / measured.len().max(1) as f32;

        let mut rgba = vec![0_u8; width as usize * height as usize * 4];
        for (dst, src) in rgba.chunks_exact_mut(4).zip(frame.rgb.chunks_exact(3)) {
            dst[0] = src[0];
            dst[1] = src[1];
            dst[2] = src[2];
            dst[3] = 255;
        }
        let final_alpha = saturn_render::composite_matting(
            &mut rgba,
            width,
            height,
            &measured,
            &options,
        )?;
        let kept_share = final_alpha.iter().filter(|&&a| a > 0.0).count() as f64
            / final_alpha.len().max(1) as f64;
        Ok(json!({
            "width": width,
            "height": height,
            "rgba": rgba,
            "alphaMean": alpha_mean,
            "foregroundShare": kept_share,
            "model": "modnet",
            "atTick": at_tick.0,
            "note": "MODNet is portrait matting: frames without people report low coverage",
        }))
    })
    .await
    .map_err(|error| format!("matting task failed: {error}"))?
}
