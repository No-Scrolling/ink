use std::sync::OnceLock;

use crate::{LP3_REFERENCE_SCALE, Mask};

const TOGGLE_CIRCLE_SIZE: f32 = 9.8;
const TOGGLE_CIRCLE_BORDER: f32 = 2.5;
const TOGGLE_MASK_PADDING: u32 = 1;
const TOGGLE_SAMPLES: u32 = 4;

impl Mask {
    pub(crate) fn solid() -> Self {
        Self::new(hash("ink_solid", 1), 1, 1, &[255])
    }

    pub(crate) fn content_bounds(&self) -> Option<crate::Rect> {
        let (mut left, mut top, mut right, mut bottom) = (self.width as usize, self.height as usize, 0, 0);
        for (index, alpha) in self.pixels.as_ref().iter().enumerate() {
            if *alpha == 0 { continue; }
            let x = index % self.width as usize;
            let y = index / self.width as usize;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
        (right > left && bottom > top).then(|| crate::Rect {
            x: left as f32 / self.width as f32,
            y: top as f32 / self.height as f32,
            width: (right - left) as f32 / self.width as f32,
            height: (bottom - top) as f32 / self.height as f32,
        })
    }

    pub fn toggle_circle(filled: bool) -> Self {
        static OFF: OnceLock<Mask> = OnceLock::new();
        static ON: OnceLock<Mask> = OnceLock::new();
        let mask = if filled { &ON } else { &OFF };
        mask.get_or_init(|| raster_toggle(filled)).clone()
    }
}

fn raster_toggle(filled: bool) -> Mask {
    let circle_dimension = (TOGGLE_CIRCLE_SIZE * LP3_REFERENCE_SCALE).round() as u32;
    let dimension = circle_dimension + TOGGLE_MASK_PADDING * 2;
    let outer_radius = circle_dimension as f32 / 2.0;
    let inner_radius = outer_radius - TOGGLE_CIRCLE_BORDER * LP3_REFERENCE_SCALE;
    let centre = dimension as f32 / 2.0;
    let mut pixels = vec![0; (dimension * dimension) as usize];

    for y in 0..dimension {
        for x in 0..dimension {
            let mut covered = 0;
            for sample_y in 0..TOGGLE_SAMPLES {
                for sample_x in 0..TOGGLE_SAMPLES {
                    let sample_x = x as f32 + (sample_x as f32 + 0.5) / TOGGLE_SAMPLES as f32;
                    let sample_y = y as f32 + (sample_y as f32 + 0.5) / TOGGLE_SAMPLES as f32;
                    let distance =
                        ((sample_x - centre).powi(2) + (sample_y - centre).powi(2)).sqrt();
                    if distance <= outer_radius && (filled || distance >= inner_radius) {
                        covered += 1;
                    }
                }
            }
            pixels[(y * dimension + x) as usize] =
                (covered * 255 / (TOGGLE_SAMPLES * TOGGLE_SAMPLES)) as u8;
        }
    }

    Mask {
        id: hash(
            if filled {
                "ink_toggle_on"
            } else {
                "ink_toggle_off"
            },
            dimension,
        ),
        width: dimension as u16,
        height: dimension as u16,
        pixels: crate::AssetBytes::Owned(pixels.into()),
    }
}

fn hash(name: &str, dimension: u32) -> u64 {
    let mut value = 0xcbf29ce484222325_u64;
    for byte in name.bytes().chain(dimension.to_le_bytes()) {
        value ^= u64::from(byte);
        value = value.wrapping_mul(0x100000001b3);
    }
    value
}
