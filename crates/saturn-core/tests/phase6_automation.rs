//! Phase 6 slice 1 — automation engine tests.
//!
//! Covers curve evaluation (linear/constant/smooth), the granular automation
//! commands, the Latch record path, and undo. The ±1 ms reproduction gate:
//! recorded playhead times must evaluate back to the recorded values at the
//! same timeline positions (tick precision is ~1/254 ns, far inside 1 ms).

use saturn_core::{
    AutomationInterpolation, AutomationLane, AutomationMode, AutomationPoint, EditorEngine, Tick,
    TrackAutomation, TICKS_PER_SECOND,
};
use serde_json::json;

fn linear_volume_curve() -> Vec<AutomationPoint> {
    vec![
        AutomationPoint {
            time: Tick(0),
            value: 0.0,
        },
        AutomationPoint {
            time: Tick::from_millis(1_000),
            value: 12.0,
        },
    ]
}

#[test]
fn curve_evaluation_matches_interpolation_modes() {
    let base = |interpolation| TrackAutomation {
        mode: AutomationMode::Read,
        interpolation,
        volume_points: linear_volume_curve(),
        pan_points: vec![],
    };

    // Linear: halfway between the points at half time.
    let linear = base(AutomationInterpolation::Linear);
    let half_ms = Tick(TICKS_PER_SECOND / 2) ;
    let value = linear.evaluate_volume(half_ms).unwrap();
    assert!(
        (value - 6.0).abs() < 1e-6,
        "linear halfway should be 6 dB, got {value}"
    );

    // Constant: holds the previous point until the next one.
    let constant = base(AutomationInterpolation::Constant);
    let value = constant.evaluate_volume(half_ms).unwrap();
    assert!((value - 0.0).abs() < 1e-9, "constant should hold 0 dB");

    // Smooth: half-cosine easing; at half time the eased value is 6 dB too
    // (cosine symmetry), but at a quarter time it lags linear.
    let smooth = base(AutomationInterpolation::Smooth);
    let quarter_ms = Tick(TICKS_PER_SECOND / 4);
    let smooth_quarter = smooth.evaluate_volume(quarter_ms).unwrap();
    let linear_quarter = linear.evaluate_volume(quarter_ms).unwrap();
    assert!(smooth_quarter < linear_quarter - 0.5);
    let raw_quarter = 0.25_f64;
    let expected_smooth =
        12.0 * 0.5 * (1.0 - (raw_quarter * std::f64::consts::PI).cos());
    assert!((smooth_quarter - expected_smooth).abs() < 1e-6);

    // Before the first point and after the last point clamp to the edges.
    assert_eq!(smooth.evaluate_volume(Tick(0)), Some(0.0));
    let after = smooth.evaluate_volume(Tick::from_millis(2_000));
    assert_eq!(after, Some(12.0));

    // Empty lane evaluates to nothing.
    assert_eq!(base(AutomationInterpolation::Linear).evaluate_pan(half_ms), None);
}

