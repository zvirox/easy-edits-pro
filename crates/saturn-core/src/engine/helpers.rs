use super::*;

pub(super) fn parse_params<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, DispatchError> {
    serde_json::from_value(value).map_err(|e| DispatchError::InvalidParameters(e.to_string()))
}

pub(super) fn find_track_index(document: &ProjectDocument, track_id: u64) -> Result<usize, DispatchError> {
    document
        .project
        .sequence
        .tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or_else(|| {
            DispatchError::InvalidParameters(format!("Timeline track {track_id} does not exist"))
        })
}

pub(super) fn find_clip(
    document: &ProjectDocument,
    track_index: usize,
    clip_index: usize,
) -> Result<&Clip, DispatchError> {
    document.project.sequence.tracks[track_index]
        .clips
        .get(clip_index)
        .ok_or_else(|| DispatchError::InvalidParameters("Timeline clip does not exist".into()))
}

pub(super) fn valid_keyframe_value(property: &KeyframeProperty, value: f64) -> bool {
    match property {
        KeyframeProperty::PositionX | KeyframeProperty::PositionY => {
            (-100_000.0..=100_000.0).contains(&value)
        }
        KeyframeProperty::Scale => (0.01..=100.0).contains(&value),
        KeyframeProperty::Rotation => (-36_000.0..=36_000.0).contains(&value),
        KeyframeProperty::Opacity => (0.0..=1.0).contains(&value),
    }
}

pub(super) fn keyframe_value_at(keyframes: &[Keyframe], property: &KeyframeProperty, time: i64) -> Option<f64> {
    let mut keys: Vec<_> = keyframes.iter().filter(|key| &key.property == property).collect();
    keys.sort_by_key(|key| key.time);
    let first = *keys.first()?;
    if time <= first.time.0 { return Some(first.value); }
    let last = *keys.last()?;
    if time >= last.time.0 { return Some(last.value); }
    let next_index = keys.iter().position(|key| key.time.0 >= time)?;
    let previous = keys[next_index - 1];
    let next = keys[next_index];
    let fraction = (time - previous.time.0) as f64 / (next.time.0 - previous.time.0) as f64;
    Some(previous.value + (next.value - previous.value) * fraction)
}


pub(super) fn get_or_create_node_graph<'a>(
    doc: &'a mut ProjectDocument,
    track_id: Option<u64>,
    clip_index: Option<usize>,
) -> Result<&'a mut NodeGraph, DispatchError> {
    if let (Some(t_id), Some(c_idx)) = (track_id, clip_index) {
        let track_idx = find_track_index(doc, t_id)?;
        find_clip(doc, track_idx, c_idx)?;
        let clip_mut = &mut doc.project.sequence.tracks[track_idx].clips[c_idx];
        if clip_mut.node_graph.is_none() {
            clip_mut.node_graph = Some(NodeGraph::default());
        }
        Ok(clip_mut.node_graph.as_mut().unwrap())
    } else {
        if doc.project.sequence.node_graph.is_none() {
            doc.project.sequence.node_graph = Some(NodeGraph::default());
        }
        Ok(doc.project.sequence.node_graph.as_mut().unwrap())
    }
}

