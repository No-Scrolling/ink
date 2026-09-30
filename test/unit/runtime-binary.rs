use crate::{AppRuntime, Event};
use std::time::{Duration, Instant};

fn message(events: &crate::EventReceiver) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap()
        {
            Event::Message(value) => return value,
            Event::Ready => {}
            Event::Error(error) => panic!("{error}"),
            _ => panic!("unexpected runtime event"),
        }
    }
}

#[test]
fn binary_payloads_preserve_subarrays_and_are_consumed_once() {
    let (runtime, events) = AppRuntime::spawn(r#"
        globalThis.__inkReceive = (message, bytes) => {
            if (!(bytes instanceof Uint8Array) || bytes.length !== 4 || bytes[3] !== 255) throw Error('invalid incoming bytes');
            __inkPost(message + ':' + Array.from(bytes).join(','));
        };
        const data = new Uint8Array([19, 0, 128, 255, 27]);
        __inkPostBytes('{"type":"call","id":1,"binary":true}', data.subarray(1, 4));
        data.fill(7);
    "#.into()).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&message(&events)).unwrap(),
        serde_json::json!({"type":"call", "id":1, "binary":true})
    );
    assert_eq!(runtime.take_bytes(1), Some(vec![0, 128, 255]));
    assert_eq!(runtime.take_bytes(1), None);
    runtime
        .send_bytes("reply".into(), vec![0, 1, 128, 255])
        .unwrap();
    assert_eq!(message(&events), "reply:0,1,128,255");
    assert!(
        runtime
            .send_bytes("reply".into(), vec![0; super::MAX_BYTES + 1])
            .is_err()
    );
}

#[test]
fn rejected_and_duplicate_payloads_do_not_replace_queued_bytes() {
    let (runtime, events) = AppRuntime::spawn(r#"
        __inkPostBytes('{"type":"call","id":8,"binary":true}', new Uint8Array([11]));
        let rejected = 0;
        try { __inkPostBytes('{"type":"call","id":8,"binary":true}', new Uint8Array([12])); } catch { rejected++; }
        try { __inkPostBytes('{"type":"call","id":9,"binary":true}', new Uint8Array(524289)); } catch { rejected++; }
        try { __inkPostBytes('{"type":"call","id":10}', new Uint8Array([13])); } catch { rejected++; }
        __inkPost(String(rejected));
    "#.into()).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&message(&events)).unwrap(),
        serde_json::json!({"type":"call", "id":8, "binary":true})
    );
    assert_eq!(message(&events), "3");
    assert_eq!(runtime.take_bytes(8), Some(vec![11]));
    assert_eq!(runtime.take_bytes(9), None);
    assert_eq!(runtime.take_bytes(10), None);
}
