//! Additional renderer coverage. Fixture generation and pixel checks stay outside timed intervals.
use super::*;
use ab_glyph::Font;
use std::collections::BTreeSet;

const LANGUAGE_LINES: [(&str, &str); 10] = [
    ("English", "Harbour lights beside the river"),
    ("French", "Été à Montréal, cœur de la ville"),
    ("Spanish", "Málaga, canción y piñata"),
    ("German", "Grüße aus Köln und Zürich"),
    ("Polish", "Łódź, żółć i śpiew"),
    ("Icelandic", "Þjóð, ævi og íslensk orð"),
    ("Greek probe", "Ελληνικά Αθήνα"),
    ("Cyrillic probe", "Русский Москва"),
    ("Arabic probe", "العربية القاهرة"),
    ("CJK probe", "日本語 東京 中文"),
];

pub(super) fn run(samples: usize, warmup: usize) -> Vec<Value> {
    vec![
        cold_multilingual(samples.min(16), warmup.min(2)),
        multilingual_turnover(samples.max(12), warmup),
        sustained_uploads(
            "sustained-full-frame-uploads",
            false,
            samples.max(120),
            warmup,
        ),
        sustained_uploads(
            "sustained-variable-frame-uploads",
            true,
            samples.max(120),
            warmup,
        ),
        artwork::scroll_arrivals(samples.max(120), warmup),
    ]
}

fn text_run(text: &str, y: f32, size: f32) -> TextRun {
    TextRun {
        text: text.into(),
        rect: Rect {
            x: 24.0,
            y,
            width: 1032.0,
            height: size * 1.5,
        },
        clip: Rect {
            x: 0.0,
            y: 0.0,
            width: WIDTH as f32,
            height: HEIGHT as f32,
        },
        font_size: size,
        colour: Colour::WHITE,
        align: TextAlign::Start,
        tabular_numbers: false,
        scrolling: false,
    }
}

fn multilingual_scene() -> Scene {
    Scene {
        revision: 1,
        width: WIDTH,
        height: HEIGHT,
        text: LANGUAGE_LINES
            .iter()
            .enumerate()
            .map(|(index, (_, text))| text_run(text, 24.0 + index as f32 * 104.0, 38.0))
            .collect(),
        ..Scene::default()
    }
}

fn font_coverage(lines: &[(&str, &str)]) -> Value {
    json!(lines.iter().map(|(language, text)| {
        // Inspect shaped text too: Arabic presentation forms can differ from source code points.
        let describe = |text: &str| text.chars().filter(|ch| !ch.is_whitespace())
            .collect::<BTreeSet<_>>().into_iter().map(|character| {
                let (font_index, font) = ink_core::font_for_character(character);
                let glyph_id = font.glyph_id(character).0;
                json!({"character":character.to_string(), "code_point":format!("U+{:04X}", character as u32),
                    "font_index":font_index, "glyph_id":glyph_id,
                    "status":if glyph_id == 0 { "missing: .notdef/tofu" }
                        else if font_index == 0 { "bundled Public Sans" } else { "fallback font" }})
            }).collect::<Vec<_>>();
        let source = describe(text);
        let visual = ink_core::visual_text(text);
        let visual_characters = describe(&visual);
        json!({"language":language,"text":text,"visual_text":visual,
            "fully_covered":visual_characters.iter().all(|ch| ch["glyph_id"] != 0),
            "source_characters":source,"rendered_characters":visual_characters})
    }).collect::<Vec<_>>())
}

fn check_multilingual(pixels: &[u8]) -> Value {
    let regions = LANGUAGE_LINES
        .iter()
        .enumerate()
        .map(|(index, (language, text))| {
            let y = 24 + index as u32 * 104;
            let ink = ink_pixels(pixels, [24, y, 1032, 57]);
            assert!(
                ink > 50,
                "{language}: expected visible glyphs or explicitly labelled .notdef glyphs"
            );
            assert_solid(pixels, [24, y + 64, 1032, 20], [0, 0, 0, 255], 0);
            json!({"language":language,"text":text,"rect":[24,y,1032,57],"non_black_pixels":ink})
        })
        .collect::<Vec<_>>();
    assert_solid(pixels, [0, 1150, WIDTH, 90], [0, 0, 0, 255], 0);
    json!({"line_regions":regions,"blank_bottom_verified":true})
}

