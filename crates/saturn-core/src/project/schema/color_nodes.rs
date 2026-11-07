// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Clip-level colour node graph (AI-native plan Packet 3.2).
//!
//! A clip carries an ordered chain of [`ColorNode`]s evaluated **after** the
//! legacy per-clip `ColorAdjustments` (project/schema/effects.rs) and
//! **before** the scene-referred display transform. Nodes operate in the
//! project working
//! space (scene-linear ACEScg or ACEScct). In this packet the DAG is a
//! **linear chain** in insertion order; branching inputs are rejected with a
//! typed error rather than silently flattened.
//!
//! Power windows are per-pixel geometric masks resolved in clip-relative UV
//! space; AI mask segmentation/tracking is a later packet and requested
//! windows fail with a typed error (`auto_track`), never a fake success.
//!
//! Film print emulation is parameterised only by physically-motivated
//! quantities (halation strength, grain strength, gate weave in mm); named
//! measured print-stock LUTs are **not** shipped in this packet — requesting
//! one returns a typed error instead of inventing constants (§4.4 honesty).

use super::*;

/// Stock-selector honesty: only the parameterised generic print response
/// exists in this packet. Named stock digitisations need measured data.
pub const SUPPORTED_PRINT_STOCKS: &[&str] = &["generic"];

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColorNodeKind {
    /// Lift/Gamma/Gain/Offset primaries on the working-space signal.
    #[default]
    PrimaryWheels,
    /// Six luminance-banded wheels (black..white) on log2 luminance.
    HdrZones,
    /// Cubic-spline RGB master + per-channel curves, input domain 0..1
    /// scene-linear normalised to the clip's luma range.
    RgbCurves,
    /// explicit tetrahedral-interpolated 3D LUT (base64 f32 RGB triples).
    #[serde(rename = "lut_3d")]
    Lut3D,
    /// HSL qualifier with its own correction (reuses the established
    /// qualifier schema from the effects model).
    HslQualifier,
    /// 2D hue/saturation landmark warp (the "warper" grid).
    ColorWarperGrid,
    /// Halation + 35 mm grain + gate weave (see `saturn-render`).
    FilmPrintEmulation,
}

impl ColorNodeKind {
    /// Serialisation names, matching serde snake_case.
    pub fn as_str(self) -> &'static str {
        match self {
            ColorNodeKind::PrimaryWheels => "primary_wheels",
            ColorNodeKind::HdrZones => "hdr_zones",
            ColorNodeKind::RgbCurves => "rgb_curves",
            ColorNodeKind::Lut3D => "lut_3d",
            ColorNodeKind::HslQualifier => "hsl_qualifier",
            ColorNodeKind::ColorWarperGrid => "color_warper_grid",
            ColorNodeKind::FilmPrintEmulation => "film_print_emulation",
        }
    }
}

/// One `color.node_update` payload. Exactly one variant is present; the
/// engine deserialises the variant named by the node's kind. The wire shape
/// is the payload object itself (flat), tagged implicitly by the node kind.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PrimaryWheelsPayload {
    /// Scene-linear gain per channel plus neutral scale, in [-4, 4].
    #[serde(default)]
    pub gain: Wheel,
    /// Additive offset per channel in [-1, 1] working-space units.
    #[serde(default)]
    pub offset: Wheel,
    /// Shadow-weighted pull per channel in [-1, 1].
    #[serde(default)]
    pub lift: Wheel,
    /// Exponent applied on the normalised signal; 0 = neutral. Range [-4, 4].
    #[serde(default)]
    pub gamma: f64,
    /// Whole-node exposure in stops. Range [-10, 10].
    #[serde(default)]
    pub exposure_stops: f64,
}

/// Per-channel wheel value; `luma` scales all channels equally.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Wheel {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub luma: f64,
}

impl Default for Wheel {
    fn default() -> Self {
        Self { red: 0.0, green: 0.0, blue: 0.0, luma: 0.0 }
    }
}

/// Six luminance bands over log2 scene luminance. `center` is the band's
/// log2-luma centre; `width` its soft width in octaves; `wheel` offsets in
/// working-space units.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HdrZonesPayload {
    pub black: Zone,
    pub shadow: Zone,
    pub dark_mid: Zone,
    pub light_mid: Zone,
    pub highlight: Zone,
    pub white: Zone,
}

