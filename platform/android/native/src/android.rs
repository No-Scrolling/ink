#[cfg(feature = "image")]
use std::ffi::c_void;
use std::ffi::{CString, c_char, c_int};
use std::io::Write;
#[cfg(feature = "image")]
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, Once};

#[cfg(feature = "image")]
use ink_core::ImageFit;
use ink_core::{
    AppDefinition, CameraPreviewKind, ControllerId, Engine, Hydration, NativeRequestKind,
    PUBLIC_SANS, ResourceError, ResourceErrorKind, StateValue, TextEdit, TextInputAction,
};
use ink_renderer_wgpu::{RenderOutcome, Renderer};
use jni::EnvUnowned;
use jni::objects::JByteArray;
#[cfg(feature = "audio")]
use jni::objects::JShortArray;
use jni::objects::{JByteBuffer, JClass, JObject, JString};
use jni::sys::{jboolean, jfloat, jint, jlong};
use ndk::native_window::NativeWindow;

const ANDROID_LOG_INFO: c_int = 4;
const ANDROID_LOG_WARN: c_int = 5;
const ANDROID_LOG_ERROR: c_int = 6;
const LOG_TAG: &[u8] = b"Ink\0";
static PANIC_HOOK: Once = Once::new();
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
    uses_persistence: bool,
    state_path: PathBuf,
    surface: Option<AttachedSurface>,
    #[cfg(feature = "audio")]
    audio: crate::audio::AudioRuntime,
}

struct AttachedSurface {
    _window: Arc<NativeWindow>,
    renderer: Renderer,
}

struct AndroidWindow(Arc<NativeWindow>);

impl wgpu::rwh::HasWindowHandle for AndroidWindow {
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
        self.0.window_handle()
    }
}

impl wgpu::rwh::HasDisplayHandle for AndroidWindow {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        Ok(wgpu::rwh::DisplayHandle::android())
    }
}