fn cold_multilingual(samples: usize, warmup: usize) -> Value {
    let scene = multilingual_scene();
    for (run, (_, expected)) in scene.text.iter().zip(LANGUAGE_LINES) {
        assert_eq!(run.text.as_str(), expected);
    }
    let mut results = Vec::new();
    let mut construction_ns = Vec::new();
    let mut endpoints = Vec::new();
    let mut memory = Vec::new();
    let mut reference = None;
    for step in 0..warmup + samples {
        let start = Instant::now();
        let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
        let create_ns = start.elapsed().as_nanos() as u64;
        let sample = render(&mut renderer, &scene);
        assert!(
            sample.cache_misses > 0,
            "a fresh renderer must rasterise text: {} misses",
            sample.cache_misses
        );
        let pixels = renderer.pixels().unwrap();
        if step == 0 {
            save_pixels("cold-multilingual-first-frame", &pixels);
        }
        let checks = check_multilingual(&pixels);
        if let Some(expected) = &reference {
            assert_eq!(
                &pixels, expected,
                "fresh renderer output must remain deterministic"
            );
        } else {
            reference = Some(pixels.clone());
        }
        if step >= warmup {
            memory.push(memory_snapshot(&renderer));
            construction_ns.push(create_ns);
            results.push(sample);
            if step == warmup || step + 1 == warmup + samples {
                let saved = save_pixels(&format!("cold-multilingual-{step}"), &pixels);
                check_multilingual(&saved.1);
                endpoints.push(json!({"step":step,"artifact":saved.0,"checks":checks}));
            }
        }
    }
    summary(
        "cold-multilingual-glyphs",
        results,
        json!({
            "cold_boundary":"fresh Renderer, glyph atlas, prepared scene and image cache per sample; process-wide font parsing may already be warm",
            "requested_samples":samples,"warmup_renderers_discarded":warmup,"text_runs":scene.text.len(),
            "characters":LANGUAGE_LINES.iter().map(|(_, text)| text.chars().count()).sum::<usize>(),
            "font_coverage":font_coverage(&LANGUAGE_LINES),
            "font_limit":"Only fonts actually selected by ink_core::font_for_character are credited; missing probes render .notdef and are not script support",
            "renderer_construction_ns":construction_ns,"memory_after_readback":memory,
            "correctness":endpoints,
            "oracle":"literal text contracts, per-line visible ink and blank bands, saved PNG pixels reloaded and checked; no OCR or claim that tofu proves script support"
        }),
    )
}

fn turnover_scene(lower: bool, revision: u64) -> Scene {
    let mut scene = atlas_scene(140.0, lower, revision);
    scene.text.push(text_run(
        if lower {
            "Málaga · Łódź · été · Grüße"
        } else {
            "Þjóð · Zürich · canción · cœur"
        },
        1100.0,
        32.0,
    ));
    scene
}

