use ink_core::{Engine, ReactTree};
use ink_protocol::{HostKind, Operation, ReactCommit};
use ink_runtime::{AppRuntime, Event, EventReceiver};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(10);

mod patterns;

struct App {
    runtime: AppRuntime,
    events: EventReceiver,
    tree: ReactTree,
    engine: Engine,
    native_views: usize,
    native_lists: usize,
}

impl App {
    fn new(fixture: &str) -> Self {
        let (source, icons) = Self::assets(fixture);
        let mut app = Self::start(source, &icons);
        app.engine.take_perf_metrics();
        app
    }

    fn assets(fixture: &str) -> (String, Vec<u8>) {
        let assets = match std::env::var_os("INK_PERF_ASSETS") {
            Some(root) => PathBuf::from(root).join(fixture),
            None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../fixtures/performance")
                .join(fixture)
                .join(".ink/android/assets"),
        };
        let source = std::fs::read_to_string(assets.join("app.js")).expect("prepared app.js");
        let icons = std::fs::read(assets.join("ink-icons-v1.bin")).expect("prepared icons");
        (source, icons)
    }

    fn start(source: String, icons: &[u8]) -> Self {
        let (runtime, events) = AppRuntime::spawn(source).unwrap();
        let mut engine = Engine::new();
        engine.set_viewport(1080, 1240);
        let mut app = Self {
            runtime,
            events,
            tree: ReactTree::with_icons(icons).unwrap(),
            engine,
            native_views: 0,
            native_lists: 0,
        };
        let mut ready = false;
        while !ready || app.engine.scene().text.is_empty() {
            match app.next() {
                Event::Commit(commit) => {
                    for op in &commit.0 {
                        if matches!(
                            op,
                            Operation::Create {
                                r#type: HostKind::NativeView,
                                ..
                            }
                        ) {
                            app.native_views += 1;
                        }
                        if matches!(
                            op,
                            Operation::Create {
                                r#type: HostKind::NativeList,
                                ..
                            }
                        ) {
                            app.native_lists += 1;
                        }
                    }
                    app.tree.apply(commit, &mut app.engine).unwrap();
                }
                Event::Message(message) => {
                    ready |= serde_json::from_str::<Value>(&message).unwrap()["type"] == "ready";
                }
                Event::Ready => {}
                event => panic!("unexpected startup event: {}", event_name(&event)),
            }
        }
        app
    }

    fn next(&self) -> Event {
        self.events
            .recv_timeout(TIMEOUT)
            .expect("runtime response timed out")
    }

    fn update(&mut self, payload: &str, inspect: bool) -> (Value, Value) {
        let start = Instant::now();
        self.runtime.send(payload.to_owned()).unwrap();
        let commit = loop {
            match self.next() {
                Event::Commit(commit) => break commit,
                Event::Ready => {}
                event => panic!("unexpected update event: {}", event_name(&event)),
            }
        };
        // Detailed bridge inspection runs only in untimed contract probes.
        let work = if inspect {
            commit_work(&commit)
        } else {
            Value::Null
        };
        let apply_start = Instant::now();
        self.tree.apply(commit, &mut self.engine).unwrap();
        let apply_ns = apply_start.elapsed().as_nanos() as u64;
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let core = self.engine.take_perf_metrics();
        (
            json!({ "elapsed_ns": elapsed_ns, "apply_ns": apply_ns,
            "nodes_measured": core.nodes_measured, "full_rebuilds": core.full_rebuilds,
            "incremental_rebuilds": core.incremental_rebuilds }),
            work,
        )
    }

    fn host_nodes(&self) -> u64 {
        self.tree.memory_diagnostics()["host_nodes"]
            .as_u64()
            .unwrap()
    }