impl AndroidEngine {
    fn new(definition: AppDefinition, state_path: PathBuf) -> Self {
        let uses_persistence = definition.uses_persistence();
        let (engine, hydration) = if !uses_persistence {
            (Engine::new(definition), Hydration::Empty)
        } else {
            match std::fs::read(&state_path) {
                Ok(bytes) => Engine::hydrate(definition, &bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    (Engine::new(definition), Hydration::Empty)
                }
                Err(error) => {
                    android_log(
                        ANDROID_LOG_WARN,
                        &format!("could not read persisted state: {error}"),
                    );
                    (Engine::new(definition), Hydration::Empty)
                }
            }
        };
        if hydration == Hydration::Invalid {
            android_log(
                ANDROID_LOG_WARN,
                "persisted state was invalid; using application defaults",
            );
        }
        Self {
            engine,
            uses_persistence,
            state_path,
            surface: None,
            #[cfg(feature = "audio")]
            audio: crate::audio::AudioRuntime::default(),
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
        let window = Arc::new(window);
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::VULKAN;
        descriptor.flags = wgpu::InstanceFlags::empty();
        let instance = wgpu::Instance::new(descriptor);
        let wgpu_surface = match instance.create_surface(AndroidWindow(window.clone())) {
            Ok(surface) => surface,
            Err(error) => {
                android_log(
                    ANDROID_LOG_ERROR,
                    &format!("failed to create wgpu surface: {error}"),
                );
                return;
            }
        };
        let renderer =
            match pollster::block_on(Renderer::new(instance, wgpu_surface, width, height)) {
                Ok(renderer) => renderer,
                Err(error) => {
                    android_log(
                        ANDROID_LOG_ERROR,
                        &format!("failed to initialise renderer: {error:#}"),
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
    }

    fn pointer(&mut self, action: i32, x: f32, y: f32) -> bool {
        let changed = match action {
            0 => {
                self.engine.pointer_down(y);
                false
            }
            1 => self.engine.pointer_up(x, y),
            2 => self.engine.pointer_move(y),
            3 => {
                self.engine.pointer_cancel();
                false
            }
            _ => false,
        };
        changed
    }

    fn scroll_by(&mut self, delta: f32) -> bool {
        self.engine.scroll_by(delta)
    }

    fn back(&mut self) -> bool {
        self.engine.back()
    }

    fn navigate(&mut self, path: &str) -> bool {
        self.engine.navigate(path)
    }

    fn resume(&mut self) -> bool {
        self.engine.resume()
    }

    fn edit_text(&mut self, edit: TextEdit) -> bool {
        self.engine.edit_text(edit)
    }

    fn complete_native(
        &mut self,
        request_id: u64,
        result: Result<StateValue, ResourceError>,
    ) -> bool {
        self.engine.complete_native(request_id, result)
    }

    #[cfg(feature = "audio")]
    fn process_audio(&mut self, samples: &[i16], sample_rate: u32) -> bool {
        self.audio.process(&mut self.engine, samples, sample_rate)
    }

    fn complete_native_json(&mut self, request_id: u64, bytes: &[u8]) -> bool {
        self.engine.complete_native_json(request_id, bytes)
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

    fn render(
        &mut self,
        text_cursor_visible: bool,
    ) -> Option<ink_renderer_wgpu::SystemGlyphRequest> {
        let Some(surface) = &mut self.surface else {
            return None;
        };
        match surface
            .renderer
            .render(self.engine.scene(), text_cursor_visible)
        {
            Ok(RenderOutcome::Presented) => {}
            Ok(RenderOutcome::Skipped) => {
                android_log(ANDROID_LOG_WARN, "surface skipped dirty frame");
            }
            Ok(RenderOutcome::SurfaceLost) => {
                android_log(ANDROID_LOG_ERROR, "Vulkan surface was lost");
                self.surface = None;
            }
            Ok(RenderOutcome::NeedsSystemGlyph(request)) => return Some(request),
            Err(error) => {
                android_log(
                    ANDROID_LOG_ERROR,
                    &format!("failed to render dirty frame: {error:#}"),
                );
            }
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
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    state_path: JString<'_>,
    app: JByteArray<'_>,
) -> jlong {
    PANIC_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|panic| {
            android_log(ANDROID_LOG_ERROR, &format!("native panic: {panic}"));
        }));
    });
    let state_path = env
        .with_env(|env| state_path.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let bytes = env
        .with_env(|env| env.convert_byte_array(&app))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let definition = match AppDefinition::decode(&bytes) {
        Ok(definition) => definition,
        Err(error) => {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("could not load app.ink: {error}"),
            );
            return 0;
        }
    };
    android_log(ANDROID_LOG_INFO, "created Ink engine");
    Box::into_raw(Box::new(Mutex::new(AndroidEngine::new(
        definition,
        PathBuf::from(state_path),
    )))) as jlong
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeUsesPersistence(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|engine| engine.uses_persistence) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativePersist(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    let Some(engine) = engine(handle) else {
        return;
    };
    persist(engine);
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
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCameraPortal<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let value = engine(handle)
        .and_then(|engine| engine.lock().ok())
        .and_then(|engine| engine.engine.scene().camera_portal)
        .map_or_else(String::new, |portal| {
            let kind = match portal.kind {
                CameraPreviewKind::Photo => "photo",
                CameraPreviewKind::Scanner => "scanner",
            };
            let left = portal.rect.x.round() as i32;
            let top = portal.rect.y.round() as i32;
            let right = (portal.rect.x + portal.rect.width).round() as i32;
            let bottom = (portal.rect.y + portal.rect.height).round() as i32;
            format!(
                "{{\"controller\":{},\"kind\":\"{}\",\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
                portal.controller.index(),
                kind,
                left,
                top,
                (right - left).max(1),
                (bottom - top).max(1),
            )
        });
    env.with_env(|env| env.new_string(value))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeSetCameraReview(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    controller: jlong,
    source: JString<'_>,
) -> jboolean {
    let source = env
        .with_env(|env| source.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| {
            engine.engine.set_camera_review(
                ControllerId::new(controller as usize),
                (!source.is_empty()).then_some(source),
            )
        }) as jboolean
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
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativePointer(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    action: jint,
    x: jfloat,
    y: jfloat,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.pointer(action, x, y)) as jboolean
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
    text_cursor_visible: jboolean,
) -> JString<'local> {
    let request = if let Some(engine) = engine(handle)
        && let Ok(mut engine) = engine.lock()
    {
        engine.render(text_cursor_visible)
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
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeNavigate(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    path: JString<'_>,
) -> jboolean {
    let Some(engine) = engine(handle) else {
        return false as jboolean;
    };
    let path = env
        .with_env(|env| path.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine.lock().expect("engine lock poisoned").navigate(&path) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeResume(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.resume()) as jboolean
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
        0 if !value.is_null() => Some(TextEdit::Insert(
            env.with_env(|env| value.try_to_string(env))
                .resolve::<jni::errors::LogErrorAndDefault>(),
        )),
        1 => Some(TextEdit::Backspace),
        2 => Some(TextEdit::Submit),
        3 => Some(TextEdit::Dismiss),
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
        .and_then(|mut engine| engine.engine.take_native_request())
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
                    NativeRequestKind::ResourceRead => 0,
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

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCompleteString(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
    value: JString<'_>,
) -> jboolean {
    let value = env
        .with_env(|env| value.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| {
            engine.complete_native(request_id as u64, Ok(StateValue::String(value)))
        }) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCompleteBytes(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    request_id: jlong,
    value: JByteArray<'_>,
) -> jboolean {
    let bytes = env
        .with_env(|env| env.convert_byte_array(&value))
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| engine.complete_native_json(request_id as u64, &bytes))
        as jboolean
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

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeUpdateController(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    controller: jlong,
    value: JString<'_>,
) -> jboolean {
    let value = env
        .with_env(|env| value.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle)
        .and_then(|engine| engine.lock().ok())
        .is_some_and(|mut engine| {
            engine
                .engine
                .update_controller_json(ControllerId::new(controller as usize), value.as_bytes())
        }) as jboolean
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
        persist(&engine);
        drop(engine);
    }
}

fn persist(engine: &Mutex<AndroidEngine>) {
    let (path, revision, bytes) = {
        let Ok(engine) = engine.lock() else {
            return;
        };
        if !engine.uses_persistence {
            return;
        }
        let snapshot = match engine.engine.persisted_snapshot() {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return,
            Err(error) => {
                android_log(ANDROID_LOG_WARN, &error.to_string());
                return;
            }
        };
        (engine.state_path.clone(), snapshot.0, snapshot.1)
    };
    let temporary = path.with_extension("tmp");
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, &path)
    })();
    match result {
        Ok(()) => {
            if let Ok(mut engine) = engine.lock() {
                engine.engine.persistence_saved(revision);
            }
        }
        Err(error) => android_log(
            ANDROID_LOG_WARN,
            &format!("could not save persisted state: {error}"),
        ),
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