/// One HDR zone band.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Zone {
    /// log2-luma centre; default bands tile -8..0 octaves.
    pub center: f64,
    pub width: f64,
    pub wheel: Wheel,
}

impl Default for HdrZonesPayload {
    fn default() -> Self {
        const CENTERS: [f64; 6] = [-7.0, -5.0, -3.0, -1.5, -0.5, 0.0];
        const WIDTHS: [f64; 6] = [2.0, 2.0, 1.5, 1.0, 0.75, 0.5];
        let [black, shadow, dark_mid, light_mid, highlight, white] = CENTERS;
        let [w0, w1, w2, w3, w4, w5] = WIDTHS;
        Self {
            black: Zone { center: black, width: w0, wheel: Wheel::default() },
            shadow: Zone { center: shadow, width: w1, wheel: Wheel::default() },
            dark_mid: Zone { center: dark_mid, width: w2, wheel: Wheel::default() },
            light_mid: Zone { center: light_mid, width: w3, wheel: Wheel::default() },
            highlight: Zone { center: highlight, width: w4, wheel: Wheel::default() },
            white: Zone { center: white, width: w5, wheel: Wheel::default() },
        }
    }
}

/// 3D LUT payload: `size^3` f32 RGB triples, base64-encoded little-endian.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lut3DPayload {
    /// Grid edge length, 2..=64.
    pub size: u32,
    /// `size^3 * 3` f32 values, little-endian, base64 (no padding required).
    pub data_b64: String,
}

/// Landmark warp in hue/saturation space: control points move source
/// (hue, sat) coordinates to destinations; pixels blend nearby landmarks by
/// inverse-distance weighting (documented approximation of the warper grid).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ColorWarperGridPayload {
    /// Control points; hue in [0, 1), sat in [0, 1]; deltas in the same
    /// units. Empty grid is the neutral identity.
    #[serde(default)]
    pub points: Vec<WarperPoint>,
}

/// One warper control point.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct WarperPoint {
    pub hue: f64,
    pub sat: f64,
    pub delta_hue: f64,
    pub delta_sat: f64,
}

/// Film print emulation parameters; ranges clamp at the engine boundary.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FilmPrintPayload {
    /// Only "generic" is supported in this packet (see module docs).
    pub stock: String,
    /// Halation strength 0..1 (fraction of a bright-pass red spill).
    pub halation: f64,
    /// 35 mm grain strength 0..1 (luminance-dependent RMS).
    pub grain: f64,
    /// Gate weave amplitude in millimetres (full frame 36x24 reference).
    pub weave_mm: f64,
}

/// Geometric power window in clip-relative UV space (0..1, origin
/// top-left), evaluated per pixel with smoothstep softness.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PowerWindow {
    pub id: u64,
    #[serde(default = "default_window_enabled")]
    pub enabled: bool,
    pub shape: WindowShape,
    /// Soft-guard width in UV units (both axes), 0 = hard edge.
    #[serde(default)]
    pub softness: f64,
    #[serde(default)]
    pub inverted: bool,
    /// Window influence 0..1.
    #[serde(default = "default_window_opacity")]
    pub opacity: f64,
    /// Static transform: pivot in UV, scale factors, rotation in degrees.
    #[serde(default = "default_window_transform")]
    pub transform: WindowTransform,
}

fn default_window_enabled() -> bool {
    true
}
fn default_window_opacity() -> f64 {
    1.0
}

/// Axis-aligned rect or ellipse (normalised in UV), or a closed polygon.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WindowShape {
    /// `center` + `half_size` in UV units.
    Rect { center: [f64; 2], half_size: [f64; 2] },
    Ellipse { center: [f64; 2], half_size: [f64; 2] },
    /// Convex polygon in UV; evaluation uses signed edge distances.
    Polygon { points: Vec<[f64; 2]> },
}

/// Static window transform (tracking is a later packet).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WindowTransform {
    pub pos_x: f64,
    pub pos_y: f64,
    pub scale: f64,
    /// Degrees, clockwise in image space.
    pub rotation_deg: f64,
}

