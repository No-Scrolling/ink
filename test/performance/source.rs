//! The same compiled fixture runs against the array baseline and the native source runtime.
use ink_core::{Engine, ReactTree};
use ink_protocol::{Operation, ReactCommit};
use ink_runtime::{AppRuntime, Event, EventReceiver};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const WIDTH: u32 = 1080;
const HEIGHT: u32 = 1240;
const TIMEOUT: Duration = Duration::from_secs(30);

struct App {
    runtime: AppRuntime,
    events: EventReceiver,
    tree: ReactTree,
    engine: Engine,
    native: Option<bool>,
    contracts: Option<Value>,
}

#[derive(Default)]
struct Observation {
    commits: usize,
    apply_ns: u64,
    diagnostics: Vec<Value>,
}

struct Expected {
    count: usize,
    sequence: usize,
    generation: usize,
    edited: bool,
    reversed: bool,
    selected: String,
}

impl Expected {
    fn new(count: usize) -> Self {
        Self {
            count,
            sequence: 0,
            generation: 0,
            edited: false,
            reversed: false,
            selected: "none".into(),
        }
    }

    fn change(&mut self, command: &str) {
        self.sequence += 1;
        match command {
            "edit" => self.edited = true,
            "replace" => {
                self.generation = self.sequence;
                self.edited = false;
                self.reversed = false;
            }
            _ => panic!("unknown measured command: {command}"),
        }
    }

    fn subtitle(&self, key: usize) -> String {
        if key == 0 && self.edited {
            format!("Edit {}", self.sequence)
        } else if self.generation > 0 {
            format!("Refresh {}", self.generation)
        } else {
            "Original".into()
        }
    }

    fn unread(&self) -> usize {
        self.count.div_ceil(2) - usize::from(self.edited && self.sequence % 2 == 1)
    }

    fn first(&self) -> usize {
        if self.reversed { self.count - 1 } else { 0 }
    }
}

impl App {
    fn start() -> (Self, Value) {
        let assets = std::env::var_os("INK_PERF_ASSETS").map_or_else(
            || {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../fixtures/performance/source/.ink/android/assets")
            },
            |root| PathBuf::from(root).join("source"),
        );
        let source =
            std::fs::read_to_string(assets.join("app.js")).expect("prepared source app.js");
        let icons = std::fs::read(assets.join("ink-icons-v1.bin")).expect("prepared source icons");
        // Disk reads are excluded; runtime, engine and icon initialisation are included.
        let start = Instant::now();
        let (runtime, events) = AppRuntime::spawn(source).unwrap();
        let mut engine = Engine::new();
        engine.set_viewport(WIDTH, HEIGHT);
        let mut app = Self {
            runtime,
            events,
            tree: ReactTree::with_icons(&icons).unwrap(),
            engine,
            native: None,
            contracts: None,
        };
        let expected = Expected::new(1000);
        let mut observation = Observation::default();
        app.wait_for(&mut observation, false, |app| {
            app.native.is_some() && app.ready(&expected)
        });
        let startup = app.sample(start, &observation);
        if let Ok(expected) = std::env::var("INK_SOURCE_EXPECT_NATIVE") {
            assert_eq!(
                app.native,
                Some(expected.parse().unwrap()),
                "wrong fixture path"
            );
        }
        app.verify(&expected);
        (app, startup)
    }

    fn send(&self, command: &str, count: Option<usize>) {
        self.runtime
            .send(json!({"type":"benchmark", "command":command, "count":count}).to_string())
            .unwrap();
    }

    fn refresh(&mut self) -> usize {
        let events = self.tree.viewport_events(&mut self.engine).unwrap();
        let count = events.len();
        for event in events {
            self.runtime.send(event.to_string()).unwrap();
        }
        count
    }

    fn receive(&mut self, event: Event, observation: &mut Observation, inspect: bool) {
        match event {
            Event::Commit(commit) => {
                if inspect {
                    observation.diagnostics.push(commit_work(&commit));
                }
                observation.commits += 1;
                let start = Instant::now();
                self.tree.apply(commit, &mut self.engine).unwrap();
                observation.apply_ns += start.elapsed().as_nanos() as u64;
            }
            Event::Message(message) => {
                let message: Value = serde_json::from_str(&message).expect("fixture JSON message");
                match message["type"].as_str() {
                    Some("ready") => {
                        self.native = Some(message["native"].as_bool().expect("fixture path"));
                    }
                    Some("contracts") => self.contracts = Some(message),
                    _ => panic!("unexpected fixture message: {message}"),
                }
            }
            Event::Ready => {}
            Event::Error(error) => panic!("source fixture failed: {error}"),
            Event::Stopped => panic!("source fixture stopped before scene readiness"),
        }
    }

