use ink_core::{ImageAnimation, ImageFit};
use std::{
    ffi::{c_int, c_void},
    os::fd::AsRawFd,
    ptr::NonNull,
    time::{Duration, Instant},
};

const IMAGE_DECODER_SUCCESS: c_int = 0;
const IMAGE_DECODER_FINISHED: c_int = -10;
const BITMAP_FORMAT_RGBA_8888: c_int = 1;

enum AImageDecoder {}
enum AImageDecoderHeaderInfo {}
enum AImageDecoderFrameInfo {}

#[link(name = "jnigraphics")]
unsafe extern "C" {
    fn AImageDecoder_createFromFd(fd: c_int, decoder: *mut *mut AImageDecoder) -> c_int;
    fn AImageDecoder_delete(decoder: *mut AImageDecoder);
    fn AImageDecoder_isAnimated(decoder: *mut AImageDecoder) -> bool;
    fn AImageDecoder_getRepeatCount(decoder: *mut AImageDecoder) -> i32;
    fn AImageDecoder_advanceFrame(decoder: *mut AImageDecoder) -> c_int;
    fn AImageDecoder_rewind(decoder: *mut AImageDecoder) -> c_int;
    fn AImageDecoderFrameInfo_create() -> *mut AImageDecoderFrameInfo;
    fn AImageDecoderFrameInfo_delete(info: *mut AImageDecoderFrameInfo);
    fn AImageDecoder_getFrameInfo(
        decoder: *mut AImageDecoder,
        info: *mut AImageDecoderFrameInfo,
    ) -> c_int;
    fn AImageDecoderFrameInfo_getDuration(info: *const AImageDecoderFrameInfo) -> i64;
    fn AImageDecoder_getHeaderInfo(decoder: *const AImageDecoder)
    -> *const AImageDecoderHeaderInfo;
    fn AImageDecoderHeaderInfo_getWidth(info: *const AImageDecoderHeaderInfo) -> i32;
    fn AImageDecoderHeaderInfo_getHeight(info: *const AImageDecoderHeaderInfo) -> i32;
    fn AImageDecoder_setAndroidBitmapFormat(decoder: *mut AImageDecoder, format: i32) -> c_int;
    fn AImageDecoder_setTargetSize(decoder: *mut AImageDecoder, width: i32, height: i32) -> c_int;
    fn AImageDecoder_getMinimumStride(decoder: *mut AImageDecoder) -> usize;
    fn AImageDecoder_decodeImage(
        decoder: *mut AImageDecoder,
        pixels: *mut c_void,
        stride: usize,
        size: usize,
    ) -> c_int;
}

pub(super) fn decode(
    path: &str,
    target_width: u32,
    target_height: u32,
    fit: ImageFit,
    looping: bool,
) -> Result<(u32, u32, Vec<u8>, Option<Box<dyn ImageAnimation>>), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("Android could not open the remote image: {error}"))?;
    let mut raw = std::ptr::null_mut();
    let result = unsafe { AImageDecoder_createFromFd(file.as_raw_fd(), &mut raw) };
    if result != IMAGE_DECODER_SUCCESS {
        return Err(format!(
            "Android could not decode the remote image ({result})"
        ));
    }
    let decoder = ImageDecoder(
        NonNull::new(raw).ok_or_else(|| "Android returned no image decoder".to_owned())?,
    );
    let header = unsafe { AImageDecoder_getHeaderInfo(decoder.0.as_ptr()) };
    if header.is_null() {
        return Err("Android returned no image header".to_owned());
    }
    let source_width = unsafe { AImageDecoderHeaderInfo_getWidth(header) };
    let source_height = unsafe { AImageDecoderHeaderInfo_getHeight(header) };
    if source_width <= 0 || source_height <= 0 {
        return Err("Remote image had invalid dimensions".to_owned());
    }
    let width_scale = target_width.max(1) as f64 / source_width as f64;
    let height_scale = target_height.max(1) as f64 / source_height as f64;
    let scale = match fit {
        ImageFit::Cover => width_scale.max(height_scale),
        ImageFit::Contain => width_scale.min(height_scale),
    }
    .min(1.0);
    let width = (source_width as f64 * scale).round().max(1.0) as u32;
    let height = (source_height as f64 * scale).round().max(1.0) as u32;
    let format = unsafe {
        AImageDecoder_setAndroidBitmapFormat(decoder.0.as_ptr(), BITMAP_FORMAT_RGBA_8888)
    };
    if format != IMAGE_DECODER_SUCCESS {
        return Err(format!(
            "Android could not convert the remote image ({format})"
        ));
    }
    if width != source_width as u32 || height != source_height as u32 {
        let scaled =
            unsafe { AImageDecoder_setTargetSize(decoder.0.as_ptr(), width as i32, height as i32) };
        if scaled != IMAGE_DECODER_SUCCESS {
            return Err(format!(
                "Android could not scale the remote image ({scaled})"
            ));
        }
    }
    let stride = unsafe { AImageDecoder_getMinimumStride(decoder.0.as_ptr()) };
    let size = stride
        .checked_mul(height as usize)
        .ok_or_else(|| "Remote image was too large".to_owned())?;
    if size > 16 * 1024 * 1024 || stride < width as usize * 4 {
        return Err("Remote image was too large".to_owned());
    }
    let mut decoded = vec![0; size];
    let result = unsafe {
        AImageDecoder_decodeImage(
            decoder.0.as_ptr(),
            decoded.as_mut_ptr().cast(),
            stride,
            size,
        )
    };
    if result != IMAGE_DECODER_SUCCESS {
        return Err(format!(
            "Android could not decode the remote image ({result})"
        ));
    }
    let animated = unsafe { AImageDecoder_isAnimated(decoder.0.as_ptr()) };
    if !animated {
        return Ok((width, height, rgba(decoded, stride, width), None));
    }
    let pixels = rgba(decoded.clone(), stride, width);
    let info = FrameInfo(
        NonNull::new(unsafe { AImageDecoderFrameInfo_create() })
            .ok_or_else(|| "Android returned no frame information".to_owned())?,
    );
    let delay = frame_delay(&decoder, &info)?;
    let repeats = if looping {
        i32::MAX
    } else {
        unsafe { AImageDecoder_getRepeatCount(decoder.0.as_ptr()) }
    };
    let animation = Some(Box::new(Animation {
        decoder,
        info,
        _file: file,
        decoded,
        stride,
        width,
        repeats,
        next: Instant::now() + delay,
        finished: false,
    }) as Box<dyn ImageAnimation>);
    Ok((width, height, pixels, animation))
}

