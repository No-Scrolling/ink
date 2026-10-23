//! Paired measurements of the unchanged public List API, including input during refresh.
use ink_core::{Engine, ReactTree, TextEdit};
use ink_protocol::{Operation, ReactCommit};
use ink_runtime::{AppRuntime, Event, EventReceiver};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(30);
const WIDTH: u32 = 1080;
const HEIGHT: u32 = 1240;

struct App {
    runtime: AppRuntime,
    events: EventReceiver,
    tree: ReactTree,
    engine: Engine,
    started: bool,
    report: Option<Value>,
}

impl App {
    fn start() -> Self {
        let assets = std::env::var_os("INK_AUTOMATIC_ASSETS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../fixtures/performance/automatic/.ink/android/assets")
            });
        let source =
            std::fs::read_to_string(assets.join("app.js")).expect("prepared automatic app.js");
        let icons = std::fs::read(assets.join("ink-icons-v1.bin")).expect("prepared icons");
        let (runtime, events) = AppRuntime::spawn(source).unwrap();
        let mut engine = Engine::new();
        engine.set_viewport(WIDTH, HEIGHT);
        let mut app = Self {
            runtime,
            events,
            tree: ReactTree::with_icons(&icons).unwrap(),
            engine,
            started: false,
            report: None,
        };
        app.wait(|app| app.started && app.row(0, 0));
        app
    }

    fn send(&self, mut message: Value) {
        message["type"] = json!("benchmark");
        self.runtime.send(message.to_string()).unwrap();
    }

    fn refresh(&mut self) -> usize {
        let events = self.tree.viewport_events(&mut self.engine).unwrap();
        let count = events.len();
        for event in events {
            self.runtime.send(event.to_string()).unwrap();
        }
        count
    }

    fn receive(&mut self, event: Event) -> Option<String> {
        match event {
            Event::Commit(commit) => {
                let value = input_value(&commit);
                self.tree.apply(commit, &mut self.engine).unwrap();
                value
            }
            Event::Message(message) => {
                let message: Value = serde_json::from_str(&message).unwrap();
                match message["type"].as_str() {
                    Some("ready") => self.started = true,
                    Some("observed") => self.report = Some(message),
                    _ => panic!("unexpected fixture message: {message}"),
                }
                None
            }
            Event::Ready => None,
            Event::Error(error) => panic!("ordinary List fixture failed: {error}"),
            Event::Stopped => panic!("ordinary List fixture stopped"),
        }
    }

    fn wait(&mut self, ready: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if self.refresh() == 0 && self.engine.list_viewports_ready() && ready(self) {
                return;
            }
            let event = self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|error| {
                    panic!(
                        "scene timed out: {error}; labels: {:?}",
                        self.engine
                            .scene()
                            .text
                            .iter()
                            .map(|run| run.text.as_str())
                            .collect::<Vec<_>>()
                    )
                });
            self.receive(event);
        }
    }

    fn label(&self, label: &str) -> bool {
        self.engine.scene().text.iter().any(|run| run.text == label)
    }

    fn row(&self, key: usize, generation: usize) -> bool {
        let scene = self.engine.scene();
        let title = format!("Item {key:05}");
        let Some(index) = scene.text.iter().position(|run| run.text == title) else {
            return false;
        };
        scene.text[index + 1..]
            .iter()
            .take_while(|run| !run.text.starts_with("Item "))
            .any(|run| run.text == format!("Generation {generation}"))
    }

    fn populate(&mut self, count: usize) {
        // Force an observable reset even for the fixture's initial size.
        self.change("edit", 999999);
        self.send(json!({"command":"populate", "count":count}));
        self.wait(|app| app.row(0, 0));
        let report = self.report();
        assert_eq!(report["count"], count);
        assert_eq!(report["first"], "0");
        assert_eq!(report["last"], (count - 1).to_string());
    }

    fn report(&mut self) -> Value {
        self.report = None;
        self.send(json!({"command":"report"}));
        self.wait(|app| app.report.is_some());
        self.report.take().unwrap()
    }

    fn change(&mut self, command: &str, generation: usize) -> Value {
        self.engine.take_perf_metrics();
        let start = Instant::now();
        self.send(json!({"command":command,"generation":generation}));
        self.wait(|app| app.row(0, generation));
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let core = self.engine.take_perf_metrics();
        self.verify_window();
        json!({"elapsed_ns":elapsed_ns,"nodes_measured":core.nodes_measured,
            "full_rebuilds":core.full_rebuilds,"incremental_rebuilds":core.incremental_rebuilds})
    }

    fn verify_window(&self) {
        let rows = self
            .engine
            .scene()
            .text
            .iter()
            .filter(|run| run.text.starts_with("Item "))
            .count();
        assert!(rows > 0 && rows < 100, "bounded native row window");
        assert!(
            self.tree.memory_diagnostics()["host_nodes"]
                .as_u64()
                .unwrap()
                < 500
        );
        assert!(self.engine.list_viewports_ready());
    }

    fn type_text(&mut self, text: &str) {
        let context: Value = serde_json::from_str(&self.engine.text_input_context()).unwrap();
        assert!(!context["id"].is_null(), "input must have focus");
        self.engine.edit_text(TextEdit::Update(
            json!({"id":context["id"],"text":context["text"],
            "value":text,"selection":text.encode_utf16().count()})
            .to_string(),
        ));
        let events = self.tree.input_events(&self.engine);
        assert_eq!(events.len(), 1);
        self.runtime.send(events[0].to_string()).unwrap();
    }

    fn press(&mut self, key: usize, index: usize, generation: usize) {
        let title = format!("Item {key:05}");
        let scene = self.engine.scene();
        let run = scene
            .text
            .iter()
            .find(|run| run.text == title)
            .expect("visible row");
        let y = run.rect.y
            + if run.scrolling {
                scene.scroll_origin - scene.scroll_offset
            } else {
                0.0
            }
            + run.rect.height / 2.0;
        assert!(self.engine.tap(run.rect.x + run.rect.width / 2.0, y));
        let request = self.engine.take_native_request().expect("native row press");
        let event = self
            .tree
            .route_native_message(request.payload().to_owned())
            .unwrap();
        self.runtime.send(event).unwrap();
        self.wait(|app| app.label(&format!("Lists {key}:{index}:Generation {generation}")));
    }
}

