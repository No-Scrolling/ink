use std::collections::HashMap;

use ink_audio::{AudioFormat, AudioProcessor, LevelProcessor, PitchProcessor};
use ink_core::{ControllerId, Engine};

struct Processor {
    enabled: bool,
    inner: Box<dyn AudioProcessor>,
}

#[derive(Default)]
pub struct AudioRuntime {
    processors: HashMap<ControllerId, Processor>,
    format: Option<AudioFormat>,
}

impl AudioRuntime {
    pub fn activate(&mut self, controller: ControllerId, kind: &str, config: &str) -> bool {
        let inner: Box<dyn AudioProcessor> = match kind {
            "level" => Box::new(LevelProcessor::new()),
            "pitch" => {
                let reference_hz = serde_json::from_str::<serde_json::Value>(config)
                    .ok()
                    .and_then(|value| value.get("referenceHz")?.as_f64())
                    .unwrap_or(440.0);
                Box::new(PitchProcessor::new(reference_hz))
            }
            _ => return false,
        };
        let mut processor = Processor {
            enabled: false,
            inner,
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
        true
    }

    pub fn process(&mut self, engine: &mut Engine, samples: &[i16], sample_rate: u32) -> bool {
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
                changed |= engine.update_controller(*controller, value);
            }
        }
        changed
    }
}
