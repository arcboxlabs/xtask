use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Finder window configuration for `create-dmg`.
#[derive(Clone, Debug)]
pub struct DmgWindow {
    /// Window top-left position.
    pub position: (u32, u32),
    /// Window size in points.
    pub size: (u32, u32),
    /// Icon size in points.
    pub icon_size: u32,
    /// App icon position in the Finder window.
    pub app_icon_position: (u32, u32),
    /// Applications drop-link position in the Finder window.
    pub app_drop_link_position: (u32, u32),
}

impl Default for DmgWindow {
    fn default() -> Self {
        Self {
            position: (200, 120),
            size: (600, 400),
            icon_size: 100,
            app_icon_position: (150, 190),
            app_drop_link_position: (450, 190),
        }
    }
}

/// Options for creating a macOS DMG with `create-dmg`.
#[derive(Clone, Debug)]
pub struct CreateDmgOptions {
    /// Volume name shown in Finder.
    pub volume_name: String,
    /// `.app` bundle to place in the DMG.
    pub app: PathBuf,
    /// Output DMG path.
    pub output: PathBuf,
    /// Finder window layout.
    pub window: DmgWindow,
    /// Optional `create-dmg --format` value, such as `ULMO`.
    pub format: Option<String>,
    /// Optional background image.
    pub background: Option<PathBuf>,
    /// Hide the `.app` filename extension.
    pub hide_app_extension: bool,
    /// Disable legacy internet-enable metadata.
    pub no_internet_enable: bool,
}

impl CreateDmgOptions {
    /// Construct options with the default Finder layout.
    pub fn new(
        volume_name: impl Into<String>,
        app: impl Into<PathBuf>,
        output: impl Into<PathBuf>,
    ) -> Self {
        Self {
            volume_name: volume_name.into(),
            app: app.into(),
            output: output.into(),
            window: DmgWindow::default(),
            format: None,
            background: None,
            hide_app_extension: false,
            no_internet_enable: true,
        }
    }
}

/// Create a DMG with `create-dmg`.
pub fn create(options: &CreateDmgOptions) -> Result<()> {
    crate::process::ensure_command("create-dmg")?;
    if !options.app.is_dir() {
        bail!("app bundle not found at {}", options.app.display());
    }
    if let Some(parent) = options.output.parent() {
        crate::fs::create_dir_all(parent)?;
    }
    crate::fs::remove_path(&options.output)?;

    let app_name = options
        .app
        .file_name()
        .and_then(|name| name.to_str())
        .context("app path has no UTF-8 filename")?;
    let mut command = Command::new("create-dmg");
    if let Some(format) = &options.format {
        command.args(["--format", format]);
    }
    command.args(["--volname", &options.volume_name]);
    if let Some(background) = &options.background {
        command.arg("--background").arg(background);
    }
    command.args([
        "--window-pos",
        &options.window.position.0.to_string(),
        &options.window.position.1.to_string(),
        "--window-size",
        &options.window.size.0.to_string(),
        &options.window.size.1.to_string(),
        "--icon-size",
        &options.window.icon_size.to_string(),
        "--icon",
        app_name,
        &options.window.app_icon_position.0.to_string(),
        &options.window.app_icon_position.1.to_string(),
        "--app-drop-link",
        &options.window.app_drop_link_position.0.to_string(),
        &options.window.app_drop_link_position.1.to_string(),
    ]);
    if options.hide_app_extension {
        command.args(["--hide-extension", app_name]);
    }
    if options.no_internet_enable {
        command.arg("--no-internet-enable");
    }
    command.arg(&options.output).arg(&options.app);

    let status = command
        .status()
        .with_context(|| format!("create DMG {}", options.output.display()))?;
    if !status.success() && !options.output.is_file() {
        bail!("create-dmg failed with {status}");
    }
    if options.output.is_file() {
        Ok(())
    } else {
        bail!("DMG was not created at {}", options.output.display())
    }
}

/// Sign a DMG file with `/usr/bin/codesign`.
pub fn sign(identity: &str, dmg: impl AsRef<Path>) -> Result<()> {
    let dmg = dmg.as_ref();
    let options = crate::apple::CodesignOptions {
        identity,
        target: dmg,
        entitlements: None,
        identifier: None,
        hardened_runtime: false,
        timestamp: true,
    };
    crate::apple::codesign(&options)
}

/// Return a compact size-and-path summary for release logs.
pub fn file_summary(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    Ok(format!(
        "{}\t{}",
        crate::hash::human_size(metadata.len()),
        path.display()
    ))
}
