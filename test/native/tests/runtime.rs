use ink_runtime::{AppRuntime, Event};
use ink_test::{WAIT, command, compiled_runtime, message, next_event};
use serde_json::json;
use std::time::Instant;

#[test]
fn cancellation_rejects_once_and_ignores_a_late_native_reply() {
    let (runtime, events) = compiled_runtime("runtime", "bridge");
    command(&runtime, "call");
    let call = message(&events, |value| value["type"] == "call");
    assert_eq!(call["module"], "fixture");
    assert_eq!(call["payload"], json!({"name":"café"}));
    assert_eq!(call["timeoutMs"], 1234);
    command(&runtime, "abort");
    let cancel = message(&events, |value| value["type"] == "cancel");
    assert_eq!(cancel["id"], call["id"]);
    assert_eq!(
        message(&events, |value| value["label"] == "rejected")["value"],
        "user-cancelled"
    );
    runtime
        .send(json!({"type":"result", "id":call["id"], "value":"too late"}).to_string())
        .unwrap();
    runtime
        .send(json!({"type":"test", "command":"echo", "value":"after reply"}).to_string())
        .unwrap();
    let deadline = Instant::now() + WAIT;
    loop {
        match next_event(&events, deadline) {
            Event::Message(source) => {
                let value: serde_json::Value = serde_json::from_str(&source).unwrap();
                assert_ne!(value["label"], "settled");
                assert_ne!(value["label"], "rejected");
                if value["label"] == "echo" {
                    assert_eq!(value["value"], "after reply");
                    break;
                }
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
}

#[test]
fn pre_aborted_requests_never_cross_the_native_bridge() {
    let (runtime, events) = compiled_runtime("runtime", "bridge");
    runtime
        .send(json!({"type":"test", "command":"call", "preAborted":true}).to_string())
        .unwrap();
    let deadline = Instant::now() + WAIT;
    loop {
        match next_event(&events, deadline) {
            Event::Message(source) => {
                let value: serde_json::Value = serde_json::from_str(&source).unwrap();
                assert_ne!(value["type"], "call");
                if value["label"] == "rejected" {
                    assert_eq!(value["value"], "before-send");
                    break;
                }
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
}

#[test]
fn native_error_shape_survives_real_quickjs_delivery() {
    let (runtime, events) = compiled_runtime("runtime", "bridge");
    command(&runtime, "call");
    let call = message(&events, |value| value["type"] == "call");
    runtime.send(json!({"type":"result", "id":call["id"], "kind":"permission-denied", "message":"No access", "retryable":true}).to_string()).unwrap();
    assert_eq!(
        message(&events, |value| value["label"] == "rejected")["value"],
        json!(["permission-denied", "No access", true])
    );
    command(&runtime, "abort");
    let deadline = Instant::now() + WAIT;
    loop {
        match next_event(&events, deadline) {
            Event::Message(source) => {
                let value: serde_json::Value = serde_json::from_str(&source).unwrap();
                assert_ne!(
                    value["type"], "cancel",
                    "settled request retained its abort listener"
                );
                if value["label"] == "aborted" {
                    assert_eq!(value["value"], true);
                    break;
                }
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
}

#[test]
fn binary_requests_own_their_copy_and_js_results_outlive_subsequent_deliveries() {
    let (runtime, events) = compiled_runtime("runtime", "bridge");
    command(&runtime, "binary");
    let call = message(&events, |value| value["type"] == "call");
    let id = call["id"].as_u64().unwrap();
    assert_eq!(runtime.take_bytes(id), Some(vec![0, 127, 128, 255]));
    assert_eq!(runtime.take_bytes(id), None);
    runtime
        .send_bytes(
            json!({"type":"result", "id":id, "value":"received"}).to_string(),
            vec![255, 128, 0],
        )
        .unwrap();
    assert_eq!(
        message(&events, |value| value["label"] == "binary-result")["value"],
        json!(["received", [255, 128, 0]])
    );
    runtime
        .send_bytes(
            json!({"type":"result", "id":id, "value":"duplicate"}).to_string(),
            vec![7, 8, 9],
        )
        .unwrap();
    command(&runtime, "retained");
    assert_eq!(
        message(&events, |value| value["label"] == "retained")["value"],
        json!([255, 128, 0])
    );
}

#[test]
fn pending_request_capacity_recovers_after_native_completion() {
    let (runtime, events) = compiled_runtime("runtime", "bridge");
    let mut ids = Vec::new();
    let deadline = Instant::now() + WAIT;
    for round in 0..8 {
        runtime
            .send(
                json!({"type":"test", "command":"pending", "count":32, "round":round}).to_string(),
            )
            .unwrap();
        loop {
            match next_event(&events, deadline) {
                Event::Message(source) => {
                    let value: serde_json::Value = serde_json::from_str(&source).unwrap();
                    if value["type"] == "call" {
                        ids.push(value["id"].clone());
                    }
                    if value["label"] == "pending-sent" {
                        assert_eq!(value["value"], round);
                        break;
                    }
                }
                Event::Error(error) => panic!("{error}"),
                _ => {}
            }
        }
    }
    runtime
        .send(json!({"type":"test", "command":"pending", "count":1, "round":8}).to_string())
        .unwrap();
    loop {
        match next_event(&events, deadline) {
            Event::Message(source) => {
                let value: serde_json::Value = serde_json::from_str(&source).unwrap();
                if value["type"] == "call" {
                    ids.push(value["id"].clone());
                }
                if value["label"] == "capacity" {
                    assert_eq!(value["value"], json!(["busy", true]));
                    break;
                }
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
    assert_eq!(ids.len(), 256);
    runtime
        .send(json!({"type":"result", "id":ids[0], "value":"released"}).to_string())
        .unwrap();
    command(&runtime, "call");
    let call = message(&events, |value| value["type"] == "call");
    assert_eq!(call["operation"], "lookup");
    assert!(!ids.contains(&call["id"]));
}

#[test]
fn native_input_limits_reject_without_poisoning_the_runtime() {
    let (runtime, events) = compiled_runtime("runtime", "bridge");
    assert!(
        runtime
            .send("x".repeat(1024 * 1024 + 1))
            .unwrap_err()
            .to_string()
            .contains("exceeds")
    );
    assert!(
        runtime
            .send_bytes("{}".into(), vec![0; 512 * 1024 + 1])
            .is_err()
    );
    runtime
        .send(json!({"type":"test", "command":"echo", "value":"still alive"}).to_string())
        .unwrap();
    assert_eq!(
        message(&events, |value| value["label"] == "echo")["value"],
        "still alive"
    );
}

#[test]
fn draining_output_releases_its_shared_byte_budget() {
    let (runtime, events) = AppRuntime::spawn("__inkPost('é'.repeat(20000));".into()).unwrap();
    let deadline = Instant::now() + WAIT;
    let mut ready = false;
    let mut output = false;
    while !ready || !output {
        match next_event(&events, deadline) {
            Event::Message(value) => {
                assert_eq!(value.len(), 40000);
                output = true;
            }
            Event::Ready => ready = true,
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
    assert_eq!(runtime.queue_metrics().0, 0);
    assert!(runtime.queue_metrics().1 >= 40000);
}

#[test]
fn stopping_interrupts_an_infinite_microtask_chain_and_closes_delivery() {
    let (runtime, events) = AppRuntime::spawn(
        "Promise.resolve().then(function again() { return Promise.resolve().then(again); });"
            .into(),
    )
    .unwrap();
    let deadline = Instant::now() + WAIT;
    assert!(matches!(next_event(&events, deadline), Event::Ready));
    let started = Instant::now();
    runtime.stop();
    assert!(runtime.send("{}".into()).is_err());
    assert!(runtime.send_latest(1, "{}".into()).is_err());
    assert!(runtime.send_bytes("{}".into(), vec![]).is_err());
    loop {
        if matches!(next_event(&events, deadline), Event::Stopped) {
            break;
        }
    }
    drop(runtime);
    assert!(started.elapsed() < WAIT);
}

#[test]
fn unhandled_rejection_is_terminal_but_a_same_turn_handler_is_not() {
    let (runtime, events) = AppRuntime::spawn("Promise.reject(Error('handled')).catch(() => {}); Promise.reject(Error('terminal rejection'));".into()).unwrap();
    let deadline = Instant::now() + WAIT;
    let mut error = false;
    loop {
        match next_event(&events, deadline) {
            Event::Error(value) => {
                assert!(value.contains("terminal rejection"));
                assert!(!value.contains("handled"));
                error = true;
            }
            Event::Stopped => break,
            _ => {}
        }
    }
    assert!(error);
    assert!(runtime.send("{}".into()).is_err());
}

fn gated_runtime() -> (
    AppRuntime,
    ink_runtime::EventReceiver,
    std::sync::mpsc::Sender<()>,
) {
    let (entered, entering) = std::sync::mpsc::channel();
    let (release, waiting) = std::sync::mpsc::channel();
    let (runtime, events) = AppRuntime::spawn_with_web_loader(
        "__inkLoadWeb(); globalThis.__inkReceive = message => __inkPost(message);".into(),
        || {},
        move || {
            entered.send(()).unwrap();
            waiting
                .recv_timeout(WAIT)
                .expect("test did not release its startup gate");
            Ok(String::new())
        },
    )
    .unwrap();
    entering.recv_timeout(WAIT).unwrap();
    (runtime, events, release)
}

#[test]
fn latest_delivery_coalesces_by_key_and_releases_replaced_payload_bytes() {
    let (runtime, events, release) = gated_runtime();
    for index in 0..100 {
        runtime
            .send_latest(7, json!({"value":index}).to_string())
            .unwrap();
    }
    runtime
        .send_latest(8, json!({"value":"other"}).to_string())
        .unwrap();
    assert!(
        runtime.queue_metrics().0 < 100,
        "coalesced messages retained old payloads"
    );
    release.send(()).unwrap();
    assert_eq!(
        message(&events, |value| value["value"] == 99),
        json!({"value":99})
    );
    assert_eq!(
        message(&events, |value| value["value"] == "other"),
        json!({"value":"other"})
    );
    assert_eq!(runtime.queue_metrics().0, 0);
}

#[test]
fn aggregate_command_bytes_are_bounded_and_terminal_failure_reclaims_the_queue() {
    let (runtime, events, release) = gated_runtime();
    for _ in 0..16 {
        runtime.send("x".repeat(1024 * 1024)).unwrap();
    }
    assert_eq!(runtime.queue_metrics().0, 16 * 1024 * 1024);
    assert!(
        runtime
            .send("x".into())
            .unwrap_err()
            .to_string()
            .contains("queue is full")
    );
    release.send(()).unwrap();
    let deadline = Instant::now() + WAIT;
    let mut error = false;
    loop {
        match next_event(&events, deadline) {
            Event::Error(value) => {
                assert!(value.contains("queue"));
                error = true;
            }
            Event::Stopped => break,
            _ => {}
        }
    }
    assert!(error);
    assert_eq!(runtime.queue_metrics().0, 0);
    assert_eq!(runtime.queue_metrics().1, 16 * 1024 * 1024);
}

#[test]
fn outbound_binary_byte_budget_recovers_after_consuming_an_owned_buffer() {
    let source = r#"
        for (let id = 1; id <= 16; id++) __inkPostBytes(JSON.stringify({type:'call', binary:true, id}), new Uint8Array(524288).fill(id));
        try { __inkPostBytes(JSON.stringify({type:'call', binary:true, id:17}), new Uint8Array([17])); }
        catch (error) { __inkPost(JSON.stringify({type:'full', message:error.message})); }
        globalThis.__inkReceive = () => __inkPostBytes(JSON.stringify({type:'call', binary:true, id:18}), new Uint8Array([18]));
    "#;
    let (runtime, events) = AppRuntime::spawn(source.into()).unwrap();
    let full = message(&events, |value| value["type"] == "full");
    assert!(
        full["message"]
            .as_str()
            .unwrap()
            .contains("binary queue is full")
    );
    assert_eq!(runtime.take_bytes(17), None);
    let first = runtime.take_bytes(1).unwrap();
    assert_eq!(first.len(), 512 * 1024);
    assert!(first.iter().all(|byte| *byte == 1));
    runtime.send("release".into()).unwrap();
    message(&events, |value| value["id"] == 18);
    assert_eq!(runtime.take_bytes(18), Some(vec![18]));
    assert_eq!(runtime.take_bytes(2).unwrap().len(), 512 * 1024);
}
