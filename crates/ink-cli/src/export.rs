use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::Args;
use ink_compiler::Project;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::output;

mod headless;

/// Frame size in pixels. Defaults to the Light Phone III screen.
#[derive(Clone, Copy, Debug)]
pub struct FrameSize {
    width: u32,
    height: u32,
}

impl std::str::FromStr for FrameSize {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let error = || format!("expected WIDTHxHEIGHT in pixels, such as 1080x1200; got {value:?}");
        let (width, height) = value.split_once(['x', '×']).ok_or_else(error)?;
        let parse = |text: &str| text.trim().parse::<u32>().ok().filter(|size| (1..=4096).contains(size));
        Ok(Self { width: parse(width).ok_or_else(error)?, height: parse(height).ok_or_else(error)? })
    }
}

/// Export arguments for other commands, parsed exactly as `ink export` parses them.
pub(crate) fn arguments<I: AsRef<std::ffi::OsStr>>(arguments: impl IntoIterator<Item = I>) -> Result<ExportArgs> {
    #[derive(clap::Parser)]
    struct Export {
        #[command(flatten)]
        arguments: ExportArgs,
    }
    let arguments = std::iter::once(std::ffi::OsString::from("export"))
        .chain(arguments.into_iter().map(|argument| argument.as_ref().to_owned()));
    Ok(<Export as clap::Parser>::try_parse_from(arguments)?.arguments)
}

struct Capture {
    png: Vec<u8>,
    pixels: Vec<u8>,
    metadata: Map<String, Value>,
}

#[derive(Args)]
#[command(
    after_help = "Examples:\n  ink export\n  ink export /settings\n  ink export --entry design/inbox.tsx --props design/inbox.json"
)]
pub struct ExportArgs {
    /// App route, including concrete dynamic segments (defaults to /)
    #[arg(conflicts_with = "entry")]
    route: Option<String>,
    /// Default-exported TSX component; bypasses the app's routes and layouts
    #[arg(long, value_name = "FILE")]
    entry: Option<PathBuf>,
    /// JSON object passed as component props, or route parameters
    #[arg(long, value_name = "FILE")]
    props: Option<PathBuf>,
    /// Module with a default async function, awaited before mounting
    #[arg(long, value_name = "FILE")]
    setup: Option<PathBuf>,
    /// Frame name (defaults to the app and route, or component filename)
    #[arg(long)]
    name: Option<String>,
    /// Destination directory, relative to the app (defaults to design/exports/<frame-name>)
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,
    /// Fixed native calls, controller states, images and emoji glyphs
    #[arg(long, value_name = "FILE")]
    fixtures: Option<PathBuf>,
    /// Wait for this console message before capturing data-dependent content
    #[arg(long, value_name = "MESSAGE")]
    ready: Option<String>,
    /// Minimum settling time after mount/readiness, in milliseconds
    #[arg(long, default_value = "500", value_parser = clap::value_parser!(u64).range(0..=60000))]
    wait: u64,
    /// Theme module to preview instead of the app's theme.ts
    #[arg(long, value_name = "FILE")]
    theme: Option<PathBuf>,
    /// Render in light mode
    #[arg(long)]
    light: bool,
    /// Simulate a display cutout, as X,Y,WIDTH,HEIGHT in pixels; repeat for several
    #[arg(long, value_name = "X,Y,WIDTH,HEIGHT", value_parser = parse_cutout)]
    cutout: Vec<ink_core::Rect>,
    /// Frame size in pixels, as WIDTHxHEIGHT
    #[arg(long, default_value = "1080x1240", value_name = "WIDTHxHEIGHT")]
    size: FrameSize,
    /// Maximum wait for readiness and stable pixels, in seconds
    #[arg(long, default_value = "30", value_parser = clap::value_parser!(u64).range(1..=300))]
    timeout: u64,
}

