//! Isolated Android output-latency probe. Build with the NDK linker; no app engine dependencies.
mod decode;

use std::{
    cell::UnsafeCell,
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicI32, AtomicI64, AtomicPtr, AtomicU64, Ordering},
    thread,
    time::Duration,
};

#[repr(C)]
struct Stream {
    _private: [u8; 0],
}
#[repr(C)]
struct Builder {
    _private: [u8; 0],
}
#[repr(C)]
struct Timespec {
    seconds: i64,
    nanoseconds: i64,
}
#[link(name = "aaudio")]
unsafe extern "C" {
    fn AAudio_createStreamBuilder(builder: *mut *mut Builder) -> i32;
    fn AAudioStreamBuilder_delete(builder: *mut Builder) -> i32;
    fn AAudioStreamBuilder_setFormat(builder: *mut Builder, value: i32);
    fn AAudioStreamBuilder_setSampleRate(builder: *mut Builder, value: i32);
    fn AAudioStreamBuilder_setChannelCount(builder: *mut Builder, value: i32);
    fn AAudioStreamBuilder_setPerformanceMode(builder: *mut Builder, value: i32);
    fn AAudioStreamBuilder_setSharingMode(builder: *mut Builder, value: i32);
    fn AAudioStreamBuilder_setUsage(builder: *mut Builder, value: i32);
    fn AAudioStreamBuilder_setContentType(builder: *mut Builder, value: i32);
    fn AAudioStreamBuilder_setDataCallback(
        builder: *mut Builder,
        callback: unsafe extern "C" fn(*mut Stream, *mut c_void, *mut c_void, i32) -> i32,
        data: *mut c_void,
    );
    fn AAudioStreamBuilder_setErrorCallback(
        builder: *mut Builder,
        callback: unsafe extern "C" fn(*mut Stream, *mut c_void, i32),
        data: *mut c_void,
    );
    fn AAudioStreamBuilder_openStream(builder: *mut Builder, stream: *mut *mut Stream) -> i32;
    fn AAudioStream_close(stream: *mut Stream) -> i32;
    fn AAudioStream_requestStart(stream: *mut Stream) -> i32;
    fn AAudioStream_getSampleRate(stream: *mut Stream) -> i32;
    fn AAudioStream_getChannelCount(stream: *mut Stream) -> i32;
    fn AAudioStream_getFormat(stream: *mut Stream) -> i32;
    fn AAudioStream_getPerformanceMode(stream: *mut Stream) -> i32;
    fn AAudioStream_getSharingMode(stream: *mut Stream) -> i32;
    fn AAudioStream_getFramesPerBurst(stream: *mut Stream) -> i32;
    fn AAudioStream_getBufferSizeInFrames(stream: *mut Stream) -> i32;
    fn AAudioStream_getDeviceId(stream: *mut Stream) -> i32;
    fn AAudioStream_setBufferSizeInFrames(stream: *mut Stream, value: i32) -> i32;
    fn AAudioStream_getFramesWritten(stream: *mut Stream) -> i64;
    fn AAudioStream_getTimestamp(
        stream: *mut Stream,
        clock: i32,
        frame: *mut i64,
        time: *mut i64,
    ) -> i32;
    fn AAudioStream_getXRunCount(stream: *mut Stream) -> i32;
    fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
}
const RATE: i32 = 48_000;
const SHARED: i32 = 1;
const FLOAT: i32 = 2;
const LOW_LATENCY: i32 = 12;
const MONOTONIC: i32 = 1;
fn clock_ns(clock: i32) -> i64 {
    let mut time = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    unsafe {
        clock_gettime(clock, &mut time);
    }
    time.seconds * 1_000_000_000 + time.nanoseconds
}
fn now() -> i64 {
    clock_ns(MONOTONIC)
}
fn check(value: i32, operation: &str) -> Result<i32, String> {
    if value < 0 {
        Err(format!("{operation}: AAudio error {value}"))
    } else {
        Ok(value)
    }
}
struct Callback {
    pending_pcm: AtomicPtr<Vec<f32>>,
    active_pcm: UnsafeCell<*const Vec<f32>>,
    request: AtomicU64,
    acknowledged: AtomicU64,
    frame: AtomicI64,
    callback_ns: AtomicI64,
    error: AtomicI32,
    callbacks: AtomicU64,
    // Only the serial AAudio data callback accesses these two fields.
    cursor: UnsafeCell<usize>,
    sequence: UnsafeCell<u64>,
}
unsafe extern "C" fn render(
    stream: *mut Stream,
    data: *mut c_void,
    output: *mut c_void,
    frames: i32,
) -> i32 {
    let state = unsafe { &*(data as *const Callback) };
    let output = unsafe { std::slice::from_raw_parts_mut(output as *mut f32, frames as usize * 2) };
    let cursor = unsafe { &mut *state.cursor.get() };
    let sequence = unsafe { &mut *state.sequence.get() };
    let request = state.request.load(Ordering::Acquire);
    if request != *sequence {
        *sequence = request;
        *cursor = 0;
        unsafe {
            *state.active_pcm.get() = state.pending_pcm.load(Ordering::Relaxed);
        }
        state.frame.store(
            unsafe { AAudioStream_getFramesWritten(stream) },
            Ordering::Relaxed,
        );
        state.callback_ns.store(now(), Ordering::Relaxed);
        state.acknowledged.store(request, Ordering::Release);
    }
    output.fill(0.0);
    let pcm = unsafe { &**state.active_pcm.get() };
    let count = output.len().min(pcm.len().saturating_sub(*cursor));
    output[..count].copy_from_slice(&pcm[*cursor..*cursor + count]);
    *cursor += count;
    state.callbacks.fetch_add(1, Ordering::Relaxed);
    0 // CONTINUE; the stream stays active after the short clip ends.
}
unsafe extern "C" fn on_error(_: *mut Stream, data: *mut c_void, error: i32) {
    unsafe { &*(data as *const Callback) }
        .error
        .store(error, Ordering::Release);
}
struct Output {
    stream: *mut Stream,
    state: Box<Callback>,
    open_ns: i64,
    // Stable allocations are retained until close; no allocation/free/lock in the callback.
    clips: Vec<Box<Vec<f32>>>,
}
impl Drop for Output {
    fn drop(&mut self) {
        // close joins callbacks before the boxed callback state is freed.
        unsafe {
            AAudioStream_close(self.stream);
        }
    }
}
impl Output {
    fn open(pcm: Vec<f32>, sharing: i32) -> Result<Self, String> {
        let mut clip = Box::new(pcm);
        let clip_ptr = &mut *clip as *mut Vec<f32>;
        let mut state = Box::new(Callback {
            cursor: UnsafeCell::new(clip.len()),
            pending_pcm: AtomicPtr::new(clip_ptr),
            active_pcm: UnsafeCell::new(clip_ptr),
            sequence: UnsafeCell::new(0),
            request: AtomicU64::new(0),
            acknowledged: AtomicU64::new(0),
            frame: AtomicI64::new(-1),
            callback_ns: AtomicI64::new(0),
            error: AtomicI32::new(0),
            callbacks: AtomicU64::new(0),
        });
        let start = now();
        let mut builder = ptr::null_mut();
        unsafe {
            check(AAudio_createStreamBuilder(&mut builder), "builder")?;
        }
        let mut stream = ptr::null_mut();
        let result = unsafe {
            AAudioStreamBuilder_setFormat(builder, FLOAT);
            AAudioStreamBuilder_setSampleRate(builder, RATE);
            AAudioStreamBuilder_setChannelCount(builder, 2);
            AAudioStreamBuilder_setPerformanceMode(builder, LOW_LATENCY);
            AAudioStreamBuilder_setSharingMode(builder, sharing);
            AAudioStreamBuilder_setUsage(builder, 1); // MEDIA
            AAudioStreamBuilder_setContentType(builder, 2); // MUSIC
            let data = (&mut *state as *mut Callback).cast();
            AAudioStreamBuilder_setDataCallback(builder, render, data);
            AAudioStreamBuilder_setErrorCallback(builder, on_error, data);
            let result = AAudioStreamBuilder_openStream(builder, &mut stream);
            AAudioStreamBuilder_delete(builder);
            result
        };
        check(result, "open")?;
        let output = Self {
            stream,
            state,
            open_ns: now() - start,
            clips: vec![clip],
        };
        unsafe {
            if AAudioStream_getSampleRate(stream) != RATE
                || AAudioStream_getChannelCount(stream) != 2
                || AAudioStream_getFormat(stream) != FLOAT
            {
                return Err("Unexpected PCM format".into());
            }
            check(
                AAudioStream_setBufferSizeInFrames(
                    stream,
                    2 * AAudioStream_getFramesPerBurst(stream),
                ),
                "buffer size",
            )?;
        }
        Ok(output)
    }
    fn start(&self) -> Result<(), String> {
        unsafe {
            check(AAudioStream_requestStart(self.stream), "start")?;
        }
        Ok(())
    }
    fn replace_pcm(&mut self, pcm: Vec<f32>) {
        let mut clip = Box::new(pcm);
        let pointer = &mut *clip as *mut Vec<f32>;
        self.clips.push(clip);
        self.state.pending_pcm.store(pointer, Ordering::Relaxed);
    }
    fn trigger(&self, sequence: u64) {
        self.state.request.store(sequence, Ordering::Release);
    }
    fn measure(&self, mode: &str, sequence: u64, start: i64, decode_ns: i64) -> Result<(), String> {
        let mut estimates = Vec::new();
        let mut references = Vec::new();
        let mut previous_position = -1;
        while now() - start < 3_000_000_000 && estimates.len() < 5 {
            check(self.state.error.load(Ordering::Acquire), "callback")?;
            if self.state.acknowledged.load(Ordering::Acquire) == sequence {
                let frame = self.state.frame.load(Ordering::Relaxed);
                let (mut position, mut timestamp) = (0, 0);
                let result = unsafe {
                    AAudioStream_getTimestamp(self.stream, MONOTONIC, &mut position, &mut timestamp)
                };
                // Use advancing endpoint timestamps after the supplied frame, not callback arrival.
                if result == 0 && position >= frame && position > previous_position {
                    let estimated =
                        timestamp + (frame - position) * 1_000_000_000 / i64::from(RATE);
                    estimates.push(estimated);
                    references.push([position, timestamp]);
                    previous_position = position;
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        if estimates.len() != 5 {
            return Err(format!("{mode}: insufficient advancing audio timestamps"));
        }
        estimates.sort_unstable();
        let callback = self.state.callback_ns.load(Ordering::Relaxed);
        if estimates[2] < start || estimates[2] < callback {
            return Err(format!(
                "{mode}: timestamp mapped before the trigger/callback"
            ));
        }
        unsafe {
            println!(
                "{{\"mode\":\"{mode}\",\"sample\":{sequence},\"trigger_ns\":{start},\"target_frame\":{},\"decode_ms\":{:.3},\"callback_ms\":{:.3},\"playout_ms\":{:.3},\"timestamp_spread_ms\":{:.3},\"open_ms\":{:.3},\"xruns\":{},\"burst\":{},\"buffer\":{},\"performance\":{},\"sharing\":{},\"device\":{},\"timestamp_references\":{:?}}}",
                self.state.frame.load(Ordering::Relaxed),
                decode_ns as f64 / 1e6,
                (callback - start) as f64 / 1e6,
                (estimates[2] - start) as f64 / 1e6,
                (estimates[4] - estimates[0]) as f64 / 1e6,
                self.open_ns as f64 / 1e6,
                AAudioStream_getXRunCount(self.stream),
                AAudioStream_getFramesPerBurst(self.stream),
                AAudioStream_getBufferSizeInFrames(self.stream),
                AAudioStream_getPerformanceMode(self.stream),
                AAudioStream_getSharingMode(self.stream),
                AAudioStream_getDeviceId(self.stream),
                references
            );
        }
        Ok(())
    }
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let sharing = if args.get(1).map(String::as_str) == Some("exclusive") {
        0
    } else {
        SHARED
    };
    let quick = args.iter().any(|arg| arg == "--quick");
    let file = args.get(2).filter(|arg| !arg.starts_with("--"));
    let pcm = if let Some(path) = file.filter(|p| p.ends_with(".mp3")) {
        let mut pcm = Vec::new();
        for sample in 1..=if quick { 1 } else { 5 } {
            let start = now();
            pcm = decode::prefix(path)?;
            println!(
                "{{\"mode\":\"decode\",\"sample\":{sample},\"decode_ms\":{:.3},\"pcm_bytes\":{}}}",
                (now() - start) as f64 / 1e6,
                pcm.len() * 4
            );
        }
        pcm
    } else if let Some(path) = file {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        if bytes.is_empty() || bytes.len() % 8 != 0 || bytes.len() > 48_000 * 8 * 2 {
            return Err("Expected <=2 seconds of stereo 48 kHz float PCM".into());
        }
        let samples: Vec<_> = bytes
            .chunks_exact(4)
            .map(|s| f32::from_le_bytes(s.try_into().unwrap()))
            .collect();
        if samples.iter().any(|s| !s.is_finite() || s.abs() > 1.0) {
            return Err("Invalid PCM samples".into());
        }
        samples
    } else {
        (0..RATE / 10)
            .flat_map(|i| {
                let ramp = (i as f32 / 240.0).min(1.0) * ((RATE / 10 - i) as f32 / 240.0).min(1.0);
                let sample =
                    0.015 * ramp * (i as f32 * 440.0 * std::f32::consts::TAU / RATE as f32).sin();
                [sample, sample]
            })
            .collect()
    };
    let modes: &[&str] = if quick { &[] } else { &["cold", "preopened"] };
    for &mode in modes {
        for sample in 1..=5 {
            let prepared = pcm.clone();
            let before_open = now();
            let output = Output::open(prepared, sharing)?;
            if mode == "preopened" {
                thread::sleep(Duration::from_millis(150));
            }
            let start = if mode == "cold" { before_open } else { now() };
            output.trigger(sample);
            output.start()?;
            output.measure(mode, sample, start, 0)?;
            thread::sleep(Duration::from_millis(120));
        }
    }
    let mut output = Output::open(pcm, sharing)?;
    output.start()?;
    thread::sleep(Duration::from_millis(500));
    for sample in 1..=if quick { 3 } else { 15 } {
        let start = now();
        output.trigger(sample);
        output.measure("warm", sample, start, 0)?;
        thread::sleep(Duration::from_millis(120 + sample * 7));
    }
    if let Some(path) = file.filter(|p| p.ends_with(".mp3")) {
        for sample in 16..=if quick { 18 } else { 25 } {
            let start = now();
            let pcm = decode::prefix(path)?;
            let decode_ns = now() - start;
            output.replace_pcm(pcm);
            output.trigger(sample);
            output.measure("warm-file", sample, start, decode_ns)?;
            thread::sleep(Duration::from_millis(150));
        }
    }
    if quick {
        return Ok(());
    }
    let callbacks = output.state.callbacks.load(Ordering::Relaxed);
    let cpu_start = clock_ns(2); // CLOCK_PROCESS_CPUTIME_ID
    let wall_start = now();
    thread::sleep(Duration::from_secs(2));
    println!(
        "{{\"mode\":\"idle\",\"wall_ms\":{:.3},\"process_cpu_ms\":{:.3},\"callbacks\":{},\"xruns\":{}}}",
        (now() - wall_start) as f64 / 1e6,
        (clock_ns(2) - cpu_start) as f64 / 1e6,
        output.state.callbacks.load(Ordering::Relaxed) - callbacks,
        unsafe { AAudioStream_getXRunCount(output.stream) }
    );
    Ok(())
}