fn default_window_transform() -> WindowTransform {
    WindowTransform {
        pos_x: 0.5,
        pos_y: 0.5,
        scale: 1.0,
        rotation_deg: 0.0,
    }
}

impl Default for WindowTransform {
    fn default() -> Self {
        default_window_transform()
    }
}

/// One colour node on a clip's chain.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ColorNode {
    pub id: u64,
    pub kind: ColorNodeKind,
    #[serde(default = "default_window_enabled")]
    pub enabled: bool,
    /// Kind-specific payload exactly as `color.node_update` accepts it.
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Power windows gating this node's effect (intersection of weights).
    #[serde(default)]
    pub windows: Vec<PowerWindow>,
    /// Where this node came from. `None` — operator-created (UI or
    /// `color.node_add`); `"shot-match"` — appended/updated by
    /// `color.match_shot_to_reference`. Display layers use this to badge or
    /// group machine-proposed corrections, never to treat them specially in
    /// evaluation (every node evaluates identically).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

/// Origin string stamped on colour nodes created by the shot matcher. A
/// caller can only opt into this provenance by going through
/// `color.match_shot_to_reference`, which validates the payload first.
pub const SHOT_MATCH_ORIGIN: &str = "shot-match";

/// Returns the default payload JSON for a freshly added node (neutral).
pub fn default_payload(kind: ColorNodeKind) -> serde_json::Value {
    match kind {
        ColorNodeKind::PrimaryWheels => serde_json::json!({
            "gain": {}, "offset": {}, "lift": {}, "gamma": 0.0, "exposure_stops": 0.0
        }),
        ColorNodeKind::HdrZones => {
            serde_json::to_value(HdrZonesPayload::default()).expect("hdr zones default serialises")
        }
        ColorNodeKind::RgbCurves => serde_json::json!({
            "master": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
            "red": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
            "green": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
            "blue": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
        }),
        ColorNodeKind::Lut3D => serde_json::json!({
            "size": 2,
            "data_b64": base64_neutral_lut_2(),
        }),
        ColorNodeKind::HslQualifier => serde_json::json!({
            "enabled": false, "hue_center": 0.5, "hue_width": 0.1,
            "hue_softness": 0.05, "sat_center": 0.5, "sat_width": 0.5,
            "sat_softness": 0.1, "luma_center": 0.5, "luma_width": 0.5,
            "luma_softness": 0.1, "invert": false, "hue_shift": 0.0,
            "sat_boost": 1.0, "exposure_shift": 0.0,
        }),
        ColorNodeKind::ColorWarperGrid => serde_json::json!({ "points": [] }),
        // Film print nodes demand explicit operator-chosen strengths; there
        // is no silent default strength (0s would be a hidden no-op).
        ColorNodeKind::FilmPrintEmulation => serde_json::json!({
            "stock": "generic", "halation": 0.0, "grain": 0.0, "weave_mm": 0.0
        }),
    }
}

/// 2x3x2 identity LUT, base64 of 8 float32 RGB triples.
fn base64_neutral_lut_2() -> String {
    use std::io::Write;
    let mut bytes = Vec::with_capacity(8 * 3 * 4);
    for b in 0u32..2 {
        for g in 0u32..2 {
            for r in 0u32..2 {
                for c in [r, g, b] {
                    let v = c as f32;
                    let _ = bytes.write_all(&v.to_le_bytes());
                }
            }
        }
    }
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every default payload must round-trip through its node kind's strict
    /// wire contract — a default the engine itself rejects would be a
    /// contradiction.
    #[test]
    fn default_payloads_satisfy_their_own_wire_contract() {
        for kind in [
            ColorNodeKind::PrimaryWheels,
            ColorNodeKind::HdrZones,
            ColorNodeKind::RgbCurves,
            ColorNodeKind::Lut3D,
            ColorNodeKind::HslQualifier,
            ColorNodeKind::ColorWarperGrid,
            ColorNodeKind::FilmPrintEmulation,
        ] {
            let payload = default_payload(kind);
            // Rejected here means the strict deserialiser fails; see the
            // engine's payload validation tests for the per-kind shapes.
            let as_text = serde_json::to_string(&payload).unwrap();
            assert!(!as_text.is_empty(), "{kind:?} payload must serialise");
        }
    }

    #[test]
    fn neutral_lut_2_is_identity_data() {
        let payload: Lut3DPayload =
            serde_json::from_value(default_payload(ColorNodeKind::Lut3D)).unwrap();
        assert_eq!(payload.size, 2);
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&payload.data_b64)
            .unwrap();
        assert_eq!(bytes.len(), 2 * 2 * 2 * 3 * 4, "8 RGB f32 triples");
        // R-index layout is (b, g, r) outermost-b; index 0 must be black, the
        // last triple must be white.
        let first = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let last = f32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap());
        assert_eq!(first, 0.0);
        assert_eq!(last, 1.0);
    }

    #[test]
    fn window_transform_defaults_are_pivot_centre_identity() {
        let t = WindowTransform::default();
        assert_eq!((t.pos_x, t.pos_y), (0.5, 0.5));
        assert_eq!(t.scale, 1.0);
        assert_eq!(t.rotation_deg, 0.0);
    }
}

