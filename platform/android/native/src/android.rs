use std::ffi::{CString, c_char, c_int};
use std::io::Write;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use ink_core::{Engine, Hydration, PUBLIC_SANS, TextEdit, TextInputAction};
use ink_renderer_wgpu::{RenderOutcome, Renderer};
use jni::EnvUnowned;
use jni::objects::{JByteBuffer, JClass, JObject, JString};
use jni::sys::{jboolean, jfloat, jint, jlong};
use ndk::native_window::NativeWindow;

use crate::generated_app;

const ANDROID_LOG_INFO: c_int = 4;
const ANDROID_LOG_WARN: c_int = 5;
const ANDROID_LOG_ERROR: c_int = 6;
const LOG_TAG: &[u8] = b"Ink\0";

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

struct AndroidEngine {
    engine: Engine,
    state_path: PathBuf,
    surface: Option<AttachedSurface>,
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
    fn new(state_path: PathBuf) -> Self {
        let (engine, hydration) = if !generated_app::USES_PERSISTENCE {
            (Engine::new(generated_app::app()), Hydration::Empty)
        } else {
            match std::fs::read(&state_path) {
                Ok(bytes) => Engine::hydrate(generated_app::app(), &bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    (Engine::new(generated_app::app()), Hydration::Empty)
                }
                Err(error) => {
                    android_log(
                        ANDROID_LOG_WARN,
                        &format!("could not read persisted state: {error}"),
                    );
                    (Engine::new(generated_app::app()), Hydration::Empty)
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
            state_path,
            surface: None,
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
        self.render();
    }

    fn resize(&mut self, width: u32, height: u32) {
        if !self.engine.set_viewport(width, height) {
            return;
        }
        if let Some(surface) = &mut self.surface {
            surface.renderer.resize(width, height);
        }
        self.render();
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
        if changed {
            self.render();
        }
        changed
    }

    fn scroll_by(&mut self, delta: f32) -> bool {
        if !self.engine.scroll_by(delta) {
            return false;
        }
        self.render();
        true
    }

    fn back(&mut self) -> bool {
        if !self.engine.back() {
            return false;
        }
        self.render();
        true
    }

    fn edit_text(&mut self, edit: TextEdit) -> bool {
        if !self.engine.edit_text(edit) {
            return false;
        }
        self.render();
        true
    }

    fn render(&mut self) {
        let Some(surface) = &mut self.surface else {
            return;
        };
        match surface.renderer.render(self.engine.scene()) {
            Ok(RenderOutcome::Presented) => {
                android_log(ANDROID_LOG_INFO, "presented dirty frame");
            }
            Ok(RenderOutcome::Skipped) => {
                android_log(ANDROID_LOG_WARN, "surface skipped dirty frame");
            }
            Ok(RenderOutcome::SurfaceLost) => {
                android_log(ANDROID_LOG_ERROR, "Vulkan surface was lost");
                self.surface = None;
            }
            Err(error) => {
                android_log(
                    ANDROID_LOG_ERROR,
                    &format!("failed to render dirty frame: {error:#}"),
                );
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCreate(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    state_path: JString<'_>,
) -> jlong {
    let state_path = env
        .with_env(|env| state_path.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    android_log(ANDROID_LOG_INFO, "created Ink engine");
    Box::into_raw(Box::new(Mutex::new(AndroidEngine::new(PathBuf::from(
        state_path,
    ))))) as jlong
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeUsesPersistence(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
) -> jboolean {
    generated_app::USES_PERSISTENCE as jboolean
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
    if !generated_app::USES_PERSISTENCE {
        return;
    }
    let (path, revision, bytes) = {
        let Ok(engine) = engine.lock() else {
            return;
        };
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

fn engine(handle: jlong) -> Option<&'static Mutex<AndroidEngine>> {
    let pointer = NonNull::new(handle as *mut Mutex<AndroidEngine>)?;
    Some(unsafe { pointer.as_ref() })
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
