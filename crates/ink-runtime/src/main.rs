use std::{env, fs};

use anyhow::{Context, Result, bail};
use ink_runtime::{AppRuntime, Event};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("usage: ink-runtime <bundle.js>")?;
    let source = fs::read_to_string(&path).with_context(|| format!("could not read {path}"))?;
    let (_runtime, events) = AppRuntime::spawn(source)?;
    for event in events {
        match event {
            Event::Message(message) => println!("{message}"),
            Event::Error(message) => bail!("{message}"),
            Event::Stopped => break,
            Event::Ready => {}
        }
    }
    Ok(())
}
