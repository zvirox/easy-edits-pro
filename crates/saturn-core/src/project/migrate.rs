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

