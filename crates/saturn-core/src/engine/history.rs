use super::*;

impl EditorEngine {
    pub(super) fn checkpoint(&mut self) {
        if self.gesture_depth > 0 {
            if self.gesture_checkpointed {
                return;
            }
            self.gesture_checkpointed = true;
        }
        self.undo.push(self.document.clone());
        let label = self
            .pending_label
            .take()
            .unwrap_or_else(|| "unknown".into());
        self.undo_labels.push(label);
        if self.undo.len() > 100 {
            self.undo.remove(0);
            self.undo_labels.remove(0);
        }
        self.redo.clear();
        self.redo_labels.clear();
    }

    pub(super) fn dispatch_history(
        &mut self,
        id: &str,
        _params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "edit.begin_gesture" => Some((|| {
                self.begin_gesture();
                Ok(json!({"gesture": true, "depth": self.gesture_depth}))
            })()),
            "edit.end_gesture" => Some((|| {
                self.end_gesture();
                Ok(json!({"gesture": self.in_gesture(), "depth": self.gesture_depth}))
            })()),
            "edit.undo" => Some((|| {
                let previous = self.undo.pop().expect("availability checked");
                let label = self.undo_labels.pop().unwrap_or_default();
                self.redo.push(self.document.clone());
                self.redo_labels.push(label);
                self.document = previous;
                Ok(json!({"undone": true}))
            })()),
            "edit.redo" => Some((|| {
                let next = self.redo.pop().expect("availability checked");
                let label = self.redo_labels.pop().unwrap_or_default();
                self.undo.push(self.document.clone());
                self.undo_labels.push(label);
                self.document = next;
                Ok(json!({"redone": true}))
            })()),
            // Query: the undo/redo stacks with entry labels, newest first.
            "history.list" => Some((|| {
                let undo: Vec<Value> = self
                    .undo_labels
                    .iter()
                    .rev()
                    .map(|label| json!({"label": label}))
                    .collect();
                let redo: Vec<Value> = self
                    .redo_labels
                    .iter()
                    .rev()
                    .map(|label| json!({"label": label}))
                    .collect();
                Ok(json!({
                    "undo": undo,
                    "redo": redo,
                    "current": self.document.project.name,
                }))
            })()),
            _ => None,
        }
    }
}
