//! Command catalogue invariants and gesture-scoped undo.

use saturn_core::{
    CommandRequest, DispatchError, EditorEngine, command_catalogue, find_command,
    search_commands, validate_catalogue,
};
use serde_json::{Value, json};

fn track_count(engine: &mut EditorEngine) -> usize {
    let document = engine
        .execute(CommandRequest::new("project.inspect", json!({})))
        .unwrap();
    document["project"]["sequence"]["tracks"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
}

fn add_track(engine: &mut EditorEngine) -> Value {
    engine
        .execute(CommandRequest::new(
            "timeline.add_track",
            json!({"kind": "video"}),
        ))
        .unwrap()
}

#[test]
fn catalogue_is_valid_and_menu_commands_document_params() {
    let problems = validate_catalogue();
    assert!(problems.is_empty(), "catalogue problems: {problems:?}");

    for command in command_catalogue() {
        if !command.menu.is_empty() {
            assert!(
                !command.params.is_empty(),
                "{} is in a menu but documents no params",
                command.id
            );
        }
        assert!(
            command.menu.is_empty() || command.menu.len() <= 2,
            "{} has a menu deeper than two levels",
            command.id
        );
    }
}

#[test]
fn catalogue_search_finds_by_id_label_and_menu_path() {
    assert_eq!(
        find_command("timeline.ripple_delete").map(|command| command.label),
        Some("Ripple Delete")
    );

    let by_id = search_commands("ripple_delete");
    assert!(by_id.iter().any(|command| command.id == "timeline.ripple_delete"));

    let by_label = search_commands("ripple delete");
    assert!(by_label.iter().any(|command| command.id == "timeline.ripple_delete"));

    let by_menu = search_commands("captions");
    assert!(by_menu.iter().any(|command| command.id == "subtitle.add"));

    assert!(find_command("does.not_exist").is_none());
}

#[test]
fn ui_only_commands_never_reach_the_engine() {
    assert!(find_command("view.zoom_in").is_some_and(|command| command.ui_only));

    let mut engine = EditorEngine::default();
    let error = engine
        .execute(CommandRequest::new("view.zoom_in", json!({})))
        .unwrap_err();
    assert!(matches!(error, DispatchError::UiOnly(id) if id == "view.zoom_in"));
}

#[test]
fn gesture_coalesces_mutations_into_one_undo_entry() {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("project.new", json!({"name": "Gesture"})))
        .unwrap();
    let before = track_count(&mut engine);

    engine.begin_gesture();
    for _ in 0..3 {
        add_track(&mut engine);
    }
    engine.end_gesture();
    assert_eq!(track_count(&mut engine), before + 3);

    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(
        track_count(&mut engine),
        before,
        "one undo must discard the whole gesture"
    );

    // Unbalanced closes are ignored, and gestures nest.
    engine.begin_gesture();
    engine.begin_gesture();
    add_track(&mut engine);
    engine.end_gesture();
    add_track(&mut engine);
    engine.end_gesture();
    engine.end_gesture();
    assert_eq!(track_count(&mut engine), before + 2);

    engine
        .execute(CommandRequest::new("edit.undo", json!({})))
        .unwrap();
    assert_eq!(track_count(&mut engine), before);
}

#[test]
fn gesture_state_is_reported_by_its_commands() {
    let mut engine = EditorEngine::default();
    let started = engine
        .execute(CommandRequest::new("edit.begin_gesture", json!({})))
        .unwrap();
    assert_eq!(started["gesture"], json!(true));

    let ended = engine
        .execute(CommandRequest::new("edit.end_gesture", json!({})))
        .unwrap();
    assert_eq!(ended["gesture"], json!(false));
}
