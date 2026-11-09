// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use super::*;
use saturn_core::job::{JobId, JobKind, JobPriority};
use std::path::PathBuf;
use std::sync::PoisonError;

impl EditorMcp {
    pub(super) async fn render_project(&self, arguments: &Value) -> Result<Value, String> {
        let output_path = arguments
            .get("output_path")
            .and_then(Value::as_str)
            .ok_or_else(|| "render_project requires output_path".to_owned())?;
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("render_project", arguments.clone()).await;
        }
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        let output_path = PathBuf::from(output_path);
        let use_mark_range = arguments
            .get("use_mark_range")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let explicit_range = parse_render_range(&arguments)?;
        if explicit_range.is_some() && use_mark_range {
            return Err("Pass either range or use_mark_range, not both".to_owned());
        }
        let settings = arguments
            .get("settings")
            .filter(|value| value.is_object())
            .map(|value| serde_json::from_value(value.clone()).map_err(|error| error.to_string()))
            .transpose()?;
        tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            let document = engine.document().clone();
            let range = if let Some(range) = explicit_range {
                Some(range)
            } else if use_mark_range {
                match (
                    document.project.sequence.mark_in,
                    document.project.sequence.mark_out,
                ) {
                    (Some(start), Some(end)) => Some(saturn_render::RenderRange { start, end }),
                    _ => {
                        return Err(
                            "Set both In and Out points before rendering the marked range"
                                .to_owned(),
                        );
                    }
                }
            } else {
                None
            };
            let settings =
                settings.unwrap_or_else(|| saturn_render::RenderSettings::from_project(&document));
            drop(engine);
            let output = output_path.clone();
            saturn_render::render_project_range_with_settings(
                &document,
                &output,
                range,
                settings,
                |_| {},
            )
            .map(|()| json!({ "rendered": true, "output_path": output.to_string_lossy() }))
        })
        .await
        .map_err(|error| format!("render task failed: {error}"))?
    }

    pub(super) async fn start_render(&self, arguments: &Value) -> Result<Value, String> {
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self.bridge_call("render_start", arguments.clone()).await;
        }
        let output_path = arguments
            .get("output_path")
            .and_then(Value::as_str)
            .ok_or_else(|| "render_start requires output_path".to_owned())?;
        let output_path = PathBuf::from(output_path);
        let use_mark_range = arguments
            .get("use_mark_range")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let explicit_range = parse_render_range(arguments)?;
        if explicit_range.is_some() && use_mark_range {
            return Err("Pass either range or use_mark_range, not both".into());
        }
        let settings_value = arguments.get("settings").filter(|value| value.is_object());
        let settings = settings_value
            .cloned()
            .map(serde_json::from_value::<saturn_render::RenderSettings>)
            .transpose()
            .map_err(|error| error.to_string())?;
        let Backend::Headless(engine) = &self.backend else {
            unreachable!()
        };
        let engine = engine.clone();
        let (document, range, settings) = tokio::task::spawn_blocking(move || {
            let engine = engine.lock().unwrap_or_else(PoisonError::into_inner);
            let document = engine.document().clone();
            let range = if let Some(range) = explicit_range {
                Some(range)
            } else if use_mark_range {
                match (
                    document.project.sequence.mark_in,
                    document.project.sequence.mark_out,
                ) {
                    (Some(start), Some(end)) => Some(saturn_render::RenderRange { start, end }),
                    _ => {
                        return Err(
                            "Set both In and Out points before rendering the marked range"
                                .to_owned(),
                        );
                    }
                }
            } else {
                None
            };
            let settings =
                settings.unwrap_or_else(|| saturn_render::RenderSettings::from_project(&document));
            Ok((document, range, settings))
        })
        .await
        .map_err(|error| format!("project snapshot task failed: {error}"))??;
        let job_output = output_path.clone();
        let label = format!(
            "MCP render: {}",
            output_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        let (id, _) = self.jobs.submit_detached(
            label,
            JobKind::ExportRendering {
                output_path: output_path.clone(),
            },
            JobPriority::Immediate,
            move |ctx| {
                saturn_render::render_project_range_with_control(
                    &document,
                    &job_output,
                    range,
                    settings,
                    |progress| {
                        ctx.update_progress(
                            progress.fraction_percent as f32 / 100.0,
                            progress.phase,
                        )
                    },
                    || ctx.is_cancelled(),
                )?;
                Ok(Some(job_output.to_string_lossy().into_owned()))
            },
        );
        Ok(json!({"job_id": id.0, "state": "queued", "output_path": output_path}))
    }

    pub(super) async fn render_job_status(&self, arguments: &Value) -> Result<Value, String> {
        let id = arguments
            .get("job_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "render_status requires an unsigned job_id".to_owned())?;
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("render_status", json!({"job_id": id}))
                .await;
        }
        self.jobs
            .get_job(JobId(id))
            .map(|job| serde_json::to_value(job).unwrap_or(Value::Null))
            .ok_or_else(|| format!("Render job {id} was not found"))
    }

    pub(super) async fn cancel_render(&self, arguments: &Value) -> Result<Value, String> {
        let id = arguments
            .get("job_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "render_cancel requires an unsigned job_id".to_owned())?;
        if matches!(&self.backend, Backend::Bridge(_)) {
            return self
                .bridge_call("render_cancel", json!({"job_id": id}))
                .await;
        }
        Ok(json!({"job_id": id, "cancellation_requested": self.jobs.cancel(JobId(id))}))
    }
}

pub(super) fn parse_render_range(arguments: &Value) -> Result<Option<saturn_render::RenderRange>, String> {
    let Some(range) = arguments.get("range") else {
        return Ok(None);
    };
    let start = range
        .get("start_ticks")
        .and_then(Value::as_i64)
        .ok_or_else(|| "range.start_ticks must be a non-negative integer".to_owned())?;
    let end = range
        .get("end_ticks")
        .and_then(Value::as_i64)
        .ok_or_else(|| "range.end_ticks must be an integer greater than start_ticks".to_owned())?;
    if start < 0 || end <= start {
        return Err("Render range must have non-negative start_ticks and end_ticks greater than start_ticks".to_owned());
    }
    Ok(Some(saturn_render::RenderRange {
        start: saturn_core::Tick(start),
        end: saturn_core::Tick(end),
    }))
}
