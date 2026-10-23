//! Keyframe evaluation for the render path (F4: keyframes baked into MP4).
//!
//! This is a deliberate 1:1 port of the preview evaluator in
//! `ui/src/engine/playback/interpolation.ts` (`solveCubicBezier` +
//! `interpolateKeyframe`) so exported frames match what the Program monitor
//! showed, including the CSS cubic-bezier easing curves and handle fallbacks.
//!
//! Timings are handled by the caller: preview evaluates keyframes with times
//! relative to the clip start (`clipTime = playhead - clip.timeline_start`);
//! the render stages the same curves shifted to timeline-absolute ticks and
//! samples them with the buffer PTS through the same convention the
//! `saturnaudioauto` element proved (`pts_to_ticks`).

use saturn_core::project::{InterpolationKind, Keyframe, KeyframeProperty};
use saturn_core::time::Tick;

/// Solve a CSS-style cubic bezier easing at `t`: returns the curve's y
/// progress for a given x (time) progress, with control points (x1, y1) and
/// (x2, y2). Ported verbatim from `solveCubicBezier` (Newton-Raphson with a
/// binary-subdivision fallback) so easing matches the preview exactly.
pub fn solve_cubic_bezier(t: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let clamped_t = t.clamp(0.0, 1.0);
    if clamped_t <= 0.0 {
        return 0.0;
    }
    if clamped_t >= 1.0 {
        return 1.0;
    }

    // B(s) = 3(1-s)^2*s*P1 + 3(1-s)*s^2*P2 + s^3
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;

    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    let sample_curve_x = |s: f64| ((ax * s + bx) * s + cx) * s;
    let sample_curve_y = |s: f64| ((ay * s + by) * s + cy) * s;
    let sample_curve_derivative_x = |s: f64| (3.0 * ax * s + 2.0 * bx) * s + cx;

    // Newton-Raphson iteration.
    let mut s = clamped_t;
    for _ in 0..8 {
        let x = sample_curve_x(s) - clamped_t;
        if x.abs() < 1e-5 {
            return sample_curve_y(s);
        }
        let d = sample_curve_derivative_x(s);
        if d.abs() < 1e-5 {
            break;
        }
        s -= x / d;
    }

    // Binary subdivision fallback.
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    s = clamped_t;
    for _ in 0..12 {
        let x = sample_curve_x(s);
        if (x - clamped_t).abs() < 1e-4 {
            return sample_curve_y(s);
        }
        if clamped_t > x {
            lo = s;
        } else {
            hi = s;
        }
        s = (hi + lo) / 2.0;
    }

    sample_curve_y(s)
}

/// Inverse of the preview's CSS visual transform (translate in px, uniform
/// scale, rotation in degrees around the frame centre, y-down screen
/// coordinates): maps an output pixel centre to the source coordinate to
/// sample. This is the affine warp `saturnkfvid` applies per frame.
#[inline]
pub fn inverse_map_point(
    out_x: f64,
    out_y: f64,
    position_x: f64,
    position_y: f64,
    scale: f64,
    rotation_deg: f64,
    center_x: f64,
    center_y: f64,
) -> (f64, f64) {
    let radians = rotation_deg.to_radians();
    let (cos, sin) = (radians.cos(), radians.sin());
    let q_x = out_x - center_x - position_x;
    let q_y = out_y - center_y - position_y;
    (
        (q_x * cos + q_y * sin) / scale + center_x,
        (-q_x * sin + q_y * cos) / scale + center_y,
    )
}

/// The preview easing progress for one keyframe segment, mirroring the
/// `interpolateKeyframe` switch exactly (including the bezier handle
/// fallbacks of 0.33/0.0 and 0.67/1.0).
fn segment_progress(
    interpolation: InterpolationKind,
    ratio: f64,
    prev: &Keyframe,
    next: &Keyframe,
) -> f64 {
    match interpolation {
        InterpolationKind::Linear => ratio,
        InterpolationKind::EaseIn => solve_cubic_bezier(ratio, 0.42, 0.0, 1.0, 1.0),
        InterpolationKind::EaseOut => solve_cubic_bezier(ratio, 0.0, 0.0, 0.58, 1.0),
        InterpolationKind::EaseInOut => solve_cubic_bezier(ratio, 0.42, 0.0, 0.58, 1.0),
        InterpolationKind::Bezier => {
            let x1 = prev.handle_out_x.unwrap_or(0.33);
            let y1 = prev.handle_out_y.unwrap_or(0.0);
            let x2 = next.handle_in_x.unwrap_or(0.67);
            let y2 = next.handle_in_y.unwrap_or(1.0);
            solve_cubic_bezier(ratio, x1, y1, x2, y2)
        }
    }
}

