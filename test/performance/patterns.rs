use super::{App, TIMEOUT, event_name};
use ink_core::{ImageData, TextEdit};
use ink_protocol::{Operation, ReactCommit};
use ink_runtime::Event;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

impl App {
    fn has_label(&self, label: &str) -> bool {
        self.engine.scene().text.iter().any(|run| run.text == label)
    }

    fn refresh(&mut self) -> usize {
        let events = self.tree.viewport_events(&mut self.engine).unwrap();
        let count = events.len();
        for event in events {
            self.runtime.send(event.to_string()).unwrap();
        }
        count
    }

    fn wait_for(&mut self, predicate: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if self.refresh() == 0 && predicate(self) {
                return;
            }
            match self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                Ok(Event::Commit(commit)) => self.tree.apply(commit, &mut self.engine).unwrap(),
                Ok(Event::Ready | Event::Message(_)) => {}
                result => panic!(
                    "scene did not settle: {}; labels: {:?}",
                    result
                        .map(|event| event_name(&event))
                        .unwrap_or_else(|error| error.to_string()),
                    self.engine
                        .scene()
                        .text
                        .iter()
                        .map(|run| run.text.as_str())
                        .collect::<Vec<_>>()
                ),
            }
        }
    }

    fn send(&self, payload: Value) {
        self.runtime.send(payload.to_string()).unwrap();
    }

    fn sample(&mut self, start: Instant) -> Value {
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let core = self.engine.take_perf_metrics();
        json!({"elapsed_ns":elapsed_ns, "nodes_measured":core.nodes_measured,
            "full_rebuilds":core.full_rebuilds, "incremental_rebuilds":core.incremental_rebuilds})
    }

    fn point(&self, prefix: &str) -> (f32, f32) {
        let scene = self.engine.scene();
        scene
            .text
            .iter()
            .find_map(|run| {
                if !run.text.starts_with(prefix) {
                    return None;
                }
                let y = run.rect.y
                    + if run.scrolling {
                        scene.scroll_origin - scene.scroll_offset
                    } else {
                        0.0
                    };
                let clip = if run.scrolling {
                    scene.scroll_clip.unwrap()
                } else {
                    run.clip
                };
                let centre = y + run.rect.height / 2.0;
                (centre >= clip.y
                    && centre < clip.y + clip.height
                    && centre >= run.clip.y
                    && centre < run.clip.y + run.clip.height)
                    .then_some((run.rect.x + run.rect.width / 2.0, centre))
            })
            .unwrap_or_else(|| panic!("no visible label: {prefix}"))
    }

    fn dispatch(&mut self) {
        let request = self
            .engine
            .take_native_request()
            .expect("native action must dispatch");
        assert_eq!(request.module(), "ink");
        let event = self
            .tree
            .route_native_message(request.payload().to_owned())
            .unwrap();
        self.runtime.send(event).unwrap();
    }

    fn press(&mut self, label: &str, expected: &str) {
        let (x, y) = self.point(label);
        assert!(self.engine.tap(x, y));
        self.dispatch();
        self.wait_for(|app| app.has_label(expected));
    }

    fn edit(&mut self, value: &str) {
        let context: Value = serde_json::from_str(&self.engine.text_input_context()).unwrap();
        assert!(
            self.engine.edit_text(TextEdit::Update(
                json!({"id":context["id"],
            "text":context["text"], "value":value, "selection":value.encode_utf16().count()})
                .to_string()
            ))
        );
        let events = self.tree.input_events(&self.engine);
        assert_eq!(events.len(), 1);
        self.runtime.send(events[0].to_string()).unwrap();
    }

    fn observe(&mut self) -> Value {
        self.send(json!({"type":"benchmark", "command":"report"}));
        loop {
            match self.next() {
                Event::Commit(commit) => {
                    self.tree.apply(commit, &mut self.engine).unwrap();
                    self.refresh();
                }
                Event::Message(message) => {
                    let value: Value = serde_json::from_str(&message).unwrap();
                    if value["type"] == "observed" {
                        return value;
                    }
                }
                Event::Ready => {}
                event => panic!("unexpected report event: {}", event_name(&event)),
            }
        }
    }
}

