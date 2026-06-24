use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Append a key-value pair to a GitHub Actions output file.
pub fn append_output(path: impl AsRef<Path>, key: &str, value: &str) -> Result<()> {
    let path = path.as_ref();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open GitHub output file {}", path.display()))?;
    writeln!(file, "{key}={value}")?;
    Ok(())
}

/// Append a key-value pair to `GITHUB_OUTPUT` when the variable is present.
pub fn append_output_env(key: &str, value: &str) -> Result<bool> {
    let Some(path) = std::env::var_os("GITHUB_OUTPUT") else {
        return Ok(false);
    };
    append_output(PathBuf::from(path), key, value)?;
    Ok(true)
}
