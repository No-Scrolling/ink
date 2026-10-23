//! Host journeys through final NativeList publication. Run this binary with an optimised profile.
use ink_core::{Engine, ReactTree, TextEdit};
use ink_data::{Config, DataHost};
use ink_protocol::{HostKind, Operation, ReactCommit};
use ink_runtime::{AppRuntime, Event, EventReceiver, NativeHost, NativeSession, Script};
use rusqlite::{Connection, OpenFlags, params};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TIMEOUT: Duration = Duration::from_secs(30);
const MESSAGE_LIMIT: usize = 1024 * 1024;
const CATALOGUE_ROWS: usize = 10_000;
const SEARCH_OBSERVERS: &str = r#"
(() => {
  const warm = globalThis.__inkPrepareDataSearch;
  const search = globalThis.__inkDataSearch;
  if (typeof warm !== "function" || typeof search !== "function") throw Error("Native data-search primitives unavailable");
  globalThis.__inkJourneyPrepareDataSearch = (...args) => {
    const cursor = warm(...args);
    if (cursor && cursor.complete) __inkPost(JSON.stringify({type:"local-cache-ready",rows:args[0].length}));
    return cursor;
  };
  globalThis.__inkJourneyDataSearch = (...args) => {
    const result = search(...args);
    if (result) __inkPost(JSON.stringify({type:"local-cache-used"}));
    return result;
  };
})();
"#;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ink-journeys-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(path.join("assets/ink-assets")).unwrap();
        Self(path)
    }

    fn catalogue(&self) {
        let mut connection =
            Connection::open(self.0.join("assets/ink-assets/journeys.db")).unwrap();
        connection.execute_batch("CREATE TABLE catalogue (id TEXT PRIMARY KEY, kind TEXT NOT NULL); PRAGMA user_version = 1;").unwrap();
        let transaction = connection.transaction().unwrap();
        {
            let mut insert = transaction
                .prepare("INSERT INTO catalogue VALUES (?1, ?2)")
                .unwrap();
            for index in 0..CATALOGUE_ROWS {
                insert
                    .execute(params![
                        format!("r{index:05}"),
                        if index % 2 == 0 { "station" } else { "harbour" }
                    ])
                    .unwrap();
            }
        }
        transaction.commit().unwrap();
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove owned benchmark directory");
    }
}

fn host(directory: &Path) -> Arc<DataHost> {
    let assets = directory.join("assets");
    DataHost::new(Config {
        store_path: directory.join("databases/ink-store.db"),
        files_directory: directory.join("files"),
        assets: Arc::new(move |name| {
            Ok(Box::new(std::fs::File::open(assets.join(name))?) as Box<dyn std::io::Read>)
        }),
    })
}

#[derive(Default)]
struct Calls {
    queries: usize,
    cancellations: usize,
    closes: usize,
}
struct ObservedHost {
    inner: Arc<DataHost>,
    calls: Arc<Mutex<Calls>>,
}
struct ObservedSession {
    inner: Box<dyn NativeSession>,
    calls: Arc<Mutex<Calls>>,
}
impl NativeHost for ObservedHost {
    fn connect(&self, responder: ink_runtime::Responder) -> Box<dyn NativeSession> {
        Box::new(ObservedSession {
            inner: self.inner.connect(responder),
            calls: self.calls.clone(),
        })
    }
}
impl NativeSession for ObservedSession {
    fn call(&mut self, call: ink_runtime::NativeCall) -> bool {
        if call.module == "sqlite" {
            let mut calls = self.calls.lock().unwrap();
            if call.operation == "query" {
                calls.queries += 1;
            }
            if call.operation == "close" {
                calls.closes += 1;
            }
        }
        self.inner.call(call)
    }
    fn cancel(&mut self, id: u64) -> bool {
        let handled = self.inner.cancel(id);
        if handled {
            self.calls.lock().unwrap().cancellations += 1;
        }
        handled
    }
}

struct Assets {
    source: String,
    icons: Vec<u8>,
}
impl Assets {
    fn load() -> Self {
        let path = std::env::var_os("INK_PERF_ASSETS")
            .map(|root| PathBuf::from(root).join("journeys"))
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../fixtures/performance/journeys/.ink/android/assets")
            });
        Self {
            source: std::fs::read_to_string(path.join("app.js")).expect("prepare journeys app.js"),
            icons: std::fs::read(path.join("ink-icons-v1.bin")).expect("prepare journeys icons"),
        }
    }
}

#[derive(Default)]
struct Publication {
    keys: Vec<String>,
    records: HashMap<String, Value>,
    complete: bool,
    serial: usize,
}
impl Publication {
    fn update(&mut self, props: &Map<String, Value>, serial: usize) {
        // NativeList sends complete props with either a complete dataset or a key/row patch.
        self.complete = props.get("hasMore") == Some(&json!(true));
        if let Some(items) = props.get("items").and_then(Value::as_array) {
            self.keys.clear();
            self.records.clear();
            for item in items {
                let key = item["key"].as_str().expect("projected key").to_owned();
                self.keys.push(key.clone());
                self.records.insert(key, item.clone());
            }
        }
        if let Some(keys) = props.get("itemKeys").and_then(Value::as_array) {
            self.keys = keys
                .iter()
                .map(|key| key.as_str().unwrap().to_owned())
                .collect();
        }
        if let Some(changes) = props.get("itemChanges").and_then(Value::as_array) {
            for item in changes {
                self.records
                    .insert(item["key"].as_str().unwrap().to_owned(), item.clone());
            }
        }
        self.serial = serial;
    }
    fn signature(&self, keys: &[String], text: &str, after: usize) -> bool {
        self.complete
            && self.serial > after
            && self.keys.len() == keys.len()
            && self.keys.first() == keys.first()
            && self.keys.last() == keys.last()
            && self
                .records
                .get(&keys[0])
                .is_some_and(|record| contains_text(record, text))
    }
}
fn contains_text(value: &Value, text: &str) -> bool {
    match value {
        Value::String(value) => value == text,
        Value::Array(values) => values.iter().any(|value| contains_text(value, text)),
        Value::Object(values) => values.values().any(|value| contains_text(value, text)),
        _ => false,
    }
}

