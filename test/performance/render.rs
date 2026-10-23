//! Host renderer workloads on the selected offscreen Vulkan device.
use ink_core::{
    Colour, Engine, ImageAnimation, ImageData, ImageFit, ImageRun, ReactTree, Rect, RemoteImage, Scene,
    TextAlign, TextRun,
};
use ink_renderer_vulkan::{RenderOutcome, Renderer};
use ink_runtime::{AppRuntime, Event, EventReceiver};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

const WIDTH: u32 = 1080;
const HEIGHT: u32 = 1240;
const TIMEOUT: Duration = Duration::from_secs(10);

#[path = "artwork.rs"]
mod artwork;
#[path = "render-coverage.rs"]
mod coverage;

/// Pixels returned by `ImageAnimation::advance`.
type FramePixels = Arc<[u8]>;

#[derive(Default)]
struct Sample {
    cpu_ns: u64,
    gpu_ns: Option<u64>,
    prepare_ns: u64,
    upload_ns: u64,
    uploaded_bytes: u64,
    cache_misses: u64,
    completion_readback_ns: u64,
    elapsed_ns: u64,
    submit_present_ns: u64,
    acquire_ns: u64,
    encode_ns: u64,
    frame_ns: u64,
    instances: u64,
    draw_calls: u64,
    staging_allocations: u64,
    staging_allocation_bytes: u64,
    staging_upload_bytes: u64,
    staging_buffer_capacity_bytes: u64,
}

fn main() {
    let mut arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let coverage_only = arguments
        .first()
        .is_some_and(|value| value == "--coverage-only");
    if coverage_only {
        arguments.remove(0);
    }
    let mut arguments = arguments.into_iter();
    let samples = arguments.next().map_or(60, |value| value.parse().unwrap());
    let warmup = arguments.next().map_or(10, |value| value.parse().unwrap());
    assert!(samples > 0, "samples must be positive");
    assert!(arguments.next().is_none(), "unexpected argument");
    let probe = Renderer::offscreen(WIDTH, HEIGHT).expect("offscreen Vulkan renderer");
    let device = probe.device_name();
    drop(probe);
    let mut workloads = if coverage_only {
        Vec::new()
    } else {
        vec![
            gif_frame("gif-frame-thumbnail", None, samples, warmup),
            gif_frame("gif-frame-full-width", Some((1080, 608)), samples, warmup),
            first_frame("first-frame-rows", samples.min(30), warmup.min(4), |_| rows_scene()),
            first_frame("first-frame-dense-text", samples.min(30), warmup.min(4), dense_text_scene),
            atlas_recovery("atlas-recovery-1-mib", 140.0, samples.min(40), warmup.min(4)),
            atlas_recovery("atlas-recovery-4-mib", 280.0, samples.min(40), warmup.min(4)),
            atlas_recovery("atlas-recovery-16-mib", 560.0, samples.min(40), warmup.min(4)),
            artwork_revisit(samples.min(12), warmup.min(2)),
            distant_text_edits(samples, warmup),
        ]
    };
    if !coverage_only {
        workloads.extend(artwork::run(samples.min(8), warmup.min(2)));
    }
    workloads.extend(coverage::run(samples, warmup));
    println!(
        "{}",
        json!({"version":1, "device":device, "offscreen":true,
        "frame":[WIDTH, HEIGHT], "coverage_only":coverage_only,
        "boundaries": {
            "cpu_ns":"wall time inside Renderer::render: preparation, upload packaging, encoding and submission; excludes construction and completion wait",
            "submit_present_ns":"CPU submit/present section; offscreen has no presentation",
            "prepare_ns":"CPU scene preparation, glyph rasterisation and cache work, including queuing texture writes",
            "upload_ns":"CPU upload packaging and staging writes, including the scroll/overlay writes",
            "gpu_ns":"GPU render command timestamps, including offscreen image-to-buffer readback; excludes the preceding upload command buffer",
            "completion_readback_ns":"wall time after render returns: fence wait plus copying mapped readback bytes into a host Vec",
            "elapsed_ns":"render call plus completion/readback; excludes fixture generation, construction, correctness checks and file writes",
            "staging_allocations":"actual staging buffer allocations at the upload allocation seam; excludes instance and readback buffers",
            "staging_allocation_bytes":"Vulkan memory requirement bytes allocated for those staging buffers",
            "staging_buffer_capacity_bytes":"maximum selected staging buffer capacity in the sample; a gauge, not allocated bytes",
            "staging_upload_bytes":"used staging bytes including alignment padding"
        }, "workloads":workloads})
    );
}