fn row_data(count: usize) -> Vec<Value> {
    const TITLES: [&str; 4] = [
        "Harbour Lights",
        "An evening recording from the old theatre beside the river",
        "Málaga at Dusk",
        "Zürich Sessions",
    ];
    (0..count)
        .map(|index| {
            json!({"id":index.to_string(),
        "title":format!("R{index:05} · {}", TITLES[index % 4]),
        "subtitle":format!("Artist {index:05} · 3:20")})
        })
        .collect()
}

fn rows(count: usize, compatibility: bool) -> App {
    let mut app = App::new("rows");
    app.send(json!({"type":"benchmark", "command":"populate", "items":row_data(count), "compatibility":compatibility}));
    app.wait_for(|app| app.has_label("Artist 00000 · 3:20"));
    assert_eq!(
        app.native_lists, 1,
        "the initially empty list starts native"
    );
    verify_rows(&app, count, false);
    app.engine.take_perf_metrics();
    app
}

fn row_ids(app: &App, visible: bool) -> Vec<usize> {
    let scene = app.engine.scene();
    scene
        .text
        .iter()
        .filter_map(|run| {
            let text = run.text.as_str();
            if text.as_bytes().first() != Some(&b'R')
                || !text.as_bytes().get(1..6)?.iter().all(u8::is_ascii_digit)
            {
                return None;
            }
            if visible {
                let clip = scene.scroll_clip.unwrap();
                let y =
                    run.rect.y + scene.scroll_origin - scene.scroll_offset + run.rect.height / 2.0;
                if y < clip.y || y >= clip.y + clip.height {
                    return None;
                }
            }
            Some(text[1..6].parse().unwrap())
        })
        .collect()
}

fn verify_rows(app: &App, count: usize, reversed: bool) {
    let scene = app.engine.scene();
    let ids = row_ids(app, false);
    assert!(
        !ids.is_empty() && ids.len() < 100,
        "row windows must remain bounded"
    );
    assert!(ids.iter().all(|id| *id < count));
    for id in &ids {
        let prefix = format!("R{id:05} · ");
        let title = scene
            .text
            .iter()
            .find(|run| run.text.starts_with(&prefix))
            .unwrap();
        let expected = match id % 4 {
            0 => "Harbour Lights",
            1 => "An evening",
            2 => "Málaga at Dusk",
            _ => "Zürich Sessions",
        };
        assert!(
            title.text[prefix.len()..].starts_with(expected),
            "row identity must retain its authored title"
        );
        let clip = scene.scroll_clip.unwrap();
        let y = title.rect.y + scene.scroll_origin - scene.scroll_offset;
        let complete = y >= clip.y
            && y + title.rect.height * if id % 4 == 1 { 3.0 } else { 2.0 } < clip.y + clip.height;
        // A row cut off at the window boundary may contain only its title.
        if *id > 0 && complete {
            let subtitle = scene
                .text
                .iter()
                .find(|run| run.text == format!("Artist {id:05} · 3:20"))
                .unwrap_or_else(|| panic!("row {id} lost its subtitle"));
            assert!(subtitle.rect.y > title.rect.y);
            if id % 4 == 1 {
                assert!(
                    scene.text.iter().any(|run| run.text.contains("from")
                        && run.rect.y > title.rect.y
                        && run.rect.y < subtitle.rect.y),
                    "long titles must retain their second line"
                );
            }
        }
    }
    assert!(
        ids.windows(2).all(|pair| if reversed {
            pair[0] > pair[1]
        } else {
            pair[0] < pair[1]
        }),
        "row order must match the supplied collection"
    );
    assert!(
        app.host_nodes() < 500,
        "retained row hosts must remain bounded"
    );
}

