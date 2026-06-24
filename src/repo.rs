use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Return the repository root for an `xtask` crate that lives directly under it.
///
/// Pass `env!("CARGO_MANIFEST_DIR")` from the *calling* `xtask` crate:
///
/// ```no_run
/// let root = xtask::repo::root_from_xtask_manifest(env!("CARGO_MANIFEST_DIR"))?;
/// # anyhow::Ok(())
/// ```
///
/// The manifest directory is an argument because a library dependency cannot use
/// its own `env!("CARGO_MANIFEST_DIR")` to discover the caller's repository.
pub fn root_from_xtask_manifest(manifest_dir: impl AsRef<Path>) -> Result<PathBuf> {
    manifest_dir
        .as_ref()
        .parent()
        .map(Path::to_path_buf)
        .context("could not locate repository root from xtask manifest directory")
}

/// Join a repository-relative path to `root`.
pub fn join(root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(path)
}

/// Return `path` if absolute, otherwise join it to `root`.
pub fn absolutize(root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.as_ref().join(path)
    }
}
