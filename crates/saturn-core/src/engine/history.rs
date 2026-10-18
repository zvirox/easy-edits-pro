use super::*;

impl EditorEngine {
    pub(super) fn checkpoint(&mut self) {
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
