use super::*;

impl EditorEngine {
    pub(super) fn dispatch_subtitles(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "subtitle.add" => Some((|| {
                let params: AddSubtitleParams = parse_params(params)?;
                if params.start_ticks.0 < 0 || params.end_ticks.0 <= params.start_ticks.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Subtitle start time must be non-negative and before end time".into(),
                    ));
                }
                if params.text.trim().is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Subtitle text cannot be empty".into(),
                    ));
                }
                self.checkpoint();
                let sub_id = self.document.project.next_id;
                self.document.project.next_id = sub_id.saturating_add(1);
                self.document.project.sequence.subtitles.push(SubtitleItem {
                    id: sub_id,
                    start_ticks: params.start_ticks,
                    end_ticks: params.end_ticks,
                    text: params.text,
                });
                self.document
                    .project
                    .sequence
                    .subtitles
                    .sort_by_key(|s| s.start_ticks);
                Ok(json!({"added": true, "id": sub_id}))
            })()),
            "subtitle.remove" => Some((|| {
                let params: RemoveSubtitleParams = parse_params(params)?;
                let initial = self.document.project.sequence.subtitles.len();
                self.checkpoint();
                self.document
                    .project
                    .sequence
                    .subtitles
                    .retain(|s| s.id != params.id);
                Ok(json!({"removed": self.document.project.sequence.subtitles.len() < initial}))
            })()),
            "subtitle.update" => Some((|| {
                let params: UpdateSubtitleParams = parse_params(params)?;
                let Some(sub) = self
                    .document
                    .project
                    .sequence
                    .subtitles
                    .iter_mut()
                    .find(|s| s.id == params.id)
                else {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Subtitle {} does not exist",
                        params.id
                    )));
                };
                let new_start = params.start_ticks.unwrap_or(sub.start_ticks);
                let new_end = params.end_ticks.unwrap_or(sub.end_ticks);
                if new_start.0 < 0 || new_end.0 <= new_start.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Invalid subtitle timing: start must precede end".into(),
                    ));
                }
                self.checkpoint();
                let sub = self
                    .document
                    .project
                    .sequence
                    .subtitles
                    .iter_mut()
                    .find(|s| s.id == params.id)
                    .unwrap();
                sub.start_ticks = new_start;
                sub.end_ticks = new_end;
                if let Some(t) = params.text {
                    sub.text = t;
                }
                self.document
                    .project
                    .sequence
                    .subtitles
                    .sort_by_key(|s| s.start_ticks);
                Ok(json!({"updated": true, "id": params.id}))
            })()),
            "subtitle.clear" => Some((|| {
                self.checkpoint();
                self.document.project.sequence.subtitles.clear();
                Ok(json!({"cleared": true}))
            })()),
            _ => None,
        }
    }
}