fn artwork_revisit(samples: usize, warmup: usize) -> Value {
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    let images = (0..48).map(|index| ImageData::Remote(RemoteImage {
        id: index + 10_000, generation: 1, width: 256, height: 256,
        pixels: [(index * 5) as u8, (index * 5) as u8, (index * 5) as u8, 255].repeat(256 * 256).into(),
    })).collect::<Vec<_>>();
    let mut revision = 0;
    let mut results = Vec::new();
    let mut peak_texture_bytes = 0;
    for step in 0..warmup + samples {
        // Revisit a small library between scans of different album covers.
        let ids = (0..6).cycle().take(24)
            .chain((6..48).map(|index| 6 + (index - 6 + step * 7) % 42));
        let mut total = Sample::default();
        for id in ids {
            revision += 1;
            let rect = Rect { x: 0.0, y: 0.0, width: 256.0, height: 256.0 };
            let scene = Scene { revision, width: WIDTH, height: HEIGHT,
                images: vec![ImageRun { avatar: None, image: images[id].clone(), zoom_id: None,
                    rect, clip: rect, fit: ImageFit::Contain, scrolling: false, transform: Default::default() }],
                ..Scene::default() };
            let sample = render(&mut renderer, &scene);
            peak_texture_bytes = peak_texture_bytes.max(renderer.memory_metrics().image_texture_bytes);
            total.accumulate(sample);
            let pixels = renderer.pixels().unwrap();
            let offset = (128 * WIDTH as usize + 128) * 4;
            for channel in 0..3 {
                assert!(pixels[offset + channel].abs_diff((id * 5) as u8) <= 1);
            }
            assert_eq!(pixels[offset + 3], 255);
        }
        if step >= warmup { results.push(total); }
    }
    summary("artwork-revisit", results, json!({"images":48, "image_bytes":262144,
        "peak_texture_bytes":peak_texture_bytes}))
}

fn distant_text_edits(samples: usize, warmup: usize) -> Value {
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    let mut scene = dense_text_scene(0);
    render(&mut renderer, &scene);
    let last = scene.text.len() - 1;
    let mut results = Vec::new();
    for step in 0..warmup + samples {
        scene.revision += 1;
        let label = if step % 2 == 0 { "1111111111" } else { "2222222222" };
        scene.text[0].text = label.into();
        scene.text[last].text = label.into();
        let sample = render(&mut renderer, &scene);
        if step >= warmup { results.push(sample); }
    }
    let pixels = renderer.pixels().unwrap();
    let pixel_hash = pixels.iter().fold(0xcbf29ce484222325u64,
        |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3));
    summary("distant-text-edits", results, json!({"changed_runs":2, "runs":scene.text.len(),
        "rgba_hash":format!("{pixel_hash:016x}")}))
}

fn summary(name: &str, samples: Vec<Sample>, extra: Value) -> Value {
    let median = |values: &mut Vec<u64>| {
        values.sort_unstable();
        values.get(values.len().saturating_sub(1) / 2).copied()
    };
    let p95 = |values: &[u64]| {
        values
            .get((values.len() * 95).div_ceil(100).saturating_sub(1))
            .copied()
    };
    let mut cpu = samples.iter().map(|sample| sample.cpu_ns).collect::<Vec<_>>();
    let mut gpu = samples.iter().filter_map(|sample| sample.gpu_ns).collect::<Vec<_>>();
    let mut prepare = samples.iter().map(|sample| sample.prepare_ns).collect::<Vec<_>>();
    let mut upload = samples.iter().map(|sample| sample.upload_ns).collect::<Vec<_>>();
    let mut bytes = samples.iter().map(|sample| sample.uploaded_bytes).collect::<Vec<_>>();
    let mut misses = samples.iter().map(|sample| sample.cache_misses).collect::<Vec<_>>();
    let mut report = json!({
        "name": name,
        "samples": samples.len(),
        "cpu_ns_median": median(&mut cpu),
        "cpu_ns_p95": p95(&cpu),
        "gpu_ns_median": median(&mut gpu),
        "prepare_ns_median": median(&mut prepare),
        "upload_ns_median": median(&mut upload),
        "uploaded_bytes_median": median(&mut bytes),
        "cache_misses_median": median(&mut misses),
        "contracts": extra,
        "raw_samples": samples.iter().map(Sample::json).collect::<Vec<_>>(),
    });
    for key in [
        "gpu_ns",
        "prepare_ns",
        "upload_ns",
        "uploaded_bytes",
        "cache_misses",
        "completion_readback_ns",
        "elapsed_ns",
        "submit_present_ns",
        "acquire_ns",
        "encode_ns",
        "frame_ns",
        "instances",
        "draw_calls",
        "staging_allocations",
        "staging_allocation_bytes",
        "staging_upload_bytes",
        "staging_buffer_capacity_bytes",
    ] {
        let mut values = samples
            .iter()
            .filter_map(|sample| sample.json()[key].as_u64())
            .collect();
        report[format!("{key}_median")] = json!(median(&mut values));
        report[format!("{key}_p95")] = json!(p95(&values));
    }
    report
}

