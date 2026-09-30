use super::*;

#[test]
fn native_clock_updates_time_speed_pause_and_seek_without_a_commit() {
    let mut engine = Engine::new();
    engine.set_viewport(1080, 1240);
    let mut tree = ReactTree::default();
    tree.apply(ReactCommit(vec![
        Operation::Create { id: 1, r#type: HostKind::Screen, props: Map::new() },
        Operation::Create { id: 2, r#type: HostKind::PlayingProgress, props: json!({
            "position":0,"duration":120,"playing":false,"showTimes":true,"clock":7,"onSeek":true
        }).as_object().unwrap().clone() },
        Operation::Insert { id:2,parent:1,before:None }, Operation::Insert { id:1,parent:0,before:None },
    ]), &mut engine).unwrap();
    assert!(engine.update_playback_clock(7, 10.0, 120.0, true, 2.0));
    engine
        .playback_progress
        .get_mut(&NodeIdentity(2))
        .unwrap()
        .started = std::time::Instant::now() - std::time::Duration::from_secs(3);
    engine.animate_scene();
    assert!(engine.scene.text.iter().any(|run| run.text == "0:16"));
    engine.update_playback_clock(7, 35.0, 120.0, false, 1.0);
    engine.animate_scene();
    assert!(engine.scene.text.iter().any(|run| run.text == "0:35"));
    assert!(!engine.playback_progress[&NodeIdentity(2)].animating());
    engine.update_playback_clock(7, 9.0, 200.0, false, 1.0);
    engine.animate_scene();
    assert!(engine.scene.text.iter().any(|run| run.text == "3:20"));
    assert!(engine.scene.text.iter().any(|run| run.text == "0:09"));
    engine.remove_playback_clock(7);
    assert!(engine.playback_clocks.is_empty());
}

#[test]
fn playback_times_are_not_layout_dimensions() {
    for (milliseconds, seconds) in [(213_000, 213.0), (180_000_000, 180_000.0), (0, 0.0)] {
        let props = HostProps::Other(Map::from_iter([("duration".into(), json!(milliseconds))]));
        assert_eq!(
            playback_seconds(&props, "duration", 1000.0).unwrap(),
            seconds
        );
    }
    for value in [json!(-1), Json::Null, json!("213000"), json!(f64::MAX)] {
        let props = HostProps::Other(Map::from_iter([("duration".into(), value)]));
        assert!(playback_seconds(&props, "duration", 1000.0).is_err());
    }
}
