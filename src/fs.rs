use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// Ensure that `path` exists and is a regular file.
pub fn ensure_file(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if metadata.is_file() {
        Ok(())
    } else {
        bail!("missing file {}", path.display())
    }
}

/// Ensure that `path` exists and is a directory.
pub fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if metadata.is_dir() {
        Ok(())
    } else {
        bail!("missing directory {}", path.display())
    }
}

/// Create `path` and any missing parent directories.
pub fn create_dir_all(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    fs::create_dir_all(path).with_context(|| format!("create directory {}", path.display()))
}

/// Copy a file, creating the destination parent directory when needed.
pub fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    if let Some(parent) = to.parent() {
        create_dir_all(parent)?;
    }
    fs::copy(from, to).with_context(|| format!("copy {} -> {}", from.display(), to.display()))
}

/// Remove a file or directory tree if it exists.
pub fn remove_path(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("remove directory {}", path.display()))
    } else {
        fs::remove_file(path).with_context(|| format!("remove file {}", path.display()))
    }
}

/// Write UTF-8 text, creating the destination parent directory when needed.
pub fn write_string(path: impl AsRef<Path>, content: impl AsRef<str>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    fs::write(path, content.as_ref()).with_context(|| format!("write {}", path.display()))
}
