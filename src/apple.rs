use std::ffi::OsStr;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use plist::Value;
use xshell::{Cmd, Shell, cmd};

const HOST_XCODE_ENV: &[&str] = &[
    "HOME",
    "TMPDIR",
    "USER",
    "LOGNAME",
    "SHELL",
    "SSH_AUTH_SOCK",
];

/// A resolved Xcode developer directory.
#[derive(Clone, Debug)]
pub struct Xcode {
    developer_dir: PathBuf,
}

impl Xcode {
    /// Resolve Xcode from `env_key`, falling back to the standard Xcode.app path.
    pub fn resolve(env_key: &str) -> Self {
        let developer_dir = std::env::var_os(env_key)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/Applications/Xcode.app/Contents/Developer"));
        Self { developer_dir }
    }

    /// Construct an [`Xcode`] from an explicit developer directory.
    pub fn new(developer_dir: impl Into<PathBuf>) -> Self {
        Self {
            developer_dir: developer_dir.into(),
        }
    }

    /// Return the developer directory.
    pub fn developer_dir(&self) -> &Path {
        &self.developer_dir
    }

    /// Return true when this Xcode installation contains an iOS platform SDK.
    pub fn has_ios_sdk(&self) -> bool {
        self.developer_dir
            .join("Platforms/iPhoneOS.platform")
            .exists()
    }

    /// Run `/usr/bin/xcrun --sdk <sdk> ...` with this `DEVELOPER_DIR` and return trimmed stdout.
    pub fn xcrun(&self, sh: &Shell, sdk: &str, args: &[&str]) -> Result<String> {
        let mut full = vec!["--sdk", sdk];
        full.extend_from_slice(args);
        Ok(cmd!(sh, "/usr/bin/xcrun {full...}")
            .env("DEVELOPER_DIR", self.developer_dir.as_os_str())
            .read()
            .with_context(|| format!("run xcrun for SDK {sdk}"))?
            .trim()
            .to_string())
    }

    /// Return the SDK path for `sdk`, for example `macosx` or `iphoneos`.
    pub fn sdk_path(&self, sh: &Shell, sdk: &str) -> Result<String> {
        self.xcrun(sh, sdk, &["--show-sdk-path"])
    }

    /// Build a command in a minimal host-Xcode environment.
    ///
    /// This is useful in nix/devenv shells where `SDKROOT`, `DEVELOPER_DIR`, and
    /// compiler wrapper variables may point SwiftPM or Xcode at an incompatible SDK.
    pub fn clean_cmd<'a>(&self, sh: &'a Shell, program: &str, args: &[&str]) -> Cmd<'a> {
        keep_host_env(sh.cmd(program).args(args).env_clear())
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("MACOSX_DEPLOYMENT_TARGET", "14.0")
            .env("DEVELOPER_DIR", self.developer_dir.as_os_str())
    }
}

fn keep_host_env(cmd: Cmd<'_>) -> Cmd<'_> {
    HOST_XCODE_ENV
        .iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (*key, value)))
        .fold(cmd, |cmd, (key, value)| cmd.env(key, value))
}

/// Options for hardened-runtime codesigning.
#[derive(Clone, Debug)]
pub struct CodesignOptions<'a> {
    /// Signing identity. Use `-` for ad-hoc signing.
    pub identity: &'a str,
    /// File or bundle to sign.
    pub target: &'a Path,
    /// Optional entitlements plist.
    pub entitlements: Option<&'a Path>,
    /// Optional explicit signing identifier.
    pub identifier: Option<&'a str>,
    /// Include `--options runtime`.
    pub hardened_runtime: bool,
    /// Include `--timestamp`. Ignored for ad-hoc signatures.
    pub timestamp: bool,
}

impl<'a> CodesignOptions<'a> {
    /// Create standard Developer ID-style options for `target` and `identity`.
    pub fn runtime(identity: &'a str, target: &'a Path) -> Self {
        Self {
            identity,
            target,
            entitlements: None,
            identifier: None,
            hardened_runtime: true,
            timestamp: true,
        }
    }
}

