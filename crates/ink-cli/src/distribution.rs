use anyhow::{Context, Result, ensure};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn installed_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?.canonicalize().ok()?;
    let root = exe.parent()?.parent()?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("sdk.json")).ok()?).ok()?;
    (metadata["distribution"] == "release").then(|| root.to_path_buf())
}

// Called before spawning any threads. These settings only affect Ink and its children.
pub fn initialise() -> Result<()> {
    let Some(root) = installed_root() else {
        return Ok(());
    };
    let sdk: serde_json::Value = serde_json::from_slice(&fs::read(root.join("sdk.json"))?)?;
    ensure!(
        sdk["version"] == env!("CARGO_PKG_VERSION"),
        "Ink CLI and SDK versions differ; reinstall Ink"
    );
    let mut paths = vec![root.join("bin")];
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        if env::var_os("ANDROID_HOME").is_none() && env::var_os("ANDROID_SDK_ROOT").is_none() {
            let android = home.join(if cfg!(target_os = "macos") {
                "Library/Android/sdk"
            } else {
                "Android/Sdk"
            });
            if android.is_dir() {
                unsafe {
                    env::set_var("ANDROID_HOME", android);
                }
            }
        }
    }
    if env::var_os("JAVA_HOME").is_none() {
        let java = Path::new(if cfg!(target_os = "macos") {
            "/Applications/Android Studio.app/Contents/jbr/Contents/Home"
        } else {
            "/opt/android-studio/jbr"
        });
        if java.is_dir() {
            unsafe {
                env::set_var("JAVA_HOME", java);
            }
        }
    }
    if let Some(java) = env::var_os("JAVA_HOME") {
        paths.push(PathBuf::from(java).join("bin"));
    }
    if let Some(android) = env::var_os("ANDROID_HOME").or_else(|| env::var_os("ANDROID_SDK_ROOT")) {
        paths.push(PathBuf::from(android).join("platform-tools"));
    }
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    if let Some(cargo) = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
    {
        paths.push(cargo.join("bin"));
    }
    unsafe {
        env::set_var("PATH", env::join_paths(paths)?);
        env::set_var("INK_SDK_ROOT", &root);
        env::set_var("INK_RELEASE_ROOT", &root);
        env::set_var(
            "RUSTUP_TOOLCHAIN",
            sdk["rust"]
                .as_str()
                .context("release has no Rust version")?,
        );
        env::set_var("RUSTUP_AUTO_INSTALL", "0");
    }
    Ok(())
}

pub fn run(action: &str, root: Option<&Path>, args: &[String]) -> Result<()> {
    let sdk = crate::android::framework_root()?;
    let mut command = Command::new("bun");
    command
        .arg(sdk.join("scripts/release/runtime.ts"))
        .arg(action)
        .args(args);
    if let Some(root) = root {
        command.arg(root);
    }
    let status = command
        .status()
        .context("could not run Ink release tooling")?;
    ensure!(status.success(), "Ink {action} failed");
    Ok(())
}

pub fn prepare(root: &Path) -> Result<()> {
    if installed_root().is_some() {
        run("prepare", Some(root), &[])?;
    }
    Ok(())
}
