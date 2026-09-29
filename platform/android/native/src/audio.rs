use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex},
};

use ink_audio::{AudioFormat, AudioProcessor, LevelProcessor, PitchProcessor};
use ink_core::{ControllerId, StateValue};

struct Processor {
    enabled: bool,
    inner: Box<dyn AudioProcessor>,
    status_only: bool,
    status: String,
}

#[derive(Default)]
struct Processors {
    processors: HashMap<ControllerId, Processor>,
    format: Option<AudioFormat>,
}

impl Processors {
    pub fn activate(&mut self, controller: ControllerId, kind: &str, config: &str) -> bool {
        let config = serde_json::from_str::<serde_json::Value>(config).unwrap_or_default();
        let inner: Box<dyn AudioProcessor> = match kind {
            "level" => Box::new(LevelProcessor::new()),
            "pitch" => {
                let reference_hz = config
                    .get("referenceHz")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(440.0);
                Box::new(PitchProcessor::new(reference_hz))
            }
            _ => return false,
        };
        let mut processor = Processor {
            enabled: false,
            inner,
            status_only: config.get("updates").and_then(serde_json::Value::as_str)
                == Some("status"),
            status: String::new(),
        };
        if let Some(format) = self.format {
            processor.inner.configure(format);
        }
        self.processors.insert(controller, processor);
        true
    }

    pub fn set_enabled(&mut self, controller: ControllerId, enabled: bool) -> bool {
        let Some(processor) = self.processors.get_mut(&controller) else {
            return false;
        };
        processor.enabled = enabled;
        if !enabled {
            processor.status.clear();
        }
        true
    }

    pub fn process(
        &mut self,
        samples: &[i16],
        sample_rate: u32,
        mut publish: impl FnMut(ControllerId, StateValue, bool) -> bool,
    ) -> bool {
        let format = AudioFormat {
            sample_rate,
            channels: 1,
        };
        if self.format != Some(format) {
            self.format = Some(format);
            for processor in self.processors.values_mut() {
                processor.inner.configure(format);
            }
        }
        let mut changed = false;
        for (controller, processor) in &mut self.processors {
            if processor.enabled
                && let Some(value) = processor.inner.process(samples)
            {
                let status = match &value {
                    StateValue::Object(fields) => fields
                        .iter()
                        .find_map(|(key, value)| match (key.as_str(), value) {
                            ("status", StateValue::String(status)) => Some(status.as_str()),
                            _ => None,
                        })
                        .unwrap_or_default(),
                    _ => "",
                };
                let notify = !processor.status_only || processor.status != status;
                if processor.status != status {
                    processor.status = status.to_owned();
                }
                changed |= publish(*controller, value, notify);
            }
        }
        changed
    }
}

