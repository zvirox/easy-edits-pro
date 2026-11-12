// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Capture Event Track (CET v0.1) parser, validator, and synthetic generator (TS0).
//!
//! Implements the open CET v0.1 specification for recording cursor, clicks,
//! key classes, window focus, zoom hints, and markers beside screen recordings.
//! Standalone specification: docs/planning/capture-event-track-v0.1.md
//! Schema: docs/schemas/cet-0.1.schema.json

use serde::{Deserialize, Serialize};
use std::fmt;

pub const CET_SPEC_VERSION: &str = "0.1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CetRecorder {
    pub name: String,
    pub version: String,
    pub platform: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CetVideo {
    pub file: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fp: Option<String>,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CetClock {
    pub t0: String,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_start_utc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CetDisplay {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub scale: f64,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CetCapture {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CetPrivacy {
    pub keys: String, // "classes_only" | "named_shortcuts" | "off"
    pub text: String, // "on" | "off"
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub redactions: Vec<String>,
}

impl Default for CetPrivacy {
    fn default() -> Self {
        Self {
            keys: "classes_only".into(),
            text: "off".into(),
            redactions: vec!["password_fields".into()],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CetTrust {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub untrusted_fields: Vec<String>,
}

/// Line 1 header record for CET v0.1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CetHeader {
    pub cet: String,
    pub r#type: String, // "header"
    pub recorder: CetRecorder,
    pub video: CetVideo,
    pub clock: CetClock,
    pub displays: Vec<CetDisplay>,
    #[serde(default)]
    pub capture: CetCapture,
    pub privacy: CetPrivacy,
    #[serde(default)]
    pub trust: CetTrust,
}

/// CET event types supported in v0.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CetEventType {
    Cursor,
    Button,
    Scroll,
    Key,
    Text,
    Window,
    Ui,
    Mark,
    ZoomHint,
    Audio,
    Pause,
    Annotation,
    #[serde(other)]
    Unknown,
}

/// A single event record in CET v0.1 (lines 2+).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CetEvent {
    pub t: u64, // Nanoseconds from first frame
    pub r#type: CetEventType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nx: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ny: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dx: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redacted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CetParseError {
    EmptyInput,
    MissingHeader,
    InvalidHeader(String),
    InvalidEvent {
        line: usize,
        error: String,
    },
    NonMonotonicTimestamp {
        line: usize,
        prev_t: u64,
        curr_t: u64,
    },
}

impl fmt::Display for CetParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "CET input is empty"),
            Self::MissingHeader => write!(f, "Line 1 is not a valid CET v0.1 header"),
            Self::InvalidHeader(e) => write!(f, "Invalid CET header: {e}"),
            Self::InvalidEvent { line, error } => {
                write!(f, "Line {line} invalid CET event: {error}")
            }
            Self::NonMonotonicTimestamp {
                line,
                prev_t,
                curr_t,
            } => {
                write!(
                    f,
                    "Line {line} non-monotonic timestamp: curr {curr_t} < prev {prev_t}"
                )
            }
        }
    }
}

impl std::error::Error for CetParseError {}

/// Parse and validate a complete `.cet.jsonl` stream or string.
pub fn parse_cet_jsonl(input: &str) -> Result<(CetHeader, Vec<CetEvent>), CetParseError> {
    let mut lines = input.lines().filter(|l| !l.trim().is_empty());
    let header_line = lines.next().ok_or(CetParseError::EmptyInput)?;

    let header: CetHeader = serde_json::from_str(header_line)
        .map_err(|e| CetParseError::InvalidHeader(e.to_string()))?;

    if header.cet != CET_SPEC_VERSION || header.r#type != "header" {
        return Err(CetParseError::InvalidHeader(format!(
            "Expected cet '{}' and type 'header', found cet '{}' and type '{}'",
            CET_SPEC_VERSION, header.cet, header.r#type
        )));
    }

    let mut events = Vec::new();
    let mut last_t = 0;

    for (idx, line) in lines.enumerate() {
        let line_num = idx + 2;
        let event: CetEvent =
            serde_json::from_str(line).map_err(|e| CetParseError::InvalidEvent {
                line: line_num,
                error: e.to_string(),
            })?;

        if event.t < last_t {
            return Err(CetParseError::NonMonotonicTimestamp {
                line: line_num,
                prev_t: last_t,
                curr_t: event.t,
            });
        }
        last_t = event.t;
        events.push(event);
    }

    Ok((header, events))
}

