use super::*;

impl EditorEngine {
    pub(super) fn dispatch_project(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "project.new" => Some((|| {
                let params: NewProjectParams = parse_params(params)?;
                if params.name.trim().is_empty() || params.sequence_name.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Project and timeline names cannot be empty".into(),
                    ));
                }
                let mut document = ProjectDocument::new(params.name.trim());
                document.project.settings.width = params.width;
                document.project.settings.height = params.height;
                document.project.settings.frame_rate = params.frame_rate;
                document.project.sequence.name = params.sequence_name.trim().to_owned();
                document.view.active_sequence = Some(params.sequence_name.trim().to_owned());
                document
                    .validate()
                    .map_err(DispatchError::InvalidParameters)?;
                self.checkpoint();
                self.document = document;
                self.current_path = None;
                Ok(json!({"name": self.document.project.name}))
            })()),
            "file.save" => Some((|| {
                let params: FilePathParams = parse_params(params)?;
                self.document
                    .save_atomic(&params.path)
                    .map_err(DispatchError::Operation)?;
                self.current_path = Some(params.path.clone());
                self.saved_snapshot = serde_json::to_string(&self.document).unwrap_or_default();
                Ok(json!({"path": params.path}))
            })()),
            "file.open" => Some((|| {
                let params: FilePathParams = parse_params(params)?;
                let loaded =
                    ProjectDocument::load(&params.path).map_err(DispatchError::Operation)?;
                self.document = loaded;
                self.current_path = Some(params.path.clone());
                self.undo.clear();
                self.redo.clear();
                self.saved_snapshot = serde_json::to_string(&self.document).unwrap_or_default();
                Ok(json!({"name": self.document.project.name, "path": params.path}))
            })()),
            "interop.import_edl" => Some((|| {
                let params: ImportEdlParams = parse_params(params)?;
                let name = params
                    .project_name
                    .unwrap_or_else(|| "Imported EDL".to_string());
                let imported = crate::interop::import_cmx3600_edl(&params.edl, &name)
                    .map_err(DispatchError::Operation)?;
                self.checkpoint();
                self.document = imported;
                self.current_path = None;
                Ok(json!({"imported": true, "name": name}))
            })()),
            "interop.import_fcpxml" => Some((|| {
                let params: ImportXmlParams = parse_params(params)?;
                let imported =
                    crate::interop::import_fcpxml(&params.xml).map_err(DispatchError::Operation)?;
                let name = imported.project.name.clone();
                self.checkpoint();
                self.document = imported;
                self.current_path = None;
                Ok(json!({"imported": true, "name": name}))
            })()),
            "interop.import_premiere_xml" => Some((|| {
                let params: ImportXmlParams = parse_params(params)?;
                let imported = crate::interop::import_premiere_xml(&params.xml)
                    .map_err(DispatchError::Operation)?;
                let name = imported.project.name.clone();
                self.checkpoint();
                self.document = imported;
                self.current_path = None;
                Ok(json!({"imported": true, "name": name}))
            })()),
            "project.switch_sequence" => Some((|| {
                let params: SwitchSequenceParams = parse_params(params)?;
                if self.document.project.sequence.id == params.sequence_id {
                    return Ok(json!({
                        "sequence_id": params.sequence_id,
                        "name": self.document.project.sequence.name,
                        "already_active": true
                    }));
                }

                let target_index = self
                    .document
                    .project
                    .sequences
                    .iter()
                    .position(|s| s.id == params.sequence_id)
                    .ok_or_else(|| {
                        DispatchError::Operation(format!(
                            "Sequence id {} not found",
                            params.sequence_id
                        ))
                    })?;

                self.checkpoint();

                // Swap current sequence with target sequence
                let old_sequence = std::mem::replace(
                    &mut self.document.project.sequence,
                    self.document.project.sequences.remove(target_index),
                );
                let new_name = self.document.project.sequence.name.clone();
                let new_id = self.document.project.sequence.id;
                self.document.project.sequences.push(old_sequence);
                self.document.view.active_sequence = Some(new_name.clone());

                Ok(json!({
                    "sequence_id": new_id,
                    "name": new_name,
                    "switched": true
                }))
            })()),
            "interop.import_otio" => Some((|| {
                let params: ImportOtioParams = parse_params(params)?;
                let project_name = params.project_name.as_deref().unwrap_or("Imported OTIO");
                let imported_doc = crate::interop::import_otio(&params.content, project_name)
                    .map_err(DispatchError::Operation)?;
                self.checkpoint();
                self.document = imported_doc;
                Ok(json!({
                    "imported": true,
                    "tracks": self.document.project.sequence.tracks.len(),
                    "media_count": self.document.project.media.len(),
                }))
            })()),
            "project.inspect" => Some(
                serde_json::to_value(&self.document)
                    .map_err(|error| DispatchError::Operation(error.to_string())),
            ),
            "project.status" => Some(Ok(json!({
                "document": &self.document,
                "path": self.current_path.as_ref(),
                "is_dirty": self.is_dirty(),
            }))),
            _ => None,
        }
    }
}
