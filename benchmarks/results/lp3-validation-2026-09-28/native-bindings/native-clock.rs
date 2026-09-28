use ink_core::{Engine, ReactTree};
use serde_json::json;
fn main() {
    let mut engine = Engine::new(); engine.set_viewport(1080, 1240);
    let mut tree = ReactTree::default();
    tree.apply(serde_json::from_value(json!([
        {"op":"create","id":1,"type":"Screen","props":{}},
        {"op":"create","id":2,"type":"PlayingProgress","props":{"position":0,"duration":120,"playing":true,"showTimes":true}},
        {"op":"insert","id":2,"parent":1,"before":null},
        {"op":"insert","id":1,"parent":0,"before":null}
    ])).unwrap(), &mut engine).unwrap();
    let labels = |engine: &Engine| engine.scene().text.iter().map(|run| run.text.to_string()).collect::<Vec<_>>();
    assert_eq!(labels(&engine), vec!["0:00", "2:00"]);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert!(engine.animate_scene());
    assert_eq!(labels(&engine), vec!["0:01", "2:00"]);
    tree.apply(serde_json::from_value(json!([
        {"op":"update","id":2,"props":{"position":65,"duration":120,"playing":false,"showTimes":true}}
    ])).unwrap(), &mut engine).unwrap();
    assert_eq!(labels(&engine), vec!["1:05", "2:00"]);
    assert!(!engine.has_scene_animations());
    assert_eq!(labels(&engine), vec!["1:05", "2:00"]);
    println!("Native playback labels advance without JavaScript; seek and pause passed");
}