struct App {
    runtime: Option<AppRuntime>,
    events: EventReceiver,
    tree: ReactTree,
    engine: Engine,
    lists: BTreeMap<usize, Publication>,
    messages: Vec<Value>,
    serial: usize,
    commits: usize,
    apply_ns: u64,
    inspection_ns: u64,
    preparing_commits: usize,
    input_ack: Option<String>,
    request_sequence: usize,
    seen_labels: Vec<String>,
    calls: Arc<Mutex<Calls>>,
}
impl Drop for App {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.stop();
            drop(runtime);
        } // Drop joins the JS thread.
    }
}
impl App {
    fn start(assets: &Assets, directory: &Path, boot: Value) -> Self {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let native = Arc::new(ObservedHost {
            inner: host(directory),
            calls: calls.clone(),
        });
        // Runtime intrinsics are read-only. Redirect only this benchmark's helper captures
        // to transparent observers; the originals and all production files remain intact.
        let observed_source = assets
            .source
            .replace("__inkPrepareDataSearch", "__inkJourneyPrepareDataSearch")
            .replace("__inkDataSearch", "__inkJourneyDataSearch");
        let source =
            format!("globalThis.__inkJourneyBoot={boot};\n{SEARCH_OBSERVERS}\n{observed_source}");
        let (runtime, events) = AppRuntime::spawn_script(
            Script::Source(source),
            || {},
            None::<fn() -> anyhow::Result<Script>>,
            Some(native as Arc<dyn NativeHost>),
        )
        .unwrap();
        let mut engine = Engine::new();
        engine.set_viewport(1080, 1240);
        Self {
            runtime: Some(runtime),
            events,
            tree: ReactTree::with_icons(&assets.icons).unwrap(),
            engine,
            lists: BTreeMap::new(),
            messages: Vec::new(),
            serial: 0,
            commits: 0,
            apply_ns: 0,
            inspection_ns: 0,
            preparing_commits: 0,
            input_ack: None,
            request_sequence: 0,
            seen_labels: Vec::new(),
            calls,
        }
    }
    fn runtime(&self) -> &AppRuntime {
        self.runtime.as_ref().unwrap()
    }
    fn send(&self, mut value: Value) {
        value["type"] = json!("benchmark");
        let payload = value.to_string();
        assert!(payload.len() < MESSAGE_LIMIT);
        self.runtime().send(payload).unwrap();
    }
    fn refresh(&mut self) {
        for event in self.tree.viewport_events(&mut self.engine).unwrap() {
            self.runtime().send(event.to_string()).unwrap();
        }
    }
    fn receive(&mut self, event: Event) {
        match event {
            Event::Commit(commit) => {
                self.serial += 1;
                self.commits += 1;
                let inspection = Instant::now();
                self.inspect(&commit);
                self.inspection_ns += ns(inspection.elapsed());
                let apply = Instant::now();
                self.tree.apply(commit, &mut self.engine).unwrap();
                self.refresh();
                self.apply_ns += ns(apply.elapsed());
                for run in &self.engine.scene().text {
                    if run.text.starts_with("Loading route ")
                        || run.text.starts_with("Route ") && run.text.ends_with(" failed")
                    {
                        if self
                            .seen_labels
                            .last()
                            .is_none_or(|last| last != run.text.as_str())
                        {
                            self.seen_labels.push(run.text.to_string());
                        }
                    }
                }
            }
            Event::Message(message) => self.messages.push(serde_json::from_str(&message).unwrap()),
            Event::Ready => {}
            Event::Error(error) => panic!("journey runtime failed: {error}"),
            Event::Stopped => panic!("runtime stopped before the journey completed"),
        }
    }
    fn inspect(&mut self, commit: &ReactCommit) {
        for operation in &commit.0 {
            if let Operation::Create { props, .. } | Operation::Update { props, .. } = operation {
                if let Some(sequence) = props
                    .get("text")
                    .and_then(Value::as_str)
                    .and_then(|text| text.strip_prefix("Requested "))
                    .and_then(|number| number.parse::<usize>().ok())
                {
                    self.request_sequence = sequence;
                }
            }
            match operation {
                Operation::Text { values, .. } => {
                    for value in values {
                        if let Some(sequence) = value
                            .strip_prefix("Requested ")
                            .and_then(|number| number.parse::<usize>().ok())
                        {
                            self.request_sequence = sequence;
                        }
                    }
                }
                Operation::Create {
                    id,
                    r#type: HostKind::NativeList,
                    props,
                } => {
                    let mut publication = Publication::default();
                    publication.update(props, self.serial);
                    self.preparing_commits += usize::from(!publication.complete);
                    self.lists.insert(*id, publication);
                }
                Operation::Update { id, props } => {
                    if let Some(list) = self.lists.get_mut(id) {
                        list.update(props, self.serial);
                        self.preparing_commits += usize::from(!list.complete);
                    }
                    if let Some(value) = props.get("value").and_then(Value::as_str) {
                        self.input_ack = Some(value.to_owned());
                    }
                }
                Operation::Remove { id, .. } => {
                    self.lists.remove(id);
                }
                _ => {}
            }
        }
    }
    fn wait(&mut self, ready: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            self.refresh();
            if self.engine.list_viewports_ready() && ready(self) {
                return;
            }
            let event = self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|error| {
                    panic!("journey timed out: {error}; labels {:?}", self.labels())
                });
            self.receive(event);
        }
    }
    fn labels(&self) -> Vec<String> {
        self.engine
            .scene()
            .text
            .iter()
            .map(|run| run.text.to_string())
            .collect()
    }
    fn label(&self, text: &str) -> bool {
        self.engine.scene().text.iter().any(|run| run.text == text)
    }
    fn visible_rows(&self) -> Vec<String> {
        let scene = self.engine.scene();
        scene
            .text
            .iter()
            .filter_map(|run| {
                let text = run.text.as_str();
                let id = text.strip_prefix("J ")?.split_whitespace().next()?;
                let y = run.rect.y
                    + if run.scrolling {
                        scene.scroll_origin - scene.scroll_offset
                    } else {
                        0.0
                    }
                    + run.rect.height / 2.0;
                let clip = if run.scrolling {
                    scene.scroll_clip?
                } else {
                    run.clip
                };
                (y >= clip.y
                    && y < clip.y + clip.height
                    && y >= run.clip.y
                    && y < run.clip.y + run.clip.height)
                    .then(|| id.to_owned())
            })
            .collect()
    }
    fn publication(&self, keys: &[String], text: &str, after: usize) -> Option<&Publication> {
        self.lists
            .values()
            .find(|list| list.signature(keys, text, after))
    }
    fn published(&self, keys: &[String], text: &str, after: usize) -> bool {
        self.publication(keys, text, after).is_some()
            && self
                .visible_rows()
                .first()
                .is_some_and(|id| self.row_displays(id, text))
    }
    fn row_displays(&self, id: &str, text: &str) -> bool {
        let scene = self.engine.scene();
        let title = format!("J {id}");
        scene
            .text
            .iter()
            .position(|run| run.text == title || run.text.starts_with(&format!("{title} ·")))
            .is_some_and(|index| {
                scene.text[index..]
                    .iter()
                    .skip(1)
                    .take_while(|run| !run.text.starts_with("J "))
                    .any(|run| run.text == text)
            })
    }
    fn verify(&self, keys: &[String], text: &str, after: usize) -> Value {
        let publication = self
            .publication(keys, text, after)
            .expect("final NativeList end-of-preparation marker");
        assert_eq!(
            publication.keys, keys,
            "independently authored full projected key order"
        );
        let visible = self.visible_rows();
        assert!(!visible.is_empty() && visible.len() < 100);
        let positions: Vec<_> = visible
            .iter()
            .map(|id| {
                keys.iter()
                    .position(|key| key == id)
                    .expect("visible row is in final dataset")
            })
            .collect();
        assert!(
            positions.windows(2).all(|pair| pair[1] == pair[0] + 1),
            "visible rows follow final key order"
        );
        assert!(self.engine.list_viewports_ready());
        assert!(
            self.tree.memory_diagnostics()["host_nodes"]
                .as_u64()
                .unwrap()
                < 600,
            "bounded mounted host nodes"
        );
        for id in &visible {
            assert!(
                self.row_displays(id, text),
                "final subtitle must actually be in the scene for {id}"
            );
            assert!(
                publication
                    .records
                    .get(id)
                    .is_some_and(|record| contains_text(record, text)),
                "final row data for {id}"
            );
        }
        json!({"visible_ids":visible,"dataset_rows":keys.len(),"first":keys.first(),"last":keys.last(),
            "native_list_serial":publication.serial,"end_of_preparation":"NativeList.hasMore=true",
            "full_key_order_verified":true,"list_viewports_ready":true})
    }
    fn point(&self, label: &str) -> (f32, f32) {
        let scene = self.engine.scene();
        scene
            .text
            .iter()
            .find_map(|run| {
                if run.text != label {
                    return None;
                }
                let y = run.rect.y
                    + if run.scrolling {
                        scene.scroll_origin - scene.scroll_offset
                    } else {
                        0.0
                    }
                    + run.rect.height / 2.0;
                let clip = if run.scrolling {
                    scene.scroll_clip?
                } else {
                    run.clip
                };
                (y >= clip.y && y < clip.y + clip.height)
                    .then_some((run.rect.x + run.rect.width / 2.0, y))
            })
            .unwrap_or_else(|| panic!("no visible control {label}; {:?}", self.labels()))
    }
    fn dispatch(&mut self) {
        let request = self.engine.take_native_request().expect("native action");
        assert_eq!(request.module(), "ink");
        let message = self
            .tree
            .route_native_message(request.payload().to_owned())
            .unwrap();
        self.runtime().send(message).unwrap();
    }
    fn press(&mut self, label: &str) {
        let (x, y) = self.point(label);
        assert!(self.engine.tap(x, y));
        self.dispatch();
    }
    fn edit(&mut self, value: &str) {
        let context: Value = serde_json::from_str(&self.engine.text_input_context()).unwrap();
        assert!(!context["id"].is_null(), "focused native input");
        assert!(
            self.engine.edit_text(TextEdit::Update(
                json!({"id":context["id"],"text":context["text"],
            "value":value,"selection":value.encode_utf16().count()})
                .to_string()
            ))
        );
        let events = self.tree.input_events(&self.engine);
        assert_eq!(events.len(), 1);
        self.runtime().send(events[0].to_string()).unwrap();
    }
    fn report(&mut self) -> Value {
        self.messages
            .retain(|message| message["type"] != "observed");
        self.send(json!({"command":"report"}));
        self.wait(|app| {
            app.messages
                .iter()
                .any(|message| message["type"] == "observed")
        });
        let index = self
            .messages
            .iter()
            .position(|message| message["type"] == "observed")
            .unwrap();
        self.messages.remove(index)
    }
    fn close_database(&mut self) {
        self.send(json!({"command":"close"}));
        self.wait(|app| {
            app.messages
                .iter()
                .any(|message| message["type"] == "closed")
        });
        assert_eq!(self.calls.lock().unwrap().closes, 1);
    }
    fn reset_metrics(&mut self) {
        self.engine.take_perf_metrics();
        self.apply_ns = 0;
        self.inspection_ns = 0;
        self.commits = 0;
        self.preparing_commits = 0;
    }
    fn sample(&mut self, started: Instant) -> Value {
        let elapsed_ns = ns(started.elapsed());
        let core = self.engine.take_perf_metrics();
        json!({"elapsed_ns":elapsed_ns,"apply_and_viewport_ns":self.apply_ns,"contract_inspection_ns":self.inspection_ns,
            "commits":self.commits,"preparing_commits":self.preparing_commits,"nodes_measured":core.nodes_measured,
            "full_rebuilds":core.full_rebuilds,"incremental_rebuilds":core.incremental_rebuilds})
    }
}
fn ns(duration: Duration) -> u64 {
    duration.as_nanos().try_into().unwrap()
}
fn original_keys(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("r{index:05}")).collect()
}
fn bench(assets: &Assets, directory: &Path, count: usize) -> App {
    let mut app = App::start(assets, directory, json!({}));
    app.wait(|app| {
        app.messages
            .iter()
            .any(|message| message["type"] == "ready")
            && app.label("J r00000")
    });
    let before = app.serial;
    app.send(json!({"command":"populate","count":count}));
    let keys = original_keys(count);
    app.wait(|app| app.published(&keys, "Original", before));
    app.verify(&keys, "Original", before);
    app
}

