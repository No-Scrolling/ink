use anyhow::{Result, ensure};

const MAGIC: &[u8] = b"INKICON\x01";
const MAX_BYTES: usize = 16 * 1024 * 1024;

pub struct ReactIcon {
    pub name: String,
    pub filled: bool,
    pub id: u64,
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

impl ReactIcon {
    pub fn encode(icons: &[Self]) -> Result<Vec<u8>> {
        let mut bytes = MAGIC.to_vec();
        for icon in icons {
            ensure!(
                icon.name.len() <= u16::MAX as usize,
                "icon name is too long"
            );
            let pixel_count = pixel_count(icon.width, icon.height)?;
            ensure!(icon.pixels.len() == pixel_count, "invalid icon pixels");
            bytes.extend_from_slice(&(icon.name.len() as u16).to_le_bytes());
            bytes.extend_from_slice(icon.name.as_bytes());
            bytes.push(u8::from(icon.filled));
            bytes.extend_from_slice(&icon.id.to_le_bytes());
            bytes.extend_from_slice(&icon.width.to_le_bytes());
            bytes.extend_from_slice(&icon.height.to_le_bytes());
            bytes.extend_from_slice(&icon.pixels);
            ensure!(bytes.len() <= MAX_BYTES, "icon assets are too large");
        }
        Ok(bytes)
    }

    pub fn decode(mut bytes: &[u8]) -> Result<Vec<Self>> {
        ensure!(bytes.len() <= MAX_BYTES, "icon assets are too large");
        ensure!(
            take(&mut bytes, MAGIC.len())? == MAGIC,
            "unsupported icon asset format; rebuild the app"
        );
        let mut icons = Vec::new();
        while !bytes.is_empty() {
            let name_len = u16::from_le_bytes(take(&mut bytes, 2)?.try_into()?) as usize;
            let name = std::str::from_utf8(take(&mut bytes, name_len)?)?.to_owned();
            let filled = take(&mut bytes, 1)?[0];
            ensure!(filled <= 1, "invalid icon variant");
            let id = u64::from_le_bytes(take(&mut bytes, 8)?.try_into()?);
            let width = u16::from_le_bytes(take(&mut bytes, 2)?.try_into()?);
            let height = u16::from_le_bytes(take(&mut bytes, 2)?.try_into()?);
            let pixels = take(&mut bytes, pixel_count(width, height)?)?.to_vec();
            icons.push(Self {
                name,
                filled: filled == 1,
                id,
                width,
                height,
                pixels,
            });
        }
        Ok(icons)
    }
}

fn pixel_count(width: u16, height: u16) -> Result<usize> {
    ensure!(
        width > 0 && height > 0 && width <= 512 && height <= 512,
        "invalid icon dimensions"
    );
    Ok(usize::from(width) * usize::from(height))
}

fn take<'a>(bytes: &mut &'a [u8], count: usize) -> Result<&'a [u8]> {
    let (value, rest) = bytes
        .split_at_checked(count)
        .ok_or_else(|| anyhow::anyhow!("truncated icon assets"))?;
    *bytes = rest;
    Ok(value)
}