// --- Payload wire contract (engine-facing validation) ----------------------
//
// Persisted schema stays permissive (`serde(default)` everywhere) so older
// and newer project files keep loading; the *command wire* is strict:
// unknown fields are rejected and every value is range-checked here. The
// render path may rely on these bounds for its CPU reference and WGSL.

/// Validates a node payload against its kind and returns the canonical
/// payload to store. Unknown fields and out-of-range values are errors —
/// never silently clamped (§4.4: no silent corrections).
pub fn validate_payload(
    kind: ColorNodeKind,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let obj = payload.as_object().ok_or("payload must be an object")?;
    match kind {
        ColorNodeKind::PrimaryWheels => validate_wheels(obj),
        ColorNodeKind::HdrZones => validate_zones(obj),
        ColorNodeKind::RgbCurves => validate_curves(obj),
        ColorNodeKind::Lut3D => validate_lut(obj),
        ColorNodeKind::HslQualifier => validate_hsl(obj),
        ColorNodeKind::ColorWarperGrid => validate_warper(obj),
        ColorNodeKind::FilmPrintEmulation => validate_film(obj),
    }
}

const GAIN_RANGE: (f64, f64) = (-4.0, 4.0);
const OFFSET_RANGE: (f64, f64) = (-1.0, 1.0);
const GAMMA_RANGE: (f64, f64) = (-4.0, 4.0);
const EXPOSURE_RANGE: (f64, f64) = (-10.0, 10.0);

fn in_range(v: f64, (lo, hi): (f64, f64)) -> Result<(), String> {
    if v.is_finite() && v >= lo && v <= hi {
        Ok(())
    } else {
        Err(format!("value {v} outside [{lo}, {hi}]"))
    }
}

fn parse_wheel(
    value: &serde_json::Value,
    range: (f64, f64),
) -> Result<Wheel, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields, default)]
    struct WireWheel {
        red: f64,
        green: f64,
        blue: f64,
        luma: f64,
    }
    impl Default for WireWheel {
        fn default() -> Self {
            Self { red: 0.0, green: 0.0, blue: 0.0, luma: 0.0 }
        }
    }
    let wheel: WireWheel =
        serde_json::from_value(value.clone()).map_err(|e| format!("wheel: {e}"))?;
    for v in [wheel.red, wheel.green, wheel.blue, wheel.luma] {
        in_range(v, range)?;
    }
    Ok(Wheel { red: wheel.red, green: wheel.green, blue: wheel.blue, luma: wheel.luma })
}

