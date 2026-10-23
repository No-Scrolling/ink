use super::*;
use ink_core::{AssetBytes, ImageAsset, ImageAssetEncoding};

fn image(index: usize, size: u32, encoded: bool) -> ImageData {
    let colour = [(index * 5) as u8, (index * 5) as u8, (index * 5) as u8, 255];
    let pixels = colour.repeat((size * size) as usize);
    if encoded {
        let (encoding, bytes) = if index % 2 == 0 {
            let mut bytes = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 95)
                .encode(&colour[..3].repeat((size * size) as usize), size, size, image::ExtendedColorType::Rgb8).unwrap();
            (ImageAssetEncoding::Jpeg, bytes)
        } else { (ImageAssetEncoding::RgbaZlib, miniz_oxide::deflate::compress_to_vec_zlib(&pixels, 6)) };
        ImageData::Asset(ImageAsset { id: index as u64 + 20_000, width: size, height: size,
            encoding, bytes: AssetBytes::Owned(bytes.into()) })
    } else {
        ImageData::Remote(RemoteImage { id: index as u64 + 20_000, generation: 1,
            width: size, height: size, pixels: pixels.into() })
    }
}

fn trace(name: &str, images: Vec<ImageData>, cycles: Vec<Vec<usize>>, samples: usize, warmup: usize) -> Value {
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    let mut revision = 0;
    let mut peak = 0;
    let mut results = Vec::new();
    for step in 0..samples + warmup {
        let mut total = Sample::default();
        for &id in &cycles[step % cycles.len()] {
            revision += 1;
            let rect = Rect { x: 0.0, y: 0.0, width: 256.0, height: 256.0 };
            let scene = Scene { revision, width: WIDTH, height: HEIGHT,
                images: vec![ImageRun { avatar: None, image: images[id].clone(), zoom_id: None,
                    rect, clip: rect, fit: ImageFit::Contain, scrolling: false, transform: Default::default() }],
                ..Scene::default() };
            let sample = render(&mut renderer, &scene);
            peak = peak.max(renderer.memory_metrics().image_texture_bytes);
            total.accumulate(sample);
            let pixels = renderer.pixels().unwrap();
            let offset = (128 * WIDTH as usize + 128) * 4;
            for channel in 0..3 { assert!(pixels[offset + channel].abs_diff((id * 5) as u8) <= 3); }
            assert_eq!(pixels[offset + 3], 255);
        }
        if step >= warmup { results.push(total); }
    }
    summary(name, results, json!({"draws_per_cycle":cycles[0].len(),"peak_texture_bytes":peak,
        "pixel_colours_verified":true,"images":images.len()}))
}

pub(super) fn run(samples: usize, warmup: usize) -> Vec<Value> {
    let shift = (0..4).map(|phase| {
        (phase * 6..phase * 6 + 6).cycle().take(60).chain(24..48).collect::<Vec<_>>()
    }).collect();
    let mixed = vec![(0..44).collect()];
    let large = vec![(0..6).cycle().take(24).collect()];
    vec![
        trace("artwork-changing-hot-set", (0..48).map(|index| image(index, 256, false)).collect(), shift, samples, warmup),
        trace("artwork-mixed-reconstruction", (0..44).map(|index| image(index, 256, index < 4)).collect(), mixed, samples, warmup),
        trace("artwork-large-covers", (0..6).map(|index| image(index, 512, false)).collect(), large, samples, warmup),
    ]
}

