#[cfg(target_os = "android")]
mod android_image;
mod compact;
mod system_glyph;

pub use compact::{RenderOutcome, Renderer};
pub use system_glyph::SystemGlyphRequest;
