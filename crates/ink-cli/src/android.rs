use std::{
    env, fs,
    io::IsTerminal,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Instant,
};

use anyhow::{Context, Result, bail};
use ink_compiler::Project;

use crate::{output, process};

const ACTIVITY_CLASS: &str = "com.vandam.ink.MainActivity";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    fn gradle_task(self) -> &'static str {
        match self {
            Self::Debug => ":app:assembleDebug",
            Self::Release => ":app:assembleRelease",
        }
    }

    fn directory(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    fn apk_name(self) -> &'static str {
        match self {
            Self::Debug => "app-debug.apk",
            Self::Release => "app-release.apk",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Debug => "development",
            Self::Release => "release",
        }
    }
}

pub struct BuildArtifact {
    pub apk: PathBuf,
}

#[derive(Clone)]
pub struct Device {
    pub serial: String,
    pub state: String,
    model: Option<String>,
    product: Option<String>,
}

impl Device {
    pub fn ready(&self) -> bool {
        self.state == "device"
    }

    pub fn name(&self) -> String {
        self.model
            .as_deref()
            .or(self.product.as_deref())
            .unwrap_or(&self.serial)
            .replace('_', " ")
    }

    pub fn description(&self) -> String {
        if self.ready() {
            format!("{} ({})", self.name(), self.serial)
        } else {
            format!("{} ({}, {})", self.name(), self.serial, self.state)
        }
    }

    fn light_server<'a>(&self, configured: &'a str) -> &'a str {
        if self.serial.starts_with("emulator-") && configured == "com.lightos" {
            "com.thelightphone.sdk.emulator"
        } else {
            configured
        }
    }

    fn matches(&self, query: &str) -> bool {
        if self.serial.eq_ignore_ascii_case(query) {
            return true;
        }
        let query = normalise(query);
        [self.model.as_deref(), self.product.as_deref()]
            .into_iter()
            .flatten()
            .flat_map(device_aliases)
            .any(|value| value.contains(&query))
    }
}

pub fn build(project: &Project, profile: Profile, verbose: bool) -> Result<BuildArtifact> {
    println!("{} · build", project.name());
    build_stage("Compile", || compile(project, profile))?;
    let artifact = build_stage("Android", || {
        assemble(project, profile, verbose, project.light_server())
    })?;
    if let Err(error) = verify_apk(&artifact.apk, verbose) {
        output::tree_root_field("Verify", "failed", true);
        return Err(error);
    }
    Ok(artifact)
}

pub fn build_precompiled_for_device(
    project: &Project,
    device: &Device,
    verbose: bool,
) -> Result<BuildArtifact> {
    let artifact = assemble(
        project,
        Profile::Debug,
        verbose,
        device.light_server(project.light_server()),
    )?;
    verify_apk(&artifact.apk, verbose)?;
    Ok(artifact)
}

fn build_stage<T>(label: &str, run: impl FnOnce() -> Result<T>) -> Result<T> {
    let started = Instant::now();
    let progress = output::tree_spinner(label, false);
    let result = run();
    progress.finish_and_clear();
    match result {
        Ok(value) => {
            output::tree_root_field(
                label,
                format!("complete · {}", output::duration(started.elapsed())),
                false,
            );
            Ok(value)
        }
        Err(error) => {
            output::tree_root_field(label, "failed", true);
            Err(error)
        }
    }
}

fn assemble(
    project: &Project,
    profile: Profile,
    verbose: bool,
    light_server: &str,
) -> Result<BuildArtifact> {
    let sdk = project_framework_root(project)?;
    fs::create_dir_all(sdk.join("target"))?;
    let build_lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(sdk.join("target/ink-android.lock"))?;
    build_lock
        .lock()
        .context("could not lock the shared native build cache")?;
    let mut gradle = gradle_command(project, profile, light_server)?;
    let message = format!("Building {} APK", profile.label());
    process::run_quiet(&mut gradle, &message, verbose)?;
    build_artifact(project, profile)
}