fn row_change(command: &str, count: usize, samples: usize, warmup: usize) -> Value {
    let mut app = rows(count, false);
    if command == "prepend" {
        app.engine.scroll_by(1100.0);
        assert_eq!(app.refresh(), 0);
    }
    let mut times = Vec::new();
    let mut max_nodes = 0;
    for step in 0..warmup + samples {
        let bit = (step + 1) % 2;
        let anchor = row_ids(&app, true)[0];
        let (x_before, y_before) = app.point(&format!("R{anchor:05}"));
        let start = Instant::now();
        app.send(match command {
            "edit" => json!({"type":"benchmark", "command":command, "subtitle":format!("Changed artist {bit}")}),
            "prepend" => json!({"type":"benchmark", "command":command, "insert":bit == 1,
                "item":{"id":"new", "title":"New arrival", "subtitle":"Inserted before the viewport"}}),
            "reorder" => json!({"type":"benchmark", "command":command}),
            _ => unreachable!(),
        });
        match app.next() {
            Event::Commit(commit) => app.tree.apply(commit, &mut app.engine).unwrap(),
            event => panic!("unexpected rows event: {}", event_name(&event)),
        }
        assert_eq!(app.refresh(), 0);
        let time = app.sample(start);
        verify_rows(&app, count, command == "reorder" && bit == 1);
        if command == "edit" {
            assert!(app.has_label(&format!("Changed artist {bit}")));
            assert!(app.has_label("Artist 00001 · 3:20"));
        } else if command == "prepend" {
            let (x_after, y_after) = app.point(&format!("R{anchor:05}"));
            assert!(
                (x_before - x_after).abs() < 0.1 && (y_before - y_after).abs() < 0.1,
                "prepending/removing must preserve the visible anchor"
            );
        }
        max_nodes = max_nodes.max(app.host_nodes());
        if step >= warmup {
            times.push(time);
        }
    }
    if command == "reorder" {
        app.send(json!({"type":"benchmark", "command":"reorder"}));
        app.wait_for(|app| row_ids(app, false).windows(2).all(|pair| pair[0] > pair[1]));
        let selected = row_ids(&app, true)[0];
        app.press(&format!("R{selected:05}"), &format!("Rows {selected}"));
    }
    json!({"name":format!("rows-{command}-{count}"), "samples":times,
        "max_host_nodes":max_nodes, "contracts":{"bounded_hosts":true, "order_and_neighbours":true,
        "anchor_preserved":command == "prepend", "reordered_callback_checked":command == "reorder"}})
}

fn row_scroll(compatibility: bool, samples: usize, warmup: usize) -> Value {
    let mut app = rows(1000, compatibility);
    let mut times = Vec::new();
    let mut window_events = 0;
    for step in 0..warmup + samples {
        let before = row_ids(&app, true)[0];
        let start = Instant::now();
        let delta = if (step / 8).is_multiple_of(2) {
            600.0
        } else {
            -600.0
        };
        assert!(app.engine.scroll_by(delta));
        let events = app.refresh();
        window_events += events;
        if events > 0 {
            match app.next() {
                Event::Commit(commit) => app.tree.apply(commit, &mut app.engine).unwrap(),
                event => panic!(
                    "expected the requested React window commit: {}",
                    event_name(&event)
                ),
            }
            app.wait_for(|app| app.engine.list_viewports_ready());
        }
        let time = app.sample(start);
        let after = row_ids(&app, true)[0];
        assert!(if delta > 0.0 {
            after > before
        } else {
            after < before
        });
        verify_rows(&app, 1000, false);
        if !compatibility {
            assert_eq!(events, 0, "native row scrolling must not enter JS");
            assert!(app.events.try_recv().is_err());
        }
        if step >= warmup {
            times.push(time);
        }
    }
    assert_eq!(
        window_events > 0,
        compatibility,
        "custom React rows must exercise the compatibility path"
    );
    json!({"name":if compatibility { "rows-react-scroll-1000" } else { "rows-native-scroll-1000" },
        "samples":times, "contracts":{"bounded_hosts":true, "window_events":window_events}})
}

