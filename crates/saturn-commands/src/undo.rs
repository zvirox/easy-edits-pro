//! Undo/Redo management system

use crate::CommandId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use uuid::Uuid;

/// Undo entry representing a reversible operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    pub id: Uuid,
    pub description: String,
    pub command_id: CommandId,
    pub forward: Value,
    pub reverse: Value,
    pub timestamp: u64,
    pub group_id: Option<Uuid>,
}

impl UndoEntry {
    pub fn new(
        description: impl Into<String>,
        forward: impl Serialize,
        reverse: impl Serialize,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            command_id: CommandId::new(""),
            forward: serde_json::to_value(forward).unwrap_or_default(),
            reverse: serde_json::to_value(reverse).unwrap_or_default(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            group_id: None,
        }
    }

    pub fn with_command(mut self, command_id: CommandId) -> Self {
        self.command_id = command_id;
        self
    }

    pub fn with_group(mut self, group_id: Uuid) -> Self {
        self.group_id = Some(group_id);
        self
    }
}

/// Undo manager with configurable history size
pub struct UndoManager {
    undo_stack: RwLock<VecDeque<UndoEntry>>,
    redo_stack: RwLock<VecDeque<UndoEntry>>,
    max_size: usize,
    current_group: RwLock<Option<Uuid>>,
}

impl UndoManager {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: RwLock::new(VecDeque::with_capacity(max_size)),
            redo_stack: RwLock::new(VecDeque::new()),
            max_size,
            current_group: RwLock::new(None),
        }
    }

    pub fn push(&self, mut entry: UndoEntry) {
        if let Some(group_id) = *self.current_group.read() {
            entry.group_id = Some(group_id);
        }

        let mut undo_stack = self.undo_stack.write();
        undo_stack.push_back(entry);

        while undo_stack.len() > self.max_size {
            undo_stack.pop_front();
        }

        self.redo_stack.write().clear();
    }

    pub fn undo(&self) -> Option<UndoEntry> {
        let entry = self.undo_stack.write().pop_back()?;
        self.redo_stack.write().push_back(entry.clone());
        Some(entry)
    }

    pub fn redo(&self) -> Option<UndoEntry> {
        let entry = self.redo_stack.write().pop_back()?;
        self.undo_stack.write().push_back(entry.clone());
        Some(entry)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.read().is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.read().is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.read().len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.read().len()
    }

    pub fn history(&self, limit: usize) -> Vec<UndoEntry> {
        self.undo_stack
            .read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn redo_history(&self, limit: usize) -> Vec<UndoEntry> {
        self.redo_stack
            .read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.undo_stack.write().clear();
        self.redo_stack.write().clear();
    }

    pub fn begin_group(&self) -> Uuid {
        let group_id = uuid::Uuid::new_v4();
        *self.current_group.write() = Some(group_id);
        group_id
    }

    pub fn end_group(&self) {
        *self.current_group.write() = None;
    }

    pub fn in_group(&self) -> bool {
        self.current_group.read().is_some()
    }

    pub fn current_group(&self) -> Option<Uuid> {
        *self.current_group.read()
    }
}

/// Transaction for grouping multiple operations
pub struct Transaction<'a> {
    manager: &'a UndoManager,
    group_id: Uuid,
    committed: bool,
}

impl<'a> Transaction<'a> {
    pub fn new(manager: &'a UndoManager) -> Self {
        let group_id = manager.begin_group();
        Self {
            manager,
            group_id,
            committed: false,
        }
    }

    pub fn push(&self, entry: UndoEntry) {
        let mut entry = entry;
        entry.group_id = Some(self.group_id);
        self.manager.push(entry);
    }

    pub fn commit(mut self) {
        self.committed = true;
        self.manager.end_group();
    }

    pub fn rollback(&mut self) {
        let mut undo_stack = self.manager.undo_stack.write();
        let mut entries_to_redo = Vec::new();

        while let Some(entry) = undo_stack.pop_back() {
            if entry.group_id == Some(self.group_id) {
                entries_to_redo.push(entry);
            } else {
                undo_stack.push_back(entry);
                break;
            }
        }

        let mut redo_stack = self.manager.redo_stack.write();
        redo_stack.retain(|e| e.group_id != Some(self.group_id));

        self.committed = true;
        self.manager.end_group();
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.rollback();
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn test_undo_redo() {
        let manager = UndoManager::new(100);

        manager.push(UndoEntry::new(
            "Action 1",
            json!({ "a": 1 }),
            json!({ "a": 0 }),
        ));
        manager.push(UndoEntry::new(
            "Action 2",
            json!({ "b": 2 }),
            json!({ "b": 0 }),
        ));
        manager.push(UndoEntry::new(
            "Action 3",
            json!({ "c": 3 }),
            json!({ "c": 0 }),
        ));

        assert_eq!(manager.undo_count(), 3);
        assert!(manager.can_undo());
        assert!(!manager.can_redo());

        let entry = manager.undo().unwrap();
        assert_eq!(entry.description, "Action 3");
        assert_eq!(manager.undo_count(), 2);
        assert_eq!(manager.redo_count(), 1);
        assert!(manager.can_redo());

        let entry = manager.redo().unwrap();
        assert_eq!(entry.description, "Action 3");
        assert_eq!(manager.undo_count(), 3);
        assert_eq!(manager.redo_count(), 0);
    }

    #[test]
    fn test_transaction_group() {
        let manager = UndoManager::new(100);

        let tx = Transaction::new(&manager);
        let group_id = tx.group_id;
        manager.push(UndoEntry::new("Group Action 1", json!({}), json!({})).with_group(group_id));
        manager.push(UndoEntry::new("Group Action 2", json!({}), json!({})).with_group(group_id));
        tx.commit();

        let entry = manager.undo().unwrap();
        assert_eq!(entry.description, "Group Action 2");
        assert_eq!(entry.group_id, Some(group_id));

        let entry = manager.undo().unwrap();
        assert_eq!(entry.description, "Group Action 1");
        assert_eq!(entry.group_id, Some(group_id));
    }

    #[test]
    fn test_transaction_rollback() {
        let manager = UndoManager::new(100);

        {
            let mut tx = Transaction::new(&manager);
            manager.push(UndoEntry::new("Action 1", json!({}), json!({})).with_group(tx.group_id));
            manager.push(UndoEntry::new("Action 2", json!({}), json!({})).with_group(tx.group_id));
            tx.rollback();
        }

        assert_eq!(manager.undo_count(), 0);
    }

    #[test]
    fn test_max_size_limit() {
        let manager = UndoManager::new(3);

        manager.push(UndoEntry::new("1", json!({}), json!({})));
        manager.push(UndoEntry::new("2", json!({}), json!({})));
        manager.push(UndoEntry::new("3", json!({}), json!({})));
        manager.push(UndoEntry::new("4", json!({}), json!({})));

        assert_eq!(manager.undo_count(), 3);
        let history = manager.history(10);
        assert_eq!(history[0].description, "4");
        assert_eq!(history[2].description, "2");
    }
}
