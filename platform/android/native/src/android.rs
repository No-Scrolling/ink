use std::ffi::{CString, c_char, c_int};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once, OnceLock};
#[cfg(feature = "benchmark")]
use std::time::Instant;

#[cfg(feature = "benchmark")]
use ink_core::PerfTraceSection;
use ink_core::{
    CameraPortal, CameraPreviewKind, ControllerId, Engine, MapPortal, NativeRequestKind, PUBLIC_SANS,
    PointerOutcome, ResourceError, ResourceErrorKind, TextEdit, TextInputAction,
};
use ink_renderer_vulkan::{RenderOutcome, Renderer};
use jni::EnvUnowned;
use jni::objects::JByteArray;
#[cfg(feature = "audio")]
use jni::objects::JShortArray;
use jni::objects::{JByteBuffer, JClass, JIntArray, JObject, JString};
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
const POINTER_HAPTIC: jint = 1 << 3;
const LOG_TAG: &[u8] = b"Ink\0";
static PANIC_HOOK: Once = Once::new();

fn pointer_result(outcome: PointerOutcome) -> jint {
    (if outcome.changed { POINTER_CHANGED } else { 0 })
        | (if outcome.haptic { POINTER_HAPTIC } else { 0 })
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

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

#[cfg(feature = "image")]
#[path = "image.rs"]
mod image;

struct AndroidEngine {
    engine: Engine,
    script: Option<javascript::ScriptRuntime>,
    javascript_error: Option<String>,
    surface: Option<AttachedSurface>,
    sent_camera_portal: Option<(Option<CameraPortal>, bool)>,
    sent_map_portal: Option<Option<MapPortal>>,
    sent_video_portal: Option<Option<MapPortal>>,
    #[cfg(feature = "audio")]
    audio: crate::audio::AudioRuntime,
    #[cfg(feature = "benchmark")]
    update_ns: u64,
}

struct AttachedSurface {
    renderer: Renderer,
    _window: NativeWindow,
}

// Vulkan buffers and surface ownership move together, exclusively under the engine mutex.
unsafe impl Send for AttachedSurface {}

impl AndroidEngine {
    fn new() -> Self {
        Self {
            engine: Engine::new(),
            script: None,
            javascript_error: None,
            surface: None,
            sent_camera_portal: None,
            sent_map_portal: None,
            sent_video_portal: None,
            #[cfg(feature = "audio")]
            audio: crate::audio::AudioRuntime::default(),
            #[cfg(feature = "benchmark")]
            update_ns: 0,
        }
    }

    fn attach(&mut self, env: &EnvUnowned<'_>, surface: &JObject<'_>, width: u32, height: u32) -> bool {
        self.surface = None;
        self.engine.set_viewport(width, height);

        let Some(window) =
            (unsafe { NativeWindow::from_surface(env.as_raw().cast(), surface.as_raw()) })
        else {
            android_log(ANDROID_LOG_ERROR, "ANativeWindow_fromSurface returned null");
            return false;
        };
        let renderer = match unsafe { Renderer::new(window.ptr().as_ptr().cast(), width, height) } {
            Ok(renderer) => renderer,
            Err(error) => {
                android_log(
                    ANDROID_LOG_ERROR,
                    &format!("failed to initialise Vulkan: {error:#}"),
                );
                return false;
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
        true
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
            && let Err(error) = script.notify_inputs(&mut self.engine)
        {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("JavaScript layout update failed: {error:#}"),
            );
            self.script = None;
        }
    }

    fn pointer(&mut self, action: i32, id: i32, x: f32, y: f32) -> jint {
        let _affinity = ink_runtime::cpu_affinity::prefer_performance();
        #[cfg(feature = "bridge-timing")]
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
            && let Err(error) = script.notify_inputs(&mut self.engine)
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
        let _affinity = ink_runtime::cpu_affinity::prefer_performance();
        let changed = self.engine.scroll_by(delta);
        if changed
            && let Some(script) = &mut self.script
            && let Err(error) = script.notify_inputs(&mut self.engine)
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
    fn publish_audio(&mut self) -> bool {
        let mut changed = false;
        for (controller, value, notify) in self.audio.take_updates() {
                let id = (controller.index() as i64).unsigned_abs();
                let value = javascript::state_json(value);
                changed |= self.engine.update_capture_state(id, &value);
                if notify && let Some(script) = &self.script
                    && let Err(error) =
                        script.notify_controller(id, value)
                {
                    android_log(
                        ANDROID_LOG_ERROR,
                        &format!("Audio state delivery failed: {error:#}"),
                    );
                }
        }
        changed
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
            .complete_native_image(request_id, width, height, pixels, None)
    }

    fn fail_native(&mut self, request_id: u64, error: ResourceError) -> bool {
        self.engine.fail_native(request_id, error)
    }

    fn render(&mut self) -> RenderOutcome {
        let _affinity = ink_runtime::cpu_affinity::prefer_performance();
        // Keep the last complete frame until the new content is ready.
        if !self.engine.list_viewports_ready() || !self.engine.camera_review_ready() || !self.engine.screen_images_ready() {
            return RenderOutcome::Skipped;
        }
        let Some(surface) = &mut self.surface else {
            return RenderOutcome::Skipped;
        };
        let mut surface_lost = false;
        let mut presented = false;
        let mut deferred = false;
        match surface.renderer.render(self.engine.scene()) {
            Ok(RenderOutcome::Presented) => {
                presented = true;
                #[cfg(all(feature = "bridge-timing", not(feature = "presentation-timing")))]
                android_log(
                    ANDROID_LOG_INFO,
                    &format!("Submitted scene={} ns={}", self.engine.scene().revision, benchmark_time_ns()),
                );
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
            Ok(RenderOutcome::Deferred) => deferred = true,
            Ok(RenderOutcome::SurfaceLost) => {
                android_log(ANDROID_LOG_ERROR, "Vulkan surface was lost");
                surface_lost = true;
            }
            Ok(RenderOutcome::NeedsSystemGlyph(request)) => return RenderOutcome::NeedsSystemGlyph(request),
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
        if surface_lost { RenderOutcome::SurfaceLost }
        else if deferred { RenderOutcome::Deferred }
        else if presented { RenderOutcome::Presented }
        else { RenderOutcome::Skipped }
    }

    fn install_system_glyph(&mut self, request_id: u64, pixels: &[u8]) -> bool {
        let Some(surface) = &mut self.surface else {
            return false;
        };
        if let Err(error) = surface
            .renderer
            .install_system_glyph(request_id, (!pixels.is_empty()).then_some(pixels))
        {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("failed to install system glyph: {error:#}"),
            );
            return false;
        }
        true
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
    let Ok(mut engines) = engines().lock() else { return 0; };
    if engines.next_id == jlong::MAX { return 0; }
    engines.next_id += 1;
    let id = engines.next_id;
    engines.active.insert(id, Arc::new(parking_lot::Mutex::new(AndroidEngine::new())));
    id
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
    if !cfg!(feature = "camera") { return true as jboolean; }
    engine(handle)
        .map(|engine| engine.lock_arc())
        .is_none_or(|engine| engine.engine.camera_review_ready()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCameraPortal<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    if !cfg!(feature = "camera") {
        return env.with_env(|env| env.new_string(""))
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    }
    let Some(mut engine) = engine(handle).map(|engine| engine.lock_arc()) else {
        return JString::default();
    };
    let scene = engine.engine.scene();
    let light = scene.light;
    let state = (scene.camera_portal, light);
    if engine.sent_camera_portal == Some(state) {
        return JString::default();
    }
    engine.sent_camera_portal = Some(state);
    let value = state.0.map_or_else(String::new, |portal| {
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
    if !cfg!(feature = "maps") {
        return env.with_env(|env| env.new_string(""))
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    }
    let Some(mut engine) = engine(handle).map(|engine| engine.lock_arc()) else {
        return JString::default();
    };
    let state = engine.engine.scene().map_portal;
    if engine.sent_map_portal == Some(state) {
        return JString::default();
    }
    engine.sent_map_portal = Some(state);
    let value = state.map_or_else(String::new, |portal| {
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
    if !cfg!(feature = "video") {
        return env.with_env(|env| env.new_string(""))
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    }
    let Some(mut engine) = engine(handle).map(|engine| engine.lock_arc()) else {
        return JString::default();
    };
    let state = engine.engine.scene().video_portal;
    if engine.sent_video_portal == Some(state) {
        return JString::default();
    }
    engine.sent_video_portal = Some(state);
    let value = state.map_or_else(String::new, |portal| {
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
) -> jboolean {
    let Some(engine) = engine(handle) else {
        return false;
    };
    let mut engine = engine.lock_arc();
    engine.attach(&env, &surface, dimension(width), dimension(height)) as jboolean
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
    let mut engine = engine.lock_arc();
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
    let mut engine = engine.lock_arc();
    engine.resize(dimension(width), dimension(height));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeImageWaitRemaining(
    _env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong,
) -> jlong {
    engine(handle).map(|engine| engine.lock_arc())
        .map_or(0, |engine| engine.engine.image_wait_remaining_ms() as jlong)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeHasSceneAnimations(
    _env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong,
) -> jboolean {
    engine(handle).map(|engine| engine.lock_arc())
        .is_some_and(|engine| engine.engine.has_scene_animations()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeAnimateScene(
    _env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong,
) -> jboolean {
    engine(handle).map(|engine| engine.lock_arc())
        .is_some_and(|mut engine| engine.engine.animate_scene()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeFrameReady(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
        .map_or(0, |mut engine| engine.image_pinch_update(scale, x, y))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeImagePinchEnd(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    if let Some(engine) = engine(handle)
        && let mut engine = engine.lock_arc()
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
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
        .is_some_and(|mut engine| engine.scroll_by(delta)) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRender<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let outcome = if let Some(engine) = engine(handle)
        && let mut engine = engine.lock_arc()
    {
        engine.render()
    } else {
        RenderOutcome::Skipped
    };
    let value = match outcome {
        RenderOutcome::Presented => "presented".to_owned(),
        RenderOutcome::Deferred => "retry".to_owned(),
        RenderOutcome::SurfaceLost => "surface-lost".to_owned(),
        RenderOutcome::NeedsSystemGlyph(request) => format!(
            "{}\n{}\n{}",
            request.id, request.pixel_size, request.grapheme,
        ),
        _ => String::new(),
    };
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeInstallSystemGlyph(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
    pixels: JIntArray<'_>,
) -> jboolean {
    let pixels = env
        .with_env(|env| -> jni::errors::Result<Vec<u8>> {
            let mut argb = vec![0; env.get_array_length(&pixels)? as usize];
            env.get_int_array_region(&pixels, 0, &mut argb)?;
            // Bitmap.getPixels returns straight-alpha ARGB; Vulkan samples RGBA.
            Ok(argb
                .into_iter()
                .flat_map(|pixel| (pixel as u32).rotate_left(8).to_be_bytes())
                .collect())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
    if let Some(engine) = engine(handle)
        && let mut engine = engine.lock_arc()
    {
        return engine.install_system_glyph(request_id as u64, &pixels) as jboolean;
    }
    false
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeBack(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .map(|engine| engine.lock_arc())
        .is_some_and(|mut engine| engine.back()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInputState(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jint {
    if !cfg!(feature = "text-input") { return 0; }
    engine(handle)
        .map(|engine| engine.lock_arc())
        .map(|engine| {
            let action = match engine.engine.text_input_action() {
                TextInputAction::Return => 0,
                TextInputAction::Search => 1,
                TextInputAction::Done => 2,
            };
            // Match syncTextInput: active, numeric, then the keyboard action.
            engine.engine.text_input_active() as jint
                | ((engine.engine.text_input_numeric() as jint) << 1)
                | (action << 2)
        })
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInputContext<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    if !cfg!(feature = "text-input") {
        return env.with_env(|env| env.new_string(""))
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    }
    let value = engine(handle)
        .map(|engine| engine.lock_arc())
        .map(|engine| engine.engine.text_input_context())
        .unwrap_or_default();
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTextInput(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    action: jint,
    value: JString<'_>,
) -> jboolean {
    if !cfg!(feature = "text-input") { return false as jboolean; }
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
        .map(|engine| engine.lock_arc())
        .is_some_and(|mut engine| engine.edit_text(edit)) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeNextRequest<'local>(
    mut env: EnvUnowned<'local>, _class: JClass<'local>, handle: jlong,
) -> JString<'local> {
    let value = engine(handle).map(|engine| engine.lock_arc()).and_then(|mut engine| {
        let started = std::time::Instant::now();
        for _ in 0..32 {
            let request = engine.engine.take_native_request()?;
            if request.module() == "ink" && request.operation() == "event" {
                #[cfg(feature = "bridge-timing")]
                android_log(ANDROID_LOG_INFO, &format!("ReactDispatch ns={}", benchmark_time_ns()));
                if let Some(script) = &engine.script && let Err(error) = script.send(request.payload().to_owned()) {
                    android_log(ANDROID_LOG_ERROR, &error.to_string());
                }
                engine.engine.complete_native_action(request.id());
                if started.elapsed() < std::time::Duration::from_millis(4) { continue; }
                break;
            }
            return Some(serde_json::json!({
                "id": request.id(), "kind": match request.kind() {
                    NativeRequestKind::Action => 1, NativeRequestKind::Cancel => 2, NativeRequestKind::Image => 3,
                },
                "controller": request.controller().map_or(-1, |id| id.index() as i64),
                "module": request.module(), "operation": request.operation(),
                "payload": request.payload(), "timeout": request.timeout_ms(),
            }).to_string());
        }
        Some("{\"yield\":true}".to_owned())
    });
    match value {
        Some(value) => env.with_env(|env| env.new_string(value)).resolve::<jni::errors::ThrowRuntimeExAndDefault>(),
        None => JString::default(),
    }
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
        .map(|engine| engine.lock_arc())
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
    let Some((width, height, fit, looping)) = engine.lock_arc().engine.image_request_target(request_id as u64)
    else {
        return false as jboolean;
    };
    let decoded = image::decode(&path, width, height, fit, looping);
    let mut engine = engine.lock_arc();
    (match decoded {
        Ok((width, height, pixels, animation)) => {
            engine.engine.complete_native_image(request_id as u64, width, height, pixels, animation)
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
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
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
        .map(|engine| engine.lock_arc())
        .is_some_and(|engine| {
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
    if let Some(engine) = engine(handle).map(|engine| engine.lock_arc()) {
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
        .map(|engine| engine.lock_arc())
        .is_some_and(|engine| {
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
    if let Some(engine) = engine(handle) {
        engine.lock_arc().audio.submit(samples, sample_rate as u32);
        true as jboolean
    } else { false as jboolean }
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
    let mut engine = engine.lock_arc();
    engine.surface = None;
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeDestroy(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    let retired = engines().lock().ok().and_then(|mut engines| engines.active.remove(&handle));
    if let Some(retired) = retired {
        std::thread::spawn(move || drop(retired));
    }
}

#[derive(Default)]
struct Engines {
    next_id: jlong,
    active: HashMap<jlong, Arc<parking_lot::Mutex<AndroidEngine>>>,
}

fn engines() -> &'static Mutex<Engines> {
    static ENGINES: OnceLock<Mutex<Engines>> = OnceLock::new();
    ENGINES.get_or_init(Mutex::default)
}

fn engine(handle: jlong) -> Option<Arc<parking_lot::Mutex<AndroidEngine>>> {
    engines().lock().ok()?.active.get(&handle).cloned()
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

#[cfg(feature = "bridge-timing")]
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