/// Synthetic generator for CET v0.1 event tracks (`cet-gen`).
pub struct CetGenerator {
    header: CetHeader,
    events: Vec<CetEvent>,
    current_t_ns: u64,
}

impl CetGenerator {
    pub fn new(video_file: &str, fps: f64, width: u32, height: u32) -> Self {
        let header = CetHeader {
            cet: CET_SPEC_VERSION.into(),
            r#type: "header".into(),
            recorder: CetRecorder {
                name: "cet-gen".into(),
                version: "0.1.0".into(),
                platform: "synthetic".into(),
            },
            video: CetVideo {
                file: video_file.into(),
                size_bytes: 10_000_000,
                fp: Some("sha256:synthetic_vector".into()),
                fps,
                width,
                height,
            },
            clock: CetClock {
                t0: "first_video_frame".into(),
                unit: "ns".into(),
                wall_start_utc: Some("2026-11-11T00:00:00.000Z".into()),
                quality: Some("exact".into()),
            },
            displays: vec![CetDisplay {
                id: "d0".into(),
                x: 0,
                y: 0,
                w: width,
                h: height,
                scale: 1.0,
                primary: true,
            }],
            capture: CetCapture {
                mode: Some("display".into()),
                display_id: Some("d0".into()),
                window_id: None,
                region: None,
            },
            privacy: CetPrivacy::default(),
            trust: CetTrust {
                untrusted_fields: vec![
                    "window.title".into(),
                    "app.name".into(),
                    "ui.label".into(),
                    "mark.note".into(),
                ],
            },
        };

        Self {
            header,
            events: Vec::new(),
            current_t_ns: 0,
        }
    }

    pub fn set_named_shortcuts_opt_in(&mut self, enabled: bool) {
        self.header.privacy.keys = if enabled {
            "named_shortcuts".into()
        } else {
            "classes_only".into()
        };
    }

    pub fn advance_ms(&mut self, ms: u64) {
        self.current_t_ns += ms * 1_000_000;
    }

    pub fn emit_cursor(&mut self, x: f64, y: f64, shape: &str) {
        let nx = (x / self.header.video.width as f64).clamp(0.0, 1.0);
        let ny = (y / self.header.video.height as f64).clamp(0.0, 1.0);
        self.events.push(CetEvent {
            t: self.current_t_ns,
            r#type: CetEventType::Cursor,
            x: Some(x),
            y: Some(y),
            nx: Some(nx),
            ny: Some(ny),
            shape: Some(shape.into()),
            button: None,
            action: None,
            dx: None,
            dy: None,
            unit: None,
            class: None,
            mods: Vec::new(),
            name: None,
            id: None,
            app: None,
            title: None,
            w: None,
            h: None,
            state: None,
            role: None,
            label: None,
            kind: None,
            note: None,
            reason: None,
            redacted: None,
            off: None,
        });
    }

