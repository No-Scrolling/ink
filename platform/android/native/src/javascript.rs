use anyhow::{Context, Result, bail};
use std::time::{Duration, Instant};
#[cfg(feature = "audio")]
use ink_core::StateValue;
use ink_core::{Engine, ReactCommit, ReactTree, TextEdit};
use ink_runtime::{AppRuntime, Event};
use jni::objects::{JByteArray, JClass, JObject, JString};
use jni::sys::{jboolean, jlong};
use jni::EnvUnowned;
use serde::Deserialize;
#[cfg(feature = "audio")]
use serde_json::Value;

use super::{ANDROID_LOG_ERROR, ANDROID_LOG_INFO, android_log, engine};

jni::bind_java_type! {
    InkActivity => com.vandam.ink.MainActivity,
    methods {
        fn on_javascript_ready {
            sig = (),
            name = "onJavaScriptReady",
        },
        fn load_web_runtime {
            sig = () -> JString,
            name = "loadWebRuntime",
            non_null = true,
        },
    },
}

pub(super) struct ScriptRuntime {
    runtime: AppRuntime,
    events: ink_runtime::EventReceiver,
    tree: ReactTree,
    calls: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaScriptMessage {
    r#type: String,
    colour_scheme: Option<String>,
    level: Option<String>,
    message: Option<String>,
}

impl ScriptRuntime {
    #[cfg(feature = "memory-diagnostics")]
    pub(super) fn memory_diagnostics(&self) -> Value {
        let mut value = self.tree.memory_diagnostics();
        let (bytes, high_water) = self.runtime.queue_metrics();
        if let Some(fields) = value.as_object_mut() {
            fields.insert("queuePayloadBytes".into(), Value::from(bytes));
            fields.insert("queuePayloadHighWaterBytes".into(), Value::from(high_water));
        }
        value
    }

    #[cfg(feature = "audio")]
    pub(super) fn notify_controller(&self, id: u64, value: Value) -> Result<()> {
        self.send(
            serde_json::json!({ "type": "controller", "id": id, "value": value })
                .to_string(),
        )
    }

    pub(super) fn send(&self, message: String) -> Result<()> {
        self.runtime.send(self.tree.route_native_message(message)?)
    }

    pub(super) fn notify_inputs(&mut self, engine: &mut Engine) -> Result<()> {
        if cfg!(feature = "text-input") {
            for event in self.tree.input_events(engine) {
                self.send(event.to_string())?;
            }
        }
        self.notify_viewports(engine)
    }

    fn notify_viewports(&mut self, engine: &mut Engine) -> Result<()> {
        for event in self.tree.viewport_events(engine)? {
            let id = event["id"].as_u64().expect("viewport event has a list ID");
            self.runtime.send_latest(id, event.to_string())?;
        }
        Ok(())
    }

    pub(super) fn edit_text(&mut self, engine: &mut Engine, edit: TextEdit) -> Result<bool> {
        let submit = if edit == TextEdit::Submit {
            self.tree.submit_event(engine)
        } else {
            None
        };
        let changed = if submit.is_some() {
            false
        } else {
            engine.edit_text(edit)
        };
        self.notify_inputs(engine)?;
        if let Some(event) = submit {
            self.send(event.to_string())?;
        }
        Ok(changed)
    }

    fn apply(&mut self, commit: ReactCommit, engine: &mut Engine) -> Result<()> {
        #[cfg(feature = "bridge-timing")]
        android_log(
            ANDROID_LOG_INFO,
            &format!("ReactApply ns={}", super::benchmark_time_ns()),
        );
        self.tree.apply(commit, engine)?;
        #[cfg(feature = "bridge-timing")]
        android_log(
            ANDROID_LOG_INFO,
            &format!(
                "ReactCommit scene={} ns={}",
                engine.scene().revision,
                super::benchmark_time_ns(),
            ),
        );
        Ok(())
    }

    fn drain(&mut self, engine: &mut Engine) -> Result<bool> {
        let mut changed = false;
        let started = Instant::now();
        for index in 0..64 {
            let Ok(event) = self.events.try_recv() else {
                break;
            };
            match event {
                Event::Ready => android_log(ANDROID_LOG_INFO, "JavaScript app ready"),
                Event::Stopped => {}
                Event::Error(message) => bail!(message),
                Event::Commit(commit) => {
                    #[cfg(feature = "bridge-timing")]
                    {
                        let now = super::benchmark_time_ns();
                        android_log(
                            ANDROID_LOG_INFO,
                            &format!("ReactDecode bytes=0 ns={now} end_ns={now}"),
                        );
                    }
                    self.apply(commit, engine)?;
                    changed = true;
                }
                Event::Message(message) => {
                    let decoded: JavaScriptMessage = serde_json::from_slice(message.as_bytes())?;
                    match decoded.r#type.as_str() {
                        "call" | "cancel" => self.calls.push(message),
                        "appearance" => {
                            let light = match decoded.colour_scheme.as_deref() {
                                Some("light") => true,
                                Some("dark") => false,
                                _ => bail!("invalid colour scheme"),
                            };
                            changed |= engine.set_colour_scheme(light);
                        }
                        "log" => android_log(
                            if decoded.level.as_deref() == Some("error") {
                                ANDROID_LOG_ERROR
                            } else {
                                ANDROID_LOG_INFO
                            },
                            decoded.message.as_deref().unwrap_or(""),
                        ),
                        "error" => {
                            bail!("{}", decoded.message.as_deref().unwrap_or("React failed"))
                        }
                        _ => bail!("unknown JavaScript message"),
                    }
                }
            }
            if index == 63 || started.elapsed() >= Duration::from_millis(4) {
                self.runtime.request_drain();
                break;
            }
        }
        self.notify_viewports(engine)?;
        Ok(changed)
    }
}

