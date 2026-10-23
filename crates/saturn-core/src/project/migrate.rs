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
    // v13 replaced the flat `subtitles` list with caption tracks. Every older
    // version that may carry subtitles converts before landing on the current
    // schema, otherwise their cues would be dropped.
    let convert_subtitles = |sequence: &mut serde_json::Map<String, serde_json::Value>| {
        let cues = sequence
            .remove("subtitles")
            .unwrap_or_else(|| serde_json::json!([]));
        sequence
            .entry("caption_tracks")
            .or_insert_with(|| {
                serde_json::json!([
                    {
                        "id": 1,
                        "name": "Captions",
                        "language": "en",
                        "cues": cues,
                        "styles": []
                    }
                ])
            });
    };
    let convert_all_sequences = |object: &mut serde_json::Map<String, serde_json::Value>| {
        if let Some(sequence) = object
            .get_mut("project")
            .and_then(|project| project.get_mut("sequence"))
            .and_then(|sequence| sequence.as_object_mut())
        {
            convert_subtitles(sequence);
        }
        if let Some(sequences) = object
            .get_mut("project")
            .and_then(|project| project.get_mut("sequences"))
            .and_then(|sequences| sequences.as_array_mut())
        {
            for sequence in sequences {
                if let Some(map) = sequence.as_object_mut() {
                    convert_subtitles(map);
                }
            }
        }
    };
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
        13 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            // v14 adds sequence graphics and project graphic templates. Every
            // sequence needs the arrays so an older file round-trips without
            // losing the fields; entity structs carry serde defaults.
            let add_graphics = |sequence: &mut serde_json::Map<String, serde_json::Value>| {
                sequence
                    .entry("graphics")
                    .or_insert_with(|| serde_json::json!([]));
            };
            if let Some(sequence) = object
                .get_mut("project")
                .and_then(|project| project.get_mut("sequence"))
                .and_then(|sequence| sequence.as_object_mut())
            {
                add_graphics(sequence);
            }
            if let Some(sequences) = object
                .get_mut("project")
                .and_then(|project| project.get_mut("sequences"))
                .and_then(|sequences| sequences.as_array_mut())
            {
                for sequence in sequences {
                    if let Some(map) = sequence.as_object_mut() {
                        add_graphics(map);
                    }
                }
            }
            if let Some(project) = object.get_mut("project").and_then(|project| project.as_object_mut())
            {
                project
                    .entry("graphic_templates")
                    .or_insert_with(|| serde_json::json!([]));
            }
            object.insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
            Ok(())
        }
        12 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            convert_all_sequences(object);
            object.insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
            Ok(())
        }
        11 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            // v12 adds descriptive asset metadata. Existing projects get empty
            // strings rather than missing fields.
            if let Some(media) = object
                .get_mut("project")
                .and_then(|project| project.get_mut("media"))
                .and_then(|media| media.as_array_mut())
            {
                for item in media {
                    if let Some(map) = item.as_object_mut() {
                        for field in ["comment", "scene", "take"] {
                            map.entry(field).or_insert_with(|| serde_json::json!(""));
                        }
                    }
                }
            }
            convert_all_sequences(object);
            object.insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
            Ok(())
        }
        10 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            // v11 adds sequence markers. Every sequence (active and stored) needs
            // the array so an older file round-trips without losing the field.
            if let Some(sequence) = object
                .get_mut("project")
                .and_then(|project| project.get_mut("sequence"))
                .and_then(|sequence| sequence.as_object_mut())
            {
                sequence
                    .entry("markers")
                    .or_insert_with(|| serde_json::json!([]));
            }
            if let Some(sequences) = object
                .get_mut("project")
                .and_then(|project| project.get_mut("sequences"))
                .and_then(|sequences| sequences.as_array_mut())
            {
                for sequence in sequences {
                    if let Some(map) = sequence.as_object_mut() {
                        map.entry("markers")
                            .or_insert_with(|| serde_json::json!([]));
                    }
                }
            }
            convert_all_sequences(object);
            object.insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
            Ok(())
        }
        2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Project root must be an object".to_string())?;
            convert_all_sequences(object);
            object.insert("schema_version".into(), PROJECT_SCHEMA_VERSION.into());
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
        legacy["project"]
            .as_object_mut()
            .unwrap()
            .remove("sequences");
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert!(loaded.project.sequences.is_empty());
        assert_eq!(loaded.project.sequence.id, 1);
    }
    #[test]
    fn v10_project_gains_marker_arrays() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("V10 project")).unwrap();
        legacy["schema_version"] = 10.into();
        legacy["project"]["sequence"]
            .as_object_mut()
            .unwrap()
            .remove("markers");
        let mut stored = serde_json::to_value(ProjectDocument::new("Stored")).unwrap();
        stored["project"]["sequence"]
            .as_object_mut()
            .unwrap()
            .remove("markers");
        legacy["project"]["sequences"] = serde_json::json!([stored["project"]["sequence"]]);
        migrate_project(&mut legacy).unwrap();
        assert_eq!(legacy["schema_version"], serde_json::json!(PROJECT_SCHEMA_VERSION));
        assert_eq!(legacy["project"]["sequence"]["markers"], serde_json::json!([]));
        assert_eq!(
            legacy["project"]["sequences"][0]["markers"],
            serde_json::json!([])
        );
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert!(loaded.project.sequence.markers.is_empty());
        assert!(loaded.project.sequences[0].markers.is_empty());
    }

    #[test]
    fn v11_project_gains_asset_metadata_fields() {
        let mut legacy = serde_json::to_value(ProjectDocument::new("V11 project")).unwrap();
        legacy["schema_version"] = 11.into();
        legacy["project"]["media"] = serde_json::json!([{
            "id": 4,
            "name": "shot.mov",
            "path": "/media/shot.mov",
            "kind": "video"
        }]);
        migrate_project(&mut legacy).unwrap();
        assert_eq!(legacy["schema_version"], serde_json::json!(PROJECT_SCHEMA_VERSION));
        assert_eq!(legacy["project"]["media"][0]["comment"], serde_json::json!(""));
        assert_eq!(legacy["project"]["media"][0]["take"], serde_json::json!(""));
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert!(loaded.project.media[0].comment.is_empty());
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
        migrate_project(&mut legacy).unwrap();
        let loaded: ProjectDocument = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
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
