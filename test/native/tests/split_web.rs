use ink_runtime::{AppRuntime, EventReceiver};
use ink_test::{command, fixture_path, message};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn compiled_web_asset_loads_on_first_use_once_and_supplies_real_globals() {
    let assets = fixture_path("runtime", "split-web");
    let source = std::fs::read_to_string(assets.join("app.js")).unwrap();
    let loads = Arc::new(AtomicUsize::new(0));
    let observed = loads.clone();
    let (runtime, events) = AppRuntime::spawn_with_web_loader(
        source,
        || {},
        move || {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(std::fs::read_to_string(
                assets.join("ink-assets/ink-web.js"),
            )?)
        },
    )
    .unwrap();

    command(&runtime, "status");
    assert_eq!(
        message(&events, |value| value["label"] == "status")["value"],
        "running"
    );
    assert_eq!(loads.load(Ordering::SeqCst), 0);

    assert_web_globals(&runtime, &events);
    assert_eq!(loads.load(Ordering::SeqCst), 1);
}

#[test]
fn compiled_inline_web_supplies_globals_without_an_optional_asset_loader() {
    let assets = fixture_path("runtime", "split-web");
    let source = std::fs::read_to_string(assets.join("app-inline.js")).unwrap();
    let (runtime, events) = AppRuntime::spawn(source).unwrap();
    assert_web_globals(&runtime, &events);
}

fn assert_web_globals(runtime: &AppRuntime, events: &EventReceiver) {
    command(runtime, "wpt");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../fixtures/runtime/split-web/url-tr46-expected.json"
    )).unwrap();
    assert_eq!(expected.as_array().unwrap().len(), 1014);
    assert_eq!(
        message(events, |value| value["label"] == "wpt")["value"],
        expected
    );
    command(runtime, "idna");
    assert_eq!(
        message(events, |value| value["label"] == "idna")["value"],
        json!([
            "example.com", "xn--mnchen-3ya.de", "xn--mnchen-3ya.de", "xn--fa-hia.de", "xn--53h.example",
            "xn--zca.com", "lookout.net", "lookout.net", "xn--s5a.com", "xn--snl.com", "xn--r5g.com",
            "xn--r8jz45g.xn--zckzah", "-x.xn--zca", "x-.xn--zca", "ab--c.xn--zca", "a..xn--zca.",
            format!("{}.xn--zca", "a".repeat(64)), null, null, null, null, null, null
        ])
    );
    assert_eq!(message(events, |value| value["label"] == "idna-setter")["value"], "https://xn--mnchen-3ya.de/");
    assert_eq!(message(events, |value| value["label"] == "idna-invalid-setter")["value"], "https://xn--mnchen-3ya.de/");
    command(runtime, "url");
    assert_eq!(
        message(events, |value| value["label"] == "url")["value"],
        json!([
            "https://example.com/caf%C3%A9?tag=one&tag=two",
            "example.com",
            ["one", "two"]
        ])
    );
    command(runtime, "bodies");
    assert_eq!(
        message(events, |value| value["label"] == "bodies")["value"],
        json!([
            [0, 128, 255, 99, 97, 102, 195, 169],
            201,
            "application/json",
            {"title":"café"},
            "{\"title\":\"café\"}",
            true,
            true
        ])
    );
    command(runtime, "streams");
    assert_eq!(
        message(events, |value| value["label"] == "streams")["value"],
        json!([
            {"value":"CAFÉ", "done":false},
            {"value":"NOTES", "done":false},
            {"done":true}
        ])
    );
}

#[test]
fn a_failed_optional_asset_read_does_not_poison_the_getter_or_runtime() {
    let assets = fixture_path("runtime", "split-web");
    let source = std::fs::read_to_string(assets.join("app.js")).unwrap();
    let loads = Arc::new(AtomicUsize::new(0));
    let observed = loads.clone();
    let (runtime, events) = AppRuntime::spawn_with_web_loader(
        source,
        || {},
        move || {
            let attempt = observed.fetch_add(1, Ordering::SeqCst);
            let filename = if attempt == 0 {
                "ink-assets/not-installed-web.js"
            } else {
                "ink-assets/ink-web.js"
            };
            Ok(std::fs::read_to_string(assets.join(filename))?)
        },
    )
    .unwrap();

    command(&runtime, "url");
    let failure = message(&events, |value| value["label"] == "error");
    assert_eq!(
        failure["value"],
        format!("Error: {}", std::io::Error::from_raw_os_error(2))
    );
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    command(&runtime, "status");
    assert_eq!(
        message(&events, |value| value["label"] == "status")["value"],
        "running"
    );
    command(&runtime, "url");
    assert_eq!(
        message(&events, |value| value["label"] == "url")["value"],
        json!([
            "https://example.com/caf%C3%A9?tag=one&tag=two",
            "example.com",
            ["one", "two"]
        ])
    );
    assert_eq!(loads.load(Ordering::SeqCst), 2);
}
