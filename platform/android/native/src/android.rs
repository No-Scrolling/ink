#[cfg(feature = "image")]
use std::ffi::c_void;
use std::ffi::{CString, c_char, c_int};
#[cfg(feature = "image")]
use std::os::fd::AsRawFd;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, Once};
#[cfg(feature = "benchmark")]
use std::time::Instant;

#[cfg(feature = "image")]
use ink_core::ImageFit;
#[cfg(feature = "benchmark")]
use ink_core::PerfTraceSection;
use ink_core::{
    CameraPreviewKind, ControllerId, Engine, NativeRequestKind, PUBLIC_SANS, PointerOutcome,
    ResourceError, ResourceErrorKind, TextEdit, TextInputAction,
};
use ink_renderer_vulkan::{RenderOutcome, Renderer, SystemGlyphRequest};
use jni::EnvUnowned;
use jni::objects::JByteArray;
#[cfg(feature = "audio")]
use jni::objects::JShortArray;
use jni::objects::{JByteBuffer, JClass, JObject, JString};
use jni::sys::{jboolean, jfloat, jint, jlong};
use ndk::native_window::NativeWindow;

#[path = "javascript.rs"]
mod javascript;

#[cfg(feature = "background")]
#[path = "worker.rs"]
mod worker;

const ANDROID_LOG_INFO: c_int = 4;
const ANDROID_LOG_WARN: c_int = 5;
const ANDROID_LOG_ERROR: c_int = 6;
const POINTER_CHANGED: jint = 1;
const POINTER_ACTIVATED: jint = 1 << 1;
const POINTER_CAPTURED: jint = 1 << 2;
const LOG_TAG: &[u8] = b"Ink\0";
static PANIC_HOOK: Once = Once::new();

fn pointer_result(outcome: PointerOutcome) -> jint {
    (if outcome.changed { POINTER_CHANGED } else { 0 })
        | (if outcome.activated {
            POINTER_ACTIVATED
        } else {
            0
        })
        | (if outcome.captured {
            POINTER_CAPTURED
        } else {
            0
        })
}

#[cfg(feature = "image")]
const IMAGE_DECODER_SUCCESS: c_int = 0;
#[cfg(feature = "image")]
const BITMAP_FORMAT_RGBA_8888: c_int = 1;

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

#[cfg(feature = "image")]
enum AImageDecoder {}
#[cfg(feature = "image")]
enum AImageDecoderHeaderInfo {}

