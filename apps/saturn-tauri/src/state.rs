// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_core::{EditorEngine, JobManager};
use saturn_render::GpuPreviewPipeline;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState(
    pub Arc<Mutex<EditorEngine>>,
    pub Arc<JobManager>,
    pub Arc<Mutex<GpuPreviewPipeline>>,
);

impl Default for AppState {
    fn default() -> Self {
        Self(
            Arc::new(Mutex::new(EditorEngine::default())),
            Arc::new(JobManager::new(3)),
            Arc::new(Mutex::new(GpuPreviewPipeline::new(1920, 1080))),
        )
    }
}
