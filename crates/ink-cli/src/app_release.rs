use std::{fs, path::Path};

use anyhow::{Result, ensure};

const FILES: &[(&str, &str)] = &[
    (
        ".github/workflows/prepare-release.yml",
        include_str!("app-release/prepare-release.yml"),
    ),
    (
        ".github/workflows/release.yml",
        include_str!("app-release/release.yml"),
    ),
    (
        ".github/ink-release.mjs",
        include_str!("app-release/ink-release.mjs"),
    ),
];

pub fn write(root: &Path) -> Result<()> {
    for (name, contents) in FILES {
        let path = root.join(name);
        if path.exists() {
            ensure!(
                fs::read_to_string(&path)? == *contents,
                "{} already exists with different contents; move it aside before running ink setup release",
                path.display()
            );
        }
    }
    for (name, contents) in FILES {
        let path = root.join(name);
        fs::create_dir_all(path.parent().expect("template has a parent"))?;
        fs::write(path, contents)?;
    }
    Ok(())
}

pub fn setup(root: &Path) -> Result<()> {
    write(root)?;
    println!("Ink · setup release");
    crate::output::tree_step("Workflows", "Prepare Release and Release", true, true);
    println!("\nAdd INK_KEYSTORE_BASE64 and INK_KEYSTORE_PASSWORD as GitHub Actions secrets.");
    println!("Set INK_KEY_ALIAS as a repository variable if your alias is not app.");
    println!("Allow GitHub Actions to create pull requests, then run Prepare Release.");
    Ok(())
}
