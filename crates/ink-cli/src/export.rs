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

const WIDTH: u32 = 1080;
const HEIGHT: u32 = 1240;

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
    /// Maximum wait for readiness and stable pixels, in seconds
    #[arg(long, default_value = "30", value_parser = clap::value_parser!(u64).range(1..=300))]
    timeout: u64,
}

pub fn run(app: &Project, args: ExportArgs) -> Result<()> {
    let project = app.for_preview();
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
    output::tree_root_field("Frame", format!("{name} · {WIDTH} × {HEIGHT}"), false);
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
        ("stableCaptures".into(), json!(3)),
        ("rgbaSha256".into(), json!(hash(&pixels))),
        ("pngSha256".into(), json!(hash(&png))),
    ]);
    write_artifacts(&directory, app, &name, &png, Value::Object(metadata))?;
    output::tree_root_field("Pixels", "three identical decoded RGBA frames", false);
    output::tree_root_field("Export", directory.to_string_lossy(), true);
    Ok(())
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
    evidence: Value,
) -> Result<()> {
    let path = directory.join("frame.png");
    fs::write(&path, png)?;
    let data_url = format!("data:image/png;base64,{}", STANDARD.encode(png));
    let identity = hash(format!("{}:{name}", project.package()).as_bytes());
    let script = include_str!("export-tldraw.js")
        .replace("__INK_ID__", &identity[..24])
        .replace("__INK_NAME__", &serde_json::to_string(name)?)
        .replace("__INK_IMAGE__", &serde_json::to_string(&data_url)?);
    fs::write(directory.join("tldraw.js"), script)?;
    let manifest = json!({ "version": 1, "name": name, "width": WIDTH, "height": HEIGHT,
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
