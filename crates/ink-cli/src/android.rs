use std::{
    env, fs,
    io::IsTerminal,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use ink_compiler::Project;

use crate::{output, process, watch};

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

pub enum BuildOutcome {
    Complete(BuildArtifact),
    Changed,
}

pub struct BuildArtifact {
    pub apk: PathBuf,
    pub duration: Duration,
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
    let mut gradle = gradle_command(project, profile)?;
    compile(project)?;
    let message = format!("Building {} APK", profile.label());
    let duration = process::run(&mut gradle, &message, verbose)?;
    build_artifact(profile, duration, verbose)
}

pub fn build_watched(
    project: &Project,
    profile: Profile,
    verbose: bool,
    baseline: &watch::Snapshot,
) -> Result<BuildOutcome> {
    let mut gradle = gradle_command(project, profile)?;
    compile(project)?;
    if watch::changed(project.root(), baseline)? {
        return Ok(BuildOutcome::Changed);
    }

    let message = format!("Building {} APK", profile.label());
    let duration =
        match process::run_cancellable(&mut gradle, &message, verbose, project.root(), baseline)? {
            process::PhaseOutcome::Complete(duration) => duration,
            process::PhaseOutcome::Changed => return Ok(BuildOutcome::Changed),
        };
    Ok(BuildOutcome::Complete(build_artifact(
        profile, duration, verbose,
    )?))
}

fn gradle_command(project: &Project, profile: Profile) -> Result<Command> {
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

    let framework = framework_root()?;
    let android = framework.join("platform/android");
    let mut gradle = Command::new(android.join("gradlew"));
    gradle
        .current_dir(&framework)
        .arg("-p")
        .arg(&android)
        .arg(format!("-PinkAppName={}", project.name()))
        .arg(format!("-PinkApplicationId={}", project.package()))
        .arg(format!("-PinkVersionName={}", project.version()))
        .arg(format!("-PinkVersionCode={}", project.version_code()))
        .arg(format!(
            "-PinkGeneratedSource={}",
            project.generated_source_path().display()
        ))
        .arg(format!(
            "-PinkAndroidResources={}",
            project.android_resources_path().display()
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

fn compile(project: &Project) -> Result<()> {
    ink_compiler::compile(project)?;
    output::success(format!("Compiled {}", project.source_path().display()));
    Ok(())
}

fn build_artifact(profile: Profile, duration: Duration, verbose: bool) -> Result<BuildArtifact> {
    let framework = framework_root()?;
    let android = framework.join("platform/android");
    let apk = android
        .join("app/build/outputs/apk")
        .join(profile.directory())
        .join(profile.apk_name());
    if !apk.is_file() {
        bail!("Gradle completed without producing {}", apk.display());
    }
    verify_apk(&apk, verbose)?;
    Ok(BuildArtifact { apk, duration })
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
    process::run(&mut command, "Installing application", verbose)?;
    Ok(())
}

pub fn launch(device: &Device, project: &Project, verbose: bool) -> Result<()> {
    let activity = format!("{}/{}", project.package(), ACTIVITY_CLASS);
    let mut command = adb(device);
    command.args(["shell", "am", "start", "-S", "-n", &activity]);
    process::run(&mut command, "Launching application", verbose)?;
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

pub fn stream_logs(device: &Device, package: &str) -> Result<()> {
    let mut logs = LogStream::start(device, package)?;
    output::info(format!(
        "Streaming logs for {package}. Press Ctrl-C to stop."
    ));
    logs.wait()
}

pub struct LogStream {
    app: Child,
    crash: Child,
}

impl LogStream {
    pub fn start(device: &Device, package: &str) -> Result<Self> {
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
            "Ink:V",
            "AndroidRuntime:E",
            "libc:F",
            "*:E",
        ])
        .stdin(Stdio::null())
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
    output::info("Checking the Ink toolchain");
    let mut healthy = true;
    healthy &= check_tool("Rust", "cargo", &["--version"]);
    healthy &= check_tool("Clippy", "cargo", &["clippy", "--version"]);
    healthy &= check_tool("Cargo NDK", "cargo", &["ndk", "--version"]);
    healthy &= check_tool("Java", "java", &["-version"]);
    healthy &= check_tool("ADB", "adb", &["version"]);

    match Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
    {
        Ok(result)
            if result.status.success()
                && String::from_utf8_lossy(&result.stdout).contains("aarch64-linux-android") =>
        {
            output::success("Rust Android target");
        }
        _ => {
            output::error("Rust target aarch64-linux-android is not installed");
            healthy = false;
        }
    }

    match apksigner() {
        Ok(path) => output::success(format!("APK signer: {}", path.display())),
        Err(error) => {
            output::error(error.to_string());
            healthy = false;
        }
    }

    match connected_devices() {
        Ok(devices) if devices.iter().all(|device| !device.ready()) => {
            output::warning("No Android device is connected");
        }
        Ok(devices) => output::success(format!(
            "Android devices: {}",
            devices
                .iter()
                .filter(|device| device.ready())
                .map(Device::description)
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Err(error) => {
            output::error(error.to_string());
            healthy = false;
        }
    }

    if healthy {
        output::success("Ink is ready");
        Ok(())
    } else {
        bail!("Ink needs attention before it can build an application")
    }
}

fn verify_apk(apk: &Path, verbose: bool) -> Result<()> {
    let signer = apksigner()?;
    let mut command = Command::new(signer);
    command.arg("verify").arg("--verbose").arg(apk);
    process::run(&mut command, "Verifying APK signature", verbose)?;
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

fn check_tool(label: &str, program: &str, arguments: &[&str]) -> bool {
    match Command::new(program).args(arguments).output() {
        Ok(result) if result.status.success() => {
            let text = if result.stdout.is_empty() {
                &result.stderr
            } else {
                &result.stdout
            };
            let output = String::from_utf8_lossy(text);
            let version = output.lines().next().unwrap_or_default().trim();
            output::success(format!("{label}: {version}"));
            true
        }
        _ => {
            output::error(format!("{label} is not available ({program})"));
            false
        }
    }
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

fn framework_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .context("could not locate the Ink framework checkout")
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
