//! Project Templates
//!
//! Defines project templates, their manifests, variables, and settings.

use crate::search_bin::SearchBinQuery;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Project template categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateCategory {
    Empty,
    ShortFilm,
    Documentary,
    YouTube,
    SocialMedia,
    Commercial,
    MusicVideo,
    Wedding,
    Corporate,
    Broadcast,
    Custom,
}

impl Default for TemplateCategory {
    fn default() -> Self {
        TemplateCategory::Empty
    }
}

impl TemplateCategory {
    pub fn default_settings(&self) -> ProjectSettings {
        match self {
            TemplateCategory::ShortFilm => ProjectSettings {
                name: "Short Film".to_string(),
                frame_rate: FrameRate::Fps24,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Footage".to_string(),
                        path: "Footage".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "Audio".to_string(),
                        path: "Audio".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Music".to_string(),
                        path: "Music".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "Graphics".to_string(),
                        path: "Graphics".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                    BinTemplate {
                        name: "Sequences".to_string(),
                        path: "Sequences".to_string(),
                        color: Some("#96CEB4".to_string()),
                    },
                ],
                sequences: vec![SequenceTemplate {
                    name: "Main".to_string(),
                    preset: "Cinematic".to_string(),
                }],
                ..Default::default()
            },
            TemplateCategory::Documentary => ProjectSettings {
                name: "Documentary".to_string(),
                frame_rate: FrameRate::Fps30,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Interviews".to_string(),
                        path: "Interviews".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "B-Roll".to_string(),
                        path: "B-Roll".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Archival".to_string(),
                        path: "Archival".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "Audio".to_string(),
                        path: "Audio".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                    BinTemplate {
                        name: "Music".to_string(),
                        path: "Music".to_string(),
                        color: Some("#96CEB4".to_string()),
                    },
                ],
                sequences: vec![SequenceTemplate {
                    name: "Main".to_string(),
                    preset: "Documentary".to_string(),
                }],
                ..Default::default()
            },
            TemplateCategory::YouTube => ProjectSettings {
                name: "YouTube Video".to_string(),
                frame_rate: FrameRate::Fps30,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "A-Roll".to_string(),
                        path: "A-Roll".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "B-Roll".to_string(),
                        path: "B-Roll".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Assets".to_string(),
                        path: "Assets".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "Audio".to_string(),
                        path: "Audio".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                    BinTemplate {
                        name: "Thumbnails".to_string(),
                        path: "Thumbnails".to_string(),
                        color: Some("#96CEB4".to_string()),
                    },
                ],
                sequences: vec![SequenceTemplate {
                    name: "Main".to_string(),
                    preset: "YouTube".to_string(),
                }],
                ..Default::default()
            },
            TemplateCategory::SocialMedia => ProjectSettings {
                name: "Social Media".to_string(),
                frame_rate: FrameRate::Fps30,
                resolution: (1080, 1920), // Vertical
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Clips".to_string(),
                        path: "Clips".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "Audio".to_string(),
                        path: "Audio".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Overlays".to_string(),
                        path: "Overlays".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                ],
                sequences: vec![
                    SequenceTemplate {
                        name: "Reel".to_string(),
                        preset: "Vertical".to_string(),
                    },
                    SequenceTemplate {
                        name: "Story".to_string(),
                        preset: "Vertical".to_string(),
                    },
                    SequenceTemplate {
                        name: "TikTok".to_string(),
                        preset: "Vertical".to_string(),
                    },
                ],
                ..Default::default()
            },
            TemplateCategory::Commercial => ProjectSettings {
                name: "Commercial".to_string(),
                frame_rate: FrameRate::Fps30,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Hero Shots".to_string(),
                        path: "Hero Shots".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "Product Shots".to_string(),
                        path: "Product Shots".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Lifestyle".to_string(),
                        path: "Lifestyle".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "Audio".to_string(),
                        path: "Audio".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                    BinTemplate {
                        name: "Graphics".to_string(),
                        path: "Graphics".to_string(),
                        color: Some("#96CEB4".to_string()),
                    },
                ],
                sequences: vec![
                    SequenceTemplate {
                        name: "Main :30".to_string(),
                        preset: "Commercial".to_string(),
                    },
                    SequenceTemplate {
                        name: "Main :15".to_string(),
                        preset: "Commercial".to_string(),
                    },
                    SequenceTemplate {
                        name: "Main :06".to_string(),
                        preset: "Commercial".to_string(),
                    },
                ],
                ..Default::default()
            },
            TemplateCategory::MusicVideo => ProjectSettings {
                name: "Music Video".to_string(),
                frame_rate: FrameRate::Fps24,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Performance".to_string(),
                        path: "Performance".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "B-Roll".to_string(),
                        path: "B-Roll".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Audio Stems".to_string(),
                        path: "Audio Stems".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "VFX".to_string(),
                        path: "VFX".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                ],
                sequences: vec![SequenceTemplate {
                    name: "Main".to_string(),
                    preset: "Music Video".to_string(),
                }],
                ..Default::default()
            },
            TemplateCategory::Wedding => ProjectSettings {
                name: "Wedding".to_string(),
                frame_rate: FrameRate::Fps30,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Ceremony".to_string(),
                        path: "Ceremony".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "Reception".to_string(),
                        path: "Reception".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Details".to_string(),
                        path: "Details".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "Audio".to_string(),
                        path: "Audio".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                ],
                sequences: vec![
                    SequenceTemplate {
                        name: "Highlight".to_string(),
                        preset: "Cinematic".to_string(),
                    },
                    SequenceTemplate {
                        name: "Full Ceremony".to_string(),
                        preset: "Documentary".to_string(),
                    },
                ],
                ..Default::default()
            },
            TemplateCategory::Corporate => ProjectSettings {
                name: "Corporate".to_string(),
                frame_rate: FrameRate::Fps30,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Interviews".to_string(),
                        path: "Interviews".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "B-Roll".to_string(),
                        path: "B-Roll".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Graphics".to_string(),
                        path: "Graphics".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "Audio".to_string(),
                        path: "Audio".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                ],
                sequences: vec![SequenceTemplate {
                    name: "Main".to_string(),
                    preset: "Corporate".to_string(),
                }],
                ..Default::default()
            },
            TemplateCategory::Broadcast => ProjectSettings {
                name: "Broadcast".to_string(),
                frame_rate: FrameRate::Fps29_97,
                resolution: (1920, 1080),
                sample_rate: 48000,
                audio_channels: 2,
                color_space: ColorSpace::Rec709,
                bins: vec![
                    BinTemplate {
                        name: "Packages".to_string(),
                        path: "Packages".to_string(),
                        color: Some("#FF6B6B".to_string()),
                    },
                    BinTemplate {
                        name: "VO".to_string(),
                        path: "VO".to_string(),
                        color: Some("#4ECDC4".to_string()),
                    },
                    BinTemplate {
                        name: "Graphics".to_string(),
                        path: "Graphics".to_string(),
                        color: Some("#45B7D1".to_string()),
                    },
                    BinTemplate {
                        name: "Music".to_string(),
                        path: "Music".to_string(),
                        color: Some("#FFBE0B".to_string()),
                    },
                ],
                sequences: vec![SequenceTemplate {
                    name: "Package".to_string(),
                    preset: "Broadcast".to_string(),
                }],
                ..Default::default()
            },
            TemplateCategory::Custom => ProjectSettings::default(),
            TemplateCategory::Empty => ProjectSettings::default(),
        }
    }
}

