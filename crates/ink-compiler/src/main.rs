use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use ink_compiler::Project;

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let command = arguments
        .next()
        .and_then(|argument| argument.into_string().ok())
        .context("usage: ink-compiler <compile|icon> <ink.toml>")?;
    let config_path = arguments
        .next()
        .map(PathBuf::from)
        .context("usage: ink-compiler <compile|icon> <ink.toml>")?;
    if arguments.next().is_some() {
        bail!("usage: ink-compiler <compile|icon> <ink.toml>");
    }
    let project = Project::load(config_path)?;

    match command.as_str() {
        "compile" => ink_compiler::compile(&project).map(|_| ()),
        "icon" => ink_compiler::generate_icon(&project),
        _ => bail!("unknown command {command:?}; expected compile or icon"),
    }
}
