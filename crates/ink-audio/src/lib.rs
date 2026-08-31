use ink_core::StateValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

pub trait AudioProcessor: Send {
    fn configure(&mut self, format: AudioFormat);
    fn process(&mut self, samples: &[i16]) -> Option<StateValue>;
    fn reset(&mut self);
}

pub struct LevelProcessor {
    format: Option<AudioFormat>,
}

impl LevelProcessor {
    pub const fn new() -> Self {
        Self { format: None }
    }
}

impl Default for LevelProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProcessor for LevelProcessor {
    fn configure(&mut self, format: AudioFormat) {
        self.format = Some(format);
    }

    fn process(&mut self, samples: &[i16]) -> Option<StateValue> {
        self.format?;
        if samples.is_empty() {
            return None;
        }
        let scale = f64::from(i16::MAX);
        let peak = samples
            .iter()
            .map(|sample| f64::from(sample.unsigned_abs()) / scale)
            .fold(0.0, f64::max)
            .min(1.0);
        let rms = (samples
            .iter()
            .map(|sample| {
                let value = f64::from(*sample) / scale;
                value * value
            })
            .sum::<f64>()
            / samples.len() as f64)
            .sqrt();
        Some(object([
            (
                "status",
                StateValue::String(if peak >= 0.98 { "clipping" } else { "active" }.to_owned()),
            ),
            ("rms", StateValue::Number(rms)),
            ("peak", StateValue::Number(peak)),
            ("error", error_value()),
        ]))
    }

    fn reset(&mut self) {
        self.format = None;
    }
}

pub struct PitchProcessor {
    format: Option<AudioFormat>,
    reference_hz: f64,
    samples: Vec<f64>,
    difference: Vec<f64>,
}

impl PitchProcessor {
    pub fn new(reference_hz: f64) -> Self {
        Self {
            format: None,
            reference_hz,
            samples: Vec::new(),
            difference: Vec::new(),
        }
    }

    fn listening(&self) -> StateValue {
        pitch_value("listening", 0.0, "", 0, 0.0, 0.0)
    }
}

impl AudioProcessor for PitchProcessor {
    fn configure(&mut self, format: AudioFormat) {
        self.format = Some(format);
    }

    fn process(&mut self, input: &[i16]) -> Option<StateValue> {
        let format = self.format?;
        if input.len() < 256 {
            return None;
        }

        self.samples.clear();
        self.samples.extend(
            input
                .iter()
                .map(|sample| f64::from(*sample) / f64::from(i16::MAX)),
        );
        let mean = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
        for sample in &mut self.samples {
            *sample -= mean;
        }
        let rms = (self
            .samples
            .iter()
            .map(|sample| sample * sample)
            .sum::<f64>()
            / self.samples.len() as f64)
            .sqrt();
        if rms < 0.008 {
            return Some(self.listening());
        }

        let sample_rate = f64::from(format.sample_rate);
        let minimum_lag = (sample_rate / 1_200.0).floor().max(2.0) as usize;
        let maximum_lag = (sample_rate / 50.0)
            .ceil()
            .min((self.samples.len() / 2) as f64) as usize;
        if maximum_lag <= minimum_lag {
            return None;
        }

        self.difference.clear();
        self.difference.resize(maximum_lag + 1, 0.0);
        for lag in 1..=maximum_lag {
            self.difference[lag] = self
                .samples
                .iter()
                .zip(self.samples.iter().skip(lag))
                .map(|(left, right)| {
                    let delta = left - right;
                    delta * delta
                })
                .sum();
        }

        let mut running = 0.0;
        for lag in 1..=maximum_lag {
            running += self.difference[lag];
            self.difference[lag] = if running > 0.0 {
                self.difference[lag] * lag as f64 / running
            } else {
                1.0
            };
        }

        let mut lag = (minimum_lag..=maximum_lag)
            .find(|&candidate| {
                self.difference[candidate] < 0.15
                    && (candidate == maximum_lag
                        || self.difference[candidate] <= self.difference[candidate + 1])
            })
            .or_else(|| {
                (minimum_lag..=maximum_lag).min_by(|left, right| {
                    self.difference[*left].total_cmp(&self.difference[*right])
                })
            })?;
        while lag < maximum_lag && self.difference[lag + 1] < self.difference[lag] {
            lag += 1;
        }
        let confidence = (1.0 - self.difference[lag]).clamp(0.0, 1.0);
        if confidence < 0.65 {
            return Some(self.listening());
        }

        let refined_lag = if lag > minimum_lag && lag < maximum_lag {
            let left = self.difference[lag - 1];
            let centre = self.difference[lag];
            let right = self.difference[lag + 1];
            let denominator = left - 2.0 * centre + right;
            if denominator.abs() > f64::EPSILON {
                lag as f64 + 0.5 * (left - right) / denominator
            } else {
                lag as f64
            }
        } else {
            lag as f64
        };
        let frequency = sample_rate / refined_lag;
        let midi = 69.0 + 12.0 * (frequency / self.reference_hz).log2();
        let nearest = midi.round() as i32;
        let note = NOTES[nearest.rem_euclid(12) as usize];
        let octave = nearest.div_euclid(12) - 1;
        let cents = (midi - f64::from(nearest)) * 100.0;
        Some(pitch_value(
            "active", frequency, note, octave, cents, confidence,
        ))
    }

    fn reset(&mut self) {
        self.format = None;
        self.samples.clear();
        self.difference.clear();
    }
}

const NOTES: [&str; 12] = [
    "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
];

fn pitch_value(
    status: &str,
    frequency_hz: f64,
    note: &str,
    octave: i32,
    cents: f64,
    confidence: f64,
) -> StateValue {
    object([
        ("status", StateValue::String(status.to_owned())),
        ("frequencyHz", StateValue::Number(frequency_hz)),
        ("note", StateValue::String(note.to_owned())),
        ("octave", StateValue::Number(f64::from(octave))),
        ("cents", StateValue::Number(cents)),
        ("confidence", StateValue::Number(confidence)),
        ("error", error_value()),
    ])
}

fn error_value() -> StateValue {
    object([
        ("kind", StateValue::String("unexpected".to_owned())),
        ("message", StateValue::String(String::new())),
        ("retryable", StateValue::Bool(false)),
    ])
}

fn object<const N: usize>(fields: [(&str, StateValue); N]) -> StateValue {
    StateValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}