/// Project settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub name: String,
    pub frame_rate: FrameRate,
    pub resolution: (u32, u32),
    pub sample_rate: u32,
    pub audio_channels: u32,
    pub color_space: ColorSpace,
    pub bins: Vec<BinTemplate>,
    pub sequences: Vec<SequenceTemplate>,
    pub search_bins: Vec<SearchBinTemplate>,
    pub graphics_templates: Vec<String>, // Template IDs
    pub custom: Value,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: "Untitled Project".to_string(),
            frame_rate: FrameRate::Fps30,
            resolution: (1920, 1080),
            sample_rate: 48000,
            audio_channels: 2,
            color_space: ColorSpace::Rec709,
            bins: Vec::new(),
            sequences: Vec::new(),
            search_bins: Vec::new(),
            graphics_templates: Vec::new(),
            custom: Value::Object(Default::default()),
        }
    }
}

/// Bin template for project structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinTemplate {
    pub name: String,
    pub path: String,
    pub color: Option<String>,
}

/// Sequence template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceTemplate {
    pub name: String,
    pub preset: String,
}

/// Search bin template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBinTemplate {
    pub name: String,
    pub query: SearchBinQuery,
    pub color: Option<String>,
}

/// Color spaces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Rec709,
    Rec2020,
    P3D65,
    Srgb,
    Linear,
    Custom,
}

impl Default for ColorSpace {
    fn default() -> Self {
        ColorSpace::Rec709
    }
}

/// Frame rates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameRate {
    Fps23_976,
    Fps24,
    Fps25,
    Fps29_97,
    Fps30,
    Fps48,
    Fps50,
    Fps59_94,
    Fps60,
    Custom { numerator: u32, denominator: u32 },
}

impl Default for FrameRate {
    fn default() -> Self {
        FrameRate::Fps30
    }
}

/// Project template manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
    pub thumbnail: Option<String>,
    pub variables: Vec<TemplateVariable>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Template variable for customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub label: String,
    pub description: String,
    pub var_type: VariableType,
    pub default: Value,
    pub required: bool,
    pub validation: Option<String>, // Regex pattern
}

/// Variable types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Integer,
    Float,
    Boolean,
    Color,
    Path,
    Choice,
}

/// Project template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub manifest: TemplateManifest,
    pub settings: ProjectSettings,
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

impl ProjectTemplate {
    pub fn new(manifest: TemplateManifest, settings: ProjectSettings) -> Self {
        Self {
            manifest,
            settings,
            source_path: None,
        }
    }

    pub fn from_directory(path: &Path) -> Result<Self, ProjectError> {
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            return Err(ProjectError::ManifestNotFound);
        }

        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest: TemplateManifest = serde_json::from_str(&manifest_content)?;

