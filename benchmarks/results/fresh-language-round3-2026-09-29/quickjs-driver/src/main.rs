use ink_runtime::{AppRuntime, Event};
use std::{env, fs, time::Duration};
fn main() {
    let source = fs::read_to_string(env::args().nth(1).expect("bundle path")).unwrap();
    let (runtime, events) = AppRuntime::spawn(source).unwrap();
    loop {
        match events.recv_timeout(Duration::from_secs(20)).expect("diagnostic timeout") {
            Event::Message(message) => {
                let value: serde_json::Value = serde_json::from_str(&message).unwrap();
                if value["type"] == "audit-final-proxy" {
                    println!("{}", serde_json::to_string_pretty(&value).unwrap());
                    runtime.stop();
                    break;
                }
                if value["type"] == "audit-final-error" || value["type"] == "error" {
                    panic!("{message}");
                }
            }
            Event::Error(error) => panic!("{error}"),
            Event::Stopped => panic!("stopped before diagnostic result"),
            Event::Ready | Event::Commit(_) => {},
        }
    }
}
