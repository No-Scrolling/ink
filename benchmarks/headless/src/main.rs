use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use ink_core::{Engine, ReactCommit, ReactTree, Scene};
use ink_runtime::{AppRuntime, Event};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Parser)]
struct Args {
    /// Compile the existing update fixture into the assets directory, then exit.
    #[arg(long)]
    prepare: Option<PathBuf>,
    #[arg(long)]
    assets: PathBuf,
    #[arg(long, default_value_t = 200, value_parser = clap::value_parser!(u32).range(1..))]
    iterations: u32,
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..))]
    warmup: u32,
    #[arg(long)]
    native_controls: bool,
    #[arg(long)]
    list_compat: bool,
    /// Expect diagnostic metadata after each native commit.
    #[arg(long)]
    react_profile: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    r#type: String,
    operations: Option<ReactCommit>,
    colour_scheme: Option<String>,
    message: Option<String>,
    react_profile: Option<Value>,
}

struct App {
    runtime: AppRuntime,
    events: ink_runtime::EventReceiver,
    tree: ReactTree,
    engine: Engine,
    react_profile: Option<Value>,
    profiling: bool,
    list_records: usize,
}

struct Sample {
    input: f64,
    javascript: f64,
    decode: f64,
    apply: f64,
    total: f64,
    bytes: Option<usize>,
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

impl App {
    fn commit(&mut self) -> Result<(Instant, f64, f64, Option<usize>)> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let event = self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .context("timed out waiting for a React commit")?;
            match event {
                Event::Ready => {}
                Event::Error(error) => bail!("JavaScript: {error}"),
                Event::Stopped => bail!("JavaScript stopped before committing"),
                Event::Commit(commit) => {
                    let received = Instant::now();
                    self.list_records = commit.0.iter().filter_map(|operation| match operation {
                        ink_protocol::Operation::Update { props, .. } => props.get("itemChanges").or_else(|| props.get("items")).and_then(Value::as_array).map(Vec::len),
                        _ => None,
                    }).sum();
                    self.tree.apply(commit, &mut self.engine)?;
                    self.tree.refresh_native_lists(&mut self.engine)?;
                    let applied = Instant::now();
                    if self.profiling {
                        let Event::Message(source) = self.events.recv_timeout(
                            deadline.saturating_duration_since(Instant::now()),
                        )? else { bail!("missing diagnostic React message"); };
                        let profile: Message = serde_json::from_str(&source)?;
                        ensure!(profile.r#type == "react-profile", "unexpected diagnostic message");
                        self.react_profile = profile.react_profile;
                    }
                    return Ok((received, 0.0, milliseconds(applied - received), None));
                }
                Event::Message(source) => {
                    let received = Instant::now();
                    let message: Message = serde_json::from_str(&source)?;
                    let decoded = Instant::now();
                    match message.r#type.as_str() {
                        "commit" => {
                            self.react_profile = message.react_profile;
                            self.tree.apply(
                                message.operations.context("missing commit operations")?,
                                &mut self.engine,
                            )?;
                            let applied = Instant::now();
                            return Ok((
                                received,
                                milliseconds(decoded - received),
                                milliseconds(applied - decoded),
                                Some(source.len()),
                            ));
                        }
                        "appearance" => {
                            self.engine
                                .set_colour_scheme(match message.colour_scheme.as_deref() {
                                    Some("light") => true,
                                    Some("dark") => false,
                                    _ => bail!("invalid appearance"),
                                });
                        }
                        _ => bail!(
                            "unexpected {} message: {:?}",
                            message.r#type,
                            message.message
                        ),
                    }
                }
            }
        }
    }

    fn tap(&mut self, x: f32, y: f32) -> Result<Sample> {
        #[cfg(target_os = "android")]
        let _affinity = ink_runtime::cpu_affinity::prefer_performance();
        self.engine.pointer_down(1, x, y);
        let start = Instant::now();
        ensure!(
            self.engine.pointer_up(1, x, y).activated,
            "tap missed its target at {x},{y}"
        );
        let request = self
            .engine
            .take_native_request()
            .context("tap produced no event")?;
        ensure!(
            request.module() == "ink" && request.operation() == "event",
            "unexpected native request"
        );
        let dispatched = Instant::now();
        self.runtime.send(self.tree.route_native_message(request.payload().to_owned())?)?;
        self.engine.complete_native_action(request.id());
        ensure!(
            self.engine.take_native_request().is_none(),
            "tap produced multiple events"
        );
        let revision = self.engine.scene().revision;
        let (received, decode, apply, bytes) = self.commit()?;
        let total = milliseconds(received - start) + decode + apply;
        ensure!(
            self.engine.scene().revision > revision,
            "commit did not update the scene"
        );
        Ok(Sample {
            input: milliseconds(dispatched - start),
            javascript: milliseconds(received - dispatched),
            decode,
            apply,
            total,
            bytes,
        })
    }

    fn label(&mut self, label: &str) -> Result<Sample> {
        let scene = self.engine.scene();
        let run = scene.text
            .iter()
            .find(|run| &*run.text == label)
            .with_context(|| format!("missing label {label}"))?;
        let rect = run.rect;
        let scroll = if run.scrolling { scene.scroll_offset - scene.scroll_origin } else { 0.0 };
        self.tap(rect.x + 2.0, rect.y + rect.height / 2.0 - scroll)
    }
}