fn multilingual_turnover(samples: usize, warmup: usize) -> Value {
    let mut references = Vec::new();
    let mut reference_artifacts = Vec::new();
    for phase in 0..2 {
        let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
        let scene = turnover_scene(phase == 1, 1);
        render(&mut renderer, &scene);
        let pixels = renderer.pixels().unwrap();
        assert!(ink_pixels(&pixels, [0, 0, WIDTH, HEIGHT]) > 1000);
        assert!(ink_pixels(&pixels, [24, 1100, 1032, 48]) > 100);
        let saved = save_pixels(&format!("atlas-reference-{phase}"), &pixels);
        reference_artifacts.push(saved.0);
        references.push(saved.1);
    }
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    let mut results = Vec::new();
    let mut memory = Vec::new();
    let mut endpoints = Vec::new();
    let mut misses = Vec::new();
    for step in 0..samples + warmup {
        let phase = step % 2;
        let sample = render(&mut renderer, &turnover_scene(phase == 1, step as u64 + 1));
        assert!(
            sample.cache_misses > 0,
            "switch must rasterise after atlas turnover"
        );
        let pixels = renderer.pixels().unwrap();
        assert_eq!(
            pixels, references[phase],
            "turnover must match an independent fresh renderer for phase {phase}"
        );
        if step >= warmup {
            misses.push(sample.cache_misses);
            memory.push(memory_snapshot(&renderer));
            results.push(sample);
            if step < warmup + 2 || step + 2 >= warmup + samples {
                let saved = save_pixels(&format!("atlas-turnover-{step}"), &pixels);
                assert_eq!(saved.1, references[phase]);
                endpoints.push(json!({"step":step,"phase":phase,"artifact":saved.0,"different_bytes_from_fresh_reference":0}));
            }
        }
    }
    summary(
        "multilingual-atlas-turnover",
        results,
        json!({
            "frames":samples,"warmup_frames":warmup,"phases":2,
        "font_sizes":[140,154,168,32],"font_coverage":font_coverage(&[
            ("Uppercase atlas", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            ("Lowercase atlas", "abcdefghijklmnopqrstuvwxyz"),
            ("Lowercase phase footer", "Málaga · Łódź · été · Grüße"),
            ("Uppercase phase footer", "Þjóð · Zürich · canción · cœur")]),
            "turnover_evidence":"new rasterisation on every alternating switch; public metrics do not expose an atlas reset counter",
            "cache_misses_per_switch":misses,"memory_after_readback":memory,
            "reference_artifacts":reference_artifacts,"correctness":endpoints,
            "oracle":"all RGBA bytes match the saved output of an independent fresh renderer for that phase; visible multilingual footer checked separately"
        }),
    )
}

// Opaque endpoint colours avoid ambiguity from colour conversion and texture interpolation.
fn upload_pixels(width: u32, height: u32, odd: bool) -> Arc<[u8]> {
    let colours = if odd {
        [
            [255, 255, 0, 255],
            [255, 0, 255, 255],
            [0, 255, 255, 255],
            [0, 0, 0, 255],
        ]
    } else {
        [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 255],
        ]
    };
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let row = if y < height / 2 { 0 } else { 2 };
        pixels.extend(colours[row].repeat((width / 2) as usize));
        pixels.extend(colours[row + 1].repeat((width - width / 2) as usize));
    }
    pixels.into()
}

fn check_upload(pixels: &[u8], width: u32, height: u32, odd: bool) {
    let expected = if odd {
        [
            [255, 255, 0, 255],
            [255, 0, 255, 255],
            [0, 255, 255, 255],
            [0, 0, 0, 255],
        ]
    } else {
        [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 255],
        ]
    };
    for (index, colour) in expected.into_iter().enumerate() {
        let x = (index as u32 % 2) * (width / 2);
        let y = (index as u32 / 2) * (height / 2);
        // Keep a two-pixel inset from geometry edges and the quadrant boundaries.
        assert_solid(
            pixels,
            [x + 2, y + 2, width / 2 - 4, height / 2 - 4],
            colour,
            1,
        );
    }
    if width < WIDTH {
        assert_solid(
            pixels,
            [width + 2, 0, WIDTH - width - 2, HEIGHT],
            [0, 0, 0, 255],
            0,
        );
    }
    if height < HEIGHT {
        assert_solid(
            pixels,
            [0, height + 2, width, HEIGHT - height - 2],
            [0, 0, 0, 255],
            0,
        );
    }
}