fn structure(assets: &Assets, command: &str, count: usize, samples: usize, warmup: usize) -> Value {
    let directory = Directory::new();
    let mut app = bench(assets, &directory.0, count);
    let mut raw = Vec::new();
    let mut last_contract = Value::Null;
    for step in 0..samples + warmup {
        // Every sample starts with the same externally authored dataset and scroll position.
        let mut expected = original_keys(count);
        app.engine.scroll_by(-f32::MAX);
        app.refresh();
        app.send(json!({"command":"populate","count":count,"sequence":step + 1}));
        app.wait(|app| {
            app.request_sequence == step + 1 && app.publication(&expected, "J r00000", 0).is_some()
        });
        app.engine.scroll_by(-f32::MAX);
        app.refresh();
        let before_height = row_height(&app, "r00000");
        let payload = match command {
            "insert" => {
                expected.insert(0, "i00000".into());
                json!({"command":"insert","id":"i00000"})
            }
            "delete" => {
                expected.remove(0);
                json!({"command":"delete","id":"r00000"})
            }
            "reorder" => {
                expected.reverse();
                json!({"command":"reorder"})
            }
            "resize" => json!({"command":"resize","expanded":true}),
            _ => unreachable!(),
        };
        let after = app.serial;
        app.reset_metrics();
        let start = Instant::now();
        app.send(payload);
        app.wait(|app| app.published(&expected, "Original", after));
        let mut sample = app.sample(start);
        last_contract = app.verify(&expected, "Original", after);
        if command == "resize" {
            let height = row_height(&app, "r00000");
            assert!(
                height > before_height + 10.0,
                "resizing must change displayed text geometry"
            );
            sample["before_title_height"] = json!(before_height);
            sample["after_title_height"] = json!(height);
        }
        // Verify both endpoints after the timer; native viewport materialisation is real.
        app.engine.scroll_by(-f32::MAX);
        app.refresh();
        assert_eq!(app.visible_rows().first(), expected.first());
        app.engine.scroll_by(f32::MAX);
        app.refresh();
        assert_eq!(app.visible_rows().last(), expected.last());
        if step >= warmup {
            raw.push(sample);
        }
    }
    json!({"name":format!("journey-list-{command}-{count}"),"raw_samples":raw,"contracts":last_contract,
        "queue_high_water_bytes":app.runtime().queue_metrics().1,
        "boundaries":{"timed":"host command to final NativeList publication and materialised scene; contract mirror included",
        "untimed":"dataset reset, full key-order verification and native scroll endpoints",
        "payload":"compact count/operation commands; datasets generated in the fixture, not sent as large JSON messages"}})
}
fn row_height(app: &App, id: &str) -> f32 {
    let prefix = format!("J {id}");
    app.engine
        .scene()
        .text
        .iter()
        .find(|run| run.text.starts_with(&prefix))
        .expect("materialised first row")
        .rect
        .height
}

