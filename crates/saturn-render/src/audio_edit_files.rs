// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Persistent, project-local derived audio with rollback cleanup.
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct PreparedAudioEdit {
    pub command: saturn_core::CommandRequest,
    pub telemetry: Value,
    files: Vec<PathBuf>,
    committed: bool,
    directory: PathBuf,
}

impl PreparedAudioEdit {
    pub(crate) fn new(project_path: Option<&Path>, id: &str) -> Result<Self, String> {
        let project = project_path
            .ok_or("Save the project before generating derived audio")?
            .canonicalize()
            .map_err(|e| format!("Saved project is unavailable: {e}"))?;
        if !project.is_file() {
            return Err("Saved project path must be a file".into());
        }
        let directory = project.with_extension("saturn-audio");
        if directory.exists()
            && std::fs::symlink_metadata(&directory)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err("Derived audio directory must not be a symlink".into());
        }
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        if directory
            .canonicalize()
            .map_err(|e| e.to_string())?
            .parent()
            != project.parent()
        {
            return Err("Derived audio directory escapes the project folder".into());
        }
        Ok(Self {
            command: saturn_core::CommandRequest::new(id, serde_json::json!({})),
            telemetry: Value::Null,
            files: vec![],
            committed: false,
            directory,
        })
    }

    pub(crate) fn allocate(&mut self) -> Result<(PathBuf, std::fs::File), String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let path = self.directory.join(format!(
            "audio-{}-{stamp}-{}.wav",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        self.files.push(path.clone());
        Ok((path, file))
    }

    pub(crate) fn write(
        &mut self,
        samples: &[f32],
        channels: u32,
        duration: saturn_core::Tick,
    ) -> Result<saturn_core::audio_edit::RenderedAudio, String> {
        let (path, mut file) = self.allocate()?;
        write_wav(&mut file, samples, channels, 48_000)?;
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(saturn_core::audio_edit::RenderedAudio {
            path,
            duration,
            channels,
        })
    }

    /// Keep generated media for save/reopen and undo/redo. Originals stay intact.
    pub fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for PreparedAudioEdit {
    fn drop(&mut self) {
        if !self.committed {
            for path in &self.files {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

pub fn write_wav(
    file: &mut impl Write,
    samples: &[f32],
    channels: u32,
    rate: u32,
) -> Result<(), String> {
    if !(1..=2).contains(&channels)
        || !(8000..=192000).contains(&rate)
        || samples.len() % channels as usize != 0
        || samples.iter().any(|v| !v.is_finite())
    {
        return Err("Invalid PCM for float WAV".into());
    }
    let bytes = u32::try_from(samples.len() * 4).map_err(|e| e.to_string())?;
    let riff_size = bytes.checked_add(36).ok_or("WAV exceeds RIFF size limit")?;
    let mut header = Vec::new();
    header.extend(b"RIFF");
    header.extend(riff_size.to_le_bytes());
    header.extend(b"WAVEfmt ");
    header.extend(16_u32.to_le_bytes());
    header.extend(3_u16.to_le_bytes());
    header.extend((channels as u16).to_le_bytes());
    header.extend(rate.to_le_bytes());
    header.extend((rate * channels * 4).to_le_bytes());
    header.extend((channels as u16 * 4).to_le_bytes());
    header.extend(32_u16.to_le_bytes());
    header.extend(b"data");
    header.extend(bytes.to_le_bytes());
    file.write_all(&header).map_err(|e| e.to_string())?;
    let mut writer = std::io::BufWriter::new(file);
    for &sample in samples {
        writer
            .write_all(&sample.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())
}