    fn verify_cells(&self, command: &str, bit: usize) {
        let labels: Vec<_> = self
            .engine
            .scene()
            .text
            .iter()
            .filter(|run| run.text.as_bytes().get(3) == Some(&b':'))
            .collect();
        assert_eq!(labels.len(), 500, "all 500 cells must remain present");
        for (index, run) in labels.iter().enumerate() {
            let value = if command == "small" && index > 0 {
                0
            } else {
                bit
            };
            assert_eq!(run.text.as_str(), format!("{index:03}:{value}"));
            assert!(run.rect.width > 0.0 && run.rect.height > 0.0);
            assert!(
                run.rect.y >= run.clip.y
                    && run.rect.y + run.rect.height <= run.clip.y + run.clip.height
            );
            assert!(
                run.rect.x >= run.clip.x
                    && run.rect.x + run.rect.width <= run.clip.x + run.clip.width
            );
            if index > 0 {
                let previous = &labels[index - 1].rect;
                if index % 20 == 0 {
                    assert!(run.rect.y >= previous.y + previous.height);
                } else {
                    assert_eq!(run.rect.y, previous.y);
                    assert!(run.rect.x >= previous.x + previous.width);
                }
            }
        }
    }

    fn verify_rows(&self, count: usize, first_value: usize) {
        let mut rows = 0;
        for run in self
            .engine
            .scene()
            .text
            .iter()
            .filter(|run| run.text.starts_with("Row "))
        {
            let index: usize = run.text[4..9].parse().unwrap();
            assert!(index < count);
            let value = if index == 0 { first_value } else { 0 };
            assert_eq!(run.text.as_str(), format!("Row {index:05}: {value}"));
            rows += 1;
        }
        assert!(
            rows > 0 && rows < 100,
            "mounted rows must be bounded by this viewport"
        );
    }

    fn first_visible_row(&self) -> usize {
        let scene = self.engine.scene();
        let clip = scene.scroll_clip.unwrap();
        scene
            .text
            .iter()
            .filter(|run| run.text.starts_with("Row "))
            .filter_map(|run| {
                let centre =
                    run.rect.y + scene.scroll_origin - scene.scroll_offset + run.rect.height / 2.0;
                (centre >= clip.y && centre < clip.y + clip.height)
                    .then(|| run.text[4..9].parse().unwrap())
            })
            .min()
            .expect("viewport must contain a visible row")
    }
}

fn event_name(event: &Event) -> String {
    match event {
        Event::Error(error) => error.clone(),
        Event::Stopped => "stopped".into(),
        Event::Ready => "ready".into(),
        Event::Message(message) => message.clone(),
        Event::Commit(_) => "commit".into(),
    }
}

fn commit_work(commit: &ReactCommit) -> Value {
    let mut creates = 0;
    let mut removes = 0;
    let mut values = 0;
    let mut changed_rows = 0;
    let mut full_datasets = 0;
    for op in &commit.0 {
        match op {
            Operation::Create { .. } => creates += 1,
            Operation::Remove { .. } => removes += 1,
            Operation::Values {
                values: sources, ..
            } => values += sources.len(),
            Operation::Update { props, .. } => {
                changed_rows += props
                    .get("itemChanges")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                full_datasets +=
                    usize::from(props.contains_key("items") || props.contains_key("itemKeys"));
            }
            _ => {}
        }
    }
    json!({ "operations": commit.0.len(), "creates": creates, "removes": removes,
        "source_values": values, "changed_rows": changed_rows, "full_datasets": full_datasets,
        "serialised_commit_bytes": serde_json::to_vec(commit).unwrap().len() })
}