fn local_search(assets: &Assets, count: usize, samples: usize, warmup: usize) -> Vec<Value> {
    let mut cold = Vec::new();
    let mut warm = Vec::new();
    let mut last = Value::Null;
    for step in 0..samples + warmup {
        let directory = Directory::new();
        let mut app = App::start(assets, &directory.0, json!({"mode":"local","count":count}));
        app.wait(|app| app.label("J lseed0"));
        assert!(
            !app.messages
                .iter()
                .any(|message| message["type"] == "local-cache-ready"
                    || message["type"] == "local-cache-used")
        );
        let harbour: Vec<_> = (0..count)
            .filter(|index| index % 2 == 1)
            .map(|index| format!("l{index:05}"))
            .collect();
        let before = app.serial;
        app.reset_metrics();
        let start = Instant::now();
        app.edit("Harbour");
        app.wait(|app| app.published(&harbour, "Local Harbour", before));
        let cold_sample = app.sample(start);
        app.verify(&harbour, "Local Harbour", before);
        assert!(
            !app.messages
                .iter()
                .any(|message| message["type"] == "local-cache-used"),
            "first search must use the unprepared application expression"
        );
        let station: Vec<_> = (0..count)
            .step_by(2)
            .map(|index| format!("l{index:05}"))
            .collect();
        let before = app.serial;
        app.edit("Station");
        app.wait(|app| app.published(&station, "Local Station", before));
        // The second search schedules production cache warming. Wait for its native cursor,
        // rather than assuming a duration or calling the warming primitive ourselves.
        app.wait(|app| {
            app.messages
                .iter()
                .any(|message| message["type"] == "local-cache-ready" && message["rows"] == count)
        });
        let used_before = app
            .messages
            .iter()
            .filter(|message| message["type"] == "local-cache-used")
            .count();
        let zurich: Vec<_> = (0..count)
            .filter(|index| index % 3 != 0)
            .map(|index| format!("l{index:05}"))
            .collect();
        let before = app.serial;
        app.reset_metrics();
        let start = Instant::now();
        app.edit("ZÜRICH");
        app.wait(|app| app.published(&zurich, "Local ZÜRICH", before));
        let warm_sample = app.sample(start);
        last = app.verify(&zurich, "Local ZÜRICH", before);
        assert!(
            app.messages
                .iter()
                .filter(|message| message["type"] == "local-cache-used")
                .count()
                > used_before,
            "cache-ready search must exercise the native data-search guard"
        );
        let report = app.report();
        assert_eq!(report["query"], "ZÜRICH");
        assert_eq!(report["ids"], json!(zurich));
        if step >= warmup {
            cold.push(cold_sample);
            warm.push(warm_sample);
        }
    }
    let boundary = json!({"timed":"native TextEdit through compiled joined-field filter and final NativeList scene; inspection included",
        "readiness":"transparent benchmark wrappers observe real native cursor.complete and native cached-search guard; no timers invented",
        "cold":"fresh runtime and stable catalogue array; first filter invocation has no cached search text",
        "cache_ready":"same catalogue identity, after the second ordinary filter schedules and completes real background warming",
        "scope":"local in-memory filter; no filesystem or network IO"});
    vec![
        json!({"name":format!("journey-local-search-unprepared-{count}"),"raw_samples":cold,
            "contracts":{"first_search_uncached":true,"joined_fields":["place","artist"],"independent_odd_ids_verified":true,"final_native_scene":true},"boundaries":boundary}),
        json!({"name":format!("journey-local-search-cache-ready-{count}"),"raw_samples":warm,
            "contracts":{"native_cache_completion_observed":true,"native_cached_guard_observed":true,"unicode_case_fold_verified":true,"final_publication":last},"boundaries":boundary}),
    ]
}