impl Sample {
    fn accumulate(&mut self, sample: Self) {
        self.gpu_ns = match (self.gpu_ns, sample.gpu_ns) {
            (None, Some(value)) if self.cpu_ns == 0 => Some(value),
            (Some(total), Some(value)) => Some(total + value),
            _ => None,
        };
        self.cpu_ns += sample.cpu_ns;
        self.prepare_ns += sample.prepare_ns;
        self.upload_ns += sample.upload_ns;
        self.uploaded_bytes += sample.uploaded_bytes;
        self.cache_misses += sample.cache_misses;
        self.completion_readback_ns += sample.completion_readback_ns;
        self.elapsed_ns += sample.elapsed_ns;
        self.submit_present_ns += sample.submit_present_ns;
        self.acquire_ns += sample.acquire_ns;
        self.encode_ns += sample.encode_ns;
        self.frame_ns += sample.frame_ns;
        self.instances += sample.instances;
        self.draw_calls += sample.draw_calls;
        self.staging_allocations += sample.staging_allocations;
        self.staging_allocation_bytes += sample.staging_allocation_bytes;
        self.staging_upload_bytes += sample.staging_upload_bytes;
        self.staging_buffer_capacity_bytes = self
            .staging_buffer_capacity_bytes
            .max(sample.staging_buffer_capacity_bytes);
    }

    fn json(&self) -> Value {
        json!({"cpu_ns":self.cpu_ns, "gpu_ns":self.gpu_ns,
            "prepare_ns":self.prepare_ns, "upload_ns":self.upload_ns,
            "uploaded_bytes":self.uploaded_bytes, "cache_misses":self.cache_misses,
            "completion_readback_ns":self.completion_readback_ns,"elapsed_ns":self.elapsed_ns,
            "submit_present_ns":self.submit_present_ns,"acquire_ns":self.acquire_ns,
            "encode_ns":self.encode_ns,"frame_ns":self.frame_ns,"instances":self.instances,
            "draw_calls":self.draw_calls,"staging_allocations":self.staging_allocations,
            "staging_allocation_bytes":self.staging_allocation_bytes,
            "staging_upload_bytes":self.staging_upload_bytes,
            "staging_buffer_capacity_bytes":self.staging_buffer_capacity_bytes})
    }
}

/// Times one `render` call, then waits for the GPU so the frame's timestamps are available.
fn render(renderer: &mut Renderer, scene: &Scene) -> Sample {
    while !renderer.frame_ready() {
        std::hint::spin_loop();
    }
    renderer.take_perf_metrics();
    let start = Instant::now();
    let outcome = renderer.render(scene).expect("render");
    let cpu_ns = start.elapsed().as_nanos() as u64;
    assert!(
        matches!(outcome, RenderOutcome::Presented),
        "unexpected outcome {outcome:?}"
    );
    let completion = Instant::now();
    renderer.pixels().expect("frame completes");
    let completion_readback_ns = completion.elapsed().as_nanos() as u64;
    let elapsed_ns = start.elapsed().as_nanos() as u64;
    let perf = renderer.take_perf_metrics();
    Sample {
        cpu_ns,
        gpu_ns: perf.gpu_ns,
        prepare_ns: perf.prepare_ns,
        upload_ns: perf.upload_ns,
        uploaded_bytes: perf.uploaded_bytes,
        cache_misses: perf.cache_misses,
        completion_readback_ns,
        elapsed_ns,
        submit_present_ns: perf.submit_present_ns,
        acquire_ns: perf.acquire_ns,
        encode_ns: perf.encode_ns,
        frame_ns: perf.frame_ns,
        instances: perf.instances,
        draw_calls: perf.draw_calls,
        staging_allocations: perf.staging_allocations,
        staging_allocation_bytes: perf.staging_allocation_bytes,
        staging_upload_bytes: perf.staging_upload_bytes,
        staging_buffer_capacity_bytes: perf.staging_buffer_capacity_bytes,
    }
}

