use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Return the lowercase hexadecimal SHA-256 digest for `path`.
pub fn sha256_file(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let mut file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let mut digest = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(&mut digest, "{byte:02x}")?;
    }
    Ok(digest)
}

/// Format a byte count for compact release logs.
pub fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}B")
    } else {
        format!("{size:.1}{}", UNITS[unit])
    }
}

/// Write a `*.sha256` sidecar next to `path` and return the sidecar path.
pub fn write_sha256_sidecar(path: impl AsRef<Path>) -> Result<std::path::PathBuf> {
    let path = path.as_ref();
    let digest = sha256_file(path)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("path has no UTF-8 filename: {}", path.display()))?;
    let sidecar = std::path::PathBuf::from(format!("{}.sha256", path.display()));
    crate::fs::write_string(&sidecar, format!("{digest}  {file_name}\n"))?;
    Ok(sidecar)
}