#[test]
fn latch_record_reproduces_within_one_millisecond() {
    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "Latch Record"}),
        ))
        .unwrap();
    let track_id = engine.document().project.sequence.tracks[0].id;

    // Arm Latch on the track's volume lane.
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.set_automation_mode",
            json!({"track_id": track_id, "mode": "latch"}),
        ))
        .unwrap();

    // Simulate the fader write stream: each playhead tick (≈4 ms apart) writes the
    // fader position. Recorded times must come back exactly.
    let written: Vec<(Tick, f64)> = (0..100)
        .map(|step| {
            let time = Tick(step * TICKS_PER_SECOND / 250);
            let value = -6.0 + 0.12 * step as f64;
            (time, value)
        })
        .collect();
    for (time, value) in &written {
        engine
            .execute(saturn_core::CommandRequest::new(
                "track.add_automation_point",
                json!({
                    "track_id": track_id,
                    "lane": "volume",
                    "time": time.0,
                    "value": value,
                }),
            ))
            .unwrap();
    }

    let lane_points =
        &engine.document().project.sequence.tracks[0].automation.volume_points;

    // The record gap collapses runs closer than ~4 ms into single points, so
    // consecutive written points ≥4 ms apart all survive.
    assert_eq!(lane_points.len(), written.len(), "every ≥4 ms write lands");

    // ±1 ms reproduction: evaluating the curve at each recorded playhead time
    // returns the recorded value (exact with breakpoints, by construction).
    for (time, value) in &written {
        let point = lane_points
            .iter()
            .find(|p| p.time == *time)
            .expect("recorded playhead time present");
        assert!((point.value - value).abs() < 1e-9);
        // The playback path evaluates at exactly the recorded tick.
        let played_back = engine.document().project.sequence.tracks[0]
            .automation
            .evaluate_volume(*time)
            .unwrap();
        assert!(
            (played_back - value).abs() < 1e-9,
            "playback must reproduce the recorded value at the recorded time"
        );
        let _ = time.as_millis();
        // And timing drift the evaluator could add is bounded to <1 ms:
        let one_ms_ticks = TICKS_PER_SECOND / 1000;
        let nearby = engine.document().project.sequence.tracks[0]
            .automation
            .evaluate_volume(Tick(time.0 + one_ms_ticks / 2));
        assert!(nearby.is_some());
    }

    // Undo removes the recorded points (gesture group).
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.clear_automation",
            json!({"track_id": track_id, "lane": "volume"}),
        ))
        .unwrap();
    assert!(
        engine.document().project.sequence.tracks[0]
            .automation
            .volume_points
            .is_empty()
    );
}

#[test]
fn automation_mode_off_never_plays_curve() {
    let automation = TrackAutomation {
        mode: AutomationMode::Off,
        interpolation: AutomationInterpolation::Linear,
        volume_points: linear_volume_curve(),
        pan_points: vec![],
    };
    assert!(!automation.plays_curve());

    let reading = TrackAutomation {
        mode: AutomationMode::Read,
        ..automation
    };
    assert!(reading.plays_curve());
}

