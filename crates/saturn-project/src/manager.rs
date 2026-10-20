//! Project Manager Module
//!
//! High-level project management combining templates, search bins, and graphics.

pub mod template;
pub mod search_bin;
pub mod manager;

pub use template::{ProjectTemplate, TemplateCategory, TemplateManifest, TemplateVariable, ProjectSettings, BinTemplate, SequenceTemplate, SearchBinTemplate};
pub use search_bin::{SearchBin, SearchBinQuery, SearchBinRule, SearchOperator, SearchField, SearchLogic, SearchBinManager, MediaItem, SearchBinMatch, presets};
pub use manager::{ProjectManager, TemplateManager, SearchBinManager as BinManager};

use saturn_timeline::*;
use saturn_graphics::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::collections::HashMap;
use indexmap::IndexMap;
use std::sync::Arc;
use parking_lot::RwLock;
use walkdir::WalkDir;
use chrono;
use std::path::{Path, PathBuf};
use std::fs;

/// Project manager for high-level project operations
pub struct ProjectManager {
    template_manager: TemplateManager,
    bin_manager: SearchBinManager,
    graphics_engine: Arc<saturn_graphics::TemplateEngine>,
    current_project: Option<Project>,
}

/// Project representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub settings: ProjectSettings,
    pub sequences: IndexMap<SequenceId, Sequence>,
    pub media: IndexMap<Uuid, MediaItem>,
    pub search_bins: IndexMap<Uuid, SearchBin>,
    pub graphics_templates: IndexMap<TemplateId, saturn_graphics::GraphicsTemplate>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

impl Project {
    pub fn new(name: String, path: PathBuf, settings: ProjectSettings) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            path,
            settings,
            sequences: IndexMap::new(),
            media: IndexMap::new(),
            search_bins: IndexMap::new(),
            graphics_templates: IndexMap::new(),
            created_at: now,
            modified_at: now,
        }
    }

    pub fn add_sequence(&mut self, sequence: Sequence) {
        self.sequences.insert(sequence.id, sequence);
        self.modified_at = chrono::Utc::now();
    }

    pub fn remove_sequence(&mut self, seq_id: SequenceId) -> Option<Sequence> {
        let seq = self.sequences.remove(&seq_id);
        self.modified_at = chrono::Utc::now();
        seq
    }

    pub fn get_sequence(&self, seq_id: SequenceId) -> Option<&Sequence> {
        self.sequences.get(&seq_id)
    }

    pub fn get_sequence_mut(&mut self, seq_id: SequenceId) -> Option<&mut Sequence> {
        self.sequences.get_mut(&seq_id)
    }

    pub fn add_media(&mut self, item: MediaItem) {
        self.media.insert(item.id, item);
        self.modified_at = chrono::Utc::now();
    }

    pub fn remove_media(&mut self, media_id: Uuid) -> Option<MediaItem> {
        let item = self.media.remove(&media_id);
        self.modified_at = chrono::Utc::now();
        item
    }

    pub fn add_search_bin(&mut self, bin: SearchBin) {
        self.search_bins.insert(bin.id, bin);
        self.modified_at = chrono::Utc::now();
    }

    pub fn remove_search_bin(&mut self, bin_id: Uuid) -> Option<SearchBin> {
        let bin = self.search_bins.remove(&bin_id);
        self.modified_at = chrono::Utc::now();
        bin
    }

    pub fn update_search_bins(&mut self) {
        for bin in self.search_bins.values_mut() {
            if bin.is_dynamic {
                // Update matches would happen here
                bin.modified_at = chrono::Utc::now();
            }
        }
    }
}

/// Template manager
pub struct TemplateManager {
    templates: IndexMap<TemplateId, ProjectTemplate>,
    template_dir: PathBuf,
}

impl TemplateManager {
    pub fn new(template_dir: PathBuf) -> Self {
        Self {
            templates: IndexMap::new(),
            template_dir,
        }
    }

