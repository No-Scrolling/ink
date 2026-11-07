//! Tiny square previews of media library items.
//!
//! Grid cells draw these in the same frame they appear, while the full thumbnail decodes.
//! Previews live in one append-only pack that is memory-mapped when the media library is enabled.

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::{Arc, Once, OnceLock, RwLock},
};

pub const PREVIEW_SIZE: u32 = 64;
const PIXEL_BYTES: usize = (PREVIEW_SIZE * PREVIEW_SIZE * 4) as usize;
const HEADER_BYTES: usize = 32;
const RECORD_BYTES: usize = HEADER_BYTES + PIXEL_BYTES;
const MAGIC: [u8; 4] = *b"IMP1";
// Rewrite the pack when replaced previews take up most of it.
const COMPACT_RATIO: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MediaKey {
    pub video: bool,
    pub id: u64,
    pub modified: i64,
}

/// Parses `ink-media://image/123?modified=456`.
pub fn media_key(url: &str) -> Option<MediaKey> {
    let rest = url.strip_prefix("ink-media://")?;
    let (kind, rest) = rest.split_once('/')?;
    let video = match kind { "image" => false, "video" => true, _ => return None };
    let (id, query) = rest.split_once('?').unwrap_or((rest, ""));
    let modified = query.split('&').find_map(|pair| pair.strip_prefix("modified="))
        .and_then(|value| value.parse().ok()).unwrap_or(0);
    Some(MediaKey { video, id: id.parse().ok()?, modified })
}

#[derive(Clone)]
enum Location {
    Mapped(usize),
    Memory(Arc<[u8]>),
}

#[derive(Clone)]
struct Entry {
    modified: i64,
    location: Location,
}

struct Pack {
    map: Option<Arc<memmap2::Mmap>>,
    file: Option<File>,
    index: HashMap<(bool, u64), Entry>,
    revision: u64,
}

pub struct MediaPreviews {
    pack: RwLock<Pack>,
}

static PREVIEWS: OnceLock<MediaPreviews> = OnceLock::new();

/// Opens the pack in `directory`. Later calls keep the first pack.
pub fn open(directory: &Path) -> &'static MediaPreviews {
    let previews = PREVIEWS.get_or_init(|| MediaPreviews { pack: RwLock::new(Pack::open(directory)) });
    static COMPACTION: Once = Once::new();
    COMPACTION.call_once(|| {
        let directory = directory.to_owned();
        let _ = std::thread::Builder::new().name("ink-preview-compaction".into())
            .spawn(move || previews.compact(&directory));
    });
    previews
}

pub fn previews() -> Option<&'static MediaPreviews> {
    PREVIEWS.get()
}

impl Pack {
    fn open(directory: &Path) -> Self {
        let empty = Self { map: None, file: None, index: HashMap::new(), revision: 0 };
        if std::fs::create_dir_all(directory).is_err() { return empty; }
        let path = directory.join("previews-64.pack");
        let Ok(file) = OpenOptions::new().read(true).append(true).create(true).open(&path) else { return empty; };
        let Ok(length) = file.metadata().map(|metadata| metadata.len() as usize) else { return empty; };
        // A torn final record from an interrupted write is dropped.
        let whole = length / RECORD_BYTES * RECORD_BYTES;
        if whole != length && file.set_len(whole as u64).is_err() { return empty; }
        let map = if whole == 0 { None } else {
            // The pack is only appended to, so mapped records never change underneath readers.
            match unsafe { memmap2::Mmap::map(&file) } { Ok(map) => Some(Arc::new(map)), Err(_) => return empty }
        };
        let mut index = HashMap::new();
        if let Some(map) = &map {
            for offset in (0..whole).step_by(RECORD_BYTES) {
                let header = &map[offset..offset + HEADER_BYTES];
                if header[..4] != MAGIC { continue; }
                let video = header[4] == 1;
                let id = u64::from_le_bytes(header[8..16].try_into().unwrap());
                let modified = i64::from_le_bytes(header[16..24].try_into().unwrap());
                index.insert((video, id), Entry { modified, location: Location::Mapped(offset + HEADER_BYTES) });
            }
        }
        Self { map, file: Some(file), index, revision: 0 }
    }

    fn pixels<'a>(&'a self, entry: &'a Entry) -> Option<&'a [u8]> {
        match &entry.location {
            Location::Mapped(offset) => self.map.as_ref().map(|map| &map[*offset..*offset + PIXEL_BYTES]),
            Location::Memory(pixels) => Some(pixels),
        }
    }
}

fn record(video: bool, id: u64, modified: i64, pixels: &[u8]) -> Vec<u8> {
    let mut record = Vec::with_capacity(RECORD_BYTES);
    record.extend_from_slice(&MAGIC);
    record.extend_from_slice(&[u8::from(video), 0, 0, 0]);
    record.extend_from_slice(&id.to_le_bytes());
    record.extend_from_slice(&modified.to_le_bytes());
    record.extend_from_slice(&[0; 8]);
    record.extend_from_slice(pixels);
    record
}