fn gradle_command(project: &Project, profile: Profile, light_server: &str) -> Result<Command> {
    let release_signing = if profile == Profile::Release {
        let signing = project.release_signing().context(
            "release signing is not configured; add [signing] with keystore and key_alias to ink.toml",
        )?;
        if !signing.keystore.is_file() {
            bail!(
                "release keystore does not exist: {}",
                signing.keystore.display()
            );
        }
        if env::var_os("INK_KEYSTORE_PASSWORD").is_none() {
            bail!("release signing requires INK_KEYSTORE_PASSWORD");
        }
        if env::var_os("INK_KEY_PASSWORD").is_none() {
            output::warning("INK_KEY_PASSWORD is unset; using INK_KEYSTORE_PASSWORD");
        }
        Some(signing)
    } else {
        None
    };

    let framework = project_framework_root(project)?;
    let android = framework.join("platform/android");
    let mut gradle = Command::new(android.join("gradlew"));
    gradle
        .current_dir(&framework)
        .arg("-p")
        .arg(&android)
        .arg("--project-cache-dir")
        .arg(project.root().join(".ink/build/gradle-cache"))
        .arg(format!(
            "-PinkBuildRoot={}",
            project.root().join(".ink/build/android").display()
        ))
        .arg(format!("-PinkAppName={}", project.name()))
        .arg(format!("-PinkApplicationId={}", project.package()))
        .arg(format!("-PinkVersionName={}", project.version()))
        .arg(format!("-PinkVersionCode={}", project.version_code()))
        .arg(format!(
            "-PinkCapabilitiesManifest={}",
            project.capability_manifest_path().display()
        ))
        .arg(format!("-PinkLightServerPackage={light_server}"))
        .arg(format!(
            "-PinkAuthRedirectUri={}",
            project.auth_redirect_uri().unwrap_or("")
        ))
        .arg(format!(
            "-PinkAppAndroid={}",
            project.root().join("android").display()
        ))
        .arg(format!(
            "-PinkAndroidResources={}",
            project.android_resources_path().display()
        ))
        .arg(format!(
            "-PinkAndroidAssets={}",
            project.android_assets_path().display()
        ));

    if let Some(signing) = release_signing {
        gradle
            .arg("-PinkSigning=release")
            .arg(format!("-PinkStoreFile={}", signing.keystore.display()))
            .arg(format!("-PinkKeyAlias={}", signing.key_alias));
    }
    gradle.arg(profile.gradle_task());
    Ok(gradle)
}

fn compile(project: &Project, profile: Profile) -> Result<()> {
    if profile == Profile::Debug {
        ink_compiler::compile_development(project)?;
    } else {
        ink_compiler::compile(project)?;
    }
    Ok(())
}

fn build_artifact(project: &Project, profile: Profile) -> Result<BuildArtifact> {
    let apk = project
        .root()
        .join(".ink/build/android/outputs/apk")
        .join(profile.directory())
        .join(profile.apk_name());
    if !apk.is_file() {
        bail!("Gradle completed without producing {}", apk.display());
    }
    Ok(BuildArtifact { apk })
}

pub fn copy_build(
    project: &Project,
    profile: Profile,
    artifact: &BuildArtifact,
) -> Result<PathBuf> {
    let output_directory = project.root().join("dist");
    fs::create_dir_all(&output_directory)
        .with_context(|| format!("could not create {}", output_directory.display()))?;
    let suffix = if profile == Profile::Debug {
        "-debug"
    } else {
        ""
    };
    let file_name = format!(
        "{}-{}-arm64{suffix}.apk",
        slug(project.name()),
        project.version()
    );
    let output_path = output_directory.join(file_name);
    fs::copy(&artifact.apk, &output_path).with_context(|| {
        format!(
            "could not copy {} to {}",
            artifact.apk.display(),
            output_path.display()
        )
    })?;
    Ok(output_path)
}

pub fn install(device: &Device, apk: &Path, verbose: bool) -> Result<()> {
    let mut command = adb(device);
    command.arg("install").arg("-r").arg(apk);
    process::run_quiet(&mut command, "Installing application", verbose)?;
    Ok(())
}

pub fn launch(device: &Device, project: &Project, verbose: bool) -> Result<()> {
    let activity = format!("{}/{}", project.package(), ACTIVITY_CLASS);
    let mut command = adb(device);
    command.args(["shell", "am", "start", "-S", "-n", &activity]);
    process::run_quiet(&mut command, "Launching application", verbose)?;
    Ok(())
}

pub fn open(device: &Device, project: &Project) -> Result<()> {
    let activity = format!("{}/{}", project.package(), ACTIVITY_CLASS);
    let mut command = adb(device);
    command.args(["shell", "am", "start", "-n", &activity]);
    process::run_quiet(&mut command, "Opening application", false)?;
    Ok(())
}

pub fn connected_devices() -> Result<Vec<Device>> {
    let result = Command::new("adb")
        .args(["devices", "-l"])
        .output()
        .context("could not run adb; run `ink doctor` for setup help")?;
    if !result.status.success() {
        bail!("adb devices failed");
    }
    Ok(String::from_utf8_lossy(&result.stdout)
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_device)
        .collect())
}

