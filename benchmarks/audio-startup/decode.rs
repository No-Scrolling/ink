//! Bounded MP3-prefix decode for timing, not a production playback/resampling implementation.
use symphonia::core::{
    audio::SampleBuffer, formats::FormatOptions, io::MediaSourceStream, probe::Hint,
};

pub fn prefix(path: &str) -> Result<Vec<f32>, String> {
    let source = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let stream = MediaSourceStream::new(Box::new(source), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let options = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let mut probed = symphonia::default::get_probe()
        .format(&hint, stream, &options, &Default::default())
        .map_err(|e| e.to_string())?;
    let track = probed.format.default_track().ok_or("No audio track")?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .map_err(|e| e.to_string())?;
    let mut samples = Vec::new();
    let mut sample_rate = 0;
    for _ in 0..256 {
        let packet = probed.format.next_packet().map_err(|e| e.to_string())?;
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder.decode(&packet).map_err(|e| e.to_string())?;
        if decoded.spec().channels.count() != 2 {
            return Err("Probe expects stereo MP3".into());
        }
        if sample_rate != 0 && sample_rate != decoded.spec().rate {
            return Err("Changing sample rate".into());
        }
        sample_rate = decoded.spec().rate;
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buffer.samples());
        if samples.len() >= sample_rate as usize / 5 * 2 + 2 {
            break;
        }
    }
    if sample_rate == 0 || samples.len() < sample_rate as usize / 5 * 2 + 2 {
        return Err("Insufficient audio".into());
    }
    // Linear conversion is deliberately limited to this short latency fixture.
    // A production music engine needs a quality resampler and full format/seek validation.
    let mut pcm = Vec::with_capacity(9_600 * 2);
    for frame in 0..9_600 {
        let position = frame as f64 * sample_rate as f64 / 48_000.0;
        let index = position as usize;
        let fraction = (position - index as f64) as f32;
        for channel in 0..2 {
            let a = samples[index * 2 + channel];
            let b = samples[(index + 1) * 2 + channel];
            pcm.push((a + (b - a) * fraction).clamp(-1.0, 1.0) * 0.15);
        }
    }
    Ok(pcm)
}
