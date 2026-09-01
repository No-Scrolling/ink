#[cfg(all(target_os = "android", feature = "image"))]
mod android_image;
#[cfg(all(target_os = "android", not(feature = "image")))]
mod android_image {
    use anyhow::{Result, anyhow};

    pub fn decode(_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
        Err(anyhow!("image decoding is not included in this app"))
    }
}
mod compact;
mod system_glyph;

#[cfg(feature = "perf")]
pub use compact::RenderPerfMetrics;
pub use compact::{RenderOutcome, Renderer};
pub use system_glyph::SystemGlyphRequest;
