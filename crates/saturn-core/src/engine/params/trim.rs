use super::*;

/// Payload for selecting an edit point as the trim target.
#[derive(Deserialize)]
pub(in crate::engine) struct TrimSelectParams {
    pub(in crate::engine) track_id: u64,
    pub(in crate::engine) clip_index: usize,
    pub(in crate::engine) edge: TrimEdge,
}

/// Payload for frame-based trim nudges; `frames` is signed.
#[derive(Deserialize)]
pub(in crate::engine) struct TrimNudgeParams {
    pub(in crate::engine) frames: i64,
    #[serde(default)]
    pub(in crate::engine) ripple: Option<bool>,
}

/// Payload for commands anchored to the playhead position.
#[derive(Deserialize)]
pub(in crate::engine) struct TrimAtParams {
    pub(in crate::engine) at: Tick,
}

/// Payload for commands where the playhead position is optional
/// (defaults to time zero).
#[derive(Deserialize)]
pub(in crate::engine) struct TrimOptionalAtParams {
    #[serde(default)]
    pub(in crate::engine) at: Option<Tick>,
}

/// Payload for applying the default transition at the selected edit point.
#[derive(Deserialize)]
pub(in crate::engine) struct TrimTransitionParams {
    #[serde(default)]
    pub(in crate::engine) duration: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) kind: Option<TransitionKind>,
}