fn input_value(commit: &ReactCommit) -> Option<String> {
    commit.0.iter().find_map(|op| match op {
        Operation::Update { props, .. } => props
            .get("value")
            .and_then(Value::as_str)
            .map(str::to_owned),
        _ => None,
    })
}

fn workload(count: usize, command: &str, samples: usize, warmup: usize) -> Value {
    let mut app = App::start();
    app.populate(count);
    let mut times = Vec::new();
    for index in 0..samples + warmup {
        let sample = app.change(command, index + 1);
        if index >= warmup {
            times.push(sample);
        }
    }
    if command == "edit" {
        assert!(app.row(1, 0), "sparse edit preserves neighbour");
    } else {
        assert!(
            app.row(1, samples + warmup),
            "dense replacement reaches neighbour"
        );
    }
    json!({"name":format!("ordinary-{command}-{count}"),"count":count,"samples":times,
        "memory":app.tree.memory_diagnostics(),"queue_high_water_bytes":app.runtime.queue_metrics().1})
}

fn typing(count: usize, samples: usize, warmup: usize) -> Value {
    const INPUTS: usize = 16;
    let interval = Duration::from_millis(2);
    let mut app = App::start();
    app.populate(count);
    let mut jobs = Vec::new();
    for job in 0..samples + warmup {
        let generation = 10000 + job;
        let start = Instant::now();
        app.send(json!({"command":"replace","generation":generation}));
        let mut sent = 0;
        let mut acknowledged = 0;
        let mut dispatch = Vec::new();
        let mut arrivals = Vec::new();
        let mut ready_ns = None;
        let mut acknowledgements = Vec::new();
        let deadline = start + TIMEOUT;
        loop {
            let now = Instant::now();
            assert!(now < deadline, "typing acknowledgement timed out");
            while sent < INPUTS && now >= start + interval * (sent as u32 + 1) {
                let intended = start + interval * (sent as u32 + 1);
                let value = format!("Draft {job}-{sent} 🙂");
                let dispatched = Instant::now();
                app.type_text(&value);
                dispatch.push(dispatched);
                arrivals.push(intended);
                sent += 1;
            }
            let outstanding = app.refresh();
            if ready_ns.is_none()
                && outstanding == 0
                && app.engine.list_viewports_ready()
                && app.row(0, generation)
            {
                ready_ns = Some(start.elapsed().as_nanos() as u64);
            }
            if sent == INPUTS && acknowledged == INPUTS && ready_ns.is_some() {
                break;
            }
            let next = if sent < INPUTS {
                start + interval * (sent as u32 + 1)
            } else {
                deadline
            };
            match app
                .events
                .recv_timeout(next.saturating_duration_since(Instant::now()))
            {
                Ok(event) => {
                    if let Some(value) = app.receive(event) {
                        let prefix = format!("Draft {job}-");
                        if let Some(index) = value
                            .strip_prefix(&prefix)
                            .and_then(|value| value.split(' ').next())
                            .and_then(|value| value.parse::<usize>().ok())
                        {
                            let at = Instant::now();
                            assert!(index < sent);
                            for input in acknowledged..=index {
                                acknowledgements.push(json!({"input":input,
                                    "arrival_to_ack_ns":at.duration_since(arrivals[input]).as_nanos() as u64,
                                    "dispatch_to_ack_ns":at.duration_since(dispatch[input]).as_nanos() as u64,
                                    "dispatch_lateness_ns":dispatch[input].duration_since(arrivals[input]).as_nanos() as u64,
                                    "before_list_ready":ready_ns.is_none()}));
                            }
                            acknowledged = acknowledged.max(index + 1);
                        }
                    }
                    let context: Value =
                        serde_json::from_str(&app.engine.text_input_context()).unwrap();
                    if sent > 0 {
                        assert_eq!(
                            context["text"],
                            format!("Draft {job}-{} 🙂", sent - 1),
                            "older JS acknowledgement must preserve latest native draft"
                        );
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => panic!("typing event stream failed: {error}"),
            }
        }
        let report = app.report();
        assert_eq!(report["draft"], format!("Draft {job}-{} 🙂", INPUTS - 1));
        assert_eq!(report["generation"], generation);
        app.verify_window();
        if job >= warmup {
            jobs.push(json!({"list_ready_ns":ready_ns.unwrap(),"inputs":acknowledgements}));
        }
    }
    json!({"name":format!("typing-during-refresh-{count}"),"count":count,"input_interval_ms":2,"jobs":jobs})
}

fn contracts(count: usize) -> Value {
    let mut app = App::start();
    app.populate(count);
    app.send(json!({"command":"replace","generation":60000}));
    app.wait(|app| app.label("Requested 60000"));
    // The displayed rows may still be old; events use the committed request.
    app.press(0, 0, 60000);
    app.wait(|app| app.row(0, 60000));
    app.send(json!({"command":"replace","generation":60001}));
    std::thread::sleep(Duration::from_millis(2));
    app.send(json!({"command":"replace","generation":60002}));
    app.wait(|app| app.row(0, 60002));
    app.send(json!({"command":"shown","value":false}));
    app.wait(|app| !app.label("Item 00000"));
    app.send(json!({"command":"replace","generation":60003}));
    app.send(json!({"command":"shown","value":true}));
    app.wait(|app| app.row(0, 60003));
    app.press(0, 0, 60003);
    let report = app.report();
    assert_eq!(report["selected"], "0:0:Generation 60003");
    app.verify_window();
    json!({"latest_requested_callback":true,"supersession":true,"unmount_remount":true,
        "bounded_native_window":true,"report":report})
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        3,
        "usage: ink-automatic-performance SAMPLES WARMUP COUNTS_COMMA_SEPARATED"
    );
    let samples: usize = args[0].parse().unwrap();
    let warmup: usize = args[1].parse().unwrap();
    let counts: Vec<usize> = args[2]
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect();
    assert!(samples > 0 && counts.iter().all(|count| [1000, 5000].contains(count)));
    let mut workloads = Vec::new();
    for count in &counts {
        for command in ["edit", "replace"] {
            workloads.push(workload(*count, command, samples, warmup));
        }
    }
    let count = *counts.iter().max().unwrap();
    workloads.push(typing(count, samples.min(8), warmup.min(2)));
    println!(
        "{}",
        json!({"workloads":workloads,"contracts":contracts(count)})
    );
}