fn image_arrival(samples: usize, warmup: usize) -> Value {
    let mut app = rows(100, false);
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        let mut items = row_data(100);
        items[0]["image"] = json!(format!("ink-file://artwork-{step}"));
        app.send(json!({"type":"benchmark", "command":"populate", "items":items}));
        match app.next() {
            Event::Commit(commit) => app.tree.apply(commit, &mut app.engine).unwrap(),
            event => panic!("unexpected artwork update: {}", event_name(&event)),
        }
        let request = app
            .engine
            .take_native_request()
            .expect("new artwork must request native bytes");
        assert!(app.engine.image_request_target(request.id()).is_some());
        let revision = app.engine.scene().image_revision;
        let grey = if step.is_multiple_of(2) { 64 } else { 192 };
        let pixels = [grey, grey, grey, 255].repeat(64 * 64);
        app.engine.take_perf_metrics();
        let start = Instant::now();
        assert!(
            app.engine
                .complete_native_image(request.id(), 64, 64, pixels, None)
        );
        let time = app.sample(start);
        assert!(app.engine.scene().image_revision > revision);
        let image = app
            .engine
            .scene()
            .images
            .first()
            .expect("completed artwork must appear");
        match &image.image {
            ImageData::Remote(image) => {
                assert_eq!((image.width, image.height), (64, 64));
                assert_eq!(
                    image.pixels[0],
                    if step.is_multiple_of(2) { 64 } else { 192 }
                );
            }
            _ => panic!("expected the supplied native artwork"),
        }
        verify_rows(&app, 100, false);
        if step >= warmup {
            times.push(time);
        }
    }
    json!({"name":"rows-decoded-image-arrival", "samples":times,
        "contracts":{"artwork_and_neighbours":true, "boundary":"decoded RGBA to native scene; no decoder or GPU"}})
}

fn startup(samples: usize, warmup: usize) -> Value {
    let (source, icons) = App::assets("navigation");
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        let start = Instant::now();
        let mut app = App::start(source.clone(), &icons);
        app.wait_for(|app| app.has_label("Open album"));
        let time = app.sample(start);
        assert!(app.has_label("No selection"));
        assert!(!app.engine.back());
        if step >= warmup {
            times.push(time);
        }
    }
    json!({"name":"startup-runtime-to-library", "samples":times,
        "contracts":{"usable_root":true, "boundary":"fresh runtime and engine; cached source, no Android process launch"}})
}

fn navigation(command: &str, samples: usize, warmup: usize) -> Value {
    let mut app = App::new("navigation");
    app.press("Save selection", "Saved selection");
    let home_nodes = app.host_nodes();
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        if command == "back" {
            app.press("Open album", "Album harbour");
        }
        app.engine.take_perf_metrics();
        let point = (command == "open").then(|| app.point("Open album"));
        let start = Instant::now();
        let expected = match command {
            "open" => {
                let (x, y) = point.unwrap();
                assert!(app.engine.tap(x, y));
                "Album harbour"
            }
            "back" => {
                assert!(app.engine.back());
                "Saved selection"
            }
            "tab" => {
                assert!(
                    app.engine
                        .tap(if step.is_multiple_of(2) { 900.0 } else { 180.0 }, 1200.0)
                );
                if step.is_multiple_of(2) {
                    "Archived albums"
                } else {
                    "Saved selection"
                }
            }
            _ => unreachable!(),
        };
        app.dispatch();
        app.wait_for(|app| app.has_label(expected));
        let time = app.sample(start);
        assert!(
            app.host_nodes() < 500,
            "navigation must not retain unbounded mounted hosts"
        );
        if step >= warmup {
            times.push(time);
        }
        if command == "open" {
            assert!(app.has_label("Track 1"));
            assert!(app.engine.back());
            app.dispatch();
            app.wait_for(|app| app.has_label("Saved selection"));
        }
        if command != "tab" {
            assert_eq!(
                app.host_nodes(),
                home_nodes,
                "popped pages must release their mounted hosts"
            );
            assert!(!app.engine.back());
        }
    }
    json!({"name":format!("navigation-{command}"), "samples":times,
        "contracts":{"state_retained":true, "bounded_hosts":true, "native_hit_testing_and_back":true}})
}

fn messages(count: usize) -> Vec<Value> {
    (0..count).map(|index| {
        let mut message = json!({"id":format!("seed-{index}"), "text":format!("Message {index:05}: Hello from Málaga"),
            "timestamp":1700000000000_u64 + index as u64 * 60000,
            "author":"Béatrice", "outgoing":index.is_multiple_of(2), "status":"delivered"});
        if index.is_multiple_of(3) {
            message["reply"] = json!({"author":"Alex", "text":"Meet by the harbour"});
        }
        message
    }).collect()
}