fn cells(fixture: &str, command: &str, samples: usize, warmup: usize) -> Value {
    let mut app = App::new(fixture);
    app.verify_cells(command, 0);
    assert_eq!(app.native_views, usize::from(fixture == "bindings"));
    let initial_nodes = app.host_nodes();
    let initial_width = app
        .engine
        .scene()
        .text
        .iter()
        .find(|run| run.text == "000:0")
        .unwrap()
        .rect
        .width;
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        let bit = (step + 1) % 2;
        let payload = json!({"type":"benchmark", "command":command, "value":bit}).to_string();
        let (time, _) = app.update(&payload, false);
        app.verify_cells(command, bit);
        if command != "resize" {
            assert_eq!(
                time["nodes_measured"], 0,
                "fixed-size text must not trigger layout"
            );
            assert_eq!(
                time["full_rebuilds"], 0,
                "fixed-size text must not rebuild the scene"
            );
        }
        assert_eq!(
            app.host_nodes(),
            initial_nodes,
            "updates must reuse the mounted controls"
        );
        let width = app
            .engine
            .scene()
            .text
            .iter()
            .find(|run| run.text.starts_with("000:"))
            .unwrap()
            .rect
            .width;
        let expected_width = initial_width
            * if command == "resize" && bit == 1 {
                14.0 / 12.0
            } else {
                1.0
            };
        assert!(
            (width - expected_width).abs() < 0.01,
            "cell width was not updated"
        );
        if step >= warmup {
            times.push(time);
        }
    }
    let mut contracts = Vec::new();
    for bit in [1, 0] {
        let payload = json!({"type":"benchmark", "command":command, "value":bit}).to_string();
        let (_, work) = app.update(&payload, true);
        app.verify_cells(command, bit);
        assert_eq!(work["creates"], 0);
        assert_eq!(work["removes"], 0);
        if fixture == "bindings" {
            assert_eq!(
                work["operations"], 1,
                "bound changes must use a single Values batch"
            );
            assert_eq!(
                work["source_values"],
                if command == "small" {
                    1
                } else if command == "resize" {
                    501
                } else {
                    500
                }
            );
        }
        contracts.push(work);
    }
    json!({"name":format!("{fixture}-{command}"), "samples":times, "contracts":contracts,
        "host_nodes":initial_nodes, "queue_high_water_bytes":app.runtime.queue_metrics().1})
}

fn list(count: usize) -> App {
    let mut app = App::new("list");
    app.update(
        &json!({"type":"benchmark", "command":"populate", "count":count}).to_string(),
        false,
    );
    // NativeList was initially mounted empty, and must remain on that path.
    assert_eq!(app.native_lists, 1);
    assert!(
        app.engine
            .scene()
            .text
            .iter()
            .any(|run| run.text == "Row 00000: 0")
    );
    assert!(
        app.tree
            .viewport_events(&mut app.engine)
            .unwrap()
            .is_empty()
    );
    app.engine.take_perf_metrics();
    app
}

fn list_edits(count: usize, samples: usize, warmup: usize) -> Value {
    let mut app = list(count);
    let initial_nodes = app.host_nodes();
    let mut times = Vec::new();
    for step in 0..warmup + samples {
        let bit = (step + 1) % 2;
        let (time, _) = app.update(
            &json!({"type":"benchmark", "command":"edit", "value":bit}).to_string(),
            false,
        );
        assert!(
            app.engine
                .scene()
                .text
                .iter()
                .any(|run| run.text == format!("Row 00000: {bit}"))
        );
        assert!(
            app.engine
                .scene()
                .text
                .iter()
                .any(|run| run.text == "Row 00001: 0")
        );
        assert_eq!(app.host_nodes(), initial_nodes);
        app.verify_rows(count, bit);
        if step >= warmup {
            times.push(time);
        }
    }
    let mut contracts = Vec::new();
    for bit in [1, 0] {
        let (_, work) = app.update(
            &json!({"type":"benchmark", "command":"edit", "value":bit}).to_string(),
            true,
        );
        assert_eq!(
            work["changed_rows"], 1,
            "single-row edits must transfer one row"
        );
        assert_eq!(
            work["full_datasets"], 0,
            "single-row edits must not transfer key order or all rows"
        );
        assert_eq!(work["creates"], 0);
        assert_eq!(work["removes"], 0);
        app.verify_rows(count, bit);
        contracts.push(work);
    }
    json!({"name":format!("list-edit-{count}"), "samples":times, "contracts":contracts,
        "host_nodes":initial_nodes, "queue_high_water_bytes":app.runtime.queue_metrics().1})
}

