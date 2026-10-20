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
            total.cpu_ns += sample.cpu_ns;
            total.prepare_ns += sample.prepare_ns;
            total.upload_ns += sample.upload_ns;
            total.uploaded_bytes += sample.uploaded_bytes;
            total.cache_misses += sample.cache_misses;
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