fn conversation() -> App {
    let mut app = App::new("conversation");
    app.send(json!({"type":"benchmark", "command":"populate", "messages":messages(128)}));
    app.wait_for(|app| {
        app.engine
            .scene()
            .text
            .iter()
            .any(|run| run.text.starts_with("Message 00127:"))
    });
    let (x, y) = app.point("Message…");
    assert!(app.engine.tap(x, y));
    assert!(app.engine.text_input_active());
    app.engine.take_perf_metrics();
    app
}

fn input(samples: usize, warmup: usize) -> Value {
    let mut app = conversation();
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        let value = format!("Draft {step} · Málaga 🙂");
        let start = Instant::now();
        app.edit(&value);
        loop {
            match app.next() {
                Event::Commit(commit) => {
                    let acknowledged = input_value(&commit).is_some_and(|text| text == value);
                    app.tree.apply(commit, &mut app.engine).unwrap();
                    app.refresh();
                    if acknowledged {
                        break;
                    }
                }
                event => panic!("unexpected input event: {}", event_name(&event)),
            }
        }
        let time = app.sample(start);
        let context: Value = serde_json::from_str(&app.engine.text_input_context()).unwrap();
        assert_eq!(context["text"], value);
        assert_eq!(context["cursor"], value.encode_utf16().count());
        assert!(app.host_nodes() < 500);
        if step >= warmup {
            times.push(time);
        }
    }
    let report = app.observe();
    assert_eq!(
        report["draft"],
        format!("Draft {} · Málaga 🙂", samples + warmup - 1)
    );
    json!({"name":"conversation-controlled-input", "samples":times,
        "contracts":{"native_editor_to_js_ack":true, "unicode_and_cursor_preserved":true}})
}

fn input_value(commit: &ReactCommit) -> Option<&str> {
    commit.0.iter().find_map(|operation| match operation {
        Operation::Update { props, .. } => props.get("value").and_then(Value::as_str),
        _ => None,
    })
}

fn conversation_change(command: &str, samples: usize, warmup: usize) -> Value {
    let mut app = conversation();
    if command == "prepend" {
        assert!(app.engine.scroll_by(-1400.0));
        assert_eq!(app.refresh(), 0);
    }
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        let scene = app.engine.scene();
        let clip = scene.scroll_clip.unwrap();
        let first = scene
            .text
            .iter()
            .find(|run| {
                let y =
                    run.rect.y + scene.scroll_origin - scene.scroll_offset + run.rect.height / 2.0;
                (run.text.starts_with("Message ") || run.text.starts_with("Arrival "))
                    && y >= clip.y
                    && y < clip.y + clip.height
            })
            .unwrap()
            .text
            .to_string();
        let before = app.point(&first);
        let start = Instant::now();
        app.send(if command == "append" {
            json!({"type":"benchmark", "command":command, "messages":[{
                "id":format!("incoming-{step}"), "text":format!("Arrival {step:05}: Hello"), "timestamp":1700010000000_u64 + step as u64 * 60000,
                "author":"Alex", "reactions":"♥ 1"}]})
        } else {
            json!({"type":"benchmark", "command":command, "insert":step.is_multiple_of(2),
                "message":{"id":"older", "text":"Older history", "timestamp":1699990000000_u64}})
        });
        if command == "append" {
            app.wait_for(|app| app.has_label(&format!("Arrival {step:05}: Hello")));
        } else {
            match app.next() {
                Event::Commit(commit) => app.tree.apply(commit, &mut app.engine).unwrap(),
                event => panic!("unexpected history event: {}", event_name(&event)),
            }
            assert_eq!(app.refresh(), 0);
        }
        let time = app.sample(start);
        if command == "prepend" {
            assert!(
                app.has_label(&first),
                "loading history must retain existing content"
            );
            let after = app.point(&first);
            assert!(
                (before.1 - after.1).abs() < 0.1,
                "older history must preserve the visible message anchor"
            );
        }
        assert!(app.host_nodes() < 500);
        if step >= warmup {
            times.push(time);
        }
    }
    let report = app.observe();
    assert_eq!(
        report["received"],
        if command == "append" {
            samples + warmup
        } else {
            0
        }
    );
    assert_eq!(
        report["ids"].as_array().unwrap().len(),
        (128 + if command == "append" {
            samples + warmup
        } else {
            0
        })
        .min(512)
    );
    json!({"name":format!("conversation-{command}"), "samples":times,
        "contracts":{"bounded_hosts":true, "history_retained":true, "received_count_verified":true}})
}

