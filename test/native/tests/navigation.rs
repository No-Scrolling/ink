use ink_test::SceneApp;
use serde_json::json;

fn navigation() -> SceneApp {
    let mut app = SceneApp::new("navigation");
    app.pump_until(|app| app.labels().contains(&"Home count 0".into()));
    app
}

fn dispatch_request(app: &mut SceneApp) {
    let request = app
        .engine
        .take_native_request()
        .expect("interaction did not dispatch");
    assert_eq!(request.module(), "ink");
    let event = app
        .tree
        .route_native_message(request.payload().to_owned())
        .unwrap();
    app.runtime.send(event).unwrap();
}

fn press(app: &mut SceneApp, label: &str, expected: &str) {
    let (_, x, y) = app.visible_label(label);
    assert!(app.engine.tap(x, y));
    dispatch_request(app);
    app.pump_until(|app| app.labels().contains(&expected.into()));
}

fn back(app: &mut SceneApp, expected: &str) {
    assert!(app.engine.back(), "a pushed page must accept native back");
    dispatch_request(app);
    app.pump_until(|app| app.labels().contains(&expected.into()));
}

#[test]
fn push_decodes_params_once_and_native_back_restores_page_state() {
    let mut app = navigation();
    press(&mut app, "Increase home", "Home count 1");
    press(&mut app, "Open encoded detail", "Detail id a/b");
    assert!(
        !app.labels()
            .iter()
            .any(|label| label.starts_with("Home count"))
    );
    back(&mut app, "Home count 1");
    assert!(
        !app.labels()
            .iter()
            .any(|label| label.starts_with("Detail id"))
    );
    assert!(
        !app.engine.back(),
        "back at the root must not replay the popped page"
    );
    assert!(app.engine.take_native_request().is_none());
}

#[test]
fn replace_removes_the_previous_detail_from_back_history() {
    let mut app = navigation();
    press(&mut app, "Increase home", "Home count 1");
    press(&mut app, "Open encoded detail", "Detail id a/b");
    press(&mut app, "Replace detail", "Replacement confirmed");
    back(&mut app, "Home count 1");
    assert!(!app.engine.back());
    assert!(app.engine.take_native_request().is_none());
}

#[test]
fn native_tab_presses_retain_each_page_state_without_creating_back_entries() {
    let mut app = navigation();
    press(&mut app, "Increase home", "Home count 1");
    // The fixture fixes a 1080 x 1240 viewport; these are touches in its two bottom tab slots.
    assert!(app.engine.tap(900.0, 1200.0));
    dispatch_request(&mut app);
    app.pump_until(|app| app.labels().contains(&"Archive count 0".into()));
    assert!(!app.labels().contains(&"Home count 1".into()));
    press(&mut app, "Increase archive", "Archive count 1");
    press(&mut app, "Open archive detail", "Detail id archive");
    back(&mut app, "Archive count 1");
    assert!(app.engine.tap(180.0, 1200.0));
    dispatch_request(&mut app);
    app.pump_until(|app| app.labels().contains(&"Home count 1".into()));
    assert!(!app.engine.back());
    assert!(app.engine.take_native_request().is_none());
    assert!(app.engine.tap(900.0, 1200.0));
    dispatch_request(&mut app);
    app.pump_until(|app| app.labels().contains(&"Archive count 1".into()));
}

#[test]
fn selecting_a_tab_from_a_covering_detail_reuses_the_tab_group_and_discards_detail_history() {
    let mut app = navigation();
    assert!(app.engine.tap(900.0, 1200.0));
    dispatch_request(&mut app);
    app.pump_until(|app| app.labels().contains(&"Archive count 0".into()));
    press(&mut app, "Increase archive", "Archive count 1");
    press(&mut app, "Open archive detail", "Detail id archive");
    press(&mut app, "Return to archive tab", "Archive count 1");
    assert!(
        !app.labels()
            .iter()
            .any(|label| label.starts_with("Detail id"))
    );
    assert!(
        !app.engine.back(),
        "selecting an existing tab group should discard its covering detail"
    );
    assert!(app.engine.take_native_request().is_none());
}

#[test]
fn native_notification_navigation_selects_static_routes_and_replaces_old_history() {
    let mut app = navigation();
    press(&mut app, "Open encoded detail", "Detail id a/b");
    app.runtime
        .send(json!({"type":"navigate", "path":"/detail/new"}).to_string())
        .unwrap();
    app.pump_until(|app| app.labels().contains(&"Static new detail".into()));
    assert!(
        !app.labels()
            .iter()
            .any(|label| label.starts_with("Detail id"))
    );
    back(&mut app, "Home count 0");
    assert!(!app.engine.back());
}

#[test]
fn obsolete_notification_route_shows_recovery_page_and_preserves_root_state() {
    let mut app = navigation();
    press(&mut app, "Increase home", "Home count 1");
    app.runtime
        .send(json!({"type":"navigate", "path":"/deleted-page"}).to_string())
        .unwrap();
    app.pump_until(|app| app.labels().contains(&"Go back".into()));
    assert!(
        app.labels()
            .join(" ")
            .contains("This notification links to a page that is no longer available.")
    );
    press(&mut app, "Go back", "Home count 1");
    assert!(!app.engine.back());
}