fn sqlite_search(assets: &Assets, samples: usize, warmup: usize) -> Value {
    const INPUTS: [&str; 6] = ["h", "ha", "har", "harb", "harbo", "harbour"];
    const ARRIVALS: usize = 16;
    let directory = Directory::new();
    directory.catalogue();
    let mut app = bench(assets, &directory.0, 1000);
    let mut raw = Vec::new();
    let mut last = Value::Null;
    for step in 0..samples + warmup {
        app.messages.clear();
        app.send(json!({"command":"search"}));
        app.wait(|app| {
            app.messages
                .iter()
                .any(|message| message["type"] == "query-started" && message["draft"] == "slow")
        });
        let cancellations_before = app.calls.lock().unwrap().cancellations;
        let before = app.serial;
        app.reset_metrics();
        let start = Instant::now();
        let deadline = start + TIMEOUT;
        let input_interval = Duration::from_millis(2);
        let arrival_interval = Duration::from_millis(1);
        let mut sent = 0;
        let mut arrivals = 0;
        let mut dispatched = Vec::new();
        let mut acknowledgements = Vec::new();
        let mut acked = 0;
        let expected: Vec<_> = (1..CATALOGUE_ROWS)
            .step_by(2)
            .map(|index| format!("r{index:05}"))
            .chain((0..ARRIVALS).map(|index| format!("a{index:05}")))
            .collect();
        let token = format!("Search harbour:{ARRIVALS}");
        loop {
            let now = Instant::now();
            assert!(now < deadline, "continuous search did not drain");
            // Arrivals and typing follow their authored schedule even when commits are queued.
            while arrivals < ARRIVALS && now >= start + arrival_interval * (arrivals as u32 + 1) {
                app.send(json!({"command":"arrival","id":format!("a{arrivals:05}")}));
                arrivals += 1;
            }
            while sent < INPUTS.len() && now >= start + input_interval * (sent as u32 + 1) {
                let intended = start + input_interval * (sent as u32 + 1);
                let dispatched_at = Instant::now();
                app.edit(INPUTS[sent]);
                dispatched.push((intended, dispatched_at));
                sent += 1;
            }
            if sent == INPUTS.len()
                && arrivals == ARRIVALS
                && app.published(&expected, &token, before)
            {
                break;
            }
            let mut next = deadline;
            if sent < INPUTS.len() {
                next = next.min(start + input_interval * (sent as u32 + 1));
            }
            if arrivals < ARRIVALS {
                next = next.min(start + arrival_interval * (arrivals as u32 + 1));
            }
            match app
                .events
                .recv_timeout(next.saturating_duration_since(Instant::now()))
            {
                Ok(event) => {
                    app.receive(event);
                    if let Some(index) = app
                        .input_ack
                        .as_ref()
                        .and_then(|text| INPUTS.iter().position(|input| input == text))
                    {
                        if index + 1 > acked {
                            let at = Instant::now();
                            assert!(index < sent);
                            for input in acked..=index {
                                acknowledgements.push(json!({"input":INPUTS[input],"coalesced":input != index,
                                    "intended_to_ack_ns":ns(at.duration_since(dispatched[input].0)),
                                    "dispatch_to_ack_ns":ns(at.duration_since(dispatched[input].1)),
                                    "dispatch_lateness_ns":ns(dispatched[input].1.duration_since(dispatched[input].0))}));
                            }
                            acked = index + 1;
                        }
                    }
                    if sent > 0 {
                        let context: Value =
                            serde_json::from_str(&app.engine.text_input_context()).unwrap();
                        assert_eq!(
                            context["text"],
                            INPUTS[sent - 1],
                            "stale input acknowledgement must not overwrite the newest native draft"
                        );
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => panic!("search runtime stream failed: {error}"),
            }
        }
        let mut sample = app.sample(start);
        last = app.verify(&expected, &token, before);
        sample["inputs"] = json!(acknowledgements);
        sample["final_input_intended_to_scene_ns"] = json!(
            sample["elapsed_ns"]
                .as_u64()
                .unwrap()
                .saturating_sub(ns(input_interval * INPUTS.len() as u32))
        );
        // The winner is queued behind the real cancelled query on the same SQLite worker.
        // Its completion plus a JS report barrier proves stale completions cannot later win.
        let report = app.report();
        assert_eq!(report["draft"], "harbour");
        assert_eq!(report["received"], ARRIVALS);
        assert_eq!(report["ids"], json!(expected));
        assert_eq!(report["statistics"]["published"], token);
        assert_eq!(report["statistics"]["active"], 0);
        let cancelled = app.calls.lock().unwrap().cancellations - cancellations_before;
        assert!(
            cancelled > 0,
            "AbortSignal must reach and be handled by the real DataHost SQLite session"
        );
        sample["native_cancellations"] = json!(cancelled);
        sample["statistics"] = report["statistics"].clone();
        if step >= warmup {
            raw.push(sample);
        }
    }
    let queries = app.calls.lock().unwrap().queries;
    app.close_database();
    json!({"name":"journey-sqlite-search-continuous-cancelled","raw_samples":raw,
        "contracts":{"final_publication":last,"all_arrivals_received":true,"obsolete_results_cannot_win":true,
        "native_cancellation_forwarded":true,"real_sqlite_queries":queries,"database_closed":true},
        "boundaries":{"timed":"scheduled native typing and continuous JS arrivals through the final winning SQLite result and NativeList scene",
        "input_interval_ms":2,"arrival_interval_ms":1,"arrivals":ARRIVALS,
        "cancellation":"real unbounded recursive SQL dispatched before typing; cancellation may interrupt queued or executing work",
        "io":"real imported read-only SQLite asset; no invented delays; OS file cache uncontrolled",
        "result_size":"5000 catalogue IDs plus 16 arrivals; projected labels derived in JS, SQL result stays below 400000 bytes",
        "intermediate_queries":"superseded results may never display; raw input acknowledgements explicitly identify coalescing"}})
}

fn routes(assets: &Assets, samples: usize, warmup: usize) -> Value {
    let directory = Directory::new();
    directory.catalogue();
    let mut app = bench(assets, &directory.0, 1000);
    app.press("Save home");
    app.wait(|app| app.label("Home 1"));
    let expected: Vec<_> = (0..24)
        .map(|offset| format!("r{:05}", CATALOGUE_ROWS - 2 - offset * 2))
        .collect();
    let mut raw = Vec::new();
    let mut last = Value::Null;
    for step in 0..samples + warmup {
        let id = format!("visit-{step}");
        let loading = format!("Loading route {id}");
        let error = format!("Route {id} failed");
        app.send(json!({"command":"route","id":id,"sequence":step + 1000}));
        app.wait(|app| app.request_sequence == step + 1000);
        app.seen_labels.clear();
        app.reset_metrics();
        let start = Instant::now();
        app.press("Open route");
        app.wait(|app| app.label(&error));
        let open_error = app.sample(start);
        assert!(
            app.seen_labels.contains(&loading),
            "SDK resource must first publish a real loading scene"
        );
        assert!(app.seen_labels.contains(&error));
        let before = app.serial;
        app.reset_metrics();
        let retry_start = Instant::now();
        app.press("RETRY ROUTE");
        let token = format!("Route {id}");
        app.wait(|app| app.published(&expected, &token, before));
        let retry = app.sample(retry_start);
        last = app.verify(&expected, &token, before);
        assert!(
            app.seen_labels
                .iter()
                .filter(|label| *label == &loading)
                .count()
                >= 2,
            "retry must re-enter SDK resource loading"
        );
        app.reset_metrics();
        let back_start = Instant::now();
        assert!(app.engine.back());
        app.dispatch();
        app.wait(|app| app.label("Home 1") && app.label("J r00000"));
        let back = app.sample(back_start);
        assert!(!app.label(&error));
        // The restored home page legitimately autofocuses its editor. Root history is
        // checked after dismissing it, since Engine.back first dismisses a focused editor.
        if app.engine.text_input_active() {
            assert!(app.engine.edit_text(TextEdit::Dismiss));
        }
        assert!(!app.engine.back());
        assert!(app.engine.take_native_request().is_none());
        let report = app.report();
        assert_eq!(report["home"], 1);
        if step >= warmup {
            let elapsed_ns = open_error["elapsed_ns"].as_u64().unwrap()
                + retry["elapsed_ns"].as_u64().unwrap()
                + back["elapsed_ns"].as_u64().unwrap();
            raw.push(json!({"elapsed_ns":elapsed_ns,"open_to_error":open_error,"retry_to_final_scene":retry,"back_to_home":back}));
        }
    }
    let queries = app.calls.lock().unwrap().queries;
    assert_eq!(queries, (samples + warmup) * 2);
    app.close_database();
    json!({"name":"journey-route-loading-error-retry-back","raw_samples":raw,
        "contracts":{"loading_error_and_retry_loading_scenes_verified":true,"real_sqlite_failure_then_success":true,
        "retry_final_publication":last,"native_hit_testing_and_back":true,"home_state_retained":true,"root_back_stack_empty":true},
        "boundaries":{"timed":"native hit test/router dispatch through SDK resource transitions and final materialised scene; per-stage raw timings included",
        "failure":"a real missing-table SQLite request, followed by a valid read on explicit resource.refresh retry",
        "cache":"unique SDK resource key per visit; loading is observed in applied scene commits, not held with an artificial delay",
        "scope":"host file router and resource lifecycle; no network stack or Android activity lifecycle"}})
}

fn saved_value() -> Value {
    json!({"schema":3,"rows":(0..1000).map(|index| json!({"id":format!("p{index:05}"),
        "title":format!("J p{index:05}"),"subtitle":"Saved Zürich","titleSize":28})).collect::<Vec<_>>()})
}
fn saved_keys() -> Vec<String> {
    (0..1000).map(|index| format!("p{index:05}")).collect()
}

fn subprocess(operation: &str, directory: &Path) -> (Value, u64) {
    let start = Instant::now();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg(operation)
        .arg(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("start isolated persistence process");
    let deadline = start + TIMEOUT + TIMEOUT;
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed-out child");
            child.wait().expect("join timed-out child");
            panic!("persistence child {operation} timed out");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    // Child reports are deliberately small enough to fit the pipe without an active reader.
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "persistence child failed: {}",
        output.status
    );
    let elapsed = ns(start.elapsed());
    assert!(output.stdout.len() < 16_384);
    (
        serde_json::from_slice(&output.stdout).expect("isolated child JSON"),
        elapsed,
    )
}

fn disk_state(directory: &Path) -> (Value, Vec<Vec<u8>>) {
    let path = directory.join("databases/ink-store.db");
    let bytes = std::fs::read(&path).expect("SDK must have created a real store file");
    assert!(bytes.starts_with(b"SQLite format 3\0"));
    let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let schema: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(schema, 1);
    let columns: Vec<String> = connection
        .prepare("PRAGMA table_info(entries)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(columns, ["key", "revision", "version", "value"]);
    let (revision, version, value): (i64, i64, String) = connection
        .query_row(
            "SELECT revision, version, value FROM entries WHERE key = 'journey-library'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(revision, 1);
    assert_eq!(version, 3);
    assert_eq!(
        serde_json::from_str::<Value>(&value).unwrap(),
        saved_value()
    );
    drop(connection);
    let wal = std::fs::read(path.with_file_name("ink-store.db-wal")).unwrap_or_default();
    let evidence = json!({"file_bytes":bytes.len(),"wal_bytes":wal.len(),"sqlite_header_verified":true,
        "integrity_check":"ok","sqlite_user_version":schema,"columns":columns,"entry_revision":revision,"entry_version":version,
        "saved_rows":1000,"independent_payload_verified":true});
    (evidence, vec![bytes, wal])
}

fn persisted_startup(samples: usize, warmup: usize) -> Value {
    let mut raw = Vec::new();
    let mut last = Value::Null;
    for step in 0..samples + warmup {
        let directory = Directory::new();
        let (seed, _) = subprocess("--seed", &directory.0);
        assert_eq!(seed["seeded"], true);
        let (before, bytes_before) = disk_state(&directory.0);
        let (mut sample, process_ns) = subprocess("--reopen", &directory.0);
        let (after, bytes_after) = disk_state(&directory.0);
        assert_eq!(
            bytes_before, bytes_after,
            "reader must reopen the exact committed database/WAL bytes"
        );
        assert_eq!(before, after);
        assert_ne!(seed["pid"], sample["pid"]);
        sample["process_spawn_to_exit_ns"] = json!(process_ns);
        sample["filesystem_cache_boundary"] = json!(
            "not evicted; newly saved files were independently read and validated immediately before this sample"
        );
        last = json!({"disk":after,"writer_exited_before_reader_started":true,"fresh_process_and_host":true,
            "same_committed_database_and_wal_bytes":true,"publication":sample["publication"]});
        if step >= warmup {
            raw.push(sample);
        }
        // Both children have exited before Directory removes database/WAL and assets.
    }
    json!({"name":"journey-persisted-startup-fresh-process","raw_samples":raw,"contracts":last,
        "boundaries":{"timed":"fresh child loads prepared JS/icons, creates DataHost/AppRuntime/ReactTree/Engine, SDK createStore reopens disk and publishes final NativeList rows",
        "process":"elapsed_ns starts inside fresh reader; separate process_spawn_to_exit_ns includes process launch, validation and teardown",
        "filesystem_cache":"OS cache is uncontrolled and recently warmed by seeding and independent disk verification, repeated explicitly in each sample",
        "persistence":"SDK write completes in a separate seed process which exits before reopening; not a simulated in-memory persistence adapter",
        "lifetime":"AppRuntime.stop plus Drop joins JS. DataHost store worker has no public shutdown/join and retains its own Arc, so process exit guarantees SQLite/worker teardown",
        "excluded":"Android activity/process launch, device flash cold-cache behaviour, packaging/install time"}})
}

fn persistence_child(operation: &str, directory: &Path) {
    if operation == "--seed" {
        let assets = Assets::load();
        let mut app = App::start(
            &assets,
            directory,
            json!({"mode":"seed","value":saved_value()}),
        );
        app.wait(|app| {
            app.messages
                .iter()
                .any(|message| message["type"] == "seeded")
        });
        drop(app);
        println!("{}", json!({"seeded":true,"pid":std::process::id()}));
    } else {
        let start = Instant::now();
        let assets = Assets::load();
        let mut app = App::start(&assets, directory, json!({"mode":"persisted"}));
        let keys = saved_keys();
        app.wait(|app| app.published(&keys, "Saved Zürich", 0));
        let mut sample = app.sample(start);
        sample["publication"] = app.verify(&keys, "Saved Zürich", 0);
        sample["pid"] = json!(std::process::id());
        drop(app);
        println!("{sample}");
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--seed" || arg == "--reopen")
    {
        assert_eq!(args.len(), 2);
        persistence_child(&args[0], Path::new(&args[1]));
        return;
    }
    assert!(
        (2..=3).contains(&args.len()),
        "usage: ink-journey-performance SAMPLES WARMUP [1000,5000,10000]"
    );
    let samples: usize = args[0].parse().expect("positive sample count");
    let warmup: usize = args[1].parse().expect("warm-up count");
    assert!(samples > 0 && samples <= 10_000 && warmup <= 10_000);
    let counts: Vec<usize> = args.get(2).map_or_else(
        || vec![1000, 5000, 10000],
        |value| value.split(',').map(|part| part.parse().unwrap()).collect(),
    );
    assert!(
        !counts.is_empty()
            && counts
                .iter()
                .all(|count| [1000, 5000, 10000].contains(count))
    );
    let assets = Assets::load();
    let mut workloads = Vec::new();
    for count in counts {
        for command in ["insert", "delete", "reorder", "resize"] {
            eprintln!("journey list {command} {count}");
            workloads.push(structure(&assets, command, count, samples, warmup));
        }
        eprintln!("journey local search {count}");
        workloads.extend(local_search(&assets, count, samples, warmup));
    }
    eprintln!("journey SQLite search");
    workloads.push(sqlite_search(&assets, samples, warmup));
    eprintln!("journey routes");
    workloads.push(routes(&assets, samples, warmup));
    eprintln!("journey persistence");
    workloads.push(persisted_startup(samples, warmup));
    println!(
        "{}",
        json!({"version":1,"suite":"host-journeys","runtime":{"optimised":!cfg!(debug_assertions),
        "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"viewport":[1080,1240]},
        "samples":samples,"warmup":warmup,"workloads":workloads,
        "contracts":{"independent_expected_ids_and_order":true,"actual_end_of_preparation_marker":true,"all_runtime_threads_stopped_and_joined":true,
        "max_command_bytes":MESSAGE_LIMIT,"finite_wait_seconds":TIMEOUT.as_secs()},
        "boundaries":{"execution":"host AppRuntime/ReactTree/Engine and genuine SDK/data backend; no pixels rendered or frame presentation measured",
        "instrumentation":"bridge key/data mirror and transparent native search readiness observers add host overhead, reported separately where timed",
        "fixture":"generated compact records at 1000/5000/10000; no full dataset is sent as a host message above 1 MiB"}})
    );
}
