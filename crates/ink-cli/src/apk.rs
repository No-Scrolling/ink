use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
    process::Command,
};

use anyhow::{Context, Result, ensure};
use ink_compiler::{Project, ReleaseSigning};
use roxmltree::{Document, Node};
use zip::ZipArchive;

const ANDROID: &str = "http://schemas.android.com/apk/res/android";

pub fn expected_certificate(signing: &ReleaseSigning) -> Result<String> {
    let fingerprint = signing.certificate_sha256.as_deref().context(
        "release signing requires signing.certificate_sha256 in ink.toml; use keytool -list -v to read the certificate SHA-256 fingerprint",
    )?;
    let fingerprint = fingerprint.replace(':', "").to_ascii_lowercase();
    ensure!(
        fingerprint.len() == 64 && fingerprint.bytes().all(|b| b.is_ascii_hexdigit()),
        "signing.certificate_sha256 must be a SHA-256 certificate fingerprint"
    );
    Ok(fingerprint)
}

fn output(command: &mut Command) -> Result<String> {
    let result = command
        .output()
        .context("could not run Android APK inspection tool")?;
    ensure!(
        result.status.success(),
        "APK inspection failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(String::from_utf8(result.stdout)?)
}

fn attribute<'a>(node: Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.attribute((ANDROID, name))
}

/// Verify the signed artefact, including merged third-party manifest declarations.
pub fn verify(project: &Project, apk: &Path, sdk: &Path, signer: &Path) -> Result<()> {
    let expected = expected_certificate(
        project
            .release_signing()
            .context("release signing is missing")?,
    )?;
    let certificates = output(
        Command::new(signer)
            .args(["verify", "--print-certs"])
            .arg(apk),
    )?;
    let actual: Vec<_> = certificates
        .lines()
        .filter_map(|line| {
            line.split_once(" certificate SHA-256 digest: ")
                .map(|(_, value)| value.trim().to_ascii_lowercase())
        })
        .collect();
    ensure!(
        actual == [expected],
        "APK signing certificate does not match signing.certificate_sha256"
    );
    output(
        Command::new(signer.with_file_name("zipalign"))
            .args(["-c", "-P", "16", "4"])
            .arg(apk),
    )?;

    let manifest = output(
        Command::new(sdk.join("cmdline-tools/latest/bin/apkanalyzer"))
            .args(["manifest", "print"])
            .arg(apk),
    )?;
    let document = Document::parse(&manifest)?;
    let root = document.root_element();
    ensure!(
        root.attribute("package") == Some(project.package()),
        "APK package does not match ink.toml"
    );
    let application = root
        .children()
        .find(|n| n.has_tag_name("application"))
        .context("APK application manifest is missing")?;
    ensure!(
        attribute(application, "debuggable") != Some("true"),
        "Release APK is debuggable"
    );
    ensure!(
        attribute(application, "testOnly") != Some("true"),
        "Release APK is test-only"
    );
    ensure!(
        attribute(application, "allowBackup") == Some("false"),
        "Release APK permits backups"
    );
    ensure!(
        attribute(application, "extractNativeLibs") == Some("false"),
        "Release APK extracts native libraries"
    );

    let generated = std::fs::read_to_string(
        project
            .root()
            .join(".ink/build/android/generated/ink/AndroidManifest.xml"),
    )?;
    let generated = Document::parse(&generated)?;
    let mut permissions: BTreeSet<String> = generated
        .descendants()
        .filter(|n| n.has_tag_name("uses-permission"))
        .filter_map(|n| attribute(n, "name").map(str::to_owned))
        .collect();
    // Added by Media3 and AndroidX, independently of Ink's capability manifest.
    permissions.insert("android.permission.WAKE_LOCK".into());
    permissions.insert(format!(
        "{}.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION",
        project.package()
    ));
    let native_manifest = project.root().join("android/src/main/AndroidManifest.xml");
    let native_source = if native_manifest.is_file() {
        std::fs::read_to_string(native_manifest)?
    } else {
        "<manifest/>".into()
    };
    let native = Document::parse(&native_source)?;
    permissions.extend(
        native
            .descendants()
            .filter(|n| n.has_tag_name("uses-permission"))
            .filter_map(|n| attribute(n, "name").map(str::to_owned)),
    );
    for permission in root
        .children()
        .filter(|n| n.has_tag_name("uses-permission") || n.has_tag_name("uses-permission-sdk-23"))
    {
        let name = attribute(permission, "name").context("APK permission has no name")?;
        ensure!(
            permissions.contains(name),
            "Unexpected APK permission: {name}; declare native permissions in android/src/main/AndroidManifest.xml"
        );
    }
    let base = Document::parse(include_str!(
        "../../../platform/android/app/src/main/AndroidManifest.xml"
    ))?;
    let component_name = |name: &str| {
        if name.starts_with('.') {
            format!("com.vandam.ink{name}")
        } else {
            name.to_owned()
        }
    };
    let exported: BTreeMap<_, _> = [base, generated, native]
        .iter()
        .flat_map(|doc| doc.descendants())
        .filter(|n| attribute(*n, "exported") == Some("true"))
        .filter_map(|n| {
            attribute(n, "name").map(|name| {
                (
                    (n.tag_name().name().to_owned(), component_name(name)),
                    attribute(n, "permission").map(str::to_owned),
                )
            })
        })
        .collect();
    for component in application
        .children()
        .filter(|n| attribute(*n, "exported") == Some("true"))
    {
        let name = attribute(component, "name").context("Exported APK component has no name")?;
        let profile_installer = name == "androidx.profileinstaller.ProfileInstallReceiver"
            && attribute(component, "permission") == Some("android.permission.DUMP");
        ensure!(
            exported
                .get(&(component.tag_name().name().to_owned(), component_name(name)))
                .is_some_and(
                    |permission| permission.as_deref() == attribute(component, "permission")
                )
                || profile_installer,
            "Unexpected exported APK component: {name}; declare intentional native exports in android/src/main/AndroidManifest.xml"
        );
    }

    let mut archive = ZipArchive::new(File::open(apk)?)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name()?.into_owned();
        let leaf = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_ascii_lowercase();
        ensure!(
            !(name.starts_with("assets/")
                && (leaf.ends_with(".map")
                    || leaf.ends_with(".js")
                    || leaf.starts_with(".env")
                    || leaf == "ink-bundle-v1.json"))
                && ![".jks", ".keystore", ".p12", ".pfx", ".pem", ".key"]
                    .iter()
                    .any(|suffix| leaf.ends_with(suffix)),
            "Development source or signing material packaged in release APK: {name}"
        );
        if name.starts_with("lib/") && name.ends_with(".so") {
            ensure!(
                name.starts_with("lib/arm64-v8a/"),
                "Unexpected native ABI: {name}"
            );
            ensure!(
                entry.compression() == zip::CompressionMethod::Stored,
                "Native library is compressed: {name}"
            );
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            verify_elf(&bytes).with_context(|| format!("Invalid native library {name}"))?;
        }
    }
    Ok(())
}

