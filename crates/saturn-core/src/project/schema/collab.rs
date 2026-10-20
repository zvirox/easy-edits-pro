use super::*;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockTargetKind {
    #[default]
    Bin,
    Sequence,
    Track,
    Clip,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserLock {
    pub user_id: String,
    pub user_name: String,
    #[serde(default = "default_user_color")]
    pub user_color: String,
    pub target_kind: LockTargetKind,
    pub target_id: u64,
    pub locked_at: u64,
}

fn default_user_color() -> String {
    "#38bdf8".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimelineChangeEntry {
    pub id: u64,
    pub user_id: String,
    pub user_name: String,
    pub timestamp: u64,
    pub action: String,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollaborationState {
    #[serde(default = "default_session_id")]
    pub session_id: String,
    #[serde(default)]
    pub active_users: Vec<String>,
    #[serde(default)]
    pub locks: Vec<UserLock>,
    #[serde(default)]
    pub change_log: Vec<TimelineChangeEntry>,
}

fn default_session_id() -> String {
    "local-collab-session".into()
}

impl Default for CollaborationState {
    fn default() -> Self {
        Self {
            session_id: default_session_id(),
            active_users: Vec::new(),
            locks: Vec::new(),
            change_log: Vec::new(),
        }
    }
}