struct App {
    runtime: AppRuntime,
    events: EventReceiver,
    tree: ReactTree,
    engine: Engine,
}

impl App {
    fn rows(image: bool) -> Self {
        let assets = match std::env::var_os("INK_PERF_ASSETS") {
            Some(root) => PathBuf::from(root).join("rows"),
            None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../fixtures/performance/rows/.ink/android/assets"),
        };
        let source = std::fs::read_to_string(assets.join("app.js")).expect("prepared app.js");
        let icons = std::fs::read(assets.join("ink-icons-v1.bin")).expect("prepared icons");
        let (runtime, events) = AppRuntime::spawn(source).unwrap();
        let mut engine = Engine::new();
        engine.set_viewport(WIDTH, HEIGHT);
        let mut app = Self { runtime, events, tree: ReactTree::with_icons(&icons).unwrap(), engine };
        app.wait_for(|_, message| message.is_some_and(|message| message["type"] == "ready"));
        let items = (0..100)
            .map(|index| {
                const TITLES: [&str; 4] = [
                    "Harbour Lights",
                    "An evening recording from the old theatre beside the river",
                    "Málaga at Dusk",
                    "Zürich Sessions",
                ];
                let mut item = json!({"id":index.to_string(),
                    "title":format!("R{index:05} · {}", TITLES[index % 4]),
                    "subtitle":format!("Artist {index:05} · 3:20")});
                if image && index == 0 {
                    item["image"] = json!("ink-file://animated-artwork");
                }
                item
            })
            .collect::<Vec<_>>();
        app.runtime
            .send(json!({"type":"benchmark", "command":"populate", "items":items}).to_string())
            .unwrap();
        app.wait_for(|app, _| {
            app.engine.list_viewports_ready()
                && app.engine.scene().text.iter().any(|run| run.text == "Artist 00000 · 3:20")
        });
        app
    }

    fn wait_for(&mut self, done: impl Fn(&Self, Option<&Value>) -> bool) {
        let deadline = Instant::now() + TIMEOUT;
        if done(self, None) {
            return;
        }
        loop {
            for event in self.tree.viewport_events(&mut self.engine).unwrap() {
                self.runtime.send(event.to_string()).unwrap();
            }
            let message = match self.events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Event::Commit(commit)) => {
                    self.tree.apply(commit, &mut self.engine).unwrap();
                    None
                }
                Ok(Event::Message(message)) => Some(serde_json::from_str::<Value>(&message).unwrap()),
                Ok(Event::Ready) => None,
                _ => panic!("rows fixture did not settle"),
            };
            if done(self, message.as_ref()) {
                return;
            }
        }
    }
}

/// Hands the engine a fresh frame on every advance, like a GIF whose delay has elapsed.
struct Frames(Vec<FramePixels>);

impl ImageAnimation for Frames {
    fn advance(&mut self) -> Option<FramePixels> {
        self.0.pop()
    }
    fn finished(&self) -> bool {
        false
    }
}

fn frame(width: u32, height: u32, step: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let value = ((x as usize + y as usize + step * 7) % 256) as u8;
            pixels.extend_from_slice(&[value, 255 - value, (step % 256) as u8, 255]);
        }
    }
    pixels
}

fn gif_frame(name: &str, size: Option<(u32, u32)>, samples: usize, warmup: usize) -> Value {
    let mut app = App::rows(true);
    let request = app.engine.take_native_request().expect("artwork requests native pixels");
    let (target_width, target_height, _, _) =
        app.engine.image_request_target(request.id()).expect("artwork has a target");
    let (width, height) = size.unwrap_or((target_width, target_height));
    let frames = (0..warmup + samples + 1)
        .rev()
        .map(|step| frame(width, height, step + 1).into())
        .collect::<Vec<FramePixels>>();
    assert!(app
        .engine
        .complete_native_image(request.id(), width, height, frame(width, height, 0), Some(Box::new(Frames(frames))))
        .changed());
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    render(&mut renderer, app.engine.scene());
    let mut results = Vec::new();
    let mut animate_ns = Vec::new();
    for step in 0..warmup + samples {
        let revision = app.engine.scene().image_revision;
        let start = Instant::now();
        assert!(app.engine.animate_scene(), "every sample advances the animation");
        let animate = start.elapsed().as_nanos() as u64;
        let sample = render(&mut renderer, app.engine.scene());
        assert!(app.engine.scene().image_revision > revision);
        let sample = Sample { cpu_ns: sample.cpu_ns + animate, ..sample };
        if step >= warmup {
            animate_ns.push(animate);
            results.push(sample);
        }
    }
    // The last frame must be on screen: compare the artwork's first pixel with the supplied frame.
    let image = app.engine.scene().images.first().expect("artwork remains visible");
    let ImageData::Remote(image) = &image.image else { panic!("expected remote artwork") };
    assert_eq!(&image.pixels[..4], &frame(width, height, warmup + samples)[..4]);
    animate_ns.sort_unstable();
    summary(name, results, json!({"image":[width, height], "animate_ns_median": animate_ns[animate_ns.len() / 2]}))
}

