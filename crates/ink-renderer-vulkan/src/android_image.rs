use std::{ffi::c_int, ptr::NonNull};

use anyhow::{Result, anyhow};

const IMAGE_DECODER_SUCCESS: c_int = 0;
const BITMAP_FORMAT_RGBA_8888: c_int = 1;
const MAX_DECODED_IMAGE_BYTES: usize = 16 * 1024 * 1024;

enum AImageDecoder {}
enum AImageDecoderHeaderInfo {}

#[link(name = "jnigraphics")]
unsafe extern "C" {
    fn AImageDecoder_createFromBuffer(
        buffer: *const u8,
        length: usize,
        decoder: *mut *mut AImageDecoder,
    ) -> c_int;
    fn AImageDecoder_delete(decoder: *mut AImageDecoder);
    fn AImageDecoder_getHeaderInfo(decoder: *const AImageDecoder)
    -> *const AImageDecoderHeaderInfo;
    fn AImageDecoderHeaderInfo_getWidth(info: *const AImageDecoderHeaderInfo) -> c_int;
    fn AImageDecoderHeaderInfo_getHeight(info: *const AImageDecoderHeaderInfo) -> c_int;
    fn AImageDecoder_setAndroidBitmapFormat(decoder: *mut AImageDecoder, format: c_int) -> c_int;
    fn AImageDecoder_getMinimumStride(decoder: *mut AImageDecoder) -> usize;
    fn AImageDecoder_decodeImage(
        decoder: *mut AImageDecoder,
        pixels: *mut u8,
        stride: usize,
        size: usize,
    ) -> c_int;
}

pub fn decode(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    let mut raw = std::ptr::null_mut();
    let result = unsafe { AImageDecoder_createFromBuffer(bytes.as_ptr(), bytes.len(), &mut raw) };
    if result != IMAGE_DECODER_SUCCESS {
        return Err(anyhow!(
            "Android could not decode a bundled image ({result})"
        ));
    }
    let decoder =
        Decoder(NonNull::new(raw).ok_or_else(|| anyhow!("Android returned no image decoder"))?);
    let header = unsafe { AImageDecoder_getHeaderInfo(decoder.0.as_ptr()) };
    if header.is_null() {
        return Err(anyhow!("Android returned no image header"));
    }
    let width = unsafe { AImageDecoderHeaderInfo_getWidth(header) };
    let height = unsafe { AImageDecoderHeaderInfo_getHeight(header) };
    if width <= 0 || height <= 0 {
        return Err(anyhow!("bundled image has invalid dimensions"));
    }
    let result = unsafe {
        AImageDecoder_setAndroidBitmapFormat(decoder.0.as_ptr(), BITMAP_FORMAT_RGBA_8888)
    };
    if result != IMAGE_DECODER_SUCCESS {
        return Err(anyhow!(
            "Android could not convert a bundled image ({result})"
        ));
    }
    let stride = unsafe { AImageDecoder_getMinimumStride(decoder.0.as_ptr()) };
    let width = width as u32;
    let height = height as u32;
    let row_bytes = width as usize * 4;
    let size = stride
        .checked_mul(height as usize)
        .ok_or_else(|| anyhow!("bundled image is too large"))?;
    if size > MAX_DECODED_IMAGE_BYTES || stride < row_bytes {
        return Err(anyhow!("bundled image is too large"));
    }
    let mut decoded = vec![0; size];
    let result = unsafe {
        AImageDecoder_decodeImage(
            decoder.0.as_ptr(),
            decoded.as_mut_ptr(),
            stride,
            decoded.len(),
        )
    };
    if result != IMAGE_DECODER_SUCCESS {
        return Err(anyhow!(
            "Android could not decode a bundled image ({result})"
        ));
    }
    let pixels = if stride == row_bytes {
        decoded
    } else {
        let mut pixels = Vec::with_capacity(row_bytes * height as usize);
        for row in decoded.chunks(stride).take(height as usize) {
            pixels.extend_from_slice(&row[..row_bytes]);
        }
        pixels
    };
    Ok((width, height, pixels))
}

struct Decoder(NonNull<AImageDecoder>);

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { AImageDecoder_delete(self.0.as_ptr()) };
    }
}
