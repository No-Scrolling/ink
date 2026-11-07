use std::sync::OnceLock;

use crate::{LP3_REFERENCE_SCALE, Mask};

const TOGGLE_CIRCLE_SIZE: f32 = 9.8;
const TOGGLE_CIRCLE_BORDER: f32 = 2.5;
const TOGGLE_MASK_PADDING: u32 = 1;
const TOGGLE_SAMPLES: u32 = 4;

impl Mask {
    pub(crate) fn cropped(id: u64, width: u16, height: u16, pixels: Vec<u8>) -> Self {
        let mut mask = Self::owned(id, width, height, pixels);
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for (index, &alpha) in mask.pixels.as_ref().iter().enumerate() {
            if alpha == 0 { continue; }
            let (x, y) = ((index % usize::from(width)) as u16, (index / usize::from(width)) as u16);
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
        if right == 0 { left = 0; top = 0; }
        let crop = crate::MaskCrop { left, top, width: right - left, height: bottom - top };
        if crop.width == width && crop.height == height { return mask; }
        let mut pixels = Vec::with_capacity(usize::from(crop.width) * usize::from(crop.height));
        for y in top..bottom {
            let start = usize::from(y) * usize::from(width) + usize::from(left);
            pixels.extend_from_slice(&mask.pixels.as_ref()[start..start + usize::from(crop.width)]);
        }
        mask.pixels = crate::AssetBytes::Owned(pixels.into());
        mask.crop = Some(crop);
        mask
    }

    /// Restore transparent borders only for the renderer's first atlas upload.
    pub fn raster_pixels(&self) -> std::borrow::Cow<'_, [u8]> {
        let Some(crop) = self.crop else { return self.pixels.as_ref().into(); };
        let mut pixels = vec![0; usize::from(self.width) * usize::from(self.height)];
        for (row, source) in self.pixels.as_ref().chunks(usize::from(crop.width).max(1)).enumerate() {
            let start = (usize::from(crop.top) + row) * usize::from(self.width) + usize::from(crop.left);
            pixels[start..start + source.len()].copy_from_slice(source);
        }
        pixels.into()
    }

    pub(crate) fn solid() -> Self {
        Self::new(hash("ink_solid", 1), 1, 1, &[255])
    }

    pub(crate) fn content_bounds(&self) -> Option<crate::Rect> {
        if let Some(crop) = self.crop {
            return (crop.width > 0 && crop.height > 0).then(|| crate::Rect {
                x: crop.left as f32 / self.width as f32,
                y: crop.top as f32 / self.height as f32,
                width: crop.width as f32 / self.width as f32,
                height: crop.height as f32 / self.height as f32,
            });
        }
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
        crop: None,
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