#[cfg(feature = "image")]
#[link(name = "jnigraphics")]
unsafe extern "C" {
    fn AImageDecoder_createFromFd(fd: c_int, decoder: *mut *mut AImageDecoder) -> c_int;
    fn AImageDecoder_delete(decoder: *mut AImageDecoder);
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

struct AndroidEngine {
    engine: Engine,
    script: Option<javascript::ScriptRuntime>,
    javascript_error: Option<String>,
    surface: Option<AttachedSurface>,
    #[cfg(feature = "audio")]
    audio: crate::audio::AudioRuntime,
    #[cfg(feature = "benchmark")]
    update_ns: u64,
}

struct AttachedSurface {
    renderer: Renderer,
    _window: NativeWindow,
}

impl AndroidEngine {
    fn new() -> Self {
        Self {
            engine: Engine::new(),
            script: None,
            javascript_error: None,
            surface: None,
            #[cfg(feature = "audio")]
            audio: crate::audio::AudioRuntime::default(),
            #[cfg(feature = "benchmark")]
            update_ns: 0,
        }
    }

    fn attach(&mut self, env: &EnvUnowned<'_>, surface: &JObject<'_>, width: u32, height: u32) {
        self.surface = None;
        self.engine.set_viewport(width, height);

        let Some(window) =
            (unsafe { NativeWindow::from_surface(env.as_raw().cast(), surface.as_raw()) })
        else {
            android_log(ANDROID_LOG_ERROR, "ANativeWindow_fromSurface returned null");
            return;
        };
        let renderer = match unsafe { Renderer::new(window.ptr().as_ptr().cast(), width, height) } {
            Ok(renderer) => renderer,
            Err(error) => {
                android_log(
                    ANDROID_LOG_ERROR,
                    &format!("failed to initialise Vulkan: {error:#}"),
                );
                return;
            }
        };

        self.surface = Some(AttachedSurface {
            _window: window,
            renderer,
        });
        android_log(
            ANDROID_LOG_INFO,
            &format!("attached Vulkan surface {width}x{height}"),
        );
    }

    fn resize(&mut self, width: u32, height: u32) {
        if !self.engine.set_viewport(width, height) {
            return;
        }
        if let Some(surface) = &mut self.surface {
            surface.renderer.resize(width, height);
        }
        self.notify_layout_inputs();
    }

    fn notify_layout_inputs(&mut self) {
        if let Some(script) = &mut self.script
            && let Err(error) = script.notify_inputs(&self.engine)
        {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("JavaScript layout update failed: {error:#}"),
            );
            self.script = None;
        }
    }

    fn pointer(&mut self, action: i32, id: i32, x: f32, y: f32) -> jint {
        #[cfg(feature = "presentation-timing")]
        if action == 1 {
            android_log(
                ANDROID_LOG_INFO,
                &format!("InputUp ns={}", benchmark_time_ns()),
            );
        }
        #[cfg(feature = "benchmark")]
        let update_trace = (action == 1).then(|| PerfTraceSection::new(b"Ink pointer update\0"));
        #[cfg(feature = "benchmark")]
        let started = Instant::now();
        let outcome = match action {
            0 => self.engine.pointer_down(id, x, y),
            1 => self.engine.pointer_up(id, x, y),
            2 => self.engine.pointer_move(id, x, y),
            4 => self.engine.pointer_long_press(id, x, y), // POINTER_LONG_PRESS in MainActivity.
            3 => {
                self.engine.pointer_cancel();
                PointerOutcome::default()
            }
            _ => PointerOutcome::default(),
        };
        if outcome.changed
            && let Some(script) = &mut self.script
            && let Err(error) = script.notify_inputs(&self.engine)
        {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("JavaScript input failed: {error:#}"),
            );
            self.script = None;
        }
        #[cfg(feature = "benchmark")]
        if action == 1 && outcome.changed {
            self.update_ns = elapsed_ns(started);
        }
        #[cfg(feature = "benchmark")]
        drop(update_trace);
        pointer_result(outcome)
    }

    fn image_pinch_begin(&mut self, x: f32, y: f32) -> jint {
        if self.engine.image_pinch_begin(x, y) {
            POINTER_CAPTURED
        } else {
            0
        }
    }

    fn image_zoom_target(&self, x: f32, y: f32) -> Option<u64> {
        self.engine.image_zoom_target(x, y)
    }

    fn image_pinch_update(&mut self, scale: f32, x: f32, y: f32) -> jint {
        POINTER_CAPTURED
            | if self.engine.image_pinch_update(scale, x, y) {
                POINTER_CHANGED
            } else {
                0
            }
    }

    fn image_double_tap(&mut self, x: f32, y: f32) -> jint {
        pointer_result(self.engine.image_double_tap(x, y))
    }

    fn scroll_by(&mut self, delta: f32) -> bool {
        let changed = self.engine.scroll_by(delta);
        if changed
            && let Some(script) = &mut self.script
            && let Err(error) = script.notify_inputs(&self.engine)
        {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("JavaScript scroll failed: {error:#}"),
            );
            self.script = None;
        }
        changed
    }

    fn back(&mut self) -> bool {
        self.engine.back()
    }

    fn edit_text(&mut self, edit: TextEdit) -> bool {
        if let Some(script) = &mut self.script {
            return match script.edit_text(&mut self.engine, edit) {
                Ok(changed) => changed,
                Err(error) => {
                    android_log(
                        ANDROID_LOG_ERROR,
                        &format!("JavaScript input failed: {error:#}"),
                    );
                    self.script = None;
                    true
                }
            };
        }
        false
    }

    #[cfg(feature = "audio")]
    fn process_audio(&mut self, samples: &[i16], sample_rate: u32) -> bool {
        self.audio
            .process(samples, sample_rate, |controller, value| {
                if let Some(script) = &self.script
                    && let Err(error) =
                        script.notify_controller((controller.index() as i64).unsigned_abs(), value)
                {
                    android_log(
                        ANDROID_LOG_ERROR,
                        &format!("Audio state delivery failed: {error:#}"),
                    );
                }
                false
            })
    }

    #[cfg(feature = "image")]
    fn complete_native_image(
        &mut self,
        request_id: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> bool {
        self.engine
            .complete_native_image(request_id, width, height, pixels)
    }

    fn fail_native(&mut self, request_id: u64, error: ResourceError) -> bool {
        self.engine.fail_native(request_id, error)
    }

    fn render(&mut self) -> Option<SystemGlyphRequest> {
        // Keep the last complete frame while rows or the camera review image load.
        if !self.engine.list_viewports_ready() || !self.engine.camera_review_ready() {
            return None;
        }
        let Some(surface) = &mut self.surface else {
            return None;
        };
        let mut surface_lost = false;
        match surface.renderer.render(self.engine.scene()) {
            Ok(RenderOutcome::Presented) => {
                #[cfg(feature = "presentation-timing")]
                {
                    android_log(
                        ANDROID_LOG_INFO,
                        &format!(
                            "Presented id={} scene={} scroll_y={} ns={}",
                            surface.renderer.present_id(),
                            self.engine.scene().revision,
                            self.engine.scroll_offset(),
                            benchmark_time_ns()
                        ),
                    );
                    match surface.renderer.presentation_times() {
                        Ok(Some(times)) => {
                            for (id, actual_ns) in times {
                                android_log(
                                    ANDROID_LOG_INFO,
                                    &format!("Presentation id={id} actual_ns={actual_ns}"),
                                );
                            }
                        }
                        Ok(None) => {
                            android_log(ANDROID_LOG_INFO, "Presentation timing unavailable")
                        }
                        Err(error) => {
                            android_log(ANDROID_LOG_WARN, &format!("Presentation timing: {error}"))
                        }
                    }
                }
            }
            Ok(RenderOutcome::Skipped) => {
                android_log(ANDROID_LOG_WARN, "surface skipped dirty frame");
            }
            Ok(RenderOutcome::SurfaceLost) => {
                android_log(ANDROID_LOG_ERROR, "Vulkan surface was lost");
                surface_lost = true;
            }
            Ok(RenderOutcome::NeedsSystemGlyph(request)) => return Some(request),
            Err(error) => {
                android_log(
                    ANDROID_LOG_ERROR,
                    &format!("failed to render dirty frame: {error:#}"),
                );
            }
        }
        #[cfg(feature = "memory-diagnostics")]
        {
            let memory = surface.renderer.memory_metrics();
            android_log(
                ANDROID_LOG_INFO,
                &format!(
                    "InkMemory {}",
                    serde_json::json!({
                        "version": 1,
                        "revision": option_env!("INK_BENCHMARK_REVISION").unwrap_or("unknown"),
                        "native": self.script.as_ref().map(|script| script.memory_diagnostics()),
                        "renderer": {
                            "instance_buffer_capacity_bytes": memory.instance_buffer_capacity_bytes,
                            "instance_snapshot_capacity_bytes": memory.instance_snapshot_capacity_bytes,
                            "font_texture_bytes": memory.font_texture_bytes,
                            "image_texture_bytes": memory.image_texture_bytes,
                            "system_glyph_texture_bytes": memory.system_glyph_texture_bytes,
                            "image_pipeline_created": memory.image_pipeline_created,
                        },
                    })
                ),
            );
        }
        #[cfg(feature = "benchmark")]
        {
            let core = self.engine.take_perf_metrics();
            let renderer = surface.renderer.take_perf_metrics();
            android_log(
                ANDROID_LOG_INFO,
                &format!(
                    "Perf revision={} update_ns={} measure_ns={} relayout_ns={} nodes_measured={} full_rebuilds={} incremental_rebuilds={} prepare_ns={} upload_ns={} acquire_ns={} encode_ns={} submit_present_ns={} frame_ns={} instances={} uploaded_bytes={} draw_calls={} cache_misses={}",
                    option_env!("INK_BENCHMARK_REVISION").unwrap_or("unknown"),
                    self.update_ns,
                    core.measure_ns,
                    core.relayout_ns,
                    core.nodes_measured,
                    core.full_rebuilds,
                    core.incremental_rebuilds,
                    renderer.prepare_ns,
                    renderer.upload_ns,
                    renderer.acquire_ns,
                    renderer.encode_ns,
                    renderer.submit_present_ns,
                    renderer.frame_ns,
                    renderer.instances,
                    renderer.uploaded_bytes,
                    renderer.draw_calls,
                    renderer.cache_misses,
                ),
            );
            if let Some(ns) = renderer.gpu_ns {
                android_log(ANDROID_LOG_INFO, &format!("GPU command_span_ns={ns}"));
            }
            self.update_ns = 0;
        }
        if surface_lost {
            self.surface = None;
        }
        None
    }

    fn install_system_glyph(&mut self, request_id: u64, pixels: &[u8]) {
        let Some(surface) = &mut self.surface else {
            return;
        };
        if let Err(error) = surface
            .renderer
            .install_system_glyph(request_id, (!pixels.is_empty()).then_some(pixels))
        {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("failed to install system glyph: {error:#}"),
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCreate(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
) -> jlong {
    PANIC_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|panic| {
            android_log(ANDROID_LOG_ERROR, &format!("native panic: {panic}"));
        }));
    });
    android_log(ANDROID_LOG_INFO, "created Ink engine");
    Box::into_raw(Box::new(Mutex::new(AndroidEngine::new()))) as jlong
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativePublicSans<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JByteBuffer<'local> {
    env.with_env(|env| unsafe {
        env.new_direct_byte_buffer(PUBLIC_SANS.as_ptr().cast_mut(), PUBLIC_SANS.len())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCameraReviewReady(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_none_or(|engine| engine.engine.camera_review_ready()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCameraPortal<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let value = engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| {
            let scene = engine.engine.scene();
            scene.camera_portal.map(|portal| (portal, scene.light))
        })
        .map_or_else(String::new, |(portal, light)| {
            let kind = match portal.kind {
                CameraPreviewKind::Photo => "photo",
                CameraPreviewKind::Scanner => "scanner",
            };
            let left = portal.rect.x.round() as i32;
            let top = portal.rect.y.round() as i32;
            let right = (portal.rect.x + portal.rect.width).round() as i32;
            let bottom = (portal.rect.y + portal.rect.height).round() as i32;
            format!(
                "{{\"controller\":{},\"kind\":\"{}\",\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"light\":{}}}",
                portal.controller.index() as i64,
                kind,
                left,
                top,
                (right - left).max(1),
                (bottom - top).max(1),
                light,
            )
        });
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeMapPortal<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let value = engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| engine.engine.scene().map_portal)
        .map_or_else(String::new, |portal| {
            serde_json::json!({
                "controller": portal.controller.index() as i64,
                "x": portal.rect.x.round() as i32,
                "y": portal.rect.y.round() as i32,
                "width": portal.rect.width.round().max(1.0) as i32,
                "height": portal.rect.height.round().max(1.0) as i32,
            }).to_string()
        });
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeVideoPortal<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let value = engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| engine.engine.scene().video_portal)
        .map_or_else(String::new, |portal| {
            serde_json::json!({
                "controller": portal.controller.index() as i64,
                "x": portal.rect.x.round() as i32,
                "y": portal.rect.y.round() as i32,
                "width": portal.rect.width.round().max(1.0) as i32,
                "height": portal.rect.height.round().max(1.0) as i32,
            }).to_string()
        });
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeAttachSurface(
    env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    surface: JObject<'_>,
    width: jint,
    height: jint,
) {
    let Some(engine) = engine(handle) else {
        return;
    };
    let Ok(mut engine) = engine.lock() else {
        return;
    };
    engine.attach(&env, &surface, dimension(width), dimension(height));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeSetKeyboardInset(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    height: jint,
) -> jboolean {
    let Some(engine) = engine(handle) else {
        return false;
    };
    let Ok(mut engine) = engine.lock() else {
        return false;
    };
    let changed = engine.engine.set_keyboard_inset(height.max(0) as u32);
    if changed {
        engine.notify_layout_inputs();
    }
    changed
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeResize(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    width: jint,
    height: jint,
) {
    let Some(engine) = engine(handle) else {
        return;
    };
    let Ok(mut engine) = engine.lock() else {
        return;
    };
    engine.resize(dimension(width), dimension(height));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeFrameReady(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|engine| {
            engine
                .surface
                .as_ref()
                .is_none_or(|surface| surface.renderer.frame_ready())
        }) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCanPresentPointerMove(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    id: jint,
    y: jfloat,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|engine| engine.engine.can_present_pointer_move(id, y)) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativePointer(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    action: jint,
    id: jint,
    x: jfloat,
    y: jfloat,
) -> jint {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map_or(0, |mut engine| engine.pointer(action, id, x, y))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeImagePinchBegin(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map_or(0, |mut engine| engine.image_pinch_begin(x, y))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeImagePinchUpdate(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    scale: jfloat,
    x: jfloat,
    y: jfloat,
) -> jint {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map_or(0, |mut engine| engine.image_pinch_update(scale, x, y))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeImagePinchEnd(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    if let Some(engine) = engine(handle)
        && let Ok(mut engine) = engine.lock()
    {
        engine.engine.image_pinch_end();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeImageZoomTarget(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    x: jfloat,
    y: jfloat,
) -> jlong {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| engine.image_zoom_target(x, y))
        .map_or(0, |target| target as jlong)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeImageDoubleTap(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    x: jfloat,
    y: jfloat,
) -> jint {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map_or(0, |mut engine| engine.image_double_tap(x, y))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeScrollBy(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    delta: jfloat,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.scroll_by(delta)) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeScrollOffset(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jfloat {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map_or(0.0, |engine| engine.engine.scroll_offset())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeScrollMaximum(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jfloat {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map_or(0.0, |engine| engine.engine.scroll_max())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRender<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let request = if let Some(engine) = engine(handle)
        && let Ok(mut engine) = engine.lock()
    {
        engine.render()
    } else {
        None
    };
    let value = request.map_or_else(String::new, |request| {
        format!(
            "{}\n{}\n{}",
            request.id, request.pixel_size, request.grapheme,
        )
    });
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeInstallSystemGlyph(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
    pixels: JByteArray<'_>,
) {
    let pixels = env
        .with_env(|env| env.convert_byte_array(&pixels))
        .resolve::<jni::errors::LogErrorAndDefault>();
    if let Some(engine) = engine(handle)
        && let Ok(mut engine) = engine.lock()
    {
        engine.install_system_glyph(request_id as u64, &pixels);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeBack(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.back()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInputActive(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|engine| engine.engine.text_input_active()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInputContext<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let value = engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map(|engine| engine.engine.text_input_context())
        .unwrap_or_default();
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInputNumeric(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|engine| engine.engine.text_input_numeric()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInputAction(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jint {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .map(|engine| match engine.engine.text_input_action() {
            TextInputAction::Return => 0,
            TextInputAction::Search => 1,
            TextInputAction::Done => 2,
        })
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInput(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    action: jint,
    value: JString<'_>,
) -> jboolean {
    let edit = match action {
        2 => Some(TextEdit::Submit),
        3 => Some(TextEdit::Dismiss),
        4 if !value.is_null() => Some(TextEdit::Update(
            env.with_env(|env| value.try_to_string(env))
                .resolve::<jni::errors::LogErrorAndDefault>(),
        )),
        _ => None,
    };
    let Some(edit) = edit else {
        return false as jboolean;
    };
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.edit_text(edit)) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeNextRequest(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jlong {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|mut engine| {
            loop {
                let request = engine.engine.take_native_request()?;
                if request.module() == "ink" && request.operation() == "event" {
                    #[cfg(feature = "presentation-timing")]
                    android_log(
                        ANDROID_LOG_INFO,
                        &format!("ReactDispatch ns={}", benchmark_time_ns()),
                    );
                    if let Some(script) = &engine.script
                        && let Err(error) = script.send(request.payload().to_owned())
                    {
                        android_log(ANDROID_LOG_ERROR, &error.to_string());
                    }
                    engine.engine.complete_native_action(request.id());
                    continue;
                }
                return Some(request);
            }
        })
        .map_or(0, |request| request.id() as jlong)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRequestModule<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    request_id: jlong,
) -> JString<'local> {
    native_request_string(&mut env, handle, request_id, |request| request.module())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRequestKind(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
) -> jint {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| {
            engine
                .engine
                .native_request(request_id as u64)
                .map(|request| match request.kind() {
                    NativeRequestKind::Action => 1,
                    NativeRequestKind::Cancel => 2,
                    NativeRequestKind::Image => 3,
                })
        })
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRequestController(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
) -> jlong {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| {
            engine
                .engine
                .native_request(request_id as u64)
                .and_then(|request| request.controller())
        })
        .map_or(-1, |controller| controller.index() as jlong)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRequestTimeoutMs(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
) -> jlong {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| {
            engine
                .engine
                .native_request(request_id as u64)
                .map(|request| request.timeout_ms() as jlong)
        })
        .unwrap_or_default()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRequestOperation<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    request_id: jlong,
) -> JString<'local> {
    native_request_string(&mut env, handle, request_id, |request| request.operation())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRequestPayload<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    request_id: jlong,
) -> JString<'local> {
    native_request_string(&mut env, handle, request_id, |request| request.payload())
}

#[cfg(feature = "image")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCompletePixels(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
    width: jint,
    height: jint,
    rgba: JByteArray<'_>,
) -> jboolean {
    if width <= 0 || height <= 0 {
        return false as jboolean;
    }
    let pixels = env
        .with_env(|env| env.convert_byte_array(&rgba))
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| {
            engine.complete_native_image(request_id as u64, width as u32, height as u32, pixels)
        }) as jboolean
}

#[cfg(feature = "image")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCompleteFile(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
    path: JString<'_>,
) -> jboolean {
    let path = env
        .with_env(|env| path.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let Some(engine) = engine(handle) else {
        return false as jboolean;
    };
    let Some((width, height, fit)) = engine
        .lock()
        .ok()
        .and_then(|engine| engine.engine.image_request_target(request_id as u64))
    else {
        return false as jboolean;
    };
    let decoded = decode_image(&path, width, height, fit);
    engine.lock().ok().is_some_and(|mut engine| match decoded {
        Ok((width, height, pixels)) => {
            engine.complete_native_image(request_id as u64, width, height, pixels)
        }
        Err(message) => engine.fail_native(
            request_id as u64,
            ResourceError::new(ResourceErrorKind::Protocol, message, false),
        ),
    }) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeFailRequest(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
    kind: jint,
    message: JString<'_>,
    retryable: jboolean,
) -> jboolean {
    let message = env
        .with_env(|env| message.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let kind = match kind {
        0 => ResourceErrorKind::Unavailable,
        1 => ResourceErrorKind::PermissionDenied,
        2 => ResourceErrorKind::Timeout,
        3 => ResourceErrorKind::Protocol,
        5 => ResourceErrorKind::PermissionBlocked,
        6 => ResourceErrorKind::LocationDisabled,
        7 => ResourceErrorKind::NfcDisabled,
        8 => ResourceErrorKind::Busy,
        _ => ResourceErrorKind::Unexpected,
    };
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| {
            engine.fail_native(
                request_id as u64,
                ResourceError::new(kind, message, retryable),
            )
        }) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCompleteAction(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.engine.complete_native_action(request_id as u64))
        as jboolean
}

#[cfg(feature = "audio")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeAudioActivate(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    controller: jlong,
    kind: JString<'_>,
    config: JString<'_>,
) -> jboolean {
    let kind = env
        .with_env(|env| kind.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let config = env
        .with_env(|env| config.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| {
            engine
                .audio
                .activate(ControllerId::new(controller as usize), &kind, &config)
        }) as jboolean
}

#[cfg(feature = "audio")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeAudioDeactivate(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    controller: jlong,
) {
    if let Some(mut engine) = engine(handle).and_then(|engine| engine.lock().ok()) {
        engine
            .audio
            .deactivate(ControllerId::new(controller as usize));
    }
}

#[cfg(feature = "audio")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeAudioSetEnabled(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    controller: jlong,
    enabled: jboolean,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| {
            engine
                .audio
                .set_enabled(ControllerId::new(controller as usize), enabled)
        }) as jboolean
}

#[cfg(feature = "audio")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeAudioSamples(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    samples: JShortArray<'_>,
    sample_rate: jint,
) -> jboolean {
    let samples = env
        .with_env(|env| {
            let mut output = vec![0_i16; samples.len(env)?];
            samples.get_region(env, 0, &mut output)?;
            Ok::<_, jni::errors::Error>(output)
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.process_audio(&samples, sample_rate as u32))
        as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeDetachSurface(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    let Some(engine) = engine(handle) else {
        return;
    };
    let Ok(mut engine) = engine.lock() else {
        return;
    };
    engine.surface = None;
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeDestroy(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    let Some(pointer) = NonNull::new(handle as *mut Mutex<AndroidEngine>) else {
        return;
    };
    unsafe {
        let engine = Box::from_raw(pointer.as_ptr());
        drop(engine);
    }
}

#[cfg(feature = "image")]
fn decode_image(
    path: &str,
    target_width: u32,
    target_height: u32,
    fit: ImageFit,
) -> Result<(u32, u32, Vec<u8>), String> {
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
    let mut pixels = if stride == width as usize * 4 {
        decoded
    } else {
        let row_bytes = width as usize * 4;
        let mut compact = Vec::with_capacity(row_bytes * height as usize);
        for row in decoded.chunks(stride).take(height as usize) {
            compact.extend_from_slice(&row[..row_bytes]);
        }
        compact
    };
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    Ok((width, height, pixels))
}

#[cfg(feature = "image")]
struct ImageDecoder(NonNull<AImageDecoder>);

#[cfg(feature = "image")]
impl Drop for ImageDecoder {
    fn drop(&mut self) {
        unsafe { AImageDecoder_delete(self.0.as_ptr()) };
    }
}

fn engine(handle: jlong) -> Option<&'static Mutex<AndroidEngine>> {
    let pointer = NonNull::new(handle as *mut Mutex<AndroidEngine>)?;
    Some(unsafe { pointer.as_ref() })
}

fn native_request_string<'local>(
    env: &mut EnvUnowned<'local>,
    handle: jlong,
    request_id: jlong,
    field: for<'a> fn(&'a ink_core::NativeRequest) -> &'a str,
) -> JString<'local> {
    let value = engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| {
            engine
                .engine
                .native_request(request_id as u64)
                .map(field)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

fn dimension(value: jint) -> u32 {
    value.max(0) as u32
}

fn android_log(priority: c_int, message: &str) {
    let Ok(message) = CString::new(message.replace('\0', "�")) else {
        return;
    };
    unsafe {
        __android_log_write(priority, LOG_TAG.as_ptr().cast(), message.as_ptr());
    }
}

#[cfg(feature = "presentation-timing")]
fn benchmark_time_ns() -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) },
        0
    );
    time.tv_sec as u64 * 1_000_000_000 + time.tv_nsec as u64
}

#[cfg(feature = "benchmark")]
fn elapsed_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos() as u64
}