fn sustained_uploads(name: &str, variable: bool, samples: usize, warmup: usize) -> Value {
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    let mut results = Vec::new();
    let mut frames = Vec::new();
    let mut endpoints = Vec::new();
    for step in 0..samples + warmup {
        let (width, height) = if variable {
            match (step / 8) % 4 {
                0 | 3 => (256, 256),
                1 => (1080, 608),
                _ => (WIDTH, HEIGHT),
            }
        } else {
            (WIDTH, HEIGHT)
        };
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
        };
        let scene = Scene {
            revision: 1,
            image_revision: step as u64 + 1,
            width: WIDTH,
            height: HEIGHT,
            images: vec![ImageRun {
                avatar: None,
                image: ImageData::Remote(RemoteImage {
                    id: 300_000,
                    generation: step as u64 + 1,
                    width,
                    height,
                    pixels: upload_pixels(width, height, step % 2 == 1),
                }),
                zoom_id: None,
                rect,
                clip: rect,
                fit: ImageFit::Contain,
                scrolling: false,
                transform: Default::default(),
            }],
            ..Scene::default()
        };
        let sample = render(&mut renderer, &scene);
        assert_eq!(
            sample.staging_upload_bytes,
            u64::from(width) * u64::from(height) * 4,
            "every frame must upload the supplied texture, without glyph uploads"
        );
        let pixels = renderer.pixels().unwrap();
        check_upload(&pixels, width, height, step % 2 == 1);
        if step >= warmup {
            frames.push(
                json!({"step":step,"image":[width,height],"generation":step+1,
                "expected_image_upload_bytes":width as u64 * height as u64 * 4,
                "memory_after_readback":memory_snapshot(&renderer)}),
            );
            results.push(sample);
            if step == warmup || step == warmup + samples / 2 || step + 1 == warmup + samples {
                let saved = save_pixels(&format!("{name}-{step}"), &pixels);
                check_upload(&saved.1, width, height, step % 2 == 1);
                endpoints.push(
                    json!({"step":step,"image":[width,height],"artifact":saved.0,
                    "quadrant_regions_verified":4,"background_verified":true}),
                );
            }
        }
    }
    summary(
        name,
        results,
        json!({"frames":samples,"warmup_frames":warmup,
            "minimum_sustained_frames":120,"source":"decoded RGBA generations; no image decode, JS or animation clock in timing",
            "size_sequence":if variable { json!([[256,256],[1080,608],[1080,1240],[256,256]]) } else { json!([[1080,1240]]) },
            "frames_per_size_phase":if variable { json!(8) } else { Value::Null },
            "frames_detail":frames,"correctness":endpoints,
            "oracle":"four independently specified opaque quadrant colours checked for every frame, plus black outside the image; saved endpoints reloaded and checked",
            "memory_scope":"renderer buffer/texture capacities and actual staging Vulkan allocations; excludes RSS, driver internals and source Arc storage"
        }),
    )
}

pub(super) fn memory_snapshot(renderer: &Renderer) -> Value {
    let memory = renderer.memory_metrics();
    json!({"instance_buffer_capacity_bytes":memory.instance_buffer_capacity_bytes,
        "instance_snapshot_capacity_bytes":memory.instance_snapshot_capacity_bytes,
        "font_texture_bytes":memory.font_texture_bytes,"image_texture_bytes":memory.image_texture_bytes,
        "system_glyph_texture_bytes":memory.system_glyph_texture_bytes,
        "retained_staging_buffer_capacity_bytes":memory.retained_staging_buffer_capacity_bytes,
        "retained_staging_memory_bytes":memory.retained_staging_memory_bytes,
        "in_flight_staging_buffer_capacity_bytes":memory.in_flight_staging_buffer_capacity_bytes})
}

pub(super) fn assert_solid(pixels: &[u8], rect: [u32; 4], expected: [u8; 4], tolerance: u8) {
    let [x, y, width, height] = rect;
    assert!(x + width <= WIDTH && y + height <= HEIGHT);
    for row in y..y + height {
        for column in x..x + width {
            let offset = ((row * WIDTH + column) * 4) as usize;
            for channel in 0..4 {
                assert!(
                    pixels[offset + channel].abs_diff(expected[channel]) <= tolerance,
                    "pixel ({column},{row}) channel {channel}: actual {}, expected {} ± {tolerance}",
                    pixels[offset + channel],
                    expected[channel]
                );
            }
        }
    }
}

pub(super) fn ink_pixels(pixels: &[u8], rect: [u32; 4]) -> usize {
    let [x, y, width, height] = rect;
    (y..y + height)
        .flat_map(|row| (x..x + width).map(move |column| ((row * WIDTH + column) * 4) as usize))
        .filter(|&offset| {
            pixels[offset..offset + 3]
                .iter()
                .any(|&channel| channel > 16)
        })
        .count()
}

pub(super) fn save_pixels(name: &str, pixels: &[u8]) -> (Value, Vec<u8>) {
    let directory = std::env::var_os("INK_PERF_RENDER_ARTIFACTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".test-output/performance/renderer-artifacts"));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{name}.png"));
    image::save_buffer(&path, pixels, WIDTH, HEIGHT, image::ColorType::Rgba8).unwrap();
    let saved = image::open(&path).unwrap().into_rgba8().into_raw();
    assert_eq!(saved, pixels, "saved PNG must preserve all readback pixels");
    let hash = saved.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    (
        json!({"path":path,"frame":[WIDTH,HEIGHT],"rgba_fnv1a64":format!("{hash:016x}"),
        "rgba_bytes":saved.len(),"saved_pixels_reloaded":true}),
        saved,
    )
}