/// Evaluates one clip's keyframe curve at `time` (ticks in the same time base
/// the curve was staged with). Mirrors `interpolateKeyframe`: edges hold, an
/// empty curve falls back, and a zero-length span snaps to the earlier value.
pub fn sample_keyframe_curve(keys: &[Keyframe], time: Tick, fallback: f64) -> f64 {
    if keys.is_empty() {
        return fallback;
    }
    debug_assert!(
        keys.windows(2).all(|pair| pair[0].time <= pair[1].time),
        "keyframe curves must be sorted by time before staging"
    );
    if time <= keys[0].time {
        return keys[0].value;
    }
    let last = keys.len() - 1;
    if time >= keys[last].time {
        return keys[last].value;
    }
    let next_index = keys.partition_point(|key| key.time < time).max(1);
    let prev = &keys[next_index - 1];
    let next = &keys[next_index];
    let span = next.time.0 - prev.time.0;
    if span <= 0 {
        return prev.value;
    }
    let ratio = ((time.0 - prev.time.0) as f64 / span as f64).clamp(0.0, 1.0);
    let progress = segment_progress(prev.interpolation, ratio, prev, next);
    prev.value + (next.value - prev.value) * progress
}

/// Per-clip visual keyframe payload staged for `saturnkfvid` instances.
///
/// All curve times are timeline-absolute ticks (clip start + key time) so the
/// element can sample them straight from the buffer PTS, exactly like the
/// audio automation element does.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VisualKeyframePayload {
    pub clip_key: u64,
    #[serde(default)]
    pub position_x: Vec<Keyframe>,
    #[serde(default)]
    pub position_y: Vec<Keyframe>,
    #[serde(default)]
    pub scale: Vec<Keyframe>,
    #[serde(default)]
    pub rotation: Vec<Keyframe>,
    #[serde(default)]
    pub opacity: Vec<Keyframe>,
}

impl VisualKeyframePayload {
    /// True when no curve would change any pixel: the element can then
    /// short-circuit to a plain alpha-opaque copy.
    pub fn is_identity(&self) -> bool {
        self.position_x.is_empty()
            && self.position_y.is_empty()
            && self.scale.is_empty()
            && self.rotation.is_empty()
            && self.opacity.is_empty()
    }

    /// Shorthand for sampling one of the payload's curves. Fallbacks match the
    /// preview: positions rest at 0 px, scale at 1.0, rotation at 0 degrees,
    /// opacity at 1.0.
    pub fn sample(&self, property: KeyframeProperty, time: Tick) -> f64 {
        let (keys, fallback): (&Vec<Keyframe>, f64) = match property {
            KeyframeProperty::PositionX => (&self.position_x, 0.0),
            KeyframeProperty::PositionY => (&self.position_y, 0.0),
            KeyframeProperty::Scale => (&self.scale, 1.0),
            KeyframeProperty::Rotation => (&self.rotation, 0.0),
            KeyframeProperty::Opacity => (&self.opacity, 1.0),
        };
        sample_keyframe_curve(keys, time, fallback)
    }
}