    fn wait_for(
        &mut self,
        observation: &mut Observation,
        inspect: bool,
        ready: impl Fn(&Self) -> bool,
    ) {
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
                        "source scene readiness timed out: {error}; labels: {:?}",
                        self.engine
                            .scene()
                            .text
                            .iter()
                            .map(|run| run.text.as_str())
                            .collect::<Vec<_>>()
                    )
                });
            self.receive(event, observation, inspect);
        }
    }

    fn has_label(&self, label: &str) -> bool {
        self.engine.scene().text.iter().any(|run| run.text == label)
    }

    fn row_has_subtitle(&self, key: usize, subtitle: &str) -> bool {
        let scene = self.engine.scene();
        let title = format!("Message {key:05}");
        let Some(index) = scene.text.iter().position(|run| run.text == title) else {
            return false;
        };
        scene.text[index + 1..]
            .iter()
            .take_while(|run| !run.text.starts_with("Message "))
            .any(|run| run.text == subtitle)
    }

    fn ready(&self, expected: &Expected) -> bool {
        // Source data and React's useSnapshot count can arrive in separate commits.
        self.row_has_subtitle(expected.first(), &expected.subtitle(expected.first()))
            && self.has_label(&format!("Unread: {}", expected.unread()))
            && self.has_label(&format!("Inbox {}", expected.selected))
    }

    fn sample(&mut self, start: Instant, observation: &Observation) -> Value {
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let core = self.engine.take_perf_metrics();
        json!({"elapsed_ns":elapsed_ns, "apply_ns":observation.apply_ns,
            "commits":observation.commits, "nodes_measured":core.nodes_measured,
            "full_rebuilds":core.full_rebuilds, "incremental_rebuilds":core.incremental_rebuilds})
    }

    fn change(
        &mut self,
        command: &str,
        expected: &mut Expected,
        inspect: bool,
    ) -> (Value, Vec<Value>) {
        expected.change(command);
        let payload = json!({"type":"benchmark", "command":command}).to_string();
        self.engine.take_perf_metrics();
        let mut observation = Observation::default();
        let start = Instant::now();
        self.runtime.send(payload).unwrap();
        self.wait_for(&mut observation, inspect, |app| app.ready(expected));
        let sample = self.sample(start, &observation);
        assert!(
            observation.commits > 0,
            "a fresh subtitle requires an applied commit"
        );
        self.verify(expected);
        (sample, observation.diagnostics)
    }

    fn populate(&mut self, count: usize, selected: &str) -> (Expected, Value) {
        let mut expected = Expected::new(count);
        expected.selected = selected.into();
        let payload = json!({"type":"benchmark", "command":"populate", "count":count}).to_string();
        self.engine.take_perf_metrics();
        let mut observation = Observation::default();
        let start = Instant::now();
        self.runtime.send(payload).unwrap();
        self.wait_for(&mut observation, false, |app| app.ready(&expected));
        let sample = self.sample(start, &observation);
        assert!(observation.commits > 0);
        self.verify(&expected);
        (expected, sample)
    }

    fn verify(&self, expected: &Expected) {
        assert!(
            self.ready(expected),
            "both the row subtitle and unread count must be current"
        );
        let scene = self.engine.scene();
        let rows: Vec<_> = scene
            .text
            .iter()
            .filter_map(|run| run.text.strip_prefix("Message ")?.parse::<usize>().ok())
            .collect();
        assert!(
            !rows.is_empty() && rows.len() < 100,
            "native row windows must be bounded"
        );
        assert_eq!(
            rows[0],
            expected.first(),
            "the full dataset order must reach the scene"
        );
        assert!(rows.iter().all(|key| *key < expected.count));
        assert!(rows.windows(2).all(|pair| if expected.reversed {
            pair[0] == pair[1] + 1
        } else {
            pair[1] == pair[0] + 1
        }));
        // The last row can be cut off before its subtitle enters the scene.
        for key in rows.iter().take(rows.len().saturating_sub(1)) {
            assert!(
                self.row_has_subtitle(*key, &expected.subtitle(*key)),
                "stale subtitle for key {key}"
            );
        }
        assert!(self.engine.list_viewports_ready());
        assert!(
            self.tree.memory_diagnostics()["host_nodes"]
                .as_u64()
                .unwrap()
                < 500,
            "host nodes must remain bounded independently of collection size"
        );
        assert!(
            scene.scroll_max > HEIGHT as f32,
            "the collection must extend beyond its native window"
        );
    }

    fn press(&mut self, key: usize, expected: &mut Expected) {
        let title = format!("Message {key:05}");
        let scene = self.engine.scene();
        let run = scene
            .text
            .iter()
            .find(|run| run.text == title)
            .expect("mounted row title");
        let y = run.rect.y + scene.scroll_origin - scene.scroll_offset + run.rect.height / 2.0;
        let clip = scene.scroll_clip.expect("native list scroll clip");
        assert!(
            y >= clip.y && y < clip.y + clip.height,
            "tap must hit a visible native row"
        );
        let x = run.rect.x + run.rect.width / 2.0;
        let selected = expected.subtitle(key);
        assert_ne!(
            expected.selected, selected,
            "tap acknowledgement must change the screen title"
        );
        assert!(self.engine.tap(x, y));
        let request = self
            .engine
            .take_native_request()
            .expect("native row press request");
        assert_eq!(request.module(), "ink");
        let event = self
            .tree
            .route_native_message(request.payload().to_owned())
            .unwrap();
        self.runtime.send(event).unwrap();
        expected.selected = selected;
        self.wait_for(&mut Observation::default(), false, |app| {
            app.ready(expected)
        });
        self.verify(expected);
    }

    fn reorder(&mut self, expected: &mut Expected) {
        expected.reversed = !expected.reversed;
        self.send("reorder", None);
        // Reversal preserves the old visible key as an anchor. Apply the change
        // before returning to the top so the first row and controls are visible.
        let deadline = Instant::now() + TIMEOUT;
        let mut observation = Observation::default();
        while observation.commits == 0 {
            let event = self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("reorder commit");
            self.receive(event, &mut observation, false);
        }
        self.engine.scroll_by(-f32::MAX);
        self.refresh();
        self.wait_for(&mut Observation::default(), false, |app| {
            app.ready(expected)
        });
        self.verify(expected);
    }

    fn check_contracts(&mut self, expected: &Expected) -> Value {
        self.contracts = None;
        self.send("contracts", None);
        let mut observation = Observation::default();
        self.wait_for(&mut observation, true, |app| {
            app.contracts.is_some() && app.ready(expected)
        });
        let report = self.contracts.take().unwrap();
        assert_eq!(report["passed"], true);
        if self.native == Some(true) {
            assert_eq!(report["retained"], true);
            assert_eq!(report["atomic"], true);
            assert_eq!(report["count"].as_u64(), Some(expected.unread() as u64));
        } else {
            assert_eq!(report["native"], false);
        }
        assert_eq!(
            observation.commits, 0,
            "contract reads and rejected writes must not publish scene work"
        );
        self.verify(expected);
        report
    }
}