fn cells(scene: &Scene, count: usize, tick: u32, reverse: bool) -> Result<()> {
    let rows: Vec<_> = scene
        .text
        .iter()
        .filter(|run| run.text.as_bytes().get(3) == Some(&b':'))
        .collect();
    ensure!(
        rows.len() == count,
        "expected {count} cells, got {}",
        rows.len()
    );
    for (index, run) in rows.iter().enumerate() {
        let id = if reverse { count - 1 - index } else { index };
        ensure!(
            run.text.as_ref() == format!("{id:03}:{}", (id + tick as usize) % 100),
            "incorrect cell {index}: {}",
            run.text
        );
        ensure!(
            run.rect.x.is_finite() && run.rect.y.is_finite() && run.rect.width > 0.0,
            "invalid cell geometry"
        );
    }
    Ok(())
}

fn check_scroll(app: &mut App) -> Result<()> {
    let before = app.engine.scene().clone();
    let bar = before.scroll_bar.context("500 cells must scroll")?;
    let thumb = bar.thumb_rect(before.scroll_offset, before.scroll_max);
    let x = thumb.x + thumb.width / 2.0;
    let y = thumb.y + thumb.height / 2.0;
    app.engine.pointer_down(7, x, y);
    app.engine.pointer_move(7, x, y + 80.0);
    app.engine.pointer_up(7, x, y + 80.0);
    ensure!(
        app.engine.scene().scroll_offset > before.scroll_offset,
        "thumb drag did not scroll content"
    );
    app.engine.scroll_by(-100_000.0);
    let clip = before.scroll_clip.context("missing scroll clip")?;
    let x = clip.x + clip.width / 2.0;
    let y = clip.y + clip.height * 0.8;
    app.engine.pointer_down(8, x, y);
    app.engine.pointer_move(8, x, y - 100.0);
    app.engine.pointer_up(8, x, y - 100.0);
    let scene = app.engine.scene();
    ensure!(scene.scroll_offset > 0.0, "content drag did not scroll");
    ensure!(
        scene
            .scroll_bar
            .context("scrollbar disappeared")?
            .thumb_rect(scene.scroll_offset, scene.scroll_max)
            .y
            > thumb.y,
        "content drag did not move thumb"
    );
    app.engine.scroll_by(-100_000.0);
    Ok(())
}

