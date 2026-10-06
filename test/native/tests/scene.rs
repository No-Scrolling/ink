use ink_test::{SceneApp, command};
use serde_json::json;

fn list() -> SceneApp {
    let mut app = SceneApp::new("lists");
    app.pump_until(|app| app.labels().iter().any(|label| label == "A: Item 0"));
    assert_eq!(
        app.native_list_creates, 1,
        "fixture must exercise the compiler's native list transform"
    );
    app
}

#[test]
fn compiled_list_scrolls_thousands_of_rows_with_bounded_scene_and_no_js_window_events() {
    let mut app = list();
    for _ in 0..50 {
        assert!(app.engine.scroll_by(1800.0));
        assert!(
            app.tree
                .viewport_events(&mut app.engine)
                .unwrap()
                .is_empty()
        );
        assert!(app.engine.list_viewports_ready());
        assert!(
            app.labels().len() < 100,
            "offscreen rows leaked into the scene"
        );
        assert!(
            app.tree.memory_diagnostics()["host_nodes"]
                .as_u64()
                .unwrap()
                < 250
        );
    }
    let (label, x, y) = app.visible_label("A: Item ");
    let index: usize = label.strip_prefix("A: Item ").unwrap().parse().unwrap();
    assert!(index > 500);
    assert_eq!(
        app.tap(x, y),
        json!({"type":"selected", "key":index.to_string(), "index":index, "prefix":"A"})
    );
}

#[test]
fn changed_captured_state_updates_native_rows_and_their_event_closures() {
    let mut app = list();
    app.runtime
        .send(json!({"type":"test", "command":"prefix", "value":"B"}).to_string())
        .unwrap();
    app.pump_until(|app| app.labels().iter().any(|label| label == "B: Item 0"));
    let (_, x, y) = app.visible_label("B: Item 0");
    assert_eq!(
        app.tap(x, y),
        json!({"type":"selected", "key":"0", "index":0, "prefix":"B"})
    );
    assert_eq!(app.native_list_creates, 1);
}

#[test]
fn prepending_preserves_the_visible_anchor_and_routes_its_new_index() {
    let mut app = list();
    app.engine.scroll_by(25000.0);
    app.refresh();
    let (label, _, before_y) = app.visible_label("A: Item ");
    let index: usize = label.strip_prefix("A: Item ").unwrap().parse().unwrap();
    let previous_commits = app.engine.scene().revision;
    command(&app.runtime, "prepend");
    app.pump_until(|app| app.engine.scene().revision > previous_commits);
    let (after, x, after_y) = app.visible_label(&label);
    assert_eq!(after, label);
    assert!(
        (before_y - after_y).abs() <= 1.0,
        "visible anchor moved from {before_y} to {after_y}"
    );
    assert_eq!(
        app.tap(x, after_y),
        json!({"type":"selected", "key":index.to_string(), "index":index+1, "prefix":"A"})
    );
}

#[test]
fn reversed_rows_dispatch_the_current_item_and_index() {
    let mut app = list();
    let revision = app.engine.scene().revision;
    command(&app.runtime, "reverse");
    app.pump_until(|app| app.engine.scene().revision > revision);
    app.engine.scroll_by(-f32::MAX);
    app.refresh();
    assert!(app.labels().iter().any(|label| label == "A: Item 1999"));
    let (_, x, y) = app.visible_label("A: Item 1999");
    assert_eq!(
        app.tap(x, y),
        json!({"type":"selected", "key":"1999", "index":0, "prefix":"A"})
    );
}

#[test]
fn emptying_a_native_list_releases_rows_actions_and_scroll_extent() {
    let mut app = list();
    command(&app.runtime, "empty");
    app.pump_until(|app| {
        !app.labels()
            .iter()
            .any(|label| label.starts_with("A: Item "))
    });
    assert_eq!(app.engine.scene().scroll_max, 0.0);
    assert!(app.engine.list_viewports_ready());
    assert!(
        app.tree.memory_diagnostics()["host_nodes"]
            .as_u64()
            .unwrap()
            < 10
    );
    assert!(!app.engine.tap(100.0, 200.0));
    assert!(app.engine.take_native_request().is_none());
}

#[test]
fn quickjs_proxy_items_fall_back_to_react_and_remain_interactive() {
    let mut app = list();
    command(&app.runtime, "proxy");
    app.pump_until(|app| app.labels().iter().any(|label| label == "A: Proxy item"));
    let (_, x, y) = app.visible_label("A: Proxy item");
    assert_eq!(
        app.tap(x, y),
        json!({"type":"selected", "key":"proxy", "index":0, "prefix":"A"})
    );
    assert_eq!(
        app.native_list_creates, 1,
        "Proxy input should replace the existing NativeList with React List"
    );
}

