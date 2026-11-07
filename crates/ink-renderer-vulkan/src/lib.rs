#[cfg(all(target_os = "android", feature = "image"))]
mod android_image;
mod gpu;
#[cfg(all(target_os = "android", not(feature = "image")))]
mod android_image {
    use anyhow::{Result, anyhow};

    pub fn decode(_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
        Err(anyhow!("image decoding is not included in this app"))
    }
}
mod compact;
mod merlin;
mod system_glyph;

#[cfg(feature = "perf")]
pub use compact::{RenderMemoryMetrics, RenderPerfMetrics};
pub use compact::{RenderOutcome, Renderer};
pub use system_glyph::SystemGlyphRequest;

/// Startup timing is opt-in with the existing presentation instrumentation.
pub struct StartupSpan {
    #[cfg(all(target_os = "android", feature = "presentation-timing"))]
    name: &'static str,
    #[cfg(all(target_os = "android", feature = "presentation-timing"))]
    started: std::time::Instant,
}
impl StartupSpan {
    pub fn new(_name: &'static str) -> Self {
        #[cfg(all(target_os = "android", feature = "presentation-timing"))]
        startup_marker(&format!("span={} phase=begin", _name));
        Self {
            #[cfg(all(target_os = "android", feature = "presentation-timing"))]
            name: _name,
            #[cfg(all(target_os = "android", feature = "presentation-timing"))]
            started: std::time::Instant::now(),
        }
    }
}
impl Drop for StartupSpan {
    fn drop(&mut self) {
        #[cfg(all(target_os = "android", feature = "presentation-timing"))]
        startup_marker(&format!("span={} phase=end duration_ns={}", self.name, self.started.elapsed().as_nanos()));
    }
}
pub fn startup_marker(_message: &str) {
    #[cfg(all(target_os = "android", feature = "presentation-timing"))]
    {
        use std::ffi::{CString, c_char, c_int};
        #[link(name = "log")]
        unsafe extern "C" {
            fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
        }
        if let Ok(message) = CString::new(format!("Startup {}", _message)) {
            unsafe { __android_log_write(4, c"Ink".as_ptr(), message.as_ptr()); }
        }
    }
}