fn rows_scene() -> Scene {
    App::rows(false).engine.scene().clone()
}

/// Latin text at six sizes: about 1,100 glyphs, all new to a fresh renderer.
fn dense_text_scene(_: usize) -> Scene {
    let characters = (' '..='~').chain('¡'..='ÿ').collect::<String>();
    let mut text = Vec::new();
    for (index, size) in [14.0, 18.0, 24.0, 32.0, 40.0, 56.0].into_iter().enumerate() {
        for (line, chunk) in characters.chars().collect::<Vec<_>>().chunks(48).enumerate() {
            text.push(TextRun {
                text: chunk.iter().collect::<String>().into(),
                rect: Rect { x: 8.0, y: 8.0 + (index * 3 + line) as f32 * 64.0, width: 1064.0, height: size * 1.4 },
                clip: Rect { x: 0.0, y: 0.0, width: WIDTH as f32, height: HEIGHT as f32 },
                font_size: size,
                colour: Colour::WHITE,
                align: TextAlign::Start,
                tabular_numbers: false,
                scrolling: false,
            });
        }
    }
    Scene { revision: 1, width: WIDTH, height: HEIGHT, text, ..Scene::default() }
}

fn first_frame(name: &str, samples: usize, warmup: usize, scene: impl Fn(usize) -> Scene) -> Value {
    let mut results = Vec::new();
    for step in 0..warmup + samples {
        let scene = scene(step);
        // A fresh renderer starts with an empty glyph atlas, like the first frame of a new screen.
        let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
        let sample = render(&mut renderer, &scene);
        assert!(sample.cache_misses > 0, "a first frame rasterises glyphs");
        if step >= warmup {
            results.push(sample);
        }
    }
    summary(name, results, json!({}))
}

/// Two screens of large glyphs that each fit the atlas but not together.
fn atlas_scene(size: f32, lower: bool, revision: u64) -> Scene {
    let letters = if lower { 'a'..='z' } else { 'A'..='Z' };
    let letters = letters.collect::<Vec<_>>();
    let text = [size, size * 1.1, size * 1.2]
        .into_iter()
        .enumerate()
        .flat_map(|(index, size)| {
            letters.chunks(12).enumerate().map(move |(line, chunk)| TextRun {
                text: chunk.iter().collect::<String>().into(),
                rect: Rect { x: 0.0, y: (index * 3 + line) as f32 * 40.0, width: WIDTH as f32, height: size * 1.3 },
                clip: Rect { x: 0.0, y: 0.0, width: WIDTH as f32, height: HEIGHT as f32 },
                font_size: size,
                colour: Colour::WHITE,
                align: TextAlign::Start,
                tabular_numbers: false,
                scrolling: false,
            })
        })
        .collect();
    Scene { revision, width: WIDTH, height: HEIGHT, text, ..Scene::default() }
}

fn atlas_recovery(name: &str, size: f32, samples: usize, warmup: usize) -> Value {
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    render(&mut renderer, &atlas_scene(size, false, 1));
    render(&mut renderer, &atlas_scene(size, true, 2));
    let atlas_bytes = renderer.memory_metrics().font_texture_bytes;
    let mut results = Vec::new();
    for step in 0..warmup + samples {
        let scene = atlas_scene(size, step % 2 == 1, step as u64 + 3);
        let sample = render(&mut renderer, &scene);
        // Every switch reclaims the atlas and rasterises the whole screen again.
        assert!(sample.cache_misses >= 100, "switch must rebuild the atlas: {}", sample.cache_misses);
        if step >= warmup {
            results.push(sample);
        }
    }
    assert_eq!(renderer.memory_metrics().font_texture_bytes, atlas_bytes, "the atlas must not grow");
    summary(name, results, json!({"atlas_bytes": atlas_bytes}))
}