fn list_scroll(count: usize, samples: usize, warmup: usize) -> Value {
    let mut app = list(count);
    let mut times = Vec::new();
    let mut max_nodes = 0;
    let mut max_rows = 0;
    for step in 0..warmup + samples {
        let delta = if (step / 8) % 2 == 0 { 180.0 } else { -180.0 };
        let before = app.engine.scene().scroll_offset;
        let before_row = app.first_visible_row();
        let start = Instant::now();
        let changed = app.engine.scroll_by(delta);
        let events = app.tree.viewport_events(&mut app.engine).unwrap();
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let core = app.engine.take_perf_metrics();
        assert!(changed && before != app.engine.scene().scroll_offset);
        assert!(
            events.is_empty(),
            "native scrolling must not ask JS to materialise rows"
        );
        assert!(app.engine.list_viewports_ready());
        let rows = app
            .engine
            .scene()
            .text
            .iter()
            .filter(|run| run.text.starts_with("Row "))
            .count();
        max_rows = max_rows.max(rows);
        max_nodes = max_nodes.max(app.host_nodes());
        app.verify_rows(count, 0);
        let after_row = app.first_visible_row();
        assert!(
            if delta > 0.0 {
                after_row > before_row
            } else {
                after_row < before_row
            },
            "visible content must move with scrolling"
        );
        assert!(app.host_nodes() < 250, "offscreen host nodes leaked");
        assert!(
            app.events.try_recv().is_err(),
            "scrolling unexpectedly produced JS work"
        );
        if step >= warmup {
            times.push(json!({"elapsed_ns":elapsed_ns,
            "nodes_measured":core.nodes_measured, "full_rebuilds":core.full_rebuilds}));
        }
    }
    app.engine.scroll_by(f32::MAX);
    assert!(
        app.tree
            .viewport_events(&mut app.engine)
            .unwrap()
            .is_empty()
    );
    assert!(
        app.engine
            .scene()
            .text
            .iter()
            .any(|run| run.text == format!("Row {:05}: 0", count - 1))
    );
    app.engine.scroll_by(-f32::MAX);
    assert!(
        app.tree
            .viewport_events(&mut app.engine)
            .unwrap()
            .is_empty()
    );
    assert!(
        app.engine
            .scene()
            .text
            .iter()
            .any(|run| run.text == "Row 00000: 0")
    );
    json!({"name":format!("list-scroll-{count}"), "samples":times,
        "max_host_nodes":max_nodes, "max_mounted_rows":max_rows,
        "contracts":{"js_window_events":0, "js_commits":0, "verified_endpoints":true}})
}

fn scrollbar_drag(samples: usize, warmup: usize) -> Value {
    let mut app = list(10000);
    let thumb = |app: &App| {
        let scene = app.engine.scene();
        scene
            .scroll_bar
            .unwrap()
            .thumb_rect(scene.scroll_offset, scene.scroll_max)
    };
    let track = app.engine.scene().scroll_bar.unwrap().track;
    let initial = thumb(&app);
    assert_eq!(initial.y, track.y);
    let x = initial.x + initial.width / 2.0;
    let grab_offset = initial.height / 4.0;
    assert!(
        app.engine
            .pointer_down(1, x, initial.y + grab_offset)
            .captured
    );
    app.engine.take_perf_metrics();
    let fractions = [
        0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0, 0.875, 0.75, 0.625, 0.5, 0.375, 0.25,
        0.125, 0.0,
    ];
    let mut times = Vec::new();
    let mut previous_fraction = 0.0;
    let mut max_nodes = 0;
    for step in 0..warmup + samples {
        let fraction = fractions[step % fractions.len()];
        let expected_top = track.y + (track.height - initial.height) * fraction;
        let before_row = app.first_visible_row();
        let start = Instant::now();
        let outcome = app.engine.pointer_move(1, x, expected_top + grab_offset);
        let moved = std::hint::black_box(thumb(&app));
        let thumb_update_ns = start.elapsed().as_nanos() as u64;
        let events = app.tree.viewport_events(&mut app.engine).unwrap();
        let ready = app.engine.list_viewports_ready();
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let core = app.engine.take_perf_metrics();
        assert!(outcome.changed && outcome.captured && !outcome.activated);
        assert!(
            (moved.y - expected_top).abs() < 0.5,
            "thumb must follow the held pointer"
        );
        assert!(
            (thumb(&app).y - expected_top).abs() < 0.5,
            "materialising rows must not move the thumb away from the pointer"
        );
        assert!(ready, "dragged viewport must contain ready rows");
        assert!(
            events.is_empty(),
            "native thumb dragging must not request JS windows"
        );
        assert!(
            app.events.try_recv().is_err(),
            "native thumb dragging must not produce JS commits"
        );
        app.verify_rows(10000, 0);
        let after_row = app.first_visible_row();
        assert!(
            if fraction > previous_fraction {
                after_row > before_row
            } else {
                after_row < before_row
            },
            "content must move in the thumb's direction"
        );
        previous_fraction = fraction;
        max_nodes = max_nodes.max(app.host_nodes());
        assert!(max_nodes < 250, "dragging must retain a bounded row window");
        assert!(thumb_update_ns > 0 && thumb_update_ns <= elapsed_ns);
        if step >= warmup {
            times.push(
                json!({"elapsed_ns":elapsed_ns, "thumb_update_ns":thumb_update_ns,
                "nodes_measured":core.nodes_measured, "full_rebuilds":core.full_rebuilds,
                "incremental_rebuilds":core.incremental_rebuilds}),
            );
        }
    }
    // Check endpoint clamping outside the timed movements.
    for (y, label, bottom) in [
        (track.y + track.height * 2.0, "Row 09999: 0", true),
        (track.y - track.height, "Row 00000: 0", false),
    ] {
        assert!(app.engine.pointer_move(1, x, y).captured);
        assert!(
            app.tree
                .viewport_events(&mut app.engine)
                .unwrap()
                .is_empty()
        );
        assert!(app.engine.list_viewports_ready());
        let rect = thumb(&app);
        let edge = if bottom { rect.y + rect.height } else { rect.y };
        let expected_edge = if bottom {
            track.y + track.height
        } else {
            track.y
        };
        assert!(
            (edge - expected_edge).abs() < 0.5,
            "thumb must clamp at the track edge"
        );
        assert!(app.engine.scene().text.iter().any(|run| run.text == label));
        app.verify_rows(10000, 0);
        assert!(app.host_nodes() < 250);
    }
    let released = app.engine.pointer_up(1, x, track.y);
    assert!(released.captured && !released.activated);
    let offset = app.engine.scene().scroll_offset;
    assert!(
        !app.engine
            .pointer_move(1, x, track.y + track.height)
            .changed
    );
    assert_eq!(app.engine.scene().scroll_offset, offset);
    assert!(app.events.try_recv().is_err());
    json!({"name":"scrollbar-drag-10000", "samples":times, "max_host_nodes":max_nodes,
        "contracts":{"rows":10000, "held_pointer":true, "grab_offset_preserved":true,
            "verified_endpoints":true, "release_stops_drag":true, "js_window_events":0, "js_commits":0}})
}