#[derive(Clone)]
struct Configuration {
    kind: String,
    config: String,
    enabled: bool,
    generation: u64,
}
#[derive(Default)]
struct Pending {
    controllers: HashMap<ControllerId, Configuration>,
    samples: Option<(Vec<i16>, u32)>,
    updates: HashMap<ControllerId, (StateValue, bool)>,
    generation: u64,
    dirty: bool,
    stopped: bool,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

pub struct AudioRuntime {
    shared: Arc<(Mutex<Pending>, Condvar)>,
}
impl Default for AudioRuntime {
    fn default() -> Self {
        let shared = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("ink-dsp".into())
            .spawn(move || {
                let mut processors = Processors::default();
                let mut configured = HashMap::new();
                loop {
                    let (controllers, samples) = {
                        let (lock, ready) = &*worker;
                        let mut pending = lock.lock().unwrap();
                        while !pending.stopped && !pending.dirty && pending.samples.is_none() {
                            pending = ready.wait(pending).unwrap();
                        }
                        if pending.stopped {
                            break;
                        }
                        pending.dirty = false;
                        (pending.controllers.clone(), pending.samples.take())
                    };
                    processors
                        .processors
                        .retain(|id, _| controllers.contains_key(id));
                    configured.retain(|id, _| controllers.contains_key(id));
                    for (id, config) in &controllers {
                        if configured.get(id) != Some(&config.generation) {
                            processors.activate(*id, &config.kind, &config.config);
                            processors.set_enabled(*id, config.enabled);
                            configured.insert(*id, config.generation);
                        }
                    }
                    let Some((samples, rate)) = samples else {
                        continue;
                    };
                    let mut updates = Vec::new();
                    processors.process(&samples, rate, |id, value, notify| {
                        updates.push((id, value, notify));
                        false
                    });
                    let wake = {
                        let mut pending = worker.0.lock().unwrap();
                        if pending.stopped {
                            break;
                        }
                        for (id, value, notify) in updates {
                            if pending.controllers.get(&id).is_some_and(|config| {
                                config.generation == controllers[&id].generation && config.enabled
                            }) {
                                let notify = notify
                                    || pending.updates.get(&id).is_some_and(|(_, notify)| *notify);
                                pending.updates.insert(id, (value, notify));
                            }
                        }
                        if pending.updates.is_empty() {
                            None
                        } else {
                            pending.wake.clone()
                        }
                    };
                    if let Some(wake) = wake {
                        wake();
                    }
                }
            })
            .expect("could not start the audio processor");
        Self { shared }
    }
}
impl AudioRuntime {
    pub fn set_waker(&self, wake: impl Fn() + Send + Sync + 'static) {
        self.shared.0.lock().unwrap().wake = Some(Arc::new(wake));
    }
    pub fn activate(&self, id: ControllerId, kind: &str, config: &str) -> bool {
        if !matches!(kind, "level" | "pitch") {
            return false;
        }
        let mut pending = self.shared.0.lock().unwrap();
        if pending.controllers.len() >= 64 && !pending.controllers.contains_key(&id) {
            return false;
        }
        pending.generation += 1;
        let generation = pending.generation;
        pending.controllers.insert(
            id,
            Configuration {
                kind: kind.into(),
                config: config.into(),
                enabled: false,
                generation,
            },
        );
        pending.updates.remove(&id);
        pending.dirty = true;
        self.shared.1.notify_one();
        true
    }
    pub fn deactivate(&self, id: ControllerId) {
        let mut pending = self.shared.0.lock().unwrap();
        pending.controllers.remove(&id);
        pending.updates.remove(&id);
        pending.dirty = true;
        self.shared.1.notify_one();
    }
    pub fn set_enabled(&self, id: ControllerId, enabled: bool) -> bool {
        let mut pending = self.shared.0.lock().unwrap();
        pending.generation += 1;
        let generation = pending.generation;
        let Some(config) = pending.controllers.get_mut(&id) else {
            return false;
        };
        config.enabled = enabled;
        config.generation = generation;
        pending.updates.remove(&id);
        pending.dirty = true;
        self.shared.1.notify_one();
        true
    }
    pub fn submit(&self, samples: Vec<i16>, rate: u32) {
        let mut pending = self.shared.0.lock().unwrap();
        pending.samples = Some((samples, rate));
        self.shared.1.notify_one();
    }
    pub fn take_updates(&self) -> Vec<(ControllerId, StateValue, bool)> {
        self.shared
            .0
            .lock()
            .unwrap()
            .updates
            .drain()
            .map(|(id, (value, notify))| (id, value, notify))
            .collect()
    }
    pub fn reset(&self) {
        let mut pending = self.shared.0.lock().unwrap();
        pending.controllers.clear();
        pending.samples = None;
        pending.updates.clear();
        pending.dirty = true;
        self.shared.1.notify_one();
    }
}
impl Drop for AudioRuntime {
    fn drop(&mut self) {
        let mut pending = self.shared.0.lock().unwrap();
        pending.stopped = true;
        pending.wake = None;
        pending.samples = None;
        self.shared.1.notify_one();
    }
}