/// Sign a binary or bundle using `/usr/bin/codesign`.
pub fn codesign(options: &CodesignOptions<'_>) -> Result<()> {
    let mut command = Command::new("/usr/bin/codesign");
    command.arg("--force");
    if options.hardened_runtime {
        command.args(["--options", "runtime"]);
    }
    if let Some(identifier) = options.identifier {
        command.args(["--identifier", identifier]);
    }
    if let Some(entitlements) = options.entitlements {
        command.arg("--entitlements").arg(entitlements);
    }
    command.args(["--sign", options.identity]);
    if options.timestamp && options.identity != "-" {
        command.arg("--timestamp");
    }
    command.arg(options.target);

    let status = command
        .status()
        .with_context(|| format!("run codesign for {}", options.target.display()))?;
    if status.success() {
        Ok(())
    } else {
        bail!(
            "codesign failed for {} with {status}",
            options.target.display()
        )
    }
}

/// Verify a code signature with `codesign --verify --strict`.
pub fn verify_signature(target: impl AsRef<Path>) -> Result<()> {
    let target = target.as_ref();
    let status = Command::new("/usr/bin/codesign")
        .args([OsStr::new("--verify"), OsStr::new("--strict")])
        .arg(target)
        .status()
        .with_context(|| format!("verify signature for {}", target.display()))?;
    if status.success() {
        Ok(())
    } else {
        bail!(
            "codesign verification failed for {} with {status}",
            target.display()
        )
    }
}

/// Read signed entitlements as XML.
pub fn entitlements_xml(target: impl AsRef<Path>) -> Result<String> {
    let target = target.as_ref();
    let output = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", "-", "--xml"])
        .arg(target)
        .output()
        .with_context(|| format!("read entitlements for {}", target.display()))?;
    if !output.status.success() {
        bail!(
            "codesign could not read entitlements for {}",
            target.display()
        );
    }
    let mut xml = String::from_utf8_lossy(&output.stdout).into_owned();
    xml.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(xml)
}

/// Overwrite a top-level string key in a plist file.
pub fn set_plist_string(plist: impl AsRef<Path>, key: &str, value: &str) -> Result<()> {
    let plist = plist.as_ref();
    let mut root =
        Value::from_file(plist).with_context(|| format!("read plist {}", plist.display()))?;
    let dict = root
        .as_dictionary_mut()
        .with_context(|| format!("plist root is not a dictionary: {}", plist.display()))?;
    dict.insert(key.into(), Value::String(value.to_string()));
    root.to_file_xml(plist)
        .with_context(|| format!("write plist {}", plist.display()))
}

/// A decoded provisioning profile matching an application identifier.
#[derive(Clone, Debug)]
pub struct ProvisioningProfile {
    /// Profile file path.
    pub path: PathBuf,
    /// Human-readable profile name.
    pub name: String,
    /// `com.apple.application-identifier` entitlement.
    pub application_identifier: String,
    /// Whether the profile provisions all devices, typical for Developer ID profiles.
    pub provisions_all_devices: bool,
}

/// Find installed provisioning profiles whose application identifier equals `app_id`.
pub fn find_provisioning_profiles(app_id: &str) -> Vec<ProvisioningProfile> {
    profile_search_roots()
        .into_iter()
        .flat_map(|root| profile_files(&root))
        .filter_map(|path| read_provisioning_profile(&path).ok())
        .filter(|profile| profile.application_identifier == app_id)
        .collect()
}