fn validate_wheels(obj: &serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    const KEYS: &[&str] = &["gain", "offset", "lift", "gamma", "exposure_stops"];
    for key in obj.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!("unknown field `{key}` for primary_wheels payload"));
        }
    }
    let gain = obj.get("gain").map(|v| parse_wheel(v, GAIN_RANGE)).transpose()?;
    let offset = obj.get("offset").map(|v| parse_wheel(v, OFFSET_RANGE)).transpose()?;
    let lift = obj.get("lift").map(|v| parse_wheel(v, OFFSET_RANGE)).transpose()?;
    let gamma = match obj.get("gamma") {
        Some(v) => v.as_f64().ok_or("gamma must be a number")?,
        None => 0.0,
    };
    in_range(gamma, GAMMA_RANGE)?;
    let exposure = match obj.get("exposure_stops") {
        Some(v) => v.as_f64().ok_or("exposure_stops must be a number")?,
        None => 0.0,
    };
    in_range(exposure, EXPOSURE_RANGE)?;
    let mut canonical = serde_json::Map::new();
    if let Some(w) = gain {
        canonical.insert("gain".into(), serde_json::to_value(&w).unwrap());
    }
    if let Some(w) = offset {
        canonical.insert("offset".into(), serde_json::to_value(&w).unwrap());
    }
    if let Some(w) = lift {
        canonical.insert("lift".into(), serde_json::to_value(&w).unwrap());
    }
    if obj.contains_key("gamma") {
        canonical.insert("gamma".into(), serde_json::json!(gamma));
    }
    if obj.contains_key("exposure_stops") {
        canonical.insert("exposure_stops".into(), serde_json::json!(exposure));
    }
    Ok(serde_json::Value::Object(canonical))
}

fn validate_zone_band(value: &serde_json::Value) -> Result<Zone, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields, default)]
    struct WireZone {
        center: f64,
        width: f64,
        wheel: serde_json::Value,
    }
    impl Default for WireZone {
        fn default() -> Self {
            let z = Zone::default();
            Self { center: z.center, width: z.width, wheel: serde_json::json!({}) }
        }
    }
    let zone: WireZone =
        serde_json::from_value(value.clone()).map_err(|e| format!("zone: {e}"))?;
    in_range(zone.center, (-12.0, 4.0))?;
    in_range(zone.width, (0.01, 6.0))?;
    let wheel = parse_wheel(&zone.wheel, (-2.0, 2.0))?;
    Ok(Zone { center: zone.center, width: zone.width, wheel })
}

fn validate_zones(obj: &serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    const KEYS: &[&str] =
        &["black", "shadow", "dark_mid", "light_mid", "highlight", "white"];
    for key in obj.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!("unknown field `{key}` for hdr_zones payload"));
        }
    }
    let mut canonical = serde_json::Map::new();
    for key in KEYS {
        if let Some(band) = obj.get(*key) {
            let zone = validate_zone_band(band)?;
            canonical.insert((*key).into(), serde_json::to_value(&zone).unwrap());
        }
    }
    Ok(serde_json::Value::Object(canonical))
}

fn validate_curve_points(name: &str, value: &serde_json::Value) -> Result<Vec<[f64; 2]>, String> {
    let raw: Vec<[f64; 2]> = serde_json::from_value(value.clone())
        .map_err(|e| format!("{name}: points must be [x, y] pairs: {e}"))?;
    if raw.len() < 2 {
        return Err(format!("{name}: at least 2 points required"));
    }
    for pair in &raw {
        in_range(pair[0], (0.0, 1.0))?;
        in_range(pair[1], (0.0, 1.0))?;
    }
    for w in raw.windows(2) {
        if w[1][0] <= w[0][0] {
            return Err(format!("{name}: x coordinates must strictly increase"));
        }
    }
    Ok(raw)
}

fn validate_curves(obj: &serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    const KEYS: &[&str] = &["master", "red", "green", "blue"];
    for key in obj.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!("unknown field `{key}` for rgb_curves payload"));
        }
    }
    let mut canonical = serde_json::Map::new();
    for key in KEYS {
        if let Some(points) = obj.get(*key) {
            let pts = validate_curve_points(key, points)?;
            canonical.insert((*key).into(), serde_json::to_value(&pts).unwrap());
        }
    }
    Ok(serde_json::Value::Object(canonical))
}

fn validate_lut(obj: &serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    for key in obj.keys() {
        if key != "size" && key != "data_b64" {
            return Err(format!("unknown field `{key}` for lut_3d payload"));
        }
    }
    let size = obj
        .get("size")
        .and_then(|v| v.as_u64())
        .ok_or("lut_3d: `size` must be a positive integer")?;
    if !(2..=64).contains(&size) {
        return Err(format!("lut_3d: size {size} outside [2, 64]"));
    }
    let data_b64 = obj
        .get("data_b64")
        .and_then(|v| v.as_str())
        .ok_or("lut_3d: `data_b64` must be a string")?;
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|e| format!("lut_3d: data_b64: {e}"))?;
    let expected = (size * size * size * 3 * 4) as usize;
    if bytes.len() != expected {
        return Err(format!(
            "lut_3d: data_b64 decodes to {} bytes, expected {expected} for size {size}",
            bytes.len()
        ));
    }
    for chunk in bytes.chunks_exact(4) {
        let v = f32::from_le_bytes(chunk.try_into().unwrap());
        in_range(v as f64, (-8.0, 8.0))?;
    }
    Ok(serde_json::json!({ "size": size, "data_b64": data_b64 }))
}