fn commit_work(commit: &ReactCommit) -> Value {
    let mut creates = 0;
    let mut removes = 0;
    let mut changed_rows = 0;
    let mut full_datasets = 0;
    let mut other_operations = 0;
    for operation in &commit.0 {
        match operation {
            Operation::Create { .. } => creates += 1,
            Operation::Remove { .. } => removes += 1,
            Operation::Update { props, .. } => {
                changed_rows += props
                    .get("itemChanges")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                full_datasets +=
                    usize::from(props.contains_key("items") || props.contains_key("itemKeys"));
            }
            Operation::Insert { .. } | Operation::Text { .. } | Operation::Hidden { .. } => {}
            // Keep this harness compilable against the archived protocol.
            #[allow(unreachable_patterns)]
            _ => other_operations += 1,
        }
    }
    json!({"operations":commit.0.len(), "creates":creates, "removes":removes,
        "changed_rows":changed_rows, "full_datasets":full_datasets,
        "other_operations":other_operations,
        "serialised_diagnostic_bytes":serde_json::to_vec(commit).unwrap().len()})
}

fn workload(command: &str, count: usize, samples: usize, warmup: usize, render: bool) -> Value {
    let (mut app, startup) = App::start();
    // A changed row/count makes even a same-size populate observable before timing ends.
    let mut initial = Expected::new(1000);
    app.change("edit", &mut initial, false);
    let (mut expected, populate) = app.populate(count, "none");
    app.press(0, &mut expected);
    app.change("edit", &mut expected, false);
    app.press(0, &mut expected);
    app.reorder(&mut expected);
    app.press(count - 1, &mut expected);
    app.reorder(&mut expected);
    app.press(0, &mut expected);
    app.change("replace", &mut expected, false);
    app.press(0, &mut expected);
    let before = app.check_contracts(&expected);
    // Restore authored order, source sequence and retained snapshot before warm-up.
    let (mut expected, _) = app.populate(count, &expected.selected);
    let mut times = Vec::with_capacity(samples);
    let mut max_host_nodes = 0;
    for step in 0..warmup + samples {
        let (sample, _) = app.change(command, &mut expected, false);
        max_host_nodes = max_host_nodes.max(
            app.tree.memory_diagnostics()["host_nodes"]
                .as_u64()
                .unwrap(),
        );
        if step >= warmup {
            times.push(sample);
        }
    }
    // Serialisation and operation inspection occur only in separate, untimed probes.
    let mut diagnostics = Vec::new();
    for _ in 0..2 {
        let (_, commits) = app.change(command, &mut expected, true);
        diagnostics.push(json!({"sequence":expected.sequence, "commits":commits}));
    }
    let after = app.check_contracts(&expected);
    let memory = app.tree.memory_diagnostics();
    let window_rows = app
        .engine
        .scene()
        .text
        .iter()
        .filter(|run| run.text.starts_with("Message "))
        .count();
    let mut rendered = Value::Null;
    if render
        && let Some(directory) =
            std::env::var_os("INK_SOURCE_RENDER_DIR").filter(|value| !value.is_empty())
    {
        use ink_renderer_vulkan::{RenderOutcome, Renderer};
        let path = PathBuf::from(directory).join("final.png");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).expect("optional offscreen renderer");
        assert!(matches!(
            renderer.render(app.engine.scene()).unwrap(),
            RenderOutcome::Presented
        ));
        let pixels = renderer.pixels().unwrap();
        image::save_buffer_with_format(
            &path,
            &pixels,
            WIDTH,
            HEIGHT,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .unwrap();
        rendered = json!({"path":path, "width":WIDTH, "height":HEIGHT,
            "selected":expected.selected, "unread":expected.unread(), "row_0_subtitle":expected.subtitle(0)});
    }
    json!({"name":format!("source-{command}-{count}"), "native":app.native.unwrap(),
        "count":count, "samples":times, "setup":{"startup":startup, "populate":populate},
        "diagnostics":diagnostics, "memory":memory, "max_host_nodes":max_host_nodes,
        "window_rows":window_rows, "queue_high_water_bytes":app.runtime.queue_metrics().1,
        "contracts":{"before":before, "after":after, "native_row_taps":true,
            "full_reorder_and_restore":true, "bounded_host_nodes_and_window":true}, "rendered":rendered})
}

fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(
        arguments.len(),
        3,
        "usage: ink-source-performance SAMPLES WARMUP COUNTS_COMMA_SEPARATED"
    );
    let samples: usize = arguments[0].parse().unwrap();
    let warmup: usize = arguments[1].parse().unwrap();
    let counts: Vec<usize> = arguments[2]
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect();
    assert!(samples > 0 && samples.is_multiple_of(2) && warmup.is_multiple_of(2));
    assert!(
        !counts.is_empty()
            && counts
                .iter()
                .all(|count| [1000, 5000, 100000].contains(count))
    );
    if counts.contains(&100000) {
        eprintln!(
            "100,000 rows are exploratory: the array baseline can exceed the 1 MiB commit input limit, so this size cannot provide the main comparison."
        );
    }
    let mut workloads = Vec::new();
    for (index, count) in counts.iter().enumerate() {
        for command in ["edit", "replace"] {
            workloads.push(workload(
                command,
                *count,
                samples,
                warmup,
                index == counts.len() - 1 && command == "replace",
            ));
        }
    }
    println!(
        "{}",
        json!({"version":1, "workloads":workloads,
        "timing":"dispatch until subtitle and unread count are applied to a ready native scene",
        "diagnostic_bytes":"separate untimed serialisation probes; not measured transport bytes"})
    );
}