        let settings_path = path.join("settings.json");
        let settings = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content)?
        } else {
            ProjectSettings::default()
        };

        let mut template = Self::new(manifest, settings);
        template.source_path = Some(path.to_path_buf());
        Ok(template)
    }

    pub fn create_project(
        &self,
        destination: &Path,
        variables: HashMap<String, Value>,
    ) -> Result<(), ProjectError> {
        // Create directory structure
        fs::create_dir_all(destination)?;

        // Create bins
        for bin in &self.settings.bins {
            let bin_path = destination.join(&bin.path);
            fs::create_dir_all(&bin_path)?;
        }

        // Create sequences (placeholder - would create actual sequence files)
        for seq in &self.settings.sequences {
            let seq_dir = destination.join("Sequences").join(&seq.name);
            fs::create_dir_all(&seq_dir)?;
        }

        // Process variables
        let mut resolved_settings = self.settings.clone();
        Self::resolve_variables(&mut resolved_settings, &variables)?;

        // Save project file
        let project_file = destination.join("project.saturn");
        let project_data = serde_json::to_vec_pretty(&resolved_settings)?;
        fs::write(&project_file, project_data)?;

        // Copy template assets if any
        if let Some(source_path) = &self.source_path {
            let assets_src = source_path.join("assets");
            if assets_src.exists() {
                let assets_dest = destination.join("Assets");
                Self::copy_dir(&assets_src, &assets_dest)?;
            }
        }

        Ok(())
    }

    fn resolve_variables(
        settings: &mut ProjectSettings,
        variables: &HashMap<String, Value>,
    ) -> Result<(), ProjectError> {
        // Replace variables in settings
        let json = serde_json::to_value(&*settings)?;
        let resolved = Self::replace_variables_in_value(json, variables)?;
        *settings = serde_json::from_value(resolved)?;
        Ok(())
    }

    fn replace_variables_in_value(
        value: Value,
        variables: &HashMap<String, Value>,
    ) -> Result<Value, ProjectError> {
        match value {
            Value::String(s) => {
                let mut result = s;
                for (key, value) in variables {
                    let placeholder = format!("{{{{{}}}}}", key);
                    if result.contains(&placeholder) {
                        let replacement = match value {
                            Value::String(s) => s.clone(),
                            _ => serde_json::to_string(value)?,
                        };
                        result = result.replace(&placeholder, &replacement);
                    }
                }
                Ok(Value::String(result))
            }
            Value::Object(obj) => {
                let mut result = serde_json::Map::new();
                for (k, v) in obj {
                    result.insert(k, Self::replace_variables_in_value(v, variables)?);
                }
                Ok(Value::Object(result))
            }
            Value::Array(arr) => {
                let mut result = Vec::new();
                for v in arr {
                    result.push(Self::replace_variables_in_value(v, variables)?);
                }
                Ok(Value::Array(result))
            }
            other => Ok(other),
        }
    }

    fn copy_dir(src: &Path, dest: &Path) -> Result<(), ProjectError> {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if src_path.is_dir() {
                Self::copy_dir(&src_path, &dest_path)?;
            } else {
                fs::copy(&src_path, &dest_path)?;
            }
        }
        Ok(())
    }
}

/// Project errors
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Manifest not found")]
    ManifestNotFound,
    #[error("Invalid manifest: {0}")]
    InvalidManifest(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Walk error: {0}")]
    WalkError(#[from] walkdir::Error),
    #[error("Variable resolution error: {0}")]
    VariableError(String),
    #[error("Template not found")]
    TemplateNotFound,
    #[error("Invalid variable: {0}")]
    InvalidVariable(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_template_category_settings() {
        let settings = TemplateCategory::ShortFilm.default_settings();
        assert_eq!(settings.frame_rate, FrameRate::Fps24);
        assert!(!settings.bins.is_empty());
        assert!(!settings.sequences.is_empty());
    }

    #[test]
    fn test_template_variable_resolution() {
        let mut variables = HashMap::new();
        variables.insert("project_name".to_string(), json!("My Film"));

        let mut settings = ProjectSettings {
            name: "{{project_name}}".to_string(),
            ..Default::default()
        };

        ProjectTemplate::resolve_variables(&mut settings, &variables).unwrap();
        assert_eq!(settings.name, "My Film");
    }

    #[test]
    fn test_create_project_from_template() {
        let dir = tempdir().unwrap();
        let template = ProjectTemplate {
            manifest: TemplateManifest {
                id: Uuid::new_v4(),
                name: "Test".to_string(),
                description: "Test template".to_string(),
                category: TemplateCategory::ShortFilm,
                version: "1.0".to_string(),
                author: "Test".to_string(),
                tags: vec![],
                thumbnail: None,
                variables: vec![],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            settings: TemplateCategory::ShortFilm.default_settings(),
            source_path: None,
        };

        let dest = dir.path().join("test_project");
        template.create_project(&dest, HashMap::new()).unwrap();

        assert!(dest.join("project.saturn").exists());
        assert!(dest.join("Footage").exists());
        assert!(dest.join("Audio").exists());
    }
}
