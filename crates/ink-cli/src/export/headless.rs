use super::*;
use ab_glyph::Font;
use anyhow::bail;
use ink_core::{Engine, ImageFit, NativeRequestKind, ReactTree};
use ink_renderer_vulkan::{RenderOutcome, Renderer};
use ink_runtime::{AppRuntime, Event};
use std::{
    collections::HashMap,
    io::Cursor,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

pub(super) fn capture(project: &Project, marker: &str, args: &ExportArgs) -> Result<Capture> {
    let fixtures = match &args.fixtures {
        Some(path) => serde_json::from_slice::<Value>(&fs::read(resolve_file(project, path)?)?)?,
        None => json!({}),
    };
    ensure!(
        fixtures.is_object(),
        "--fixtures must contain an object with calls, controllers, images or glyphs"
    );
    for key in ["calls", "controllers", "images", "glyphs"] {
        ensure!(
            fixtures.get(key).is_none_or(Value::is_object),
            "fixture {key} must be an object"
        );
    }
    let assets = project.android_assets_path();
    let source = fs::read_to_string(assets.join("app.js"))?;
    let web = assets.join("ink-assets/ink-web.js");
    let (runtime, events) = AppRuntime::spawn_with_web_loader(
        source,
        || {},
        move || fs::read_to_string(&web).with_context(|| format!("read {}", web.display())),
    )?;
    let mut engine = Engine::new();
    engine.set_viewport(WIDTH, HEIGHT);
    let mut tree = ReactTree::with_icons(&fs::read(assets.join("ink-icons-v1.bin"))?)?;
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT)?;
    let mut store = HashMap::<String, Value>::new();
    let deadline = Instant::now() + Duration::from_secs(args.timeout);
    let mut mounted = false;
    let mut ready = args.ready.is_none();
    let mut ready_at = None;
    let mut previous = Vec::new();
    let mut stable = 0;
    loop {
        ensure!(
            Instant::now() < deadline,
            "preview did not reach readiness and stable pixels within {} seconds",
            args.timeout
        );
        match events.recv_timeout(Duration::from_millis(20)) {
            Ok(Event::Commit(commit)) => {
                tree.apply(commit, &mut engine)?;
            }
            Ok(Event::Message(source)) => {
                let message: Value = serde_json::from_str(&source)?;
                match message["type"].as_str() {
                    Some("log") => {
                        let text = message["message"].as_str().unwrap_or_default();
                        ensure!(!text.contains("INK_EXPORT_FAILED"), "{text}");
                        mounted |= text.contains(marker);
                        ready |= args
                            .ready
                            .as_ref()
                            .is_some_and(|ready| text.contains(ready));
                    }
                    Some("error") => bail!("preview failed: {}", message["message"]),
                    Some("call") => {
                        let key = format!(
                            "{}.{}",
                            message["module"].as_str().unwrap_or_default(),
                            message["operation"].as_str().unwrap_or_default()
                        );
                        let value = if let Some(value) = fixtures["calls"].get(&key) {
                            value.clone()
                        } else if message["module"] == "store" {
                            store_call(&message, &mut store)?
                        } else {
                            bail!(
                                "preview needs a fixture for {key}; add it to --fixtures or supply data with --entry/--props/--setup"
                            );
                        };
                        let value = match value {
                            Value::String(value) => value,
                            value => serde_json::to_string(&value)?,
                        };
                        runtime.send(
                            json!({"type":"result", "id":message["id"], "value":value}).to_string(),
                        )?;
                        if message["operation"] == "activate"
                            && let Some(controller) = message["controller"].as_u64()
                            && let Some(state) = fixtures["controllers"]
                                .get(message["module"].as_str().unwrap_or_default())
                        {
                            if message["module"] == "audio" {
                                engine.update_capture_state(controller, state);
                            }
                            runtime.send(json!({"type":"controller", "id":message["controller"], "value":state}).to_string())?;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Error(error)) => bail!("preview JavaScript failed: {error}"),
            Ok(Event::Stopped) => bail!("preview JavaScript stopped before export"),
            Ok(Event::Ready) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => bail!("preview runtime disconnected"),
        }
        for event in tree.viewport_events(&mut engine)? {
            runtime.send(event.to_string())?;
        }
        while let Some(request) = engine.take_native_request() {
            if request.kind() == NativeRequestKind::Image {
                let payload: Value = serde_json::from_str(request.payload())?;
                let source = payload["source"]
                    .as_str()
                    .or_else(|| payload["url"].as_str())
                    .context("invalid preview image source")?;
                let path =
                    if let Some(path) = fixtures["images"].get(source).and_then(Value::as_str) {
                        resolve_file(project, Path::new(path))?
                    } else if request.module() == "assets" {
                        assets.join(source)
                    } else {
                        bail!("preview image needs an images fixture for {source}");
                    };
                let (width, height, fit, _) = engine
                    .image_request_target(request.id())
                    .context("preview image has no target")?;
                let image = decode_image(&path, width, height, fit)?;
                ensure!(
                    engine.complete_native_image(
                        request.id(),
                        image.width(),
                        image.height(),
                        image.into_raw(),
                        None
                    ),
                    "preview image request was rejected"
                );
            } else if request.kind() == NativeRequestKind::Action {
                let message = tree.route_native_message(request.payload().to_owned())?;
                runtime.send(message)?;
                engine.complete_native_action(request.id());
            }
        }
        if mounted && ready && ready_at.is_none() {
            ready_at = Some(Instant::now());
        }
        if !engine.list_viewports_ready() {
            continue;
        }
        for run in &engine.scene().text {
            for grapheme in ink_core::text_graphemes(&run.text)
                .filter(|grapheme| !ink_core::is_emoji_grapheme(grapheme))
            {
                for character in grapheme
                    .chars()
                    .filter(|character| !character.is_whitespace() && !character.is_control())
                {
                    ensure!(
                        ink_core::font_for_character(character)
                            .1
                            .glyph_id(character)
                            .0
                            != 0,
                        "no bundled font supports {character:?}; use an image fixture for this text"
                    );
                }
            }
        }
        match renderer.render(engine.scene())? {
            RenderOutcome::Presented => {}
            RenderOutcome::NeedsSystemGlyph(request) => {
                let path = fixtures["glyphs"][&request.grapheme][request.pixel_size.to_string()]
                    .as_str()
                    .with_context(|| {
                        format!(
                            "preview needs a glyphs fixture for {:?} at {} pixels, or an Ink icon",
                            request.grapheme, request.pixel_size
                        )
                    })?;
                let image = image::open(resolve_file(project, Path::new(path))?)?.into_rgba8();
                ensure!(
                    image.width() == u32::from(request.pixel_size)
                        && image.height() == u32::from(request.pixel_size),
                    "glyph fixture must be {} × {}",
                    request.pixel_size,
                    request.pixel_size
                );
                renderer.install_system_glyph(request.id, Some(image.as_raw()))?;
                continue;
            }
            _ => continue,
        }
        if ready_at.is_none_or(|time| time.elapsed() < Duration::from_millis(args.wait)) {
            continue;
        }
        let pixels = renderer.pixels()?;
        if pixels == previous {
            stable += 1;
        } else {
            stable = 1;
            previous = pixels;
        }
        if stable < 3 {
            continue;
        }
        let mut png = Cursor::new(Vec::new());
        image::RgbaImage::from_raw(WIDTH, HEIGHT, previous.clone())
            .context("invalid offscreen pixel buffer")?
            .write_to(&mut png, image::ImageFormat::Png)?;
        runtime.stop();
        return Ok(Capture {
            png: png.into_inner(),
            pixels: previous,
            metadata: Map::from_iter([
                ("renderer".into(), json!("Ink offscreen Vulkan")),
                ("device".into(), json!(renderer.device_name())),
                ("emulatorRequired".into(), json!(false)),
            ]),
        });
    }
}

fn decode_image(
    path: &Path,
    target_width: u32,
    target_height: u32,
    fit: ImageFit,
) -> Result<image::RgbaImage> {
    // Match AImageDecoder: sampled PNG decode, premultiplied 4-bit bilinear filtering,
    // then the native host's conversion back to straight alpha for Vulkan textures.
    let mut source = image::open(path)
        .with_context(|| format!("read preview image {}", path.display()))?
        .into_rgba8();
    let scale_x = target_width.max(1) as f64 / f64::from(source.width());
    let scale_y = target_height.max(1) as f64 / f64::from(source.height());
    let scale = match fit {
        ImageFit::Cover => scale_x.max(scale_y),
        ImageFit::Contain => scale_x.min(scale_y),
    }
    .min(1.0);
    let width = (f64::from(source.width()) * scale).round().max(1.0) as u32;
    let height = (f64::from(source.height()) * scale).round().max(1.0) as u32;
    for pixel in source.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        let sample = (source.width() / width)
            .min(source.height() / height)
            .max(1);
        if sample > 1 {
            source = image::RgbaImage::from_fn(
                (source.width() / sample).max(1),
                (source.height() / sample).max(1),
                |x, y| *source.get_pixel(x * sample + sample / 2, y * sample + sample / 2),
            );
        }
    }
    let mut image = if source.dimensions() == (width, height) {
        source
    } else {
        let inverse_x = 1.0 / (width as f32 / source.width() as f32);
        let inverse_y = 1.0 / (height as f32 / source.height() as f32);
        let fixed = |value: f32| (f64::from(value) * 4294967296.0) as i64;
        let start_x = fixed(0.5 * inverse_x) - 2147483648;
        let step_x = fixed(inverse_x);
        image::RgbaImage::from_fn(width, height, |x, y| {
            let x = start_x + i64::from(x) * step_x;
            let y = fixed((y as f32 + 0.5) * inverse_y) - 2147483648;
            let fx = ((x >> 28) & 15) as u32;
            let fy = ((y >> 28) & 15) as u32;
            let sample = |dx: i32, dy: i32| {
                source.get_pixel(
                    ((x >> 32) as i32 + dx).clamp(0, source.width() as i32 - 1) as u32,
                    ((y >> 32) as i32 + dy).clamp(0, source.height() as i32 - 1) as u32,
                )
            };
            let a = sample(0, 0);
            let b = sample(1, 0);
            let c = sample(0, 1);
            let d = sample(1, 1);
            image::Rgba(std::array::from_fn(|channel| {
                ((u32::from(a[channel]) * (16 - fx) * (16 - fy)
                    + u32::from(b[channel]) * fx * (16 - fy)
                    + u32::from(c[channel]) * (16 - fx) * fy
                    + u32::from(d[channel]) * fx * fy)
                    >> 8) as u8
            }))
        })
    };
    for pixel in image.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel.0[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    Ok(image)
}

fn store_call(message: &Value, store: &mut HashMap<String, Value>) -> Result<Value> {
    let payload = &message["payload"];
    let key = payload["key"]
        .as_str()
        .context("invalid preview store key")?;
    match message["operation"].as_str() {
        Some("get") => Ok(store.get(key).cloned().unwrap_or(Value::Null)),
        Some("replace" | "write") => {
            let current = store
                .get(key)
                .map_or(0, |value| value["revision"].as_u64().unwrap_or_default());
            let committed =
                message["operation"] == "replace" || payload["revision"].as_u64() == Some(current);
            if committed {
                store.insert(key.into(), json!({"revision":current + 1, "version":payload["version"], "value":payload["value"]}));
            }
            Ok(json!({"committed":committed}))
        }
        _ => bail!("preview needs a fixture for store.{}", message["operation"]),
    }
}