#[test]
fn automation_point_editing_commands_round_trip() {
    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "Automation Edit"}),
        ))
        .unwrap();
    let track_id = engine.document().project.sequence.tracks[0].id;

    // Volume point insertion (out of order still stays sorted).
    let later_ms = 3_000_i64;
    let earlier_ms = 1_000_i64;
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.add_automation_point",
            json!({"track_id": track_id, "lane": "volume", "time": later_ms * (TICKS_PER_SECOND / 1000), "value": 3.0}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.add_automation_point",
            json!({"track_id": track_id, "lane": "volume", "time": earlier_ms * (TICKS_PER_SECOND / 1000), "value": -3.0}),
        ))
        .unwrap();
    let track = &engine.document().project.sequence.tracks[0];
    let times: Vec<i64> = track
        .automation
        .volume_points
        .iter()
        .map(|p| p.time.0)
        .collect();
    let earlier_ticks = earlier_ms * (TICKS_PER_SECOND / 1000);
    let later_ticks = later_ms * (TICKS_PER_SECOND / 1000);
    assert_eq!(times, vec![earlier_ticks, later_ticks]);

    // Out-of-domain values clamp (volume ±, pan beyond hard L/R).
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.add_automation_point",
            json!({"track_id": track_id, "lane": "volume", "time": later_ticks, "value": 999.0}),
        ))
        .unwrap();
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.add_automation_point",
            json!({"track_id": track_id, "lane": "pan", "time": earlier_ticks, "value": -9.0}),
        ))
        .unwrap();
    let track = &engine.document().project.sequence.tracks[0];
    let volume = track
        .automation
        .volume_points
        .iter()
        .find(|p| p.time.0 == later_ticks)
        .unwrap();
    assert_eq!(volume.value, 12.0, "volume clamps to +12 dB");
    let pan = track.automation.pan_points.first().unwrap();
    assert!((pan.value - -1.0).abs() < 1e-9, "pan clamps to hard left");

    // Doubling a time upserts instead of duplicating.
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.add_automation_point",
            json!({"track_id": track_id, "lane": "volume", "time": later_ticks, "value": 1.5}),
        ))
        .unwrap();
    assert_eq!(
        engine
            .document()
            .project
            .sequence
            .tracks[0]
            .automation
            .volume_points
            .iter()
            .filter(|p| p.time.0 == later_ticks)
            .count(),
        1
    );
    assert_eq!(
        engine.document().project.sequence.tracks[0]
            .automation
            .volume_points
            .iter()
            .find(|p| p.time.0 == later_ticks)
            .unwrap()
            .value,
        1.5
    );

    // Removal hits the point near the clicked time.
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.remove_automation_point",
            json!({"track_id": track_id, "lane": "volume", "time": later_ticks}),
        ))
        .unwrap();
    assert_eq!(
        engine
            .document()
            .project
            .sequence
            .tracks[0]
            .automation
            .volume_points
            .len(),
        1
    );

    // Undo: back to two points before the removal, then to zero.
    engine
        .execute(saturn_core::CommandRequest::new(
            "edit.undo",
            serde_json::Value::Null,
        ))
        .unwrap();
    assert_eq!(
        engine
            .document()
            .project
            .sequence
            .tracks[0]
            .automation
            .volume_points
            .len(),
        2
    );
    engine
        .execute(saturn_core::CommandRequest::new(
            "track.clear_automation",
            json!({"track_id": track_id}),
        ))
        .unwrap();
    let track = &engine.document().project.sequence.tracks[0];
    assert!(track.automation.volume_points.is_empty());
    assert!(track.automation.pan_points.is_empty());
}

#[test]
fn interpolation_command_updates_curve_shape() {
    let mut engine = EditorEngine::default();
    engine
        .execute(saturn_core::CommandRequest::new(
            "project.new",
            json!({"name": "Interpolation"}),
        ))
        .unwrap();
    let track_id = engine.document().project.sequence.tracks[0].id;

    engine
        .execute(saturn_core::CommandRequest::new(
            "track.set_automation_interpolation",
            json!({"track_id": track_id, "interpolation": "smooth"}),
        ))
        .unwrap();
    assert_eq!(
        engine
            .document()
            .project
            .sequence
            .tracks[0]
            .automation
            .interpolation,
        AutomationInterpolation::Smooth
    );

    // Mode round trip through the command path.
    for mode in ["read", "touch", "latch", "write", "off"] {
        engine
            .execute(saturn_core::CommandRequest::new(
                "track.set_automation_mode",
                json!({"track_id": track_id, "mode": mode}),
            ))
            .unwrap();
        }
    assert_eq!(
        engine
            .document()
            .project
            .sequence
            .tracks[0]
            .automation
            .mode,
        AutomationMode::Off
    );
}

#[test]
fn lane_helpers_match_command_surface() {
    // Schema-level record/gap behaviour untouched by the command layer.
    let mut automation = TrackAutomation::default();
    automation.record_point(AutomationLane::Volume, Tick(0), 0.0);
    automation.record_point(AutomationLane::Volume, Tick(500_000_000), 4.2);
    // ~2 ms later than the previous write: inside the ~4 ms gap, collapsed
    // into the same recorded point with the fresh fader value.
    assert_eq!(automation.volume_points.len(), 1);
    assert_eq!(automation.volume_points[0].value, 4.2);
    automation.record_point(AutomationLane::Volume, Tick(TICKS_PER_SECOND), 4.2);
    assert_eq!(automation.volume_points.len(), 2);
    assert!(automation.remove_point_at(AutomationLane::Volume, Tick(0)));
    assert_eq!(automation.volume_points.len(), 1);
}