fn burst(app: &App, arrival: usize) {
    let batch = (0..4)
        .map(|offset| {
            json!({"id":format!("burst-{}", arrival * 4 + offset),
        "text":format!("Burst {}: See https://example.test/harbour", arrival * 4 + offset),
        "timestamp":1700020000000_u64 + (arrival * 4 + offset) as u64 * 60000, "author":"Alex"})
        })
        .collect::<Vec<_>>();
    app.send(json!({"type":"benchmark", "command":"append", "messages":batch}));
}

fn under_load(samples: usize, warmup: usize, arrival_ms: u64) -> Value {
    let mut app = conversation();
    let count = samples + warmup;
    let start = Instant::now();
    let input_interval = Duration::from_millis(10);
    let arrival_interval = Duration::from_millis(arrival_ms);
    let target_arrivals = (count as u64 * 10).div_ceil(arrival_ms) as usize;
    let received = target_arrivals * 4;
    let mut dispatches = Vec::new();
    let mut input_index = 0;
    let mut arrivals = 0;
    let mut acknowledged = 0;
    let mut times = Vec::new();
    let deadline = start + input_interval * count as u32 + TIMEOUT;
    while acknowledged < count {
        assert!(
            Instant::now() < deadline,
            "conversation backlog did not drain"
        );
        let now = Instant::now();
        if arrivals < target_arrivals && now >= start + arrival_interval * arrivals as u32 {
            burst(&app, arrivals);
            arrivals += 1;
        }
        if input_index < count && now >= start + input_interval * input_index as u32 {
            let dispatched = Instant::now();
            app.edit(&format!("Scheduled draft {input_index} 🙂"));
            dispatches.push((dispatched, dispatched.elapsed()));
            input_index += 1;
        }
        match app.events.recv_timeout(Duration::from_micros(200)) {
            Ok(Event::Commit(commit)) => {
                let ack = input_value(&commit)
                    .and_then(|value| value.strip_prefix("Scheduled draft "))
                    .and_then(|value| value.split(' ').next())
                    .map(|value| value.parse::<usize>().unwrap());
                app.tree.apply(commit, &mut app.engine).unwrap();
                app.refresh();
                let completed = Instant::now();
                if let Some(index) = ack {
                    while acknowledged <= index {
                        let intended = start + input_interval * acknowledged as u32;
                        let mut time = app.sample(intended);
                        time["elapsed_ns"] =
                            json!(completed.duration_since(intended).as_nanos() as u64);
                        time["dispatch_lateness_ns"] = json!(
                            dispatches[acknowledged]
                                .0
                                .duration_since(intended)
                                .as_nanos() as u64
                        );
                        time["dispatch_to_ack_ns"] = json!(
                            completed
                                .duration_since(dispatches[acknowledged].0)
                                .as_nanos() as u64
                        );
                        time["input_dispatch_ns"] =
                            json!(dispatches[acknowledged].1.as_nanos() as u64);
                        if acknowledged >= warmup {
                            times.push(time);
                        }
                        acknowledged += 1;
                    }
                }
                let context: Value =
                    serde_json::from_str(&app.engine.text_input_context()).unwrap();
                assert_eq!(
                    context["text"],
                    format!("Scheduled draft {} 🙂", input_index - 1),
                    "an older commit must not overwrite newer typing"
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Ok(Event::Ready) => {}
            result => panic!(
                "unexpected loaded input event: {}",
                result
                    .map(|event| event_name(&event))
                    .unwrap_or_else(|error| error.to_string())
            ),
        }
    }
    // Finish the authored arrival schedule independently of acknowledgement speed.
    while arrivals < target_arrivals {
        let due = start + arrival_interval * arrivals as u32;
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        burst(&app, arrivals);
        arrivals += 1;
    }
    app.wait_for(|app| {
        app.engine
            .scene()
            .text
            .iter()
            .any(|run| run.text.starts_with(&format!("Burst {}:", received - 1)))
    });
    let report = app.observe();
    assert_eq!(report["draft"], format!("Scheduled draft {} 🙂", count - 1));
    assert_eq!(report["received"], received);
    let expected_ids: Vec<Value> = (0..128)
        .map(|index| json!(format!("seed-{index}")))
        .chain((0..received).map(|index| json!(format!("burst-{index}"))))
        .skip((128 + received).saturating_sub(512))
        .collect();
    assert_eq!(
        report["ids"],
        json!(expected_ids),
        "the retained history must contain the exact newest messages, in order"
    );
    assert!(app.host_nodes() < 500);
    json!({"name":if arrival_ms == 5 { "conversation-input-under-load" } else { "conversation-input-background" }, "samples":times,
        "queue_high_water_bytes":app.runtime.queue_metrics().1,
        "contracts":{"input_interval_ms":10, "arrival_interval_ms":arrival_ms, "messages_per_arrival":4,
        "intended_arrival_to_ack":true, "all_arrivals_received":true, "newest_draft_preserved":true}})
}

fn playback(native_clock: bool, samples: usize, warmup: usize) -> Value {
    let mut app = App::new("playback");
    let nodes = app.host_nodes();
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        let bit = (step + 1) % 2;
        let start = Instant::now();
        if native_clock {
            assert!(app.engine.update_playback_clock(
                7,
                if bit == 1 { 35.0 } else { 9.0 },
                240.0,
                false,
                1.0
            ));
            assert!(app.engine.animate_scene());
        } else {
            app.send(
                json!({"type":"benchmark", "command":"state", "playing":bit == 1,
                "position":if bit == 1 { 35000 } else { 9000 }, "track":bit}),
            );
            app.wait_for(|app| {
                app.has_label(if bit == 1 {
                    "Northern Lines"
                } else {
                    "Harbour Lights"
                })
            });
        }
        let time = app.sample(start);
        assert!(app.has_label(if bit == 1 { "0:35" } else { "0:09" }));
        assert!(app.has_label("4:00"));
        assert_eq!(app.host_nodes(), nodes);
        if native_clock {
            assert_eq!(time["nodes_measured"], 0);
            assert!(!app.engine.has_scene_animations());
            assert!(
                app.events.try_recv().is_err(),
                "clock anchors must not cause JS commits"
            );
        }
        if step >= warmup {
            times.push(time);
        }
    }
    if native_clock {
        app.engine.take_perf_metrics();
        assert!(app.engine.update_playback_clock(7, 59.95, 240.0, true, 1.0));
        assert!(app.engine.has_scene_animations());
        std::thread::sleep(Duration::from_millis(80));
        assert!(app.engine.animate_scene());
        assert!(
            app.has_label("1:00"),
            "native progress must advance the elapsed label without JS"
        );
        assert!(app.engine.update_playback_clock(7, 60.0, 240.0, false, 1.0));
        app.engine.animate_scene();
        assert!(!app.engine.has_scene_animations());
        assert!(
            !app.engine.animate_scene(),
            "paused playback must stop animation work"
        );
        assert!(app.events.try_recv().is_err());
        assert_eq!(app.engine.take_perf_metrics().nodes_measured, 0);
    }
    json!({"name":if native_clock { "playback-native-clock" } else { "playback-state-change" }, "samples":times,
        "contracts":{"time_and_duration_verified":true, "controls_retained":true, "native_only":native_clock}})
}

pub(super) fn run(samples: usize, warmup: usize) -> Vec<Value> {
    vec![
        row_change("edit", 100, samples, warmup),
        row_change("edit", 1000, samples, warmup),
        row_change("prepend", 1000, samples, warmup),
        row_change("reorder", 1000, samples, warmup),
        row_scroll(false, samples, warmup),
        row_scroll(true, samples, warmup),
        image_arrival(samples, warmup),
        startup(samples, warmup),
        navigation("open", samples, warmup),
        navigation("back", samples, warmup),
        navigation("tab", samples, warmup),
        input(samples, warmup),
        conversation_change("append", samples, warmup),
        conversation_change("prepend", samples, warmup),
        under_load(samples, warmup, 50),
        under_load(samples, warmup, 5),
        playback(true, samples, warmup),
        playback(false, samples, warmup),
    ]
}