    pub fn load_templates(&mut self) -> Result<(), ProjectError> {
        if !self.template_dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(&self.template_dir).min_depth(1).max_depth(1) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                let template = ProjectTemplate::from_directory(entry.path())?;
                self.templates.insert(template.manifest.id, template);
            }
        }
        Ok(())
    }

    pub fn get_template(&self, id: TemplateId) -> Option<&ProjectTemplate> {
        self.templates.get(&id)
    }

    pub fn list_templates(&self) -> Vec<&ProjectTemplate> {
        self.templates.values().collect()
    }

    pub fn list_by_category(&self, category: TemplateCategory) -> Vec<&ProjectTemplate> {
        self.templates.values()
            .filter(|t| t.manifest.category == category)
            .collect()
    }

    pub fn create_project(&self, template_id: TemplateId, destination: &Path, variables: HashMap<String, Value>) -> Result<Project, ProjectError> {
        let template = self.templates.get(&template_id)
            .ok_or(ProjectError::TemplateNotFound)?;

        let project = template.create_project(destination, variables)?;
        // Would load the created project here
        Ok(Project::new(
            template.manifest.name.clone(),
            destination.to_path_buf(),
            template.settings.clone(),
        ))
    }
}

/// Search bin manager (re-export)
pub type BinManager = SearchBinManager;

impl ProjectManager {
    pub fn new(template_dir: PathBuf) -> Self {
        let graphics_engine = Arc::new(saturn_graphics::TemplateEngine::new());
        Self {
            template_manager: TemplateManager::new(template_dir),
            bin_manager: SearchBinManager::new(),
            graphics_engine,
            current_project: None,
        }
    }

    pub fn load_templates(&mut self) -> Result<(), ProjectError> {
        self.template_manager.load_templates()
    }

    pub fn create_project(&mut self, template_id: TemplateId, destination: &Path, variables: HashMap<String, Value>) -> Result<Project, ProjectError> {
        let project = self.template_manager.create_project(template_id, destination, variables)?;
        self.current_project = Some(project.clone());
        Ok(project)
    }

    pub fn open_project(&mut self, path: &Path) -> Result<Project, ProjectError> {
        let project_file = path.join("project.saturn");
        if !project_file.exists() {
            return Err(ProjectError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Project file not found"
            )));
        }

        let data = fs::read_to_string(&project_file)?;
        let settings: ProjectSettings = serde_json::from_str(&data)?;

        let mut project = Project::new(
            settings.name.clone(),
            path.to_path_buf(),
            settings,
        );

        // Load sequences, media, bins from project directory
        // This would be implemented based on project structure

        self.current_project = Some(project.clone());
        Ok(project)
    }

    pub fn save_project(&self) -> Result<(), ProjectError> {
        if let Some(project) = &self.current_project {
            let project_file = project.path.join("project.saturn");
            let data = serde_json::to_string_pretty(&project.settings)?;
            fs::write(&project_file, data)?;
        }
        Ok(())
    }

    pub fn current_project(&self) -> Option<&Project> {
        self.current_project.as_ref()
    }

    pub fn current_project_mut(&mut self) -> Option<&mut Project> {
        self.current_project.as_mut()
    }

    pub fn template_manager(&self) -> &TemplateManager {
        &self.template_manager
    }

    pub fn template_manager_mut(&mut self) -> &mut TemplateManager {
        &mut self.template_manager
    }

    pub fn bin_manager(&self) -> &SearchBinManager {
        &self.bin_manager
    }

    pub fn bin_manager_mut(&mut self) -> &mut SearchBinManager {
        &mut self.bin_manager
    }

    pub fn graphics_engine(&self) -> Arc<saturn_graphics::TemplateEngine> {
        Arc::clone(&self.graphics_engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_project_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = ProjectManager::new(dir.path().to_path_buf());
        assert!(manager.current_project().is_none());
    }

    #[test]
    fn test_template_manager() {
        let dir = tempdir().unwrap();
        let mut manager = TemplateManager::new(dir.path().to_path_buf());
        // Would test template loading
    }
}