fn idle() -> Value {
    let mut app = App::new("bindings");
    app.verify_cells("small", 0);
    let revision = app.engine.scene().revision;
    let mut probes = 0;
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(250) {
        assert!(!app.engine.has_scene_animations());
        assert!(!app.engine.animate_scene());
        assert!(
            app.tree
                .viewport_events(&mut app.engine)
                .unwrap()
                .is_empty()
        );
        assert_eq!(app.engine.scene().revision, revision);
        match app.events.recv_timeout(Duration::from_millis(10)) {
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            result => panic!(
                "settled runtime produced work: {}",
                result
                    .map(|event| event_name(&event))
                    .unwrap_or_else(|error| error.to_string())
            ),
        }
        probes += 1;
    }
    let core = app.engine.take_perf_metrics();
    assert_eq!(core.nodes_measured, 0);
    assert_eq!(core.full_rebuilds, 0);
    assert_eq!(app.runtime.queue_metrics().0, 0);
    json!({"name":"idle", "observation_ns":start.elapsed().as_nanos() as u64,
        "probes":probes, "commits":0, "nodes_measured":0, "scene_changes":0})
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: ink-performance SAMPLES WARMUP");
    let samples: usize = args[1].parse().unwrap();
    let warmup: usize = args[2].parse().unwrap();
    assert!(samples > 0 && samples.is_multiple_of(2) && warmup.is_multiple_of(2));
    let mut workloads = Vec::new();
    for fixture in ["bindings", "react"] {
        for command in ["small", "bulk", "resize"] {
            workloads.push(cells(fixture, command, samples, warmup));
        }
    }
    for count in [100, 1000, 10000] {
        workloads.push(list_edits(count, samples, warmup));
        workloads.push(list_scroll(count, samples, warmup));
    }
    workloads.push(scrollbar_drag(samples, warmup));
    workloads.extend(patterns::run(samples, warmup));
    println!(
        "{}",
        json!({"version":1, "workloads":workloads, "idle":idle()})
    );
}