pub fn select_device(requested: Option<&str>) -> Result<Device> {
    let devices = connected_devices()?;
    let ready = devices
        .into_iter()
        .filter(Device::ready)
        .collect::<Vec<_>>();

    let selected = if let Some(requested) = requested {
        let matches = ready
            .iter()
            .filter(|device| device.matches(requested))
            .cloned()
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => bail!("Android device {requested:?} is not connected"),
            [device] => device.clone(),
            _ => bail!(
                "device name {requested:?} is ambiguous:\n  {}",
                matches
                    .iter()
                    .map(Device::description)
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ),
        }
    } else if let Some(serial) = remembered_device() {
        ready
            .iter()
            .find(|device| device.serial == serial)
            .cloned()
            .context("the remembered Android device is disconnected; use --device <DEVICE>")?
    } else {
        match ready.as_slice() {
            [] => bail!("no Android device is connected"),
            [device] => device.clone(),
            _ => bail!(
                "multiple Android devices are connected; select one with --device <DEVICE>:\n  {}",
                ready
                    .iter()
                    .map(Device::description)
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ),
        }
    };
    remember_device(&selected.serial)?;
    Ok(selected)
}

pub fn stream_logs(device: &Device, package: &str, resources: bool) -> Result<()> {
    let mut logs = LogStream::start(device, package, resources)?;
    logs.wait()
}

pub struct LogStream {
    app: Child,
    crash: Child,
}