/// Renderer-only decoded artwork arrival, scroll transforms and cache revisits.
pub(super) fn scroll_arrivals(samples: usize, warmup: usize) -> Value {
    let images = (0..24)
        .map(|index| image(index, 512, false))
        .collect::<Vec<_>>();
    let row_path = (0..10).chain((0..10).rev()).collect::<Vec<_>>();
    let clip = Rect {
        x: 0.0,
        y: 64.0,
        width: WIDTH as f32,
        height: 1136.0,
    };
    let mut renderer = Renderer::offscreen(WIDTH, HEIGHT).unwrap();
    let mut results = Vec::new();
    let mut frames = Vec::new();
    let mut endpoints = Vec::new();
    let mut unique = std::collections::BTreeSet::new();
    let mut revisit_misses = 0;
    for step in 0..warmup + samples {
        let tick = step / 2;
        let top_row = row_path[tick % row_path.len()];
        let arrived = step % 2 == 1;
        let fractional_scroll = if tick % 2 == 0 { 0.0 } else { 64.0 };
        let scroll_offset = top_row as f32 * 360.0 + fractional_scroll;
        let run_clip = Rect { y: clip.y + scroll_offset, ..clip };
        let mut scene = Scene {
            revision: step as u64 + 1,
            image_revision: step as u64 + 1,
            width: WIDTH,
            height: HEIGHT,
            scroll_offset,
            scroll_clip: Some(clip),
            ..Scene::default()
        };
        let mut checks = Vec::new();
        let mut visible_ids = Vec::new();
        for relative_row in 0..3 {
            for column in 0..2 {
                let id = (top_row + relative_row) * 2 + column;
                let x = if column == 0 { 64.0 } else { 608.0 };
                let y = 96.0 + (top_row + relative_row) as f32 * 360.0;
                let rect = Rect {
                    x,
                    y,
                    width: 256.0,
                    height: 256.0,
                };
                let present = arrived || relative_row != 1;
                if present {
                    scene.images.push(ImageRun {
                        avatar: None,
                        image: images[id].clone(),
                        zoom_id: None,
                        rect,
                        clip: run_clip,
                        fit: ImageFit::Contain,
                        scrolling: true,
                        transform: Default::default(),
                    });
                    visible_ids.push(id);
                    unique.insert(id);
                }
                scene.text.push(TextRun {
                    text: format!("Cover {id:02}").into(),
                    rect: Rect {
                        x,
                        y: y + 268.0,
                        width: 256.0,
                        height: 40.0,
                    },
                    clip: run_clip,
                    font_size: 28.0,
                    colour: Colour::WHITE,
                    align: TextAlign::Start,
                    tabular_numbers: true,
                    scrolling: true,
                });
                // The centre is safely within the scroll clip even for the partially clipped top row.
                checks.push((
                    id,
                    present,
                    [x as u32 + 112, (y - scroll_offset) as u32 + 112, 32, 32],
                    [x as u32, (y - scroll_offset) as u32 + 268, 256, 40],
                ));
            }
        }
        let sample = render(&mut renderer, &scene);
        if tick >= 10 {
            revisit_misses += sample.cache_misses;
        }
        let pixels = renderer.pixels().unwrap();
        let validate = |pixels: &[u8]| {
            for &(id, present, rect, text_rect) in &checks {
                let colour = if present { (id * 5) as u8 } else { 0 };
                coverage::assert_solid(pixels, rect, [colour, colour, colour, 255], 1);
                assert!(
                    coverage::ink_pixels(pixels, text_rect) > 30,
                    "scrolling cover label {id} must remain visible"
                );
            }
            coverage::assert_solid(pixels, [0, 0, WIDTH, 64], [0, 0, 0, 255], 0);
            coverage::assert_solid(pixels, [0, 1200, WIDTH, 40], [0, 0, 0, 255], 0);
        };
        validate(&pixels);
        if step >= warmup {
            frames.push(json!({"step":step,"top_row":top_row,"scroll_offset":scroll_offset,
                "phase":if arrived { "decoded artwork arrives" } else { "scroll with two pending covers" },
                "visible_ids":visible_ids,"verified_image_regions":checks.len(),
                "verified_text_regions":scene.text.len(),"memory_after_readback":coverage::memory_snapshot(&renderer)}));
            results.push(sample);
            if step < warmup + 2 || step + 2 >= warmup + samples {
                let saved =
                    coverage::save_pixels(&format!("artwork-scroll-arrival-{step}"), &pixels);
                validate(&saved.1);
                endpoints.push(json!({"step":step,"artifact":saved.0,
                    "pending_regions_black":!arrived,"labels_visible":true,"scroll_clip_verified":true}));
            }
        }
    }
    assert_eq!(
        unique.len(),
        24,
        "trace must exceed the eight-cover cache working set"
    );
    assert!(
        revisit_misses > 0,
        "revisits must exercise cache reconstruction after turnover"
    );
    summary(
        "artwork-arrival-scroll-turnover",
        results,
        json!({
            "frames":samples,"warmup_frames":warmup,"minimum_sustained_frames":120,
            "unique_images":24,"image":[512,512],"source_image_bytes":24*512*512*4,
            "maximum_images_per_scene":6,"scrolling":true,"scroll_clip":[0,64,1080,1136],
            "revisit_cache_misses_including_warmup":revisit_misses,"frames_detail":frames,
            "correctness":endpoints,
            "oracle":"independent cover ID-to-greyscale values, pending cover regions black, visible labels and fixed black clipping bands checked every frame; saved endpoints reloaded and checked",
            "scope":"renderer-only decoded RGBA arrival; no network, native decode, engine event dispatch, JS, input latency or presentation timing"
        }),
    )
}
