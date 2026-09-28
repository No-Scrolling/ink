use std::collections::HashMap;

use ink_audio::{AudioFormat, AudioProcessor, LevelProcessor, PitchProcessor};
use ink_core::{ControllerId, StateValue};

struct Processor {
    enabled: bool,
    inner: Box<dyn AudioProcessor>,
    status_only: bool,
    status: String,
}

#[derive(Default)]
pub struct AudioRuntime {
    processors: HashMap<ControllerId, Processor>,
    format: Option<AudioFormat>,
}

impl AudioRuntime {
    pub fn activate(&mut self, controller: ControllerId, kind: &str, config: &str) -> bool {
        let config = serde_json::from_str::<serde_json::Value>(config).unwrap_or_default();
        let inner: Box<dyn AudioProcessor> = match kind {
            "level" => Box::new(LevelProcessor::new()),
            "pitch" => {
                let reference_hz = config.get("referenceHz")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(440.0);
                Box::new(PitchProcessor::new(reference_hz))
            }
            _ => return false,
        };
        let mut processor = Processor {
            enabled: false,
            inner,
            status_only: config.get("updates").and_then(serde_json::Value::as_str) == Some("status"),
            status: String::new(),
        };
        if let Some(format) = self.format {
            processor.inner.configure(format);
        }
        self.processors.insert(controller, processor);
        true
    }

    pub fn deactivate(&mut self, controller: ControllerId) {
        if let Some(mut processor) = self.processors.remove(&controller) {
            processor.inner.reset();
        }
    }

    pub fn set_enabled(&mut self, controller: ControllerId, enabled: bool) -> bool {
        let Some(processor) = self.processors.get_mut(&controller) else {
            return false;
        };
        processor.enabled = enabled;
        if !enabled { processor.status.clear(); }
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
                    StateValue::Object(fields) => fields.iter().find_map(|(key, value)| match (key.as_str(), value) {
                        ("status", StateValue::String(status)) => Some(status.as_str()), _ => None,
                    }).unwrap_or_default(),
                    _ => "",
                };
                let notify = !processor.status_only || processor.status != status;
                if processor.status != status { processor.status = status.to_owned(); }
                changed |= publish(*controller, value, notify);
            }
        }
        changed
    }
}
