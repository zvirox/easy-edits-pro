use super::*;


pub(crate) fn migrate_project(value: &mut serde_json::Value) -> Result<(), String> {
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "Project file has no schema_version".to_string())? as u32;
    let fmt = value.get("format").and_then(serde_json::Value::as_str);
    if fmt != Some(PROJECT_FORMAT) && fmt != Some(LEGACY_PROJECT_FORMAT) {
        return Err("This is not a recognized Easy Edits Pro project file".into());
    }
    match version {
        PROJECT_SCHEMA_VERSION => Ok(()),
        1 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            object.insert("schema_version".into(), 2.into());
            object.entry("view").or_insert_with(|| {
                serde_json::json!({
                    "active_sequence": "Sequence 1",
                    "timeline_zoom": 1.0,
                    "timeline_scroll": 0.0
                })
            });
            migrate_project(value)
        }
        2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 => {
            value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?
                .insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
            Ok(())
        }
        unsupported => Err(format!(
            "Project schema version {unsupported} is not supported"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

        #[test]
        fn v9_project_migrates_to_v10_schema() {
            let mut legacy = serde_json::to_value(ProjectDocument::new("V9 project")).unwrap();
            legacy["schema_version"] = 9.into();
            legacy["project"].as_object_mut().unwrap().remove("sequences");
            migrate_project(&mut legacy).unwrap();
            let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
            assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
            assert!(loaded.project.sequences.is_empty());
            assert_eq!(loaded.project.sequence.id, 1);
        }
        #[test]
        fn v1_project_migrates_to_current_schema() {
            let mut legacy = serde_json::to_value(ProjectDocument::new("Legacy")).unwrap();
            legacy["schema_version"] = 1.into();
            legacy.as_object_mut().unwrap().remove("view");
            migrate_project(&mut legacy).unwrap();
            let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
            assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
            assert_eq!(loaded.view.active_sequence.as_deref(), Some("Sequence 1"));
        }
        #[test]
        fn v8_project_migrates_to_v9_schema() {
            let mut legacy = serde_json::to_value(ProjectDocument::new("V8 project")).unwrap();
            legacy["schema_version"] = 8.into();
            legacy["project"].as_object_mut().unwrap().remove("collaboration");
            migrate_project(&mut legacy).unwrap();
            let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
            assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
            assert_eq!(loaded.project.collaboration.session_id, "local-collab-session");
            assert!(loaded.project.sequence.node_graph.is_none());
        }
        #[test]
        fn v2_project_migrates_with_default_edit_properties() {
            let mut legacy = serde_json::to_value(ProjectDocument::new("V2 project")).unwrap();
            legacy["schema_version"] = 2.into();
            for track in legacy["project"]["sequence"]["tracks"]
                .as_array_mut()
                .unwrap()
            {
                track.as_object_mut().unwrap().remove("gain_db");
            }
            legacy["project"]["sequence"]["tracks"][0]["clips"] = serde_json::json!([
                {"media_id": 3, "timeline_start": 0, "source_in": 0, "duration": 100}
            ]);
            migrate_project(&mut legacy).unwrap();
            let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
            assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
            assert_eq!(loaded.project.sequence.tracks[0].gain_db, 0.0);
            let clip = &loaded.project.sequence.tracks[0].clips[0];
            assert_eq!(clip.color.exposure, 0.0);
            assert_eq!(clip.color.contrast, 1.0);
            assert!(clip.keyframes.is_empty());
        }

}