/// Resolve a provisioning profile from an environment override or installed profiles.
///
/// When multiple installed profiles match, a Developer ID profile
/// (`ProvisionsAllDevices = true`) is preferred if it is unique.
pub fn resolve_provisioning_profile(
    env_key: &str,
    app_id: &str,
) -> Result<Option<ProvisioningProfile>> {
    if let Ok(path) = std::env::var(env_key) {
        let path = PathBuf::from(path);
        if !path.exists() {
            bail!("{env_key} does not exist: {}", path.display());
        }
        return read_provisioning_profile(&path).map(Some);
    }

    let profiles = find_provisioning_profiles(app_id);
    match profiles.as_slice() {
        [] => Ok(None),
        [profile] => Ok(Some(profile.clone())),
        many => {
            let developer_id = many
                .iter()
                .filter(|profile| profile.provisions_all_devices)
                .collect::<Vec<_>>();
            if let [profile] = developer_id.as_slice() {
                return Ok(Some((*profile).clone()));
            }

            let mut message = format!(
                "multiple installed provisioning profiles matched {app_id}; set {env_key} to choose one:"
            );
            for profile in many {
                message.push_str(&format!(
                    "\n  - {} ({}, ProvisionsAllDevices={})",
                    profile.path.display(),
                    profile.name,
                    profile.provisions_all_devices
                ));
            }
            bail!(message)
        }
    }
}

/// Decode one `.provisionprofile` or `.mobileprovision` file.
pub fn read_provisioning_profile(path: impl AsRef<Path>) -> Result<ProvisioningProfile> {
    let path = path.as_ref();
    let output = Command::new("/usr/bin/security")
        .args(["cms", "-D", "-i"])
        .arg(path)
        .output()
        .with_context(|| format!("decode provisioning profile {}", path.display()))?;
    if !output.status.success() {
        bail!("security cms failed for {}", path.display());
    }

    let plist = Value::from_reader_xml(Cursor::new(output.stdout))?;
    let root = plist
        .as_dictionary()
        .with_context(|| format!("profile root is not a dictionary: {}", path.display()))?;
    let entitlements = root
        .get("Entitlements")
        .and_then(Value::as_dictionary)
        .context("profile has no Entitlements dictionary")?;
    let application_identifier = entitlements
        .get("com.apple.application-identifier")
        .and_then(Value::as_string)
        .context("profile has no com.apple.application-identifier entitlement")?
        .to_string();

    Ok(ProvisioningProfile {
        path: path.to_path_buf(),
        name: root
            .get("Name")
            .and_then(Value::as_string)
            .unwrap_or("<unnamed>")
            .to_string(),
        application_identifier,
        provisions_all_devices: root
            .get("ProvisionsAllDevices")
            .and_then(Value::as_boolean)
            .unwrap_or(false),
    })
}

fn profile_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let standard = PathBuf::from(&home).join("Library/MobileDevice/Provisioning Profiles");
        if standard.exists() {
            roots.push(standard);
        } else {
            roots.push(PathBuf::from(home).join("Developer"));
        }
    }
    roots
}

fn profile_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|ext| ext.to_str());
            if matches!(ext, Some("provisionprofile" | "mobileprovision")) {
                files.push(path);
            }
        }
    }
    files
}

/// Submit an artifact to Apple notarization and staple it after acceptance.
pub fn notarize_and_staple(artifact: impl AsRef<Path>, keychain_profile: &str) -> Result<()> {
    let artifact = artifact.as_ref();
    let output = Command::new("/usr/bin/xcrun")
        .args([
            "notarytool",
            "submit",
            &artifact.to_string_lossy(),
            "--keychain-profile",
            keychain_profile,
            "--wait",
            "--timeout",
            "90m",
        ])
        .output()
        .with_context(|| format!("submit {} for notarization", artifact.display()))?;
    let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    print!("{log}");

    if !output.status.success() || !log.contains("status: Accepted") {
        bail!(
            "notarization did not reach Accepted for {}",
            artifact.display()
        );
    }

    let status = Command::new("/usr/bin/xcrun")
        .args(["stapler", "staple", &artifact.to_string_lossy()])
        .status()
        .with_context(|| format!("staple notarization ticket to {}", artifact.display()))?;
    if status.success() {
        Ok(())
    } else {
        bail!("stapler failed for {} with {status}", artifact.display())
    }
}
