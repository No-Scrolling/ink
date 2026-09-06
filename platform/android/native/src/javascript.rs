use std::sync::mpsc::Receiver;

use anyhow::{Result, bail};
#[cfg(feature = "audio")]
use ink_core::StateValue;
use ink_core::{Engine, ReactTree, TextEdit};
use ink_runtime::{AppRuntime, Event};
use jni::objects::{JByteArray, JClass, JObject, JString};
use jni::sys::{jboolean, jlong};
use jni::{EnvUnowned, jni_sig, jni_str};
use serde_json::Value;

use super::{ANDROID_LOG_ERROR, ANDROID_LOG_INFO, android_log, engine};

pub(super) struct ScriptRuntime {
    runtime: AppRuntime,
    events: Receiver<Event>,
    tree: ReactTree,
    calls: Vec<Value>,
}

impl ScriptRuntime {
    #[cfg(feature = "audio")]
    pub(super) fn notify_controller(&self, id: u64, value: StateValue) -> Result<()> {
        self.send(
            serde_json::json!({ "type": "controller", "id": id, "value": state_json(value) })
                .to_string(),
        )
    }

    pub(super) fn send(&self, message: String) -> Result<()> {
        self.runtime.send(message)
    }

    pub(super) fn notify_inputs(&mut self, engine: &Engine) -> Result<()> {
        for event in self
            .tree
            .input_events(engine)
            .into_iter()
            .chain(self.tree.viewport_events(engine))
        {
            self.send(event.to_string())?;
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

    fn drain(&mut self, engine: &mut Engine) -> Result<bool> {
        let mut changed = false;
        for _ in 0..256 {
            let Ok(event) = self.events.try_recv() else {
                break;
            };
            match event {
                Event::Ready => android_log(ANDROID_LOG_INFO, "JavaScript app ready"),
                Event::Stopped => {}
                Event::Error(message) => bail!(message),
                Event::Message(message) => {
                    let mut message: Value = serde_json::from_str(&message)?;
                    match message["type"].as_str() {
                        Some("call" | "cancel") => self.calls.push(message),
                        Some("appearance") => {
                            let light = match message["colourScheme"].as_str() {
                                Some("light") => true,
                                Some("dark") => false,
                                _ => bail!("invalid colour scheme"),
                            };
                            changed |= engine.set_colour_scheme(light);
                        }
                        Some("commit") => {
                            self.tree.apply(message["operations"].take(), engine)?;
                            changed = true;
                        }
                        Some("log") => android_log(
                            if message["level"] == "error" {
                                ANDROID_LOG_ERROR
                            } else {
                                ANDROID_LOG_INFO
                            },
                            message["message"].as_str().unwrap_or(""),
                        ),
                        Some("error") => {
                            bail!("{}", message["message"].as_str().unwrap_or("React failed"))
                        }
                        _ => bail!("unknown JavaScript message"),
                    }
                }
            }
        }
        for event in self.tree.viewport_events(engine) {
            self.send(event.to_string())?;
        }
        Ok(changed)
    }
}

#[cfg(feature = "audio")]
fn state_json(value: StateValue) -> Value {
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
                env.new_global_ref(activity)?,
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
    let runtime = AppRuntime::spawn_with_waker(source, move || {
        let result: jni::errors::Result<()> = vm.attach_current_thread(|env| {
            env.call_method(
                &activity,
                jni_str!("onJavaScriptReady"),
                jni_sig!(() -> void),
                &[],
            )?;
            Ok(())
        });
        if let Err(error) = result {
            android_log(ANDROID_LOG_ERROR, &error.to_string());
        }
    });
    match runtime {
        Ok((runtime, events)) => {
            let Some(engine) = engine(handle) else {
                return false as jboolean;
            };
            let Ok(mut engine) = engine.lock() else {
                return false as jboolean;
            };
            engine.javascript_error = None;
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
    let Ok(mut engine) = engine.lock() else {
        return false as jboolean;
    };
    let Some(mut script) = engine.script.take() else {
        return false as jboolean;
    };
    match script.drain(&mut engine.engine) {
        Ok(changed) => {
            engine.script = Some(script);
            changed as jboolean
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
        .and_then(|engine| engine.lock().ok())
        .and_then(|mut engine| {
            engine
                .script
                .as_mut()
                .map(|script| std::mem::take(&mut script.calls))
        })
        .unwrap_or_default();
    env.with_env(|env| env.new_string(Value::Array(calls).to_string()))
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
        && let Ok(engine) = engine.lock()
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
        .and_then(|engine| engine.lock().ok().map(|engine| engine.engine.scene().light))
        .unwrap_or(false) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_MainActivity_nativeTakeJavaScriptError<'local>(
    mut env: EnvUnowned<'local>, _class: JClass<'local>, handle: jlong,
) -> JString<'local> {
    let error = engine(handle).and_then(|engine| engine.lock().ok())
        .and_then(|mut engine| engine.javascript_error.take()).unwrap_or_default();
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
    let Ok(engine) = engine.lock() else { return false as jboolean; };
    engine.script.as_ref().is_some_and(|script| script.runtime.evaluate_development(source).is_ok()) as jboolean
}