impl LogStream {
    pub fn start(device: &Device, package: &str, resources: bool) -> Result<Self> {
        let format = if std::io::stdout().is_terminal() {
            "color,threadtime"
        } else {
            "threadtime"
        };
        let uid = package_uid(device, package)?;
        let mut app = adb(device);
        app.args([
            "logcat",
            "-b",
            "default",
            &format!("--uid={uid}"),
            "-T",
            "1",
            "-v",
            format,
        ]);
        if resources {
            app.args(["InkResource:D", "*:S"]);
        } else {
            app.args(["Ink:V", "AndroidRuntime:E", "libc:F", "*:E"]);
        }
        app.stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let app = app.spawn().context("could not start application Logcat")?;

        let mut crash = adb(device);
        crash
            .args([
                "logcat",
                "-b",
                "crash",
                &format!("--uid={uid}"),
                "-T",
                "1",
                "-v",
                format,
                "AndroidRuntime:E",
                "DEBUG:E",
                "libc:F",
                "*:S",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let crash = crash.spawn().context("could not start crash Logcat")?;
        Ok(Self { app, crash })
    }

    pub fn wait(&mut self) -> Result<()> {
        let status = self.app.wait()?;
        if status.success() {
            Ok(())
        } else {
            bail!("Logcat stopped with {status}")
        }
    }
}

impl Drop for LogStream {
    fn drop(&mut self) {
        self.app.kill().ok();
        self.crash.kill().ok();
        self.app.wait().ok();
        self.crash.wait().ok();
    }
}

pub fn doctor() -> Result<()> {
    println!("Ink · doctor");
    let mut toolchain_issues = Vec::new();
    if !check_tool("bun", &["--version"]) {
        toolchain_issues.push(("Bun", "bun is not available".to_owned()));
    }
    let cargo_ready = check_tool("cargo", &["--version"]);
    if cargo_ready {
        if !check_tool("cargo", &["clippy", "--version"]) {
            toolchain_issues.push(("Clippy", "cargo clippy is not available".to_owned()));
        }
        if !check_tool("cargo", &["ndk", "--version"]) {
            toolchain_issues.push(("Cargo NDK", "cargo ndk is not available".to_owned()));
        }
    } else {
        toolchain_issues.push(("Rust", "cargo is not available".to_owned()));
    }
    if !check_tool("java", &["-version"]) {
        toolchain_issues.push(("Java", "java is not available".to_owned()));
    }
    let adb_ready = check_tool("adb", &["version"]);
    if !adb_ready {
        toolchain_issues.push(("ADB", "adb is not available".to_owned()));
    }
    let framework = match framework_root() {
        Ok(path) => Some(path),
        Err(error) => {
            toolchain_issues.push(("Ink SDK", error.to_string()));
            None
        }
    };
    output::tree_root_field(
        "Toolchain",
        if toolchain_issues.is_empty() {
            "ready · Bun, Rust, Java"
        } else {
            "needs attention"
        },
        false,
    );
    for (index, (label, detail)) in toolchain_issues.iter().enumerate() {
        output::tree_field(false, index + 1 == toolchain_issues.len(), label, detail);
    }

    let mut android_issues = Vec::new();
    let sdk = match android_sdk() {
        Ok(path) => Some(path),
        Err(error) => {
            android_issues.push(("SDK", error.to_string()));
            None
        }
    };
    if let (Some(framework), Some(sdk)) = (&framework, &sdk)
        && let Err(error) = android_ndk(framework, sdk)
    {
        android_issues.push(("NDK", error.to_string()));
    }
    if cargo_ready {
        let target = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output();
        if !target.is_ok_and(|result| {
            result.status.success()
                && String::from_utf8_lossy(&result.stdout).contains("aarch64-linux-android")
        }) {
            android_issues.push((
                "Rust target",
                "aarch64-linux-android is not installed".to_owned(),
            ));
        }
    }
    if let Err(error) = apksigner() {
        android_issues.push(("APK signer", error.to_string()));
    }
    output::tree_root_field(
        "Android",
        if !android_issues.is_empty() {
            "needs attention"
        } else if framework.is_none() || !cargo_ready {
            "partly checked"
        } else {
            "ready · SDK, NDK, signer"
        },
        false,
    );
    for (index, (label, detail)) in android_issues.iter().enumerate() {
        output::tree_field(false, index + 1 == android_issues.len(), label, detail);
    }

    let (device, device_error) = if adb_ready {
        match connected_devices() {
            Ok(devices) => {
                let ready: Vec<_> = devices.iter().filter(|device| device.ready()).collect();
                let name = match ready.as_slice() {
                    [] => "None connected".to_owned(),
                    [device] => device.name(),
                    _ => format!("{} connected", ready.len()),
                };
                (name, None)
            }
            Err(error) => ("needs attention".to_owned(), Some(error)),
        }
    } else {
        ("unavailable".to_owned(), None)
    };
    let healthy =
        toolchain_issues.is_empty() && android_issues.is_empty() && device_error.is_none();
    output::tree_root_field("Device", device, true);
    if let Some(error) = device_error {
        output::tree_field(true, true, "ADB", error.to_string());
    }
    if healthy {
        Ok(())
    } else {
        Err(output::ReportedError.into())
    }
}

fn verify_apk(apk: &Path, verbose: bool) -> Result<()> {
    let signer = apksigner()?;
    let mut command = Command::new(signer);
    command.arg("verify").arg("--verbose").arg(apk);
    process::run_quiet(&mut command, "Verifying APK signature", verbose)?;
    Ok(())
}

fn apksigner() -> Result<PathBuf> {
    let sdk = android_sdk()?;
    let build_tools = sdk.join("build-tools");
    let mut versions = Vec::new();
    for entry in fs::read_dir(&build_tools)
        .with_context(|| format!("could not read {}", build_tools.display()))?
    {
        let signer = entry
            .with_context(|| format!("could not read {}", build_tools.display()))?
            .path()
            .join("apksigner");
        if signer.is_file() {
            versions.push(signer);
        }
    }
    versions.sort();
    versions
        .pop()
        .context("Android apksigner is not installed; install Android SDK Build Tools")
}

fn android_sdk() -> Result<PathBuf> {
    for variable in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(value) = env::var_os(variable) {
            let path = PathBuf::from(value);
            if path.is_dir() {
                return Ok(path);
            }
        }
    }
    if let Some(home) = env::var_os("HOME") {
        let path = PathBuf::from(home).join("Library/Android/sdk");
        if path.is_dir() {
            return Ok(path);
        }
    }
    bail!("Android SDK not found; set ANDROID_HOME")
}

fn android_ndk(framework: &Path, sdk: &Path) -> Result<PathBuf> {
    let gradle = fs::read_to_string(framework.join("platform/android/app/build.gradle.kts"))
        .context("could not read the SDK's required Android NDK version")?;
    let version = gradle
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("ndkVersion = \"")?
                .split('"')
                .next()
        })
        .context("the Ink SDK does not declare an Android NDK version")?;
    let ndk = sdk.join("ndk").join(version);
    let properties = fs::read_to_string(ndk.join("source.properties")).unwrap_or_default();
    let revision = properties.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == "Pkg.Revision").then_some(value.trim())
    });
    let has_compiler = fs::read_dir(ndk.join("toolchains/llvm/prebuilt")).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| entry.path().join("bin/clang").is_file())
    });
    if revision != Some(version) || !has_compiler {
        bail!(
            "Android NDK {version} is missing or incomplete; install it with sdkmanager \"ndk;{version}\""
        );
    }
    Ok(ndk)
}

fn check_tool(program: &str, arguments: &[&str]) -> bool {
    Command::new(program)
        .args(arguments)
        .output()
        .is_ok_and(|result| result.status.success())
}