fn summary(mut values: Vec<f64>) -> Value {
    values.sort_by(f64::total_cmp);
    let count = values.len();
    let median = if count % 2 == 0 {
        (values[count / 2 - 1] + values[count / 2]) / 2.0
    } else {
        values[count / 2]
    };
    json!({ "mean": values.iter().sum::<f64>() / count as f64, "median": median, "p95": values[(count * 95).div_ceil(100) - 1], "min": values[0], "max": values[count - 1] })
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(project) = args.prepare {
        let project = ink_compiler::Project::load(project.join("ink.toml"))?;
        ink_compiler::compile(&project)?;
        fs::create_dir_all(&args.assets)?;
        for file in ["app.js", "ink-icons-v1.bin"] {
            fs::copy(
                project.android_assets_path().join(file),
                args.assets.join(file),
            )?;
        }
        return Ok(());
    }
    let started = Instant::now();
    let (runtime, events) = AppRuntime::spawn(fs::read_to_string(args.assets.join("app.js"))?)?;
    let mut app = App {
        runtime,
        events,
        react_profile: None,
        profiling: args.react_profile,
        list_records: 0,
        tree: ReactTree::with_icons(&fs::read(args.assets.join("ink-icons-v1.bin"))?)?,
        engine: Engine::new(),
    };
    app.engine.set_viewport(1080, 1240);
    app.commit()?;
    if args.native_controls { return native_controls(&mut app, &args, started); }
    if args.list_compat { return list_compat(&mut app, &args, started); }
    app.label("500")?;
    app.label("Resize off")?;
    cells(app.engine.scene(), 500, 0, false)?;
    let even_rects: Vec<_> = app
        .engine
        .scene()
        .text
        .iter()
        .filter(|run| run.text.as_bytes().get(3) == Some(&b':'))
        .map(|run| run.rect)
        .collect();
    let mount_ms = milliseconds(started.elapsed());
    let updates = args
        .warmup
        .checked_add(args.iterations)
        .context("too many updates")?;
    let mut samples = Vec::with_capacity(args.iterations as usize);
    let mut react_profiles = Vec::new();
    let mut validation = Duration::ZERO;
    let mut warmup_ms = 0.0;
    for tick in 1..=updates {
        let sample = app.tap(984.0, 64.0)?;
        let check = Instant::now();
        cells(app.engine.scene(), 500, tick, false)?;
        for (index, run) in app
            .engine
            .scene()
            .text
            .iter()
            .filter(|run| run.text.as_bytes().get(3) == Some(&b':'))
            .enumerate()
        {
            let previous = even_rects[index];
            let gap = if tick % 2 == 1 {
                previous.width / 30.0
            } else {
                0.0
            };
            let expected_x = previous.x + (index % 10) as f32 * gap;
            ensure!(
                (run.rect.x - expected_x).abs() < 0.01 && run.rect.y == previous.y,
                "incorrect resized cell geometry at {index}"
            );
        }
        validation += check.elapsed();
        if tick > args.warmup {
            samples.push(sample);
            if let Some(profile) = app.react_profile.take() {
                react_profiles.push(profile);
            }
        } else {
            warmup_ms += sample.total;
        }
    }
    let compatibility_started = Instant::now();
    let tick = updates;
    check_scroll(&mut app)?;
    app.label("Reverse")?;
    cells(app.engine.scene(), 500, tick, true).context("after reversing")?;
    app.label("Hide")?;
    cells(app.engine.scene(), 0, tick, false)?;
    ensure!(
        app.engine.scene().scroll_bar.is_none(),
        "hidden content left a scrollbar"
    );
    app.label("Show")?;
    // Activity can commit the visible shell before revealing its children.
    if !app
        .engine
        .scene()
        .text
        .iter()
        .any(|run| run.text.starts_with("000:"))
    {
        app.commit()?;
    }
    cells(app.engine.scene(), 500, tick, true).context("after showing")?;
    app.label("100")?;
    cells(app.engine.scene(), 100, 0, true)?;
    ensure!(
        app.engine.scene().scroll_bar.is_none(),
        "100 cells should fit without a scrollbar"
    );
    let compatibility_ms = milliseconds(compatibility_started.elapsed());
    let shutdown_started = Instant::now();
    drop(app);
    let shutdown_ms = milliseconds(shutdown_started.elapsed());
    let suite_ms = milliseconds(started.elapsed());
    let input_ms: f64 = samples.iter().map(|s| s.input).sum();
    let javascript_ms: f64 = samples.iter().map(|s| s.javascript).sum();
    let decode_ms: f64 = samples.iter().map(|s| s.decode).sum();
    let apply_ms: f64 = samples.iter().map(|s| s.apply).sum();
    let validation_ms = milliseconds(validation);
    let harness_ms = suite_ms
        - mount_ms
        - warmup_ms
        - input_ms
        - javascript_ms
        - decode_ms
        - apply_ms
        - validation_ms
        - compatibility_ms
        - shutdown_ms;
    ensure!(
        react_profiles.is_empty() || react_profiles.len() == samples.len(),
        "missing React profiles"
    );
    let mut result = json!({
        "workload": "500 cells + resize", "iterations": args.iterations, "warmup": args.warmup,
        "platform": {"os": std::env::consts::OS, "arch": std::env::consts::ARCH},
        "mountAndSetupMs": mount_ms, "cellValidationMs": validation_ms,
        "suiteMs": suite_ms,
        "waterfallMs": {
            "mountAndSetup": mount_ms,
            "warmupUpdates": warmup_ms,
            "measuredInput": input_ms,
            "measuredJavascriptAndTransport": javascript_ms,
            "measuredDecode": decode_ms,
            "measuredApplyAndLayout": apply_ms,
            "cellValidation": validation_ms,
            "compatibilityChecks": compatibility_ms,
            "shutdown": shutdown_ms,
            "harnessOverhead": harness_ms,
        },
        "milliseconds": {
            "inputToDispatch": summary(samples.iter().map(|s| s.input).collect()),
            "javascriptAndTransport": summary(samples.iter().map(|s| s.javascript).collect()),
            "decode": summary(samples.iter().map(|s| s.decode).collect()),
            "applyAndLayout": summary(samples.iter().map(|s| s.apply).collect()),
            "inputToScene": summary(samples.iter().map(|s| s.total).collect()),
        },
        "inputToSceneSamplesMs": samples.iter().map(|sample| sample.total).collect::<Vec<_>>(),
        "commitBytes": samples.iter().all(|s| s.bytes.is_some()).then(|| summary(samples.iter().filter_map(|s| s.bytes.map(|b| b as f64)).collect())),
        "transport": "native batch",
        "checks": ["all 500 labels on every update", "row gap alternation", "thumb drag", "content drag", "reverse", "hide/show", "count and scrollbar change"],
        "note": "CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown."
    });
    if !react_profiles.is_empty() {
        result["reactProfiles"] = json!(react_profiles);
        result["note"] = json!(format!(
            "{} React diagnostic instrumentation is enabled; use uninstrumented runs for speed comparisons.",
            result["note"].as_str().unwrap()
        ));
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn native_controls(app: &mut App, args: &Args, started: Instant) -> Result<()> {
    let mut scroll_samples = Vec::new();
    for _ in 0..100 {
        let now = Instant::now();
        app.engine.scroll_by(1000.0);
        let events = app.tree.viewport_events(&mut app.engine)?;
        scroll_samples.push(milliseconds(now.elapsed()));
        ensure!(events.is_empty(), "native scrolling requested JavaScript window work");
        ensure!(app.engine.list_viewports_ready(), "native viewport left unloaded rows");
        ensure!(app.engine.scene().text.len() < 150, "native list rendered the whole dataset");
    }
    let scene = app.engine.scene();
    let clip = scene.scroll_clip.context("missing list viewport")?;
    let label = scene.text.iter().find(|run| run.text.starts_with("Row ")
        && run.rect.y - (scene.scroll_offset - scene.scroll_origin) >= clip.y
        && run.rect.y - (scene.scroll_offset - scene.scroll_origin) + run.rect.height <= clip.y + clip.height)
        .context("no visible row")?.text.to_string();
    let key = label.replace("Row ", "row-");
    app.label(&label)?;
    ensure!(app.engine.scene().text.iter().any(|run| run.text == key), "row action lost its key");
    app.label("Prepend")?;
    ensure!(app.engine.scene().text.iter().any(|run| run.text == "1001 keyed rows"), "prepend failed");
    app.label("Reverse")?;
    ensure!(app.engine.scene().text.iter().any(|run| run.text == label), "reverse lost visible anchor");
    app.label("Remove")?;
    ensure!(app.engine.scene().text.iter().any(|run| run.text == "1000 keyed rows"), "keyed removal failed");
    ensure!(!app.engine.scene().text.iter().any(|run| run.text == label), "removed row still present");
    app.label("Reset")?;
    app.engine.scroll_by(-f32::MAX);
    app.tree.refresh_native_lists(&mut app.engine)?;
    let scene = app.engine.scene();
    let input = scene.text.iter().find(|run| run.text == "Edit this row").context("missing row input")?;
    let x = input.rect.x + 2.0;
    let y = input.rect.y + input.rect.height / 2.0 - scene.scroll_offset + scene.scroll_origin;
    app.engine.pointer_down(1, x, y);
    app.engine.pointer_up(1, x, y);
    ensure!(app.engine.text_input_active(), "row input did not focus");
    let mut edit: Value = serde_json::from_str(&app.engine.text_input_context())?;
    edit["value"] = json!("日本語 👩🏽‍🚀");
    edit["selection"] = json!("日本語 👩🏽‍🚀".encode_utf16().count());
    ensure!(app.engine.edit_text(ink_core::TextEdit::Update(edit.to_string())), "row edit rejected");
    let events = app.tree.input_events(&app.engine);
    ensure!(events.len() == 1, "row edit produced unexpected events");
    let event = app.tree.route_native_message(events[0].to_string())?;
    ensure!(serde_json::from_str::<Value>(&event)?["args"][0] == "row-0", "edit routed to the wrong key");
    app.runtime.send(event)?;
    app.commit()?;
    app.engine.edit_text(ink_core::TextEdit::Dismiss);
    app.engine.scroll_by(100_000.0);
    app.tree.refresh_native_lists(&mut app.engine)?;
    app.engine.scroll_by(-f32::MAX);
    app.tree.refresh_native_lists(&mut app.engine)?;
    ensure!(app.engine.scene().text.iter().any(|run| run.text == "日本語 👩🏽‍🚀"), "recycled row lost its edited value");
    app.engine.pointer_down(1, x, y);
    app.engine.pointer_up(1, x, y);
    let mut edit: Value = serde_json::from_str(&app.engine.text_input_context())?;
    edit["value"] = json!("Edited again");
    edit["selection"] = json!(12);
    ensure!(app.engine.edit_text(ink_core::TextEdit::Update(edit.to_string())), "recycled input edit rejected");
    let events = app.tree.input_events(&app.engine);
    ensure!(events.len() == 1 && events[0]["args"][1] == 2, "recycling reset the input acknowledgement count");
    app.runtime.send(app.tree.route_native_message(events[0].to_string())?)?;
    app.commit()?;
    app.engine.edit_text(ink_core::TextEdit::Dismiss);
    let mut samples = Vec::new();
    for tick in 1..=args.warmup + args.iterations {
        let sample = app.label("Refresh")?;
        for run in &app.engine.scene().text {
            if run.text.contains(" · ") { ensure!(run.text.ends_with(&format!(" · {tick}")), "stale row after refresh"); }
        }
        if tick > args.warmup { samples.push(sample); }
    }
    let totals: Vec<_> = samples.iter().map(|sample| sample.total).collect();
    println!("{}", json!({
        "workload":"1000 keyed rows; update data and materialise native viewport", "iterations":args.iterations,"warmup":args.warmup,
        "milliseconds":{
            "inputToScene":summary(totals.clone()),
            "javascriptAndTransport":summary(samples.iter().map(|s|s.javascript).collect()),
            "applyAndLayout":summary(samples.iter().map(|s|s.apply).collect()),
            "nativeScroll":summary(scroll_samples),
        },
        "inputToSceneSamplesMs":totals,"suiteMs":milliseconds(started.elapsed()),"waterfallMs":{},
        "checks":["native-only scroll", "bounded visible scene", "row action key", "prepend", "reverse anchor", "keyed deletion", "controlled Unicode input survives recycling", "monotonic input acknowledgements", "all mounted labels updated"],
        "note":"CPU only. Native scrolling includes window materialisation and layout; no JS window event. Data updates replace all 1000 records in JavaScript. This is separate from the all-500-cells resize workload."
    }));
    Ok(())
}

fn list_compat(app: &mut App, args: &Args, started: Instant) -> Result<()> {
    fn windows(app: &mut App) -> Result<usize> {
        let mut count = 0;
        for _ in 0..8 {
            let events = app.tree.viewport_events(&mut app.engine)?;
            if events.is_empty() { break; }
            count += events.len();
            for event in events { app.runtime.send(event.to_string())?; }
            app.runtime.send(json!({"type":"list-barrier"}).to_string())?;
            loop {
                match app.events.recv_timeout(Duration::from_secs(5))? {
                    Event::Commit(commit) => { app.tree.apply(commit, &mut app.engine)?; app.tree.refresh_native_lists(&mut app.engine)?; }
                    Event::Message(source) if serde_json::from_str::<Value>(&source)?["type"] == "list-settled" => break,
                    Event::Error(error) => bail!("JavaScript: {error}"),
                    Event::Ready => {},
                    _ => bail!("unexpected event while settling list window"),
                }
            }
        }
        ensure!(app.engine.list_viewports_ready(), "list left visible rows unloaded");
        Ok(count)
    }
    fn visible_label(app: &App) -> Result<String> {
        let scene = app.engine.scene();
        let clip = scene.scroll_clip.context("missing list clip")?;
        scene.text.iter().find(|run| run.text.starts_with("Row ")
            && run.rect.y - scene.scroll_offset + scene.scroll_origin >= clip.y
            && run.rect.y + run.rect.height - scene.scroll_offset + scene.scroll_origin <= clip.y + clip.height)
            .map(|run| run.text.to_string()).context("missing visible row")
    }
    let mut results = serde_json::Map::new();
    let mut window_callbacks = serde_json::Map::new();
    for mode in ["Native", "React", "Row", "Rich"] {
        if mode != "Native" { app.label(mode).with_context(|| format!("select mode {mode}"))?; }
        app.engine.scroll_by(-f32::MAX); windows(app)?;
        let mut samples = Vec::new(); let mut callbacks = 0;
        for index in 0..args.warmup + args.iterations {
            let before = Instant::now();
            app.engine.scroll_by(if (index / 50) % 2 == 0 { 1000.0 } else { -1000.0 });
            let count = windows(app)?;
            let elapsed = milliseconds(before.elapsed());
            callbacks += count;
            if index >= args.warmup { samples.push(elapsed); }
            ensure!(app.engine.scene().text.len() < 160, "list rendered all rows");
            visible_label(app)?;
        }
        ensure!((callbacks == 0) == matches!(mode, "Native" | "Row"), "wrong window owner for {mode}: {callbacks} callbacks");
        let label = visible_label(app)?;
        let key = label.replace("Row ", "row-");
        app.label(&label).with_context(|| format!("select row {label} in {mode}"))?;
        let capture = app.engine.scene().text.iter().find_map(|run| run.text.strip_prefix("Capture ")).unwrap_or("0");
        let expected = if mode == "React" { format!("{key}:1") } else if capture != "0" { format!("{key}@{capture}") } else { key.clone() };
        ensure!(app.engine.scene().text.iter().any(|run| run.text == expected), "row action failed for {mode}");
        if mode == "React" {
            app.label(&label).with_context(|| format!("select row {label} in {mode}"))?;
            ensure!(app.engine.scene().text.iter().any(|run| run.text == format!("{key}:2")), "custom React state was lost");
        }
        app.label("Prepend").with_context(|| format!("prepend in {mode}"))?; windows(app)?;
        ensure!(app.engine.scene().text.iter().any(|run| run.text == format!("{mode}: 1001 rows")), "prepend failed");
        if matches!(mode, "Native" | "Row") {
            app.label("Reverse").with_context(|| format!("reverse in {mode}"))?; windows(app)?;
            ensure!(app.engine.scene().text.iter().any(|run| run.text == label), "native reorder lost anchor");
        }
        app.label("Reset").with_context(|| format!("reset in {mode}"))?; windows(app)?;
        results.insert(format!("{mode}Scroll"), summary(samples));
        window_callbacks.insert(mode.into(), json!(callbacks));
        if matches!(mode, "Native" | "Row") {
            app.engine.scroll_by(-f32::MAX); windows(app)?;
            let mut unrelated = Vec::new();
            let mut edits = Vec::new();
            for index in 0..args.warmup + args.iterations {
                let sample = app.label("Unrelated")?;
                ensure!(app.list_records == 0, "unrelated state resent list records");
                if index >= args.warmup { unrelated.push(sample.total); }
                let sample = app.label("Edit")?;
                ensure!(app.list_records == 1, "single-row edit did not send exactly one record");
                if index >= args.warmup { edits.push(sample.total); }
                ensure!(app.engine.scene().text.iter().any(|run| run.text.starts_with("Edited ")), "single row edit was not applied");
            }
            let capture = app.engine.scene().text.iter().find_map(|run| run.text.strip_prefix("Capture ").map(str::to_owned)).context("missing capture")?;
            app.label("Row 1")?;
            ensure!(app.engine.scene().text.iter().any(|run| run.text == format!("row-1@{capture}")), "unchanged row callback retained stale state");
            let edited = app.engine.scene().text.iter().find(|run| run.text.starts_with("Edited ")).context("missing edited row")?.text.to_string();
            app.label(&edited)?;
            ensure!(app.engine.scene().text.iter().any(|run| run.text == format!("row-0@{capture}")), "row callback retained stale state");
            app.label("Delete")?; windows(app)?;
            ensure!(!app.engine.scene().text.iter().any(|run| run.text.starts_with("Edited ")), "deleted row remained mounted");
            app.label("Reset")?; windows(app)?;
            results.insert(format!("{mode}Unrelated"), summary(unrelated));
            results.insert(format!("{mode}Edit"), summary(edits));
        }
    }
    app.label("Built-ins")?; windows(app)?;
    for _ in 0..20 {
        app.engine.scroll_by(1500.0);
        ensure!(windows(app)? == 0, "ReorderList required a JavaScript window callback");
    }
    app.engine.scroll_by(-f32::MAX); windows(app)?;
    let row = app.engine.scene().text.iter().find(|run| run.text == "Row 0").context("missing reorder row")?.rect;
    let mut arrows: Vec<_> = app.engine.scene().masks.iter().filter(|mask| (mask.rect.y + mask.rect.height / 2.0 - row.y - row.height / 2.0).abs() < 40.0)
        .map(|mask| mask.rect).collect();
    arrows.sort_by(|a, b| a.x.total_cmp(&b.x));
    let down = arrows.first().context("missing reorder arrow")?;
    app.tap(down.x + down.width / 2.0, down.y + down.height / 2.0)?;
    let first = app.engine.scene().text.iter().find(|run| run.text == "Row 1").context("missing moved row")?.rect.y;
    let second = app.engine.scene().text.iter().find(|run| run.text == "Row 0").context("missing moved row")?.rect.y;
    ensure!(first < second, "ReorderList callback did not reorder rows");
    app.label("Messages")?; windows(app)?;
    ensure!(app.engine.scene().text.iter().any(|run| run.text.contains("Row 999")), "conversation did not open at the end");
    for _ in 0..20 {
        app.engine.scroll_by(-1500.0);
        ensure!(windows(app)? == 0, "ConversationScreen required a JavaScript window callback");
        ensure!(app.engine.scene().text.len() < 160, "conversation materialised every message");
    }
    println!("{}", json!({"workload":"Existing List API: native templates and React compatibility", "iterations":args.iterations,"warmup":args.warmup,
        "milliseconds":results,"javascriptWindowCallbacks":window_callbacks,"suiteMs":milliseconds(started.elapsed()),"waterfallMs":{},
        "checks":["Text/Stack compiled to native", "Row compiled to native", "custom React state preserved", "bounded windows", "visible rows loaded", "row callbacks", "prepend", "native reverse anchor", "single row edits", "fresh callback state", "deletion", "zero records for unrelated updates; one record per edit", "native reorder list scroll and actions", "native conversation initial end and scrolling"],
        "note":"CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing."
    }));
    Ok(())
}
