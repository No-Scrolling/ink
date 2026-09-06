use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, mpsc::Receiver},
    time::Duration,
};

use ink_runtime::{AppRuntime, Event};
use jni::{
    EnvUnowned,
    objects::{JClass, JString},
    sys::{jboolean, jlong},
};
use serde_json::json;

use super::{ANDROID_LOG_ERROR, android_log};

struct Worker {
    runtime: AppRuntime,
    events: Mutex<Receiver<Event>>,
}

#[derive(Default)]
struct Workers {
    next_id: jlong,
    active: HashMap<jlong, Arc<Worker>>,
}

fn workers() -> &'static Mutex<Workers> {
    static WORKERS: OnceLock<Mutex<Workers>> = OnceLock::new();
    WORKERS.get_or_init(Mutex::default)
}

fn worker(handle: jlong) -> Option<Arc<Worker>> {
    workers().lock().ok()?.active.get(&handle).cloned()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_InkWorker_nativeStart(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    source: JString<'_>,
) -> jlong {
    let source = env
        .with_env(|env| source.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let Ok(mut workers) = workers().lock() else {
        return 0;
    };
    if workers.active.len() >= 8 || workers.next_id == jlong::MAX {
        return 0;
    }
    match AppRuntime::spawn(source) {
        Ok((runtime, events)) => {
            workers.next_id += 1;
            let id = workers.next_id;
            workers.active.insert(
                id,
                Arc::new(Worker {
                    runtime,
                    events: Mutex::new(events),
                }),
            );
            id
        }
        Err(error) => {
            android_log(
                ANDROID_LOG_ERROR,
                &format!("Could not start worker: {error:#}"),
            );
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_InkWorker_nativeReceive(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
    message: JString<'_>,
) -> jboolean {
    let message = env
        .with_env(|env| message.try_to_string(env))
        .resolve::<jni::errors::LogErrorAndDefault>();
    worker(handle).is_some_and(|worker| worker.runtime.send(message).is_ok()) as jboolean
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_InkWorker_nativeNext<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JString<'local> {
    let event = worker(handle).and_then(|worker| {
        worker
            .events
            .lock()
            .ok()?
            .recv_timeout(Duration::from_secs(1))
            .ok()
    });
    let message = match event {
        Some(Event::Message(message)) => message,
        Some(Event::Error(message)) => {
            json!({ "type": "worker-error", "message": message }).to_string()
        }
        Some(Event::Stopped) => json!({ "type": "worker-stopped" }).to_string(),
        Some(Event::Ready) | None => String::new(),
    };
    env.with_env(|env| env.new_string(message))
        .resolve::<jni::errors::LogErrorAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_InkWorker_nativeStop(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    let worker = workers()
        .lock()
        .ok()
        .and_then(|mut workers| workers.active.remove(&handle));
    drop(worker);
}