fn package_uid(device: &Device, package: &str) -> Result<u32> {
    let result = adb(device)
        .args(["shell", "cmd", "package", "list", "packages", "-U", package])
        .output()
        .context("could not resolve the application UID")?;
    if !result.status.success() {
        bail!("could not resolve the application UID");
    }
    String::from_utf8_lossy(&result.stdout)
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?.strip_prefix("package:")?;
            let uid = fields.next()?.strip_prefix("uid:")?.parse().ok()?;
            (name == package).then_some(uid)
        })
        .with_context(|| format!("{package} is not installed on {}", device.name()))
}

fn parse_device(line: &str) -> Option<Device> {
    let mut fields = line.split_whitespace();
    let serial = fields.next()?.to_owned();
    let state = fields.next()?.to_owned();
    let mut model = None;
    let mut product = None;
    for field in fields {
        if let Some(value) = field.strip_prefix("model:") {
            model = Some(value.to_owned());
        } else if let Some(value) = field.strip_prefix("product:") {
            product = Some(value.to_owned());
        }
    }
    Some(Device {
        serial,
        state,
        model,
        product,
    })
}

fn adb(device: &Device) -> Command {
    let mut command = Command::new("adb");
    command.arg("-s").arg(&device.serial);
    command
}

pub(crate) fn project_framework_root(project: &Project) -> Result<PathBuf> {
    framework_root_for_app(project.root())
}

pub(crate) fn framework_root_for_app(root: &Path) -> Result<PathBuf> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("package.json"))?)?;
    let dependency = manifest["dependencies"]["ink"]
        .as_str()
        .context("package.json must declare an ink dependency")?;
    let candidate = if let Some(path) = dependency.strip_prefix("file:") {
        let package = root.join(path).canonicalize().context(
            "Ink dependency is missing; update its file: path in package.json and run bun install",
        )?;
        package
            .parent()
            .and_then(Path::parent)
            .context("Ink dependency must point to packages/ink in an SDK checkout")?
            .to_path_buf()
    } else if dependency.starts_with("workspace:") {
        root.ancestors()
            .find(|path| path.join("sdk.json").is_file())
            .context("workspace ink dependency has no enclosing SDK checkout")?
            .to_path_buf()
    } else {
        bail!("local Ink SDK requires a file: dependency pointing to its packages/ink directory")
    };
    validate_framework_root(candidate)
}

pub(crate) fn framework_root() -> Result<PathBuf> {
    let candidate = if let Some(root) = env::var_os("INK_SDK_ROOT") {
        PathBuf::from(root)
    } else {
        let config = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .context("set INK_SDK_ROOT to an Ink SDK checkout")?;
        config.join("ink/sdk/current")
    };
    validate_framework_root(candidate)
}

fn validate_framework_root(candidate: PathBuf) -> Result<PathBuf> {
    let root = candidate.canonicalize().with_context(|| {
        format!(
            "Ink SDK not found at {}; set INK_SDK_ROOT to the SDK checkout",
            candidate.display()
        )
    })?;
    if !root.join("platform/android/gradlew").is_file()
        || !root.join("packages/ink/package.json").is_file()
    {
        bail!("{} is not an Ink SDK", root.display());
    }
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("sdk.json"))
            .context("SDK metadata missing; update the SDK checkout")?,
    )?;
    if metadata["protocolVersion"].as_u64() != Some(1) {
        bail!("unsupported SDK protocol; use a compatible Ink CLI and SDK");
    }
    Ok(root)
}

fn device_state_path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|directory| directory.join("ink/device"))
}

pub fn remembered_device() -> Option<String> {
    fs::read_to_string(device_state_path()?)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn remember_device(serial: &str) -> Result<()> {
    let Some(path) = device_state_path() else {
        return Ok(());
    };
    if fs::read_to_string(&path).is_ok_and(|current| current.trim() == serial) {
        return Ok(());
    }
    fs::create_dir_all(path.parent().expect("the device state has a parent"))?;
    fs::write(&path, format!("{serial}\n"))?;
    Ok(())
}

fn normalise(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn device_aliases(value: &str) -> Vec<String> {
    let tokens = value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| match token.to_ascii_lowercase().as_str() {
            "ii" => "2".to_owned(),
            "iii" => "3".to_owned(),
            other => other.to_owned(),
        })
        .collect::<Vec<_>>();
    let compact = tokens.concat();
    let initials = tokens
        .iter()
        .filter_map(|token| token.chars().next())
        .collect::<String>();
    vec![normalise(value), compact, initials]
}

fn slug(name: &str) -> String {
    let slug = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    slug.trim_matches('-').to_owned()
}
