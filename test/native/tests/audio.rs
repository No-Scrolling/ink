use ink_audio::{AudioFormat, AudioProcessor, LevelProcessor, PitchProcessor};
use ink_core::StateValue;

fn field<'a>(state: &'a StateValue, name: &str) -> &'a StateValue {
    let StateValue::Object(fields) = state else {
        panic!("expected an audio state object, got {state:?}");
    };
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("audio state is missing {name}"))
}

fn number(state: &StateValue, name: &str) -> f64 {
    let StateValue::Number(value) = field(state, name) else {
        panic!("{name} must be a number");
    };
    *value
}

fn text(state: &StateValue, name: &str) -> String {
    let StateValue::String(value) = field(state, name) else {
        panic!("{name} must be text");
    };
    value.clone()
}

fn mono(sample_rate: u32) -> AudioFormat {
    AudioFormat {
        sample_rate,
        channels: 1,
    }
}

fn tone(frequency: f64, sample_rate: u32, amplitude: f64, offset: f64) -> Vec<i16> {
    (0..4096)
        .map(|index| {
            let phase =
                std::f64::consts::TAU * frequency * f64::from(index) / f64::from(sample_rate);
            (amplitude * phase.sin() + offset).round() as i16
        })
        .collect()
}

#[test]
fn processors_require_configuration_and_reset_discards_it() {
    let samples = tone(440.0, 48_000, 8_000.0, 0.0);
    let mut level = LevelProcessor::new();
    let mut pitch = PitchProcessor::new(440.0);
    for processor in [&mut level as &mut dyn AudioProcessor, &mut pitch] {
        assert!(processor.process(&samples).is_none());
        processor.configure(mono(48_000));
        assert!(processor.process(&samples).is_some());
        assert!(processor.process(&[]).is_none());
        processor.reset();
        assert!(processor.process(&samples).is_none());
        processor.configure(mono(48_000));
        assert!(processor.process(&samples).is_some());
    }
}

#[test]
fn silence_has_zero_signal_level() {
    let mut processor = LevelProcessor::new();
    processor.configure(mono(48_000));
    let state = processor.process(&[0; 1024]).unwrap();
    assert_eq!(number(&state, "peak"), 0.0);
    assert_eq!(number(&state, "rms"), 0.0);
    assert_eq!(text(&state, "status"), "active");
}

#[test]
fn alternating_full_scale_and_silence_has_known_energy() {
    let mut processor = LevelProcessor::new();
    processor.configure(mono(48_000));
    let state = processor.process(&[i16::MAX, 0, -i16::MAX, 0]).unwrap();
    assert_eq!(number(&state, "peak"), 1.0);
    assert!((number(&state, "rms") - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    assert_eq!(text(&state, "status"), "clipping");
}

#[test]
fn negative_full_scale_is_reported_without_integer_overflow() {
    let mut processor = LevelProcessor::new();
    processor.configure(mono(48_000));
    let state = processor.process(&[i16::MIN; 256]).unwrap();
    assert_eq!(number(&state, "peak"), 1.0);
    assert_eq!(text(&state, "status"), "clipping");
    let rms = number(&state, "rms");
    assert!(
        rms.is_finite() && (rms - 1.0).abs() < 0.0001,
        "full-scale RMS: {rms}"
    );
}

#[test]
fn known_tones_produce_their_musical_notes_at_different_sample_rates() {
    for sample_rate in [16_000, 44_100, 48_000] {
        for (frequency, note, octave) in
            [(440.0, "A", 4.0), (880.0, "A", 5.0), (261.625565, "C", 4.0)]
        {
            let mut processor = PitchProcessor::new(440.0);
            processor.configure(mono(sample_rate));
            let state = processor
                .process(&tone(frequency, sample_rate, 8_000.0, 0.0))
                .unwrap();
            assert_eq!(
                text(&state, "status"),
                "active",
                "{frequency} Hz at {sample_rate} Hz"
            );
            assert_eq!(text(&state, "note"), note);
            assert_eq!(number(&state, "octave"), octave);
            let measured = number(&state, "frequencyHz");
            assert!(
                (measured - frequency).abs() / frequency < 0.003,
                "expected {frequency} Hz at {sample_rate} Hz, got {measured}"
            );
            assert!(number(&state, "cents").abs() < 5.0, "{state:?}");
            assert!((0.65..=1.0).contains(&number(&state, "confidence")));
        }
    }
}

#[test]
fn a_nonstandard_tuning_reference_changes_note_interpretation() {
    let mut processor = PitchProcessor::new(432.0);
    processor.configure(mono(48_000));
    let state = processor
        .process(&tone(432.0, 48_000, 8_000.0, 0.0))
        .unwrap();
    assert_eq!(text(&state, "note"), "A");
    assert_eq!(number(&state, "octave"), 4.0);
    assert!(number(&state, "cents").abs() < 5.0);
    assert!((number(&state, "frequencyHz") - 432.0).abs() < 1.0);
}

#[test]
fn silence_and_constant_dc_do_not_report_a_note() {
    let mut processor = PitchProcessor::new(440.0);
    processor.configure(mono(48_000));
    for samples in [vec![0; 4096], vec![1200; 4096]] {
        let state = processor.process(&samples).unwrap();
        assert_eq!(text(&state, "status"), "listening");
        assert_eq!(text(&state, "note"), "");
        assert_eq!(number(&state, "frequencyHz"), 0.0);
        assert_eq!(number(&state, "confidence"), 0.0);
    }
}

#[test]
fn sample_rate_changes_and_signal_polarity_preserve_the_detected_note() {
    let mut processor = PitchProcessor::new(440.0);
    for (sample_rate, amplitude, offset) in [(44_100, 5_000.0, 0.0), (48_000, -12_000.0, 1800.0)] {
        processor.configure(mono(sample_rate));
        let state = processor
            .process(&tone(440.0, sample_rate, amplitude, offset))
            .unwrap();
        assert_eq!(text(&state, "note"), "A");
        assert_eq!(number(&state, "octave"), 4.0);
        assert!((number(&state, "frequencyHz") - 440.0).abs() < 1.0);
        assert!(number(&state, "cents").abs() < 5.0);
    }
}
