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

#[tauri::command]
pub async fn run_background_matting(
    rgba_bytes: Vec<u8>,
    width: u32,
    height: u32,
    model: Option<String>,
    threshold: Option<f32>,
    feather_radius: Option<f32>,
    background_mode: Option<String>,
) -> Result<Vec<u8>, String> {
    if rgba_bytes.len() != (width as usize * height as usize * 4) {
        return Err("Invalid buffer length for specified dimensions".into());
    }

    let mut buf = rgba_bytes;
    let mut temporal = saturn_render::MattingTemporalState::default();

    let model_kind = match model.as_deref().unwrap_or("rvm") {
        "modnet" => saturn_render::MattingModel::ModNet,
        _ => saturn_render::MattingModel::RobustVideoMatting,
    };

    let bg_mode = match background_mode.as_deref().unwrap_or("transparent") {
        "green" | "greenscreen" => saturn_render::BackgroundMode::GreenScreen,
        "black" => saturn_render::BackgroundMode::SolidColor { r: 0, g: 0, b: 0 },
        "white" => saturn_render::BackgroundMode::SolidColor {
            r: 255,
            g: 255,
            b: 255,
        },
        _ => saturn_render::BackgroundMode::Transparent,
    };

    let options = saturn_render::MattingOptions {
        model: model_kind,
        threshold: threshold.unwrap_or(0.5).clamp(0.0, 1.0),
        feather_radius: feather_radius.unwrap_or(2.0).clamp(0.0, 20.0),
        background_mode: bg_mode,
        temporal_coherence: true,
    };

    saturn_render::run_background_matting(
        &mut buf,
        width as usize,
        height as usize,
        &options,
        &mut temporal,
    );
    Ok(buf)
}