    pub fn emit_click(&mut self, x: f64, y: f64, button: &str) {
        self.emit_cursor(x, y, "hand");
        self.events.push(CetEvent {
            t: self.current_t_ns,
            r#type: CetEventType::Button,
            x: Some(x),
            y: Some(y),
            nx: None,
            ny: None,
            shape: None,
            button: Some(button.into()),
            action: Some("down".into()),
            dx: None,
            dy: None,
            unit: None,
            class: None,
            mods: Vec::new(),
            name: None,
            id: None,
            app: None,
            title: None,
            w: None,
            h: None,
            state: None,
            role: None,
            label: None,
            kind: None,
            note: None,
            reason: None,
            redacted: None,
            off: None,
        });
        self.advance_ms(50);
        self.events.push(CetEvent {
            t: self.current_t_ns,
            r#type: CetEventType::Button,
            x: Some(x),
            y: Some(y),
            nx: None,
            ny: None,
            shape: None,
            button: Some(button.into()),
            action: Some("up".into()),
            dx: None,
            dy: None,
            unit: None,
            class: None,
            mods: Vec::new(),
            name: None,
            id: None,
            app: None,
            title: None,
            w: None,
            h: None,
            state: None,
            role: None,
            label: None,
            kind: None,
            note: None,
            reason: None,
            redacted: None,
            off: None,
        });
    }

    pub fn emit_key(&mut self, class: &str, mods: &[&str], name: Option<&str>) {
        let key_name = if self.header.privacy.keys == "named_shortcuts" {
            name.map(str::to_string)
        } else {
            None
        };
        self.events.push(CetEvent {
            t: self.current_t_ns,
            r#type: CetEventType::Key,
            x: None,
            y: None,
            nx: None,
            ny: None,
            shape: None,
            button: None,
            action: Some("down".into()),
            dx: None,
            dy: None,
            unit: None,
            class: Some(class.into()),
            mods: mods.iter().map(|m| m.to_string()).collect(),
            name: key_name,
            id: None,
            app: None,
            title: None,
            w: None,
            h: None,
            state: None,
            role: None,
            label: None,
            kind: None,
            note: None,
            reason: None,
            redacted: None,
            off: None,
        });
    }

    pub fn emit_window(&mut self, app: &str, title: &str, x: f64, y: f64, w: f64, h: f64) {
        self.events.push(CetEvent {
            t: self.current_t_ns,
            r#type: CetEventType::Window,
            x: Some(x),
            y: Some(y),
            nx: None,
            ny: None,
            shape: None,
            button: None,
            action: None,
            dx: None,
            dy: None,
            unit: None,
            class: None,
            mods: Vec::new(),
            name: None,
            id: Some(format!("w_{app}")),
            app: Some(app.into()),
            title: Some(title.into()),
            w: Some(w),
            h: Some(h),
            state: Some("focused".into()),
            role: None,
            label: None,
            kind: None,
            note: None,
            reason: None,
            redacted: None,
            off: None,
        });
    }

    pub fn emit_mark(&mut self, kind: &str, note: &str) {
        self.events.push(CetEvent {
            t: self.current_t_ns,
            r#type: CetEventType::Mark,
            x: None,
            y: None,
            nx: None,
            ny: None,
            shape: None,
            button: None,
            action: None,
            dx: None,
            dy: None,
            unit: None,
            class: None,
            mods: Vec::new(),
            name: None,
            id: None,
            app: None,
            title: None,
            w: None,
            h: None,
            state: None,
            role: None,
            label: None,
            kind: Some(kind.into()),
            note: Some(note.into()),
            reason: None,
            redacted: None,
            off: None,
        });
    }

    pub fn emit_zoom_hint(&mut self, x: f64, y: f64, w: f64, h: f64, reason: &str) {
        self.events.push(CetEvent {
            t: self.current_t_ns,
            r#type: CetEventType::ZoomHint,
            x: Some(x),
            y: Some(y),
            nx: None,
            ny: None,
            shape: None,
            button: None,
            action: None,
            dx: None,
            dy: None,
            unit: None,
            class: None,
            mods: Vec::new(),
            name: None,
            id: None,
            app: None,
            title: None,
            w: Some(w),
            h: Some(h),
            state: None,
            role: None,
            label: None,
            kind: None,
            note: None,
            reason: Some(reason.into()),
            redacted: None,
            off: None,
        });
    }

    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        out.push_str(&serde_json::to_string(&self.header).unwrap_or_default());
        out.push('\n');
        for ev in &self.events {
            out.push_str(&serde_json::to_string(ev).unwrap_or_default());
            out.push('\n');
        }
        out
    }
}