/// Builds the staged payload for one clip: filters keyframes by visual
/// property, shifts their clip-relative times to timeline-absolute ticks
/// (`timeline_start + key.time`), and sorts each curve. Returns `None` when
/// the clip has no visual keyframes at all so the render path can skip the
/// element entirely.
pub fn build_visual_payload(
    clip_key: u64,
    timeline_start: Tick,
    keyframes: &[Keyframe],
) -> Option<VisualKeyframePayload> {
    let mut payload = VisualKeyframePayload {
        clip_key,
        ..Default::default()
    };
    for key in keyframes {
        let shifted = Keyframe {
            property: key.property.clone(),
            time: Tick(timeline_start.0.saturating_add(key.time.0)),
            value: key.value,
            interpolation: key.interpolation,
            handle_in_x: key.handle_in_x,
            handle_in_y: key.handle_in_y,
            handle_out_x: key.handle_out_x,
            handle_out_y: key.handle_out_y,
        };
        match key.property {
            KeyframeProperty::PositionX => payload.position_x.push(shifted),
            KeyframeProperty::PositionY => payload.position_y.push(shifted),
            KeyframeProperty::Scale => payload.scale.push(shifted),
            KeyframeProperty::Rotation => payload.rotation.push(shifted),
            KeyframeProperty::Opacity => payload.opacity.push(shifted),
        }
    }
    if payload.is_identity() {
        return None;
    }
    payload.position_x.sort_by_key(|key| key.time);
    payload.position_y.sort_by_key(|key| key.time);
    payload.scale.sort_by_key(|key| key.time);
    payload.rotation.sort_by_key(|key| key.time);
    payload.opacity.sort_by_key(|key| key.time);
    Some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(
        property: KeyframeProperty,
        time_ms: i64,
        value: f64,
        interpolation: InterpolationKind,
    ) -> Keyframe {
        let mut keyframe = Keyframe::new(property, Tick::from_millis(time_ms), value);
        keyframe.interpolation = interpolation;
        keyframe
    }

    fn opacity_curve(kind: InterpolationKind) -> Vec<Keyframe> {
        vec![
            key(KeyframeProperty::Opacity, 0, 0.0, kind),
            key(KeyframeProperty::Opacity, 2_000, 1.0, kind),
        ]
    }

    #[test]
    fn bezier_boundaries_and_linear_curve_match_ts_evaluator() {
        // Mirrors playbackEngine.test.ts "solveCubicBezier calculates
        // standard curve values".
        assert_eq!(solve_cubic_bezier(0.0, 0.0, 0.0, 1.0, 1.0), 0.0);
        assert_eq!(solve_cubic_bezier(1.0, 0.0, 0.0, 1.0, 1.0), 1.0);
        let mid_linear = solve_cubic_bezier(0.5, 0.0, 0.0, 1.0, 1.0);
        assert!(
            (mid_linear - 0.5).abs() < 0.01,
            "expected ~0.5, got {mid_linear}"
        );
        let mid_ease_in = solve_cubic_bezier(0.5, 0.42, 0.0, 1.0, 1.0);
        assert!(
            mid_ease_in < 0.5,
            "ease-in at 0.5 should be < 0.5, got {mid_ease_in}"
        );
        // Out-of-range inputs clamp like the TS version.
        assert_eq!(solve_cubic_bezier(-0.5, 0.0, 0.0, 1.0, 1.0), 0.0);
        assert_eq!(solve_cubic_bezier(1.5, 0.0, 0.0, 1.0, 1.0), 1.0);
    }

    #[test]
    fn curve_edges_hold_and_empty_curves_fall_back() {
        assert!(
            (sample_keyframe_curve(&opacity_curve(InterpolationKind::Linear), Tick::from_millis(-1), 0.7) - 0.0)
                .abs()
                < 1e-9
        );
        assert!(
            (sample_keyframe_curve(&opacity_curve(InterpolationKind::Linear), Tick::from_millis(9_999), 0.7) - 1.0)
                .abs()
                < 1e-9
        );
        assert!(
            (sample_keyframe_curve(&[], Tick::from_millis(5), 0.7) - 0.7).abs() < 1e-9,
            "empty curve falls back"
        );
        // Midpoint of the 0..1 linear ramp is 0.5 (same tolerance as TS).
        let mid =
            sample_keyframe_curve(&opacity_curve(InterpolationKind::Linear), Tick::from_millis(1_000), 1.0);
        assert!((mid - 0.5).abs() < 0.05, "expected ~0.5, got {mid}");
    }

    #[test]
    fn easing_kinds_order_like_the_preview() {
        let t = Tick::from_millis(1_000);
        let linear = sample_keyframe_curve(&opacity_curve(InterpolationKind::Linear), t, 1.0);
        let ease_in = sample_keyframe_curve(&opacity_curve(InterpolationKind::EaseIn), t, 1.0);
        let ease_out = sample_keyframe_curve(&opacity_curve(InterpolationKind::EaseOut), t, 1.0);
        assert!(
            ease_in < linear && linear < ease_out,
            "ease ordering at t=0.5"
        );
    }

    #[test]
    fn bezier_handles_deviate_from_linear() {
        // A strong ease-in handle pair should lag linear mid-segment.
        let mut curve = opacity_curve(InterpolationKind::Bezier);
        curve[0].handle_out_x = Some(0.8);
        curve[0].handle_out_y = Some(0.05);
        curve[1].handle_in_x = Some(0.9);
        curve[1].handle_in_y = Some(1.0);
        let mid = sample_keyframe_curve(&curve, Tick::from_millis(1_000), 1.0);
        assert!(mid < 0.35, "bezier mid should lag hard, got {mid}");
    }

    #[test]
    fn inverse_map_point_inverts_the_css_transform() {
        let (cx, cy) = (100.0_f64, 80.0_f64);
        // Identity: the source point is the output point.
        let (u, v) = inverse_map_point(103.0, 84.0, 0.0, 0.0, 1.0, 0.0, cx, cy);
        assert!((u - 103.0).abs() < 1e-9 && (v - 84.0).abs() < 1e-9);

        // Pure translation: the source sits anti-translated.
        let (u, v) = inverse_map_point(103.0, 84.0, 10.0, 20.0, 1.0, 0.0, cx, cy);
        assert!((u - 93.0).abs() < 1e-9 && (v - 64.0).abs() < 1e-9);

        // 90-degree CSS rotation (clockwise on y-down screens): a point 10 px
        // right of centre is drawn from 10 px right of centre rotated so the
        // output lands 10 px below centre. Its inverse must sample from the
        // source 10 px right of centre.
        let (u, v) = inverse_map_point(cx, cy + 10.0, 0.0, 0.0, 1.0, 90.0, cx, cy);
        assert!((u - (cx + 10.0)).abs() < 1e-9, "u={u}");
        assert!((v - cy).abs() < 1e-9, "v={v}");

        // Scale 2 doubles the source distance.
        let (u, v) = inverse_map_point(cx + 10.0, cy, 0.0, 0.0, 2.0, 0.0, cx, cy);
        assert!((u - (cx + 5.0)).abs() < 1e-9 && (v - cy).abs() < 1e-9);

        // Round trip: forward mapping the inverse result returns the output.
        let (u, v) = inverse_map_point(cx + 7.0, cy - 3.0, 4.0, -2.0, 1.5, 30.0, cx, cy);
        let radians = 30.0_f64.to_radians();
        let (cos, sin) = (radians.cos(), radians.sin());
        let d_x = (u - cx) * 1.5;
        let d_y = (v - cy) * 1.5;
        let out_x = cx + 4.0 + d_x * cos - d_y * sin;
        let out_y = cy - 2.0 + d_x * sin + d_y * cos;
        assert!((out_x - (cx + 7.0)).abs() < 1e-9, "out_x={out_x}");
        assert!((out_y - (cy - 3.0)).abs() < 1e-9, "out_y={out_y}");
    }

    #[test]
    fn zero_span_snaps_to_earlier_value() {
        let curve = vec![
            key(KeyframeProperty::Opacity, 1_000, 0.25, InterpolationKind::Linear),
            key(KeyframeProperty::Opacity, 1_000, 0.75, InterpolationKind::Linear),
            key(KeyframeProperty::Opacity, 3_000, 1.0, InterpolationKind::Linear),
        ];
        // At exactly the duplicated time the earlier key holds.
        let at = sample_keyframe_curve(&curve, Tick::from_millis(1_000), 0.0);
        assert!((at - 0.25).abs() < 1e-9);
    }

    #[test]
    fn payload_builder_shifts_times_and_drops_empty_clips() {
        let keyframes = vec![
            key(KeyframeProperty::Opacity, 0, 0.0, InterpolationKind::Linear),
            key(KeyframeProperty::Scale, 500, 1.5, InterpolationKind::Linear),
        ];
        let payload = build_visual_payload(7, Tick::from_millis(10_000), &keyframes)
            .expect("visual keyframes build a payload");
        assert_eq!(payload.clip_key, 7);
        assert_eq!(payload.opacity[0].time, Tick::from_millis(10_000));
        assert_eq!(payload.scale[0].time, Tick::from_millis(10_500));
        // Sampling uses absolute times; before the first key its value holds
        // and empty curves fall back to the preview defaults.
        assert!(
            (payload.sample(KeyframeProperty::Scale, Tick::from_millis(9_500)) - 1.5).abs() < 1e-9
        );
        assert!(
            (payload.sample(KeyframeProperty::PositionX, Tick::from_millis(10_500)) - 0.0).abs()
                < 1e-9
        );

        assert!(build_visual_payload(8, Tick::default(), &[]).is_none());
    }
}
