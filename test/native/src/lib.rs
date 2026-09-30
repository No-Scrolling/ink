use ink_core::{Engine, ReactTree};
use ink_runtime::{AppRuntime, Event, EventReceiver};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub const WAIT: Duration = Duration::from_secs(10);

pub fn fixture_path(suite: &str, fixture: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures")
        .join(suite)
        .join(fixture)
        .join(".ink/android/assets")
}

pub fn compiled_runtime(suite: &str, fixture: &str) -> (AppRuntime, EventReceiver) {
    let path = fixture_path(suite, fixture).join("app.js");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "read {}: {error}; run bun test/native/prepare.ts {suite}",
            path.display()
        )
    });
    AppRuntime::spawn(source).unwrap()
}

pub fn next_event(events: &EventReceiver, deadline: Instant) -> Event {
    events
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .expect("runtime did not produce the expected event before the deadline")
}

pub fn message(events: &EventReceiver, predicate: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + WAIT;
    loop {
        match next_event(events, deadline) {
            Event::Message(source) => {
                let value: Value = serde_json::from_str(&source).unwrap();
                if predicate(&value) {
                    return value;
                }
            }
            Event::Error(error) => panic!("runtime error: {error}"),
            Event::Stopped => panic!("runtime stopped before the expected message"),
            _ => {}
        }
    }
}

pub fn command(runtime: &AppRuntime, command: &str) {
    runtime
        .send(json!({"type":"test", "command":command}).to_string())
        .unwrap();
}

pub struct SceneApp {
    pub runtime: AppRuntime,
    pub events: EventReceiver,
    pub tree: ReactTree,
    pub engine: Engine,
    pub native_list_creates: usize,
    pub native_view_creates: usize,
    pub values_commits: usize,
}

impl SceneApp {
    pub fn new(fixture: &str) -> Self {
        let assets = fixture_path("scene", fixture);
        let icons = std::fs::read(assets.join("ink-icons-v1.bin")).unwrap();
        let (runtime, events) = compiled_runtime("scene", fixture);
        let mut engine = Engine::new();
        engine.set_viewport(1080, 1240);
        Self {
            runtime,
            events,
            tree: ReactTree::with_icons(&icons).unwrap(),
            engine,
            native_list_creates: 0,
            native_view_creates: 0,
            values_commits: 0,
        }
    }

    pub fn pump_until(&mut self, predicate: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + WAIT;
        loop {
            self.refresh();
            if predicate(self) {
                return;
            }
            match self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|_| {
                    panic!(
                        "scene did not reach the expected state; visible labels: {:?}",
                        self.labels()
                    )
                }) {
                Event::Commit(commit) => {
                    for operation in &commit.0 {
                        if matches!(
                            operation,
                            ink_protocol::Operation::Create {
                                r#type: ink_protocol::HostKind::NativeList,
                                ..
                            }
                        ) {
                            self.native_list_creates += 1;
                        }
                        if matches!(
                            operation,
                            ink_protocol::Operation::Create {
                                r#type: ink_protocol::HostKind::NativeView,
                                ..
                            }
                        ) {
                            self.native_view_creates += 1;
                        }
                        if matches!(operation, ink_protocol::Operation::Values { .. }) {
                            self.values_commits += 1;
                        }
                    }
                    self.tree.apply(commit, &mut self.engine).unwrap();
                }
                Event::Error(error) => panic!("runtime error: {error}"),
                Event::Stopped => panic!("runtime stopped while waiting for scene"),
                _ => {}
            }
        }
    }

    pub fn refresh(&mut self) {
        for event in self.tree.viewport_events(&mut self.engine).unwrap() {
            self.runtime.send(event.to_string()).unwrap();
        }
    }

    pub fn labels(&self) -> Vec<String> {
        self.engine
            .scene()
            .text
            .iter()
            .map(|run| run.text.to_string())
            .collect()
    }

    pub fn visible_label(&self, prefix: &str) -> (String, f32, f32) {
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
                let x = run.rect.x + run.rect.width / 2.0;
                let centre = y + run.rect.height / 2.0;
                let clip = if run.scrolling {
                    scene.scroll_clip.unwrap()
                } else {
                    run.clip
                };
                (centre >= clip.y
                    && centre < clip.y + clip.height
                    && centre >= run.clip.y
                    && centre < run.clip.y + run.clip.height)
                    .then(|| (run.text.to_string(), x, centre))
            })
            .expect("expected a visible matching label")
    }

    pub fn tap(&mut self, x: f32, y: f32) -> Value {
        self.engine.tap(x, y);
        let request = self
            .engine
            .take_native_request()
            .unwrap_or_else(|| panic!("tap at ({x}, {y}) did not dispatch an action"));
        assert_eq!(request.module(), "ink");
        let routed = self
            .tree
            .route_native_message(request.payload().to_owned())
            .unwrap();
        self.runtime.send(routed).unwrap();
        message(&self.events, |value| value["type"] == "selected")
    }
}
