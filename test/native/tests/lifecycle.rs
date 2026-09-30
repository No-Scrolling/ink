use ink_runtime::Event;
use ink_test::{SceneApp, WAIT, command, next_event};
use serde_json::{Value, json};
use std::time::Instant;

fn lifecycle() -> SceneApp {
    let mut app = SceneApp::new("lifecycle");
    app.pump_until(|app| app.labels().contains(&"Initial value".into()));
    app
}

fn change_mode(app: &mut SceneApp, mode: &str, expected: &str) {
    command(&app.runtime, mode);
    app.pump_until(|app| app.labels().join(" ").contains(expected));
}

fn report(app: &mut SceneApp) -> Value {
    command(&app.runtime, "report");
    let deadline = Instant::now() + WAIT;
    loop {
        match next_event(&app.events, deadline) {
            Event::Message(source) => {
                let value: Value = serde_json::from_str(&source).unwrap();
                if value["type"] == "observed" {
                    return value;
                }
            }
            Event::Commit(commit) => {
                app.values_commits += commit
                    .0
                    .iter()
                    .filter(|operation| matches!(operation, ink_protocol::Operation::Values { .. }))
                    .count();
                app.tree.apply(commit, &mut app.engine).unwrap();
                app.refresh();
            }
            Event::Error(error) => panic!("runtime error: {error}"),
            Event::Stopped => panic!("runtime stopped before lifecycle report"),
            _ => {}
        }
    }
}

fn source(app: &SceneApp) {
    app.runtime
        .send(json!({"type":"source"}).to_string())
        .unwrap();
}

#[test]
fn activity_detaches_subscriptions_and_reconnects_without_duplicate_delivery() {
    let mut app = lifecycle();
    source(&app);
    app.pump_until(|app| app.labels().contains(&"Delivery 1".into()));
    change_mode(&mut app, "hide", "Hidden");
    source(&app);
    assert_eq!(
        report(&mut app),
        json!({"type":"observed", "starts":1, "stops":1, "deliveries":1, "actions":0})
    );
    change_mode(&mut app, "show", "Delivery 1");
    source(&app);
    app.pump_until(|app| app.labels().contains(&"Delivery 2".into()));
    assert_eq!(
        report(&mut app),
        json!({"type":"observed", "starts":2, "stops":1, "deliveries":2, "actions":0})
    );
    assert_eq!(
        app.native_view_creates, 1,
        "revealing Activity must retain its existing native view"
    );
}

#[test]
fn hidden_value_and_collection_edits_are_replayed_when_activity_reappears() {
    let mut app = lifecycle();
    change_mode(&mut app, "hide", "Hidden");
    assert_eq!(report(&mut app)["stops"], 1);
    let hidden_labels = app.labels();
    let hidden_values_commits = app.values_commits;
    command(&app.runtime, "mutate-hidden");
    assert_eq!(report(&mut app)["stops"], 1);
    assert_eq!(
        app.values_commits, hidden_values_commits,
        "hidden values must wait for reattachment"
    );
    assert_eq!(
        app.labels(),
        hidden_labels,
        "hidden edits must not change the visible page"
    );
    assert!(!app.labels().contains(&"Inserted while hidden".into()));
    change_mode(&mut app, "show", "Value changed while hidden");
    app.pump_until(|app| app.labels().contains(&"Inserted while hidden".into()));
    assert_eq!(
        app.labels().join(" "),
        "Native lifecycle Value changed while hidden Invoke native action Row changed while hidden Inserted while hidden"
    );
    assert!(!app.labels().contains(&"Initial row".into()));
    assert_eq!(app.native_view_creates, 1);
}

#[test]
fn unmount_disposes_native_subscription_and_ignores_a_previously_captured_action() {
    let mut app = lifecycle();
    let (_, x, y) = app.visible_label("Invoke native action");
    assert!(app.engine.tap(x, y));
    let request = app.engine.take_native_request().unwrap();
    let delayed_action = app
        .tree
        .route_native_message(request.payload().to_owned())
        .unwrap();
    change_mode(&mut app, "remove", "Removed");
    app.runtime.send(delayed_action).unwrap();
    source(&app);
    assert_eq!(
        report(&mut app),
        json!({"type":"observed", "starts":1, "stops":1, "deliveries":0, "actions":0})
    );
    assert!(!app.labels().contains(&"Invoke native action".into()));
    change_mode(&mut app, "show", "Initial value");
    let (_, x, y) = app.visible_label("Invoke native action");
    assert!(app.engine.tap(x, y));
    let request = app.engine.take_native_request().unwrap();
    app.runtime
        .send(
            app.tree
                .route_native_message(request.payload().to_owned())
                .unwrap(),
        )
        .unwrap();
    app.pump_until(|app| app.labels().contains(&"Action 1".into()));
    assert_eq!(
        report(&mut app),
        json!({"type":"observed", "starts":2, "stops":1, "deliveries":0, "actions":1})
    );
    assert_eq!(
        app.native_view_creates, 2,
        "a removed page must mount a new native view"
    );
}