impl MediaPreviews {
    fn compact(&self, directory: &Path) {
        let snapshot = {
            let Ok(pack) = self.pack.read() else { return; };
            let Some(file) = &pack.file else { return; };
            let Ok(length) = file.metadata().map(|metadata| metadata.len() as usize) else { return; };
            if pack.index.len() * COMPACT_RATIO >= length / RECORD_BYTES { return; }
            Pack { map: pack.map.clone(), file: None, index: pack.index.clone(), revision: pack.revision }
        };
        let path = directory.join("previews-64.pack");
        let temporary = path.with_extension("tmp");
        let rewrite = || -> std::io::Result<()> {
            let mut file = File::create(&temporary)?;
            for (&(video, id), entry) in &snapshot.index {
                if let Some(pixels) = snapshot.pixels(entry) {
                    file.write_all(&record(video, id, entry.modified, pixels))?;
                }
            }
            file.sync_all()?;
            let file = OpenOptions::new().read(true).append(true).open(&temporary)?;
            let map = if snapshot.index.is_empty() { None } else {
                // This inode is complete and remains immutable after replacement, except for appends.
                Some(Arc::new(unsafe { memmap2::Mmap::map(&file)? }))
            };
            let index = snapshot.index.iter().enumerate().map(|(index, (&key, entry))| {
                (key, Entry { modified: entry.modified, location: Location::Mapped(index * RECORD_BYTES + HEADER_BYTES) })
            }).collect();
            let Ok(mut pack) = self.pack.write() else { return Ok(()); };
            // An insert during the rewrite wins. Leave compaction for a later process launch.
            if pack.revision != snapshot.revision { return Ok(()); }
            std::fs::rename(&temporary, &path)?;
            *pack = Pack { map, file: Some(file), index, revision: snapshot.revision };
            drop(pack);
            // A crash leaves either complete pack; existing readers retain the old mapping.
            File::open(directory)?.sync_all()
        };
        let _ = rewrite();
        let _ = std::fs::remove_file(temporary);
    }

    /// RGBA pixels of a `PREVIEW_SIZE` square, when the stored preview matches `key`.
    pub fn get(&self, key: MediaKey) -> Option<Arc<[u8]>> {
        let pack = self.pack.read().ok()?;
        let entry = pack.index.get(&(key.video, key.id)).filter(|entry| entry.modified == key.modified)?;
        match &entry.location {
            Location::Memory(pixels) => Some(pixels.clone()),
            Location::Mapped(_) => pack.pixels(entry).map(Arc::from),
        }
    }

    pub fn contains(&self, key: MediaKey) -> bool {
        self.pack.read().is_ok_and(|pack| pack.index.get(&(key.video, key.id)).is_some_and(|entry| entry.modified == key.modified))
    }

    pub fn insert(&self, key: MediaKey, pixels: Vec<u8>) {
        if pixels.len() != PIXEL_BYTES { return; }
        let Ok(mut pack) = self.pack.write() else { return; };
        if pack.index.get(&(key.video, key.id)).is_some_and(|entry| entry.modified == key.modified) { return; }
        if let Some(file) = &mut pack.file
            && file.write_all(&record(key.video, key.id, key.modified, &pixels)).is_err()
        {
            pack.file = None;
        }
        pack.index.insert((key.video, key.id), Entry { modified: key.modified, location: Location::Memory(pixels.into()) });
        pack.revision += 1;
    }
}

/// Centre-crops RGBA pixels to a square and averages them down to a `PREVIEW_SIZE` preview.
pub fn square_preview(pixels: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || pixels.len() != width as usize * height as usize * 4 { return None; }
    let side = width.min(height) as usize;
    let (left, top) = ((width as usize - side) / 2, (height as usize - side) / 2);
    let size = PREVIEW_SIZE as usize;
    let bounds: Vec<_> = (0..=size).map(|index| index * side / size).collect();
    let mut output = vec![0; PIXEL_BYTES];
    for row in 0..size {
        let (y0, y1) = (bounds[row], bounds[row + 1].max(bounds[row] + 1).min(side));
        for column in 0..size {
            let (x0, x1) = (bounds[column], bounds[column + 1].max(bounds[column] + 1).min(side));
            let mut sum = [0_u32; 4];
            for y in y0..y1 {
                let start = ((top + y) * width as usize + left + x0) * 4;
                for pixel in pixels[start..start + (x1 - x0) * 4].chunks_exact(4) {
                    for channel in 0..4 { sum[channel] += u32::from(pixel[channel]); }
                }
            }
            let count = ((y1 - y0) * (x1 - x0)) as u32;
            let target = (row * size + column) * 4;
            for channel in 0..4 { output[target + channel] = ((sum[channel] + count / 2) / count) as u8; }
        }
    }
    Some(output)
}