pub fn run(app: &Project, args: ExportArgs) -> Result<()> {
    let project = match &args.theme {
        Some(theme) => app.with_theme(Some(resolve_file(app, theme)?)).for_preview(),
        None => app.for_preview(),
    };
    fs::create_dir_all(project.work_path())?;
    let project_lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(project.work_path().join("export.lock"))?;
    project_lock
        .try_lock()
        .context("another Ink export is compiling this project")?;
    let entry = args
        .entry
        .as_deref()
        .map(|path| resolve_file(app, path))
        .transpose()?;
    let setup = args
        .setup
        .as_deref()
        .map(|path| resolve_file(app, path))
        .transpose()?;
    let route = args.route.as_deref().unwrap_or("/");
    ensure!(
        route.starts_with('/'),
        "route must start with /; use --entry for a TSX file"
    );
    let props = match args.props.as_deref() {
        Some(path) => serde_json::from_slice::<Value>(&fs::read(resolve_file(app, path)?)?)?,
        None => json!({}),
    };
    ensure!(props.is_object(), "--props must contain a JSON object");
    let name = args.name.clone().unwrap_or_else(|| match &entry {
        Some(path) => path.file_stem().unwrap().to_string_lossy().into_owned(),
        None if route == "/" => app.name().to_owned(),
        None => format!("{} · {route}", app.name()),
    });
    ensure!(!name.trim().is_empty(), "frame name must not be empty");
    let directory = app.root().join(
        args.out
            .clone()
            .unwrap_or_else(|| PathBuf::from("design/exports").join(slug(&name))),
    );
    fs::create_dir_all(&directory)?;
    let directory = directory.canonicalize()?;
    let output_lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join(".export.lock"))?;
    output_lock
        .try_lock()
        .context("another Ink export is writing this directory")?;

    println!("{} · export", app.name());
    let FrameSize { width, height } = args.size;
    output::tree_root_field("Frame", format!("{name} · {width} × {height}"), false);
    let marker = format!(
        "INK_EXPORT_{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let source = entry_source(
        &project,
        entry.as_deref(),
        setup.as_deref(),
        route,
        &props,
        &marker,
    )?;
    let progress = output::tree_spinner("Render", false);
    let result = (|| {
        ink_compiler::compile_preview(&project, &source, entry.is_none())?;
        headless::capture(&project, &marker, &args)
    })();
    progress.finish_and_clear();
    let Capture {
        png,
        pixels,
        mut metadata,
    } = result?;
    metadata.extend([
        (
            "route".into(),
            json!(if entry.is_none() { Some(route) } else { None }),
        ),
        ("entry".into(), json!(entry)),
        ("setup".into(), json!(setup)),
        ("props".into(), props),
        ("fixtures".into(), json!(args.fixtures)),
        ("theme".into(), json!(args.theme)),
        ("light".into(), json!(args.light)),
        ("cutouts".into(), json!(args.cutout.iter().map(|cutout| [cutout.x, cutout.y, cutout.width, cutout.height]).collect::<Vec<_>>())),
        ("stableCaptures".into(), json!(3)),
        ("rgbaSha256".into(), json!(hash(&pixels))),
        ("pngSha256".into(), json!(hash(&png))),
    ]);
    write_artifacts(&directory, app, &name, &png, args.size, Value::Object(metadata))?;
    output::tree_root_field("Pixels", "three identical decoded RGBA frames", false);
    output::tree_root_field("Export", directory.to_string_lossy(), true);
    Ok(())
}

fn parse_cutout(value: &str) -> Result<ink_core::Rect, String> {
    let parts = value.split(',').map(|part| part.trim().parse::<f32>()).collect::<Result<Vec<_>, _>>();
    match parts.as_deref() {
        Ok([x, y, width, height]) if *width > 0.0 && *height > 0.0 => Ok(ink_core::Rect { x: *x, y: *y, width: *width, height: *height }),
        _ => Err(format!("expected X,Y,WIDTH,HEIGHT in pixels, such as 480,0,120,90; got {value:?}")),
    }
}

fn resolve_file(project: &Project, path: &Path) -> Result<PathBuf> {
    let resolved = project
        .root()
        .join(path)
        .canonicalize()
        .with_context(|| format!("could not find {}", path.display()))?;
    ensure!(resolved.is_file(), "{} must be a file", path.display());
    Ok(resolved)
}

fn entry_source(
    project: &Project,
    entry: Option<&Path>,
    setup: Option<&Path>,
    route: &str,
    props: &Value,
    marker: &str,
) -> Result<String> {
    let app = entry
        .map(Path::to_owned)
        .unwrap_or_else(|| project.work_path().join("routes.tsx"));
    let setup_import = match setup {
        Some(path) => format!("import setup from {};\n", serde_json::to_string(path)?),
        None => String::new(),
    };
    let initialise = if setup.is_some() {
        "await setup();"
    } else {
        ""
    };
    let component_props = if entry.is_some() {
        props.clone()
    } else {
        json!({"initialDestination": {"path":route,"params":props}})
    };
    Ok(format!(
        "import {{ createElement, useEffect }} from 'react';\nimport {{ render }} from 'ink/renderer';\nimport App from {};\n{setup_import}\nfunction Preview() {{ useEffect(() => {{ console.info({}); }}, []); return createElement(App, {}); }}\nasync function start() {{ {initialise} render(createElement(Preview)); }}\nvoid start().catch(error => {{ console.error('INK_EXPORT_FAILED', error.stack ?? String(error)); }});\n",
        serde_json::to_string(&app)?,
        serde_json::to_string(marker)?,
        component_props,
    ))
}

fn write_artifacts(
    directory: &Path,
    project: &Project,
    name: &str,
    png: &[u8],
    size: FrameSize,
    evidence: Value,
) -> Result<()> {
    let path = directory.join("frame.png");
    fs::write(&path, png)?;
    let data_url = format!("data:image/png;base64,{}", STANDARD.encode(png));
    let identity = hash(format!("{}:{name}", project.package()).as_bytes());
    let script = include_str!("export-tldraw.js")
        .replace("__INK_ID__", &identity[..24])
        .replace("__INK_NAME__", &serde_json::to_string(name)?)
        .replace("__INK_WIDTH__", &size.width.to_string())
        .replace("__INK_HEIGHT__", &size.height.to_string())
        .replace("__INK_IMAGE__", &serde_json::to_string(&data_url)?);
    fs::write(directory.join("tldraw.js"), script)?;
    let manifest = json!({ "version": 1, "name": name, "width": size.width, "height": size.height,
        "image": path, "evidence": evidence,
        "tldraw": { "script": directory.join("tldraw.js") },
    });
    fs::write(
        directory.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn slug(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "frame".into()
    } else {
        slug
    }
}
