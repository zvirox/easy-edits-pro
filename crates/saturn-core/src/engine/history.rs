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
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
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
                self.redo.push(self.document.clone());
                self.document = previous;
                Ok(json!({"undone": true}))
            })()),
            "edit.redo" => Some((|| {
                let next = self.redo.pop().expect("availability checked");
                self.undo.push(self.document.clone());
                self.document = next;
                Ok(json!({"redone": true}))
            })()),
            _ => None,
        }
    }
}
