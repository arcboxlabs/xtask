use std::path::Path;

use anyhow::{Context, Result, bail};
use xshell::Shell;

/// Create an [`xshell::Shell`] with context suitable for CLI errors.
pub fn shell() -> Result<Shell> {
    Shell::new().context("create shell")
}

/// Return true when `program` exists somewhere on `PATH`.
pub fn command_exists(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| {
            let candidate = dir.join(program);
            candidate.is_file()
        })
    })
}

/// Fail with a user-facing message when `program` is not on `PATH`.
pub fn ensure_command(program: &str) -> Result<()> {
    if command_exists(program) {
        Ok(())
    } else {
        bail!("{program} not found on PATH")
    }
}

/// Temporarily change the shell's current directory for a command block.
pub fn push_dir<'a>(sh: &'a Shell, path: impl AsRef<Path>) -> xshell::PushDir<'a> {
    sh.push_dir(path)
}