fn rgba(decoded: Vec<u8>, stride: usize, width: u32) -> Vec<u8> {
    let mut pixels = if stride == width as usize * 4 {
        decoded
    } else {
        let mut pixels = Vec::with_capacity(decoded.len());
        for row in decoded.chunks_exact(stride) {
            pixels.extend_from_slice(&row[..width as usize * 4]);
        }
        pixels
    };
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    pixels
}

fn frame_delay(decoder: &ImageDecoder, info: &FrameInfo) -> Result<Duration, String> {
    let result = unsafe { AImageDecoder_getFrameInfo(decoder.0.as_ptr(), info.0.as_ptr()) };
    if result != IMAGE_DECODER_SUCCESS {
        return Err(format!("Android could not read image frame ({result})"));
    }
    let ns = unsafe { AImageDecoderFrameInfo_getDuration(info.0.as_ptr()) };
    Ok(Duration::from_nanos(if ns <= 0 {
        100_000_000
    } else {
        ns.max(20_000_000) as u64
    }))
}

struct Animation {
    decoder: ImageDecoder,
    info: FrameInfo,
    _file: std::fs::File,
    // Android composites partial frames into the previous premultiplied pixels.
    decoded: Vec<u8>,
    stride: usize,
    width: u32,
    repeats: i32,
    next: Instant,
    finished: bool,
}

// Owned decoder state moves from the image worker to the engine mutex; it is never shared concurrently.
unsafe impl Send for Animation {}

impl ImageAnimation for Animation {
    fn finished(&self) -> bool {
        self.finished
    }

    fn advance(&mut self) -> Option<Vec<u8>> {
        if self.finished || Instant::now() < self.next {
            return None;
        }
        let mut result = unsafe { AImageDecoder_advanceFrame(self.decoder.0.as_ptr()) };
        if result == IMAGE_DECODER_FINISHED {
            if self.repeats == 0 {
                self.finished = true;
                return None;
            }
            if self.repeats != i32::MAX {
                self.repeats -= 1;
            }
            result = unsafe { AImageDecoder_rewind(self.decoder.0.as_ptr()) };
        }
        if result == IMAGE_DECODER_SUCCESS {
            result = unsafe {
                AImageDecoder_decodeImage(
                    self.decoder.0.as_ptr(),
                    self.decoded.as_mut_ptr().cast(),
                    self.stride,
                    self.decoded.len(),
                )
            };
        }
        if result != IMAGE_DECODER_SUCCESS {
            self.finished = true;
            super::android_log(
                super::ANDROID_LOG_WARN,
                &format!("Image animation stopped ({result})"),
            );
            return None;
        }
        match frame_delay(&self.decoder, &self.info) {
            Ok(delay) => self.next = Instant::now() + delay,
            Err(message) => {
                self.finished = true;
                super::android_log(super::ANDROID_LOG_WARN, &message);
            }
        }
        Some(rgba(self.decoded.clone(), self.stride, self.width))
    }
}

struct FrameInfo(NonNull<AImageDecoderFrameInfo>);
impl Drop for FrameInfo {
    fn drop(&mut self) {
        unsafe { AImageDecoderFrameInfo_delete(self.0.as_ptr()) };
    }
}

struct ImageDecoder(NonNull<AImageDecoder>);

impl Drop for ImageDecoder {
    fn drop(&mut self) {
        unsafe { AImageDecoder_delete(self.0.as_ptr()) };
    }
}
