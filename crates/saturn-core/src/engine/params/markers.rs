use super::*;
use crate::project::MarkerKind;

#[derive(Deserialize)]
pub(in crate::engine) struct AddMarkerParams {
    pub(in crate::engine) at: Tick,
    #[serde(default)]
    pub(in crate::engine) duration: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
    #[serde(default)]
    pub(in crate::engine) kind: MarkerKind,
    #[serde(default)]
    pub(in crate::engine) color: String,
    #[serde(default)]
    pub(in crate::engine) note: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct AddRangeMarkerParams {
    pub(in crate::engine) start: Tick,
    pub(in crate::engine) end: Tick,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
    #[serde(default)]
    pub(in crate::engine) kind: MarkerKind,
    #[serde(default)]
    pub(in crate::engine) color: String,
    #[serde(default)]
    pub(in crate::engine) note: String,
}

#[derive(Deserialize)]
pub(in crate::engine) struct UpdateMarkerParams {
    pub(in crate::engine) id: u64,
    #[serde(default)]
    pub(in crate::engine) at: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) duration: Option<Tick>,
    #[serde(default)]
    pub(in crate::engine) name: Option<String>,
    #[serde(default)]
    pub(in crate::engine) kind: Option<MarkerKind>,
    #[serde(default)]
    pub(in crate::engine) color: Option<String>,
    #[serde(default)]
    pub(in crate::engine) note: Option<String>,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MarkerIdParams {
    pub(in crate::engine) id: u64,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MarkerNavParams {
    pub(in crate::engine) from: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MarkerSplitParams {
    pub(in crate::engine) at: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct RippleMarkersParams {
    pub(in crate::engine) from: Tick,
    pub(in crate::engine) delta: Tick,
}

#[derive(Deserialize)]
pub(in crate::engine) struct MarkerImportParams {
    pub(in crate::engine) csv: String,
    #[serde(default)]
    pub(in crate::engine) replace: bool,
}