#[test]
fn updating_a_held_rows_closure_preserves_its_press_identity() {
    let mut app = list();
    let (_, x, y) = app.visible_label("A: Item 0");
    app.engine.pointer_down(1, x, y);
    app.runtime
        .send(json!({"type":"test", "command":"prefix", "value":"B"}).to_string())
        .unwrap();
    app.pump_until(|app| app.labels().contains(&"B: Item 0".into()));
    assert!(app.engine.pointer_up(1, x, y).activated);
    let request = app.engine.take_native_request().unwrap();
    app.runtime
        .send(
            app.tree
                .route_native_message(request.payload().to_owned())
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        ink_test::message(&app.events, |value| value["type"] == "selected"),
        json!({"type":"selected", "key":"0", "index":0, "prefix":"B"})
    );
}

#[test]
fn removing_a_held_row_does_not_activate_its_replacement() {
    let mut app = list();
    let (_, x, y) = app.visible_label("A: Item 0");
    app.engine.pointer_down(1, x, y);
    app.runtime
        .send(json!({"type":"test", "command":"remove", "key":"0"}).to_string())
        .unwrap();
    app.pump_until(|app| !app.labels().contains(&"A: Item 0".into()));
    assert!(!app.engine.pointer_up(1, x, y).activated);
    assert!(app.engine.take_native_request().is_none());
    let (_, x, y) = app.visible_label("A: Item 1");
    assert_eq!(
        app.tap(x, y),
        json!({"type":"selected", "key":"1", "index":0, "prefix":"A"})
    );
}

fn edit(app: &mut SceneApp, value: &str, selection: usize) -> bool {
    let context: serde_json::Value =
        serde_json::from_str(&app.engine.text_input_context()).unwrap();
    app.engine.edit_text(ink_core::TextEdit::Update(
        json!({"id":context["id"], "text":context["text"], "value":value, "selection":selection})
            .to_string(),
    ))
}

#[test]
fn delayed_react_input_commits_cannot_overwrite_newer_native_keystrokes() {
    let mut app = SceneApp::new("input");
    app.pump_until(|app| !app.engine.text_input_context().is_empty());
    assert!(edit(&mut app, "first", 5));
    let first = app.tree.input_events(&app.engine);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0]["args"], json!(["first", 1]));
    app.runtime.send(first[0].to_string()).unwrap();
    let deadline = std::time::Instant::now() + ink_test::WAIT;
    let delayed_commit = loop {
        match ink_test::next_event(&app.events, deadline) {
            ink_runtime::Event::Commit(commit) => break commit,
            ink_runtime::Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    };
    assert!(edit(&mut app, "first🙂", 7));
    let second = app.tree.input_events(&app.engine);
    assert_eq!(second[0]["args"], json!(["first🙂", 2]));
    app.tree.apply(delayed_commit, &mut app.engine).unwrap();
    let context: serde_json::Value =
        serde_json::from_str(&app.engine.text_input_context()).unwrap();
    assert_eq!(context["text"], "first🙂");
    assert_eq!(context["cursor"], 7);
    app.runtime.send(second[0].to_string()).unwrap();
    app.pump_until(|app| {
        app.labels()
            .iter()
            .any(|label| label.contains("Echo: first🙂"))
    });
    let submitted = app.tree.submit_event(&app.engine).unwrap();
    assert_eq!(submitted["args"], json!(["first🙂"]));
    app.runtime.send(submitted.to_string()).unwrap();
    assert_eq!(
        ink_test::message(&app.events, |value| value["type"] == "submitted")["value"],
        "first🙂"
    );
}

#[test]
fn native_editor_rejects_a_cursor_inside_a_surrogate_pair_without_mutating_text() {
    let mut app = SceneApp::new("input");
    app.pump_until(|app| !app.engine.text_input_context().is_empty());
    assert!(!edit(&mut app, "🙂", 1));
    let context: serde_json::Value =
        serde_json::from_str(&app.engine.text_input_context()).unwrap();
    assert_eq!(context["text"], "");
    assert!(edit(&mut app, "🙂", 2));
    let context: serde_json::Value =
        serde_json::from_str(&app.engine.text_input_context()).unwrap();
    assert_eq!(context["text"], "🙂");
    assert_eq!(context["cursor"], 2);
}