fn verify_elf(bytes: &[u8]) -> Result<()> {
    let number = |offset: usize, width: usize| -> Result<u64> {
        let end = offset.checked_add(width).context("ELF offset overflow")?;
        let data = bytes.get(offset..end).context("Truncated ELF header")?;
        let mut value = [0; 8];
        value[..width].copy_from_slice(data);
        Ok(u64::from_le_bytes(value))
    };
    ensure!(
        bytes.get(..6) == Some(b"\x7fELF\x02\x01"),
        "Expected a little-endian ELF64 library"
    );
    ensure!(number(18, 2)? == 183, "Expected ARM64 ELF machine");
    let offset = usize::try_from(number(32, 8)?)?;
    let size = usize::try_from(number(54, 2)?)?;
    ensure!(size >= 56, "Invalid ELF program header size");
    for i in 0..usize::try_from(number(56, 2)?)? {
        let header = offset
            .checked_add(i.checked_mul(size).context("ELF offset overflow")?)
            .context("ELF offset overflow")?;
        let kind = number(header, 4)?;
        let flags = number(header + 4, 4)?;
        if kind == 1 {
            ensure!(
                number(header + 48, 8)? >= 16384,
                "ELF LOAD segment is not aligned for 16 KB pages"
            );
            ensure!(
                flags & 3 != 3,
                "ELF LOAD segment is writable and executable"
            );
        }
        if kind == 0x6474e551 {
            ensure!(flags & 1 == 0, "ELF stack is executable");
        }
    }
    Ok(())
}