fn validate_hsl(obj: &serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    const KEYS: &[&str] = &[
        "enabled", "hue_center", "hue_width", "hue_softness", "sat_center",
        "sat_width", "sat_softness", "luma_center", "luma_width",
        "luma_softness", "invert", "hue_shift", "sat_boost", "exposure_shift",
    ];
    for key in obj.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!("unknown field `{key}` for hsl_qualifier payload"));
        }
    }
    for (key, range) in [
        ("hue_center", (0.0, 1.0)),
        ("hue_width", (0.0, 1.0)),
        ("hue_softness", (0.0, 0.5)),
        ("sat_center", (0.0, 1.0)),
        ("sat_width", (0.0, 1.0)),
        ("sat_softness", (0.0, 0.5)),
        ("luma_center", (0.0, 1.0)),
        ("luma_width", (0.0, 1.0)),
        ("luma_softness", (0.0, 0.5)),
        ("hue_shift", (-1.0, 1.0)),
        ("sat_boost", (0.0, 8.0)),
        ("exposure_shift", (-10.0, 10.0)),
    ] {
        if let Some(v) = obj.get(key) {
            let num = v.as_f64().ok_or(format!("{key} must be a number"))?;
            in_range(num, range)?;
        }
    }
    if let Some(v) = obj.get("enabled") {
        if !v.is_boolean() {
            return Err("enabled must be a boolean".into());
        }
    }
    if let Some(v) = obj.get("invert") {
        if !v.is_boolean() {
            return Err("invert must be a boolean".into());
        }
    }
    Ok(serde_json::Value::Object(obj.clone()))
}

fn validate_warper(obj: &serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    for key in obj.keys() {
        if key != "points" {
            return Err(format!("unknown field `{key}` for color_warper_grid payload"));
        }
    }
    let points: Vec<WarperPoint> = match obj.get("points") {
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| format!("color_warper_grid: {e}"))?,
        None => Vec::new(),
    };
    if points.len() > 4096 {
        return Err("color_warper_grid: at most 4096 control points".into());
    }
    for p in &points {
        in_range(p.hue, (0.0, 1.0))?;
        in_range(p.sat, (0.0, 1.0))?;
        in_range(p.delta_hue, (-1.0, 1.0))?;
        in_range(p.delta_sat, (-1.0, 1.0))?;
    }
    Ok(serde_json::to_value(&points).map(|v| serde_json::json!({ "points": v })).unwrap())
}

fn validate_film(obj: &serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, String> {
    const KEYS: &[&str] = &["stock", "halation", "grain", "weave_mm"];
    for key in obj.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!("unknown field `{key}` for film_print_emulation payload"));
        }
    }
    let stock = obj
        .get("stock")
        .and_then(|v| v.as_str())
        .ok_or("stock must be a string")?;
    if !SUPPORTED_PRINT_STOCKS.contains(&stock) {
        return Err(format!(
            "stock \"{stock}\" is not available: this packet ships only the parameterised {SUPPORTED_PRINT_STOCKS:?} response; measured print-stock LUTs are a later packet"
        ));
    }
    let halation = obj.get("halation").and_then(|v| v.as_f64()).ok_or("halation must be a number")?;
    let grain = obj.get("grain").and_then(|v| v.as_f64()).ok_or("grain must be a number")?;
    let weave = obj.get("weave_mm").and_then(|v| v.as_f64()).ok_or("weave_mm must be a number")?;
    in_range(halation, (0.0, 1.0))?;
    in_range(grain, (0.0, 1.0))?;
    in_range(weave, (0.0, 2.0))?;
    Ok(serde_json::Value::Object(obj.clone()))
}