#[cfg(feature = "audio")]
pub(super) fn state_json(value: StateValue) -> Value {
    match value {
        StateValue::Null => Value::Null,
        StateValue::Number(value) => Value::from(value),
        StateValue::Bool(value) => Value::Bool(value),
        StateValue::String(value) => Value::String(value),
        StateValue::List(values) => Value::Array(values.into_iter().map(state_json).collect()),
        StateValue::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, state_json(value)))
                .collect(),
        ),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeStartJavaScript(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    source: JString<'_>,
    icons: JByteArray<'_>,
    activity: JObject<'_>,
) -> jboolean {
    let result = env
        .with_env(|env| -> jni::errors::Result<_> {
            Ok(Some((
                source.try_to_string(env)?,
                env.convert_byte_array(&icons)?,
                env.get_java_vm()?,
                env.new_global_ref(env.cast_local::<InkActivity>(activity)?)?,
            )))
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
    let Some((source, icons, vm, activity)) = result else {
        return false as jboolean;
    };
    let tree = match ReactTree::with_icons(&icons) {
        Ok(tree) => tree,
        Err(error) => {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("invalid icon assets: {error:#}"),
            );
            return false as jboolean;
        }
    };
    let loader_vm = vm.clone();
    let activity = std::sync::Arc::new(activity);
    let loader_activity = activity.clone();
    #[cfg(feature = "audio")]
    let (audio_vm, audio_activity) = (loader_vm.clone(), loader_activity.clone());
    let runtime = AppRuntime::spawn_with_web_loader(
        source,
        move || {
            #[cfg(feature = "bridge-timing")]
            android_log(
                ANDROID_LOG_INFO,
                &format!("ReactReady ns={}", super::benchmark_time_ns()),
            );
            if let Err(error) = vm.attach_current_thread(|env| activity.on_javascript_ready(env)) {
                android_log(ANDROID_LOG_ERROR, &error.to_string());
            }
        },
        move || {
            let source = loader_vm.attach_current_thread(|env| -> jni::errors::Result<String> {
                loader_activity.load_web_runtime(env)?.try_to_string(env)
            })?;
            Ok(source)
        },
    );
    match runtime {
        Ok((runtime, events)) => {
            let Some(engine) = engine(handle) else {
                return false as jboolean;
            };
            let mut engine = engine.lock_arc();
            engine.javascript_error = None;
            engine.engine.reset_session();
            engine.sent_camera_portal = None;
            engine.sent_map_portal = None;
            engine.sent_video_portal = None;
            #[cfg(feature = "audio")]
            {
                engine.audio.reset();
                engine.audio.set_waker(move || {
                    let _ = audio_vm.attach_current_thread(|env| audio_activity.on_javascript_ready(env));
                });
            }
            engine.script = Some(ScriptRuntime {
                runtime,
                events,
                tree,
                calls: Vec::new(),
            });
            true as jboolean
        }
        Err(error) => {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("could not start JavaScript: {error}"),
            );
            false as jboolean
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeDrainJavaScript(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    let Some(engine) = engine(handle) else {
        return false as jboolean;
    };
    let mut engine = engine.lock_arc();
    #[cfg(feature = "audio")]
    let audio_changed = engine.publish_audio();
    #[cfg(not(feature = "audio"))]
    let audio_changed = false;
    let Some(mut script) = engine.script.take() else {
        return false as jboolean;
    };
    let _affinity = ink_runtime::cpu_affinity::prefer_performance();
    match script.drain(&mut engine.engine) {
        Ok(changed) => {
            engine.script = Some(script);
            (changed || audio_changed) as jboolean
        }
        Err(error) => {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("JavaScript app failed: {error:#}"),
            );
            engine.javascript_error = Some(format!("{error:#}"));
            false as jboolean
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTakeJavaScriptCalls<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let calls = engine(handle)
        .map(|engine| engine.lock_arc())
        .and_then(|mut engine| {
            engine
                .script
                .as_mut()
                .map(|script| std::mem::take(&mut script.calls))
        })
        .unwrap_or_default();
    if calls.is_empty() {
        return JString::default();
    }
    env.with_env(|env| env.new_string(format!("[{}]", calls.join(","))))
        .resolve::<jni::errors::LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeJavaScriptReceive(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    message: JString<'_>,
) {
    let message = env
        .with_env(|env| message.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    if let Some(engine) = engine(handle)
        && let engine = engine.lock_arc()
        && let Some(script) = &engine.script
        && let Err(error) = script.send(message)
    {
        android_log(ANDROID_LOG_ERROR, &error.to_string());
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeIsLightAppearance(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) -> jboolean {
    engine(handle)
        .map(|engine| engine.lock_arc().engine.scene().light)
        .unwrap_or(false) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTakeJavaScriptError<'local>(
    mut env: EnvUnowned<'local>, _class: JClass<'local>, handle: jlong,
) -> JString<'local> {
    let Some(error) = engine(handle).map(|engine| engine.lock_arc())
        .and_then(|mut engine| engine.javascript_error.take())
    else {
        return JString::default();
    };
    env.with_env(|env| env.new_string(error)).resolve::<jni::errors::LogErrorAndDefault>()
}

#[cfg(debug_assertions)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRefreshJavaScript(
    mut env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong, source: JString<'_>,
) -> jboolean {
    let source = env.with_env(|env| source.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let Some(engine) = engine(handle) else { return false as jboolean; };
    let engine = engine.lock_arc();
    engine.script.as_ref().is_some_and(|script| script.runtime.evaluate_development(source).is_ok()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTakeJavaScriptBytes<'local>(
    mut env: EnvUnowned<'local>, _class: JClass<'local>, handle: jlong, id: jlong,
) -> JByteArray<'local> {
    let bytes = engine(handle).map(|engine| engine.lock_arc()).and_then(|engine| engine.script.as_ref().and_then(|script| script.runtime.take_bytes(id as u64)));
    match bytes {
        Some(bytes) => env.with_env(|env| env.byte_array_from_slice(&bytes)).resolve::<jni::errors::LogErrorAndDefault>(),
        None => JByteArray::default(),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeJavaScriptReceiveBytes(
    mut env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong, message: JString<'_>, bytes: JByteArray<'_>,
) -> jboolean {
    let (message, bytes) = env.with_env(|env| -> jni::errors::Result<_> {
        Ok((message.try_to_string(env)?, env.convert_byte_array(&bytes)?))
    }).resolve::<jni::errors::LogErrorAndDefault>();
    (engine(handle).map(|engine| engine.lock_arc()).is_some_and(|engine| engine.script.as_ref().is_some_and(|script| script.runtime.send_bytes(message, bytes).is_ok()))) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeReplaceCollection<'local>(
    mut env: EnvUnowned<'local>, _class: JClass<'local>, handle: jlong, view: jlong, source: jlong, revision: jlong, rows: JString<'local>,
) -> JString<'local> {
    let rows = env.with_env(|env| rows.try_to_string(env)).resolve::<jni::errors::LogErrorAndDefault>();
    let result = (|| -> Result<()> {
        anyhow::ensure!(view > 0 && source > 0 && revision > 0, "Invalid native collection target");
        let mut engine = engine(handle).context("App is no longer mounted")?.lock_arc();
        let rows = serde_json::from_str(&rows)?;
        let mut script = engine.script.take().context("App is no longer mounted")?;
        let result = script.tree.replace_view_collection(view as usize, source as usize, revision as u64, rows, &mut engine.engine);
        engine.script = Some(script);
        anyhow::ensure!(result?, "Native view is no longer mounted");
        Ok(())
    })();
    let message = result.err().map(|error| format!("{error:#}")).unwrap_or_default();
    env.with_env(|env| env.new_string(message)).resolve::<jni::errors::LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeCaptureState(
    mut env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong, controller: jlong, value: JString<'_>,
) -> jboolean {
    let value = env.with_env(|env| value.try_to_string(env)).resolve::<jni::errors::LogErrorAndDefault>();
    engine(handle).map(|engine| engine.lock_arc()).is_some_and(|mut engine| {
        if value.is_empty() { engine.engine.remove_capture_state(controller as u64); return false; }
        serde_json::from_str(&value).ok().is_some_and(|value| engine.engine.update_capture_state(controller as u64, &value))
    }) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativePlaybackClock(
    _env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong, controller: jlong,
    position: f32, duration: f32, playing: jboolean, speed: f32,
) -> jboolean {
    engine(handle).map(|engine| engine.lock_arc()).is_some_and(|mut engine|
        engine.engine.update_playback_clock(controller as u64, position, duration, playing, speed)) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeRemovePlaybackClock(
    _env: EnvUnowned<'_>, _class: JClass<'_>, handle: jlong, controller: jlong,
) {
    if let Some(mut engine) = engine(handle).map(|engine| engine.lock_arc()) {
        engine.engine.remove_playback_clock(controller as u64);
    }
}